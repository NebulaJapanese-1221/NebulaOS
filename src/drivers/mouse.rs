use super::ps2;
use crate::kernel::interrupts::InterruptStackFrame;
use crate::kernel::io;
use core::sync::atomic::{AtomicUsize, AtomicU8, AtomicBool, Ordering};

#[derive(Debug, Clone, Copy)]
pub struct MousePacket {
    pub x: i16,
    pub y: i16,
    pub buttons: u8,
    pub wheel: i8,
}

const BUFFER_SIZE: usize = 256;
pub struct MouseBuffer {
    packets: [u64; BUFFER_SIZE],
    ready: [AtomicBool; BUFFER_SIZE],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl MouseBuffer {
    pub const fn new() -> Self {
        const DEFAULT_READY: AtomicBool = AtomicBool::new(false);
        Self {
            packets: [0; BUFFER_SIZE],
            ready: [DEFAULT_READY; BUFFER_SIZE],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, packet: MousePacket) {
        // Pack 6 bytes into u64: buttons(8), wheel(8), x(16), y(16)
        let val = (packet.buttons as u64) | 
                  ((packet.wheel as u8 as u64) << 8) | 
                  ((packet.x as u16 as u64) << 16) | 
                  ((packet.y as u16 as u64) << 32);

        loop {
            let head = self.head.load(Ordering::Relaxed);
            let tail = self.tail.load(Ordering::Acquire);
            let next = (head + 1) % BUFFER_SIZE;

            if next == tail { return; } // Buffer full, drop packet

            if self.head.compare_exchange_weak(head, next, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                // SAFETY: We have atomically reserved this slot via compare_exchange on 'head'.
                // No other producer can write to this index until the consumer processes it.
                unsafe {
                    let packets_ptr = self.packets.as_ptr() as *mut u64;
                    core::ptr::write_volatile(packets_ptr.add(head), val);
                }
                self.ready[head].store(true, Ordering::Release);
                break;
            }
            core::hint::spin_loop();
        }
    }

    pub fn pop(&self) -> Option<MousePacket> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head { return None; }

        if self.ready[tail].load(Ordering::Acquire) {
            // SAFETY: The 'ready' flag with Acquire ordering ensures that the producer 
            // has finished writing to this slot.
            let val = unsafe {
                core::ptr::read_volatile(self.packets.as_ptr().add(tail))
            };
            self.ready[tail].store(false, Ordering::Release);
            self.tail.store((tail + 1) % BUFFER_SIZE, Ordering::Release);

            Some(MousePacket {
                buttons: val as u8,
                wheel: (val >> 8) as i8,
                x: (val >> 16) as i16,
                y: (val >> 32) as i16,
            })
        } else {
            None
        }
    }
}

pub static MOUSE_BUFFER: MouseBuffer = MouseBuffer::new();

// State for the interrupt handler to track packet assembly
static mut BYTE_CYCLE: u8 = 0;
static mut BYTES: [u8; 4] = [0; 4];
static MOUSE_ID: AtomicU8 = AtomicU8::new(0); // Use AtomicU8 for thread-safe access

pub fn initialize() -> Result<(), &'static str> {
    crate::serial_println!("[MOUSE] Initializing PS/2 mouse...");

    unsafe {
        // 1. Disable PS/2 devices to prevent them from sending data during setup
        ps2::write_command(0xAD); // Disable keyboard
        ps2::write_command(0xA7); // Disable mouse

        // 2. Flush the output buffer to discard any garbage bytes from the BIOS
        while (ps2::read_status() & ps2::STATUS_OUTPUT_BUFFER) != 0 {
            ps2::read_data();
        }

        // 3. Set the controller configuration byte
        ps2::write_command(0x20); // Get Config Byte
        let mut status = ps2::read_data();
        status |= 0x02; // Enable IRQ12 (Mouse)
        status &= !0x20; // Enable Mouse Clock (Clear disable bit)
        ps2::write_command(0x60); // Set Config Byte
        ps2::write_data(status);

        // 4. Send commands to the mouse itself
        // Reset mouse first to ensure clean state
        ps2::send_device_command(0xFF, true)?;
        if ps2::wait_and_read()? != 0xAA { return Err("Mouse reset: Self-test failed"); }
        if ps2::wait_and_read()? != 0x00 { return Err("Mouse reset: Unexpected ID"); }

        // Set Defaults (Note: this often disables extensions, so we do it before the magic sequence)
        ps2::send_device_command(0xF6, true)?;

        // Set Scaling 1:1 (Command 0xE6) to ensure predictable relative movement
        ps2::send_device_command(0xE6, true)?;

        // Set Resolution (Command 0xE8, Data 0x03 = 8 counts/mm) for higher precision
        ps2::send_device_command(0xE8, true)?;
        ps2::send_device_command(0x03, true)?;

        // Enable Intellimouse Extensions (Magic Sequence: 200, 100, 80)
        for rate in [200, 100, 80] {
            ps2::send_device_command(0xF3, true)?;
            ps2::send_device_command(rate, true)?;
        }

        // Get Device ID to verify extension is enabled (should be 3 or 4)
        ps2::send_device_command(0xF2, true)?;
        let id = ps2::wait_and_read()?;
        MOUSE_ID.store(id, Ordering::Relaxed);
        crate::serial_println!("[MOUSE] Device ID: {:#x}", id);

        // Set final sample rate to 100Hz for smoother tracking
        ps2::send_device_command(0xF3, true)?;
        ps2::send_device_command(100, true)?;

        ps2::send_device_command(0xF4, true)?; // Enable Scanning

        // 5. Enable the devices
        ps2::write_command(0xAE); // Enable keyboard
        ps2::write_command(0xA8); // Enable mouse
    }
    crate::serial_println!("[MOUSE] PS/2 mouse initialization complete.");
    Ok(())
}

