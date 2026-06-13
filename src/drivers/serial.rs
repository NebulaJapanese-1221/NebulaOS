use crate::mouse::{outb, inb};
use crate::sync::Spinlock;
use core::fmt;

pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn init(&self) {
        unsafe {
            outb(self.port + 1, 0x00);    // Disable all interrupts
            outb(self.port + 3, 0x80);    // Enable DLAB (set baud rate divisor)
            outb(self.port + 0, 0x03);    // Set divisor to 3 (38400 baud)
            outb(self.port + 1, 0x00);
            outb(self.port + 3, 0x03);    // 8 bits, no parity, one stop bit
            outb(self.port + 2, 0xC7);    // Enable FIFO, clear them, with 14-byte threshold
            outb(self.port + 4, 0x0B);    // IRQs enabled, RTS/DSR set
        }
    }

    fn is_transmit_empty(&self) -> bool {
        unsafe { inb(self.port + 5) & 0x20 != 0 }
    }

    pub fn send(&mut self, data: u8) {
        while !self.is_transmit_empty() {}
        unsafe {
            outb(self.port, data);
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

pub static SERIAL_PORT: Spinlock<SerialPort> = Spinlock::new(SerialPort::new(0x3F8));

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let _ = write!($crate::serial::SERIAL_PORT.lock(), $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}