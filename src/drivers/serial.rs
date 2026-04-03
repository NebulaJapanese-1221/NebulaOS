use crate::kernel::io;
use core::fmt;
use spin::Mutex;

const PORT: u16 = 0x3F8; // COM1

pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn init(&mut self) {
        unsafe {
            io::outb(self.port + 1, 0x00); // Disable all interrupts
            io::outb(self.port + 3, 0x80); // Enable DLAB (set baud rate divisor)
            io::outb(self.port + 0, 0x03); // Set divisor to 3 (lo byte) 38400 baud
            io::outb(self.port + 1, 0x00); //                  (hi byte)
            io::outb(self.port + 3, 0x03); // 8 bits, no parity, one stop bit
            io::outb(self.port + 2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
            io::outb(self.port + 4, 0x0B); // IRQs enabled, RTS/DSR set
        }
    }

    pub fn send(&mut self, data: u8) {
        unsafe {
            let mut timeout = 100000;
            // Wait for transmit buffer to be empty
            while (io::inb(self.port + 5) & 0x20) == 0 {
                timeout -= 1;
                if timeout == 0 { return; } // Give up if it takes too long
            }
            io::outb(self.port, data);
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}

pub static SERIAL1: Mutex<SerialPort> = Mutex::new(SerialPort::new(PORT));

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        $crate::drivers::serial::SERIAL1.lock().write_fmt(format_args!($($arg)*)).unwrap();
    });
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}