pub fn handle_interrupt() {
    loop {
        let status = unsafe { ps2::read_status() };
        if (status & ps2::STATUS_OUTPUT_BUFFER) == 0 { break; }
        let byte = unsafe { ps2::read_data() };

        // Only process if it IS from the mouse (Bit 5 set)
        if (status & ps2::STATUS_MOUSE_DATA) != 0 {
            unsafe {
                // Process the byte immediately to form a packet
                match BYTE_CYCLE {
                    0 => {
                        // Bit 3 of byte 0 must be 1 for a valid packet
                        if (byte & 0x08) != 0 {
                            BYTES[0] = byte;
                            BYTE_CYCLE = 1;
                        }
                    }
                    1 => {
                        BYTES[1] = byte;
                        BYTE_CYCLE = 2;
                    }
                    2 => {
                        BYTES[2] = byte;
                        BYTE_CYCLE = 0;

                        // If Intellimouse (ID 3 or 4), expect a 4th byte
                        if MOUSE_ID.load(Ordering::Relaxed) == 3 || MOUSE_ID.load(Ordering::Relaxed) == 4 {
                            BYTE_CYCLE = 3;
                        } else {
                            let mut x = BYTES[1] as i16;
                            let mut y = BYTES[2] as i16;
                            
                            // Handle PS/2 9-bit signed values by checking sign bits in Byte 0
                            if (BYTES[0] & 0x10) != 0 { x |= 0xFF00u16 as i16; }
                            if (BYTES[0] & 0x20) != 0 { y |= 0xFF00u16 as i16; }

                            // Packet complete, push to buffer
                            MOUSE_BUFFER.push(MousePacket {
                                buttons: BYTES[0] & 0x07,
                                x,
                                y,
                                wheel: 0,
                            });
                        }
                    }
                    3 => {
                        BYTES[3] = byte;
                        BYTE_CYCLE = 0;

                        let mut x = BYTES[1] as i16;
                        let mut y = BYTES[2] as i16;
                        if (BYTES[0] & 0x10) != 0 { x |= 0xFF00u16 as i16; }
                        if (BYTES[0] & 0x20) != 0 { y |= 0xFF00u16 as i16; }

                        let mouse_id = MOUSE_ID.load(Ordering::Relaxed);
                        let wheel = if mouse_id == 3 {
                            byte as i8 // ID 3: standard signed byte
                        } else if mouse_id == 4 {
                            let val = byte & 0x0F; // ID 4: lower 4 bits are wheel
                            if (val & 0x08) != 0 { (val | 0xF0) as i8 } else { val as i8 }
                        } else { 0 };

                        MOUSE_BUFFER.push(MousePacket {
                            buttons: BYTES[0] & 0x07,
                            x,
                            y,
                            wheel,
                        });
                    }
                    _ => {
                        BYTE_CYCLE = 0;
                    }
                }
            }
        }
    }
}

pub extern "x86-interrupt" fn interrupt_handler(_frame: &mut InterruptStackFrame) {
    handle_interrupt();
    unsafe {
        io::outb(0xA0, 0x20); // EOI for slave PIC
        io::outb(0x20, 0x20); // EOI for master PIC
    }
}

pub fn get_packet() -> Option<MousePacket> {
    MOUSE_BUFFER.pop()
}
