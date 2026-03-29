use crate::kernel::io;
use crate::kernel::process::IrqSafeMutex;
use core::fmt;
use core::sync::atomic::{AtomicUsize, AtomicU8, AtomicBool, Ordering};

/// Standard 16550A UART implementation for serial communication.
pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    pub const fn new(base: u16) -> Self {
        Self { base }
    }

    /// Initializes the serial port (38400 baud, 8N1).
    pub fn init(&self) {
        unsafe {
            io::outb(self.base + 1, 0x00);    // Disable all interrupts
            io::outb(self.base + 3, 0x80);    // Enable DLAB (set baud rate divisor)
            io::outb(self.base + 0, 0x03);    // Set divisor to 3 (38400 baud) lo byte
            io::outb(self.base + 1, 0x00);    //                  hi byte
            io::outb(self.base + 3, 0x03);    // 8 bits, no parity, one stop bit
            io::outb(self.base + 2, 0xC7);    // Enable FIFO, clear them, with 14-byte threshold
            io::outb(self.base + 4, 0x0B);    // IRQs enabled, RTS/DSR set
        }
    }

    fn is_transmit_empty(&self) -> bool {
        unsafe { (io::inb(self.base + 5) & 0x20) != 0 }
    }

    pub fn write_byte(&self, byte: u8) {
        // Wait for the transmit buffer to be empty
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }
        unsafe {
            io::outb(self.base, byte);
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

const SERIAL_BUF_SIZE: usize = 16384;

pub struct SerialQueue {
    buffer: [AtomicU8; SERIAL_BUF_SIZE],
    ready: [AtomicBool; SERIAL_BUF_SIZE],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl SerialQueue {
    pub const fn new() -> Self {
        const DEFAULT_U8: AtomicU8 = AtomicU8::new(0);
        const DEFAULT_BOOL: AtomicBool = AtomicBool::new(false);
        Self {
            buffer: [DEFAULT_U8; SERIAL_BUF_SIZE],
            ready: [DEFAULT_BOOL; SERIAL_BUF_SIZE],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn push_byte(&self, byte: u8) {
        loop {
            let head = self.head.load(Ordering::Relaxed);
            let tail = self.tail.load(Ordering::Acquire);
            let next = (head + 1) % SERIAL_BUF_SIZE;

            if next == tail { return; } // Buffer full, drop byte

            // Attempt to reserve the slot at 'head'
            if self.head.compare_exchange_weak(head, next, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                self.buffer[head].store(byte, Ordering::Relaxed);
                self.ready[head].store(true, Ordering::Release);
                break;
            }
            core::hint::spin_loop();
        }
    }

    pub fn pop_byte(&self) -> Option<u8> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            None
        } else if self.ready[tail].load(Ordering::Acquire) {
            let byte = self.buffer[tail].load(Ordering::Relaxed);
            self.ready[tail].store(false, Ordering::Release);
            self.tail.store((tail + 1) % SERIAL_BUF_SIZE, Ordering::Release);
            Some(byte)
        } else {
            None // Producer hasn't finished writing yet
        }
    }
}

/// The global instance for COM1, protected by an IrqSafeMutex.
/// This ensures that interrupts are disabled during logging, preventing deadlocks.
pub static SERIAL1: IrqSafeMutex<SerialPort> = IrqSafeMutex::new(SerialPort::new(0x3F8));

/// Flag indicating if the background serial output task is running.
pub static SERIAL_TASK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// The asynchronous output buffer for serial logging.
pub static SERIAL_OUT_QUEUE: SerialQueue = SerialQueue::new();

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    if SERIAL_TASK_ACTIVE.load(Ordering::Relaxed) {
        struct AtomicWriteProxy;
        impl Write for AtomicWriteProxy {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                for b in s.bytes() {
                    SERIAL_OUT_QUEUE.push_byte(b);
                }
                Ok(())
            }
        }
        let _ = AtomicWriteProxy.write_fmt(args);
    } else {
        // Fallback to direct hardware write if the background task is not active
        let _ = SERIAL1.lock().write_fmt(args);
    }
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::drivers::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}