use super::ps2;
use crate::kernel::interrupts::InterruptStackFrame;
use crate::kernel::io;
use spin::Mutex;
use core::arch::asm;

#[derive(Debug, Clone, Copy)]
pub struct MousePacket {
    pub x: i16,
    pub y: i16,
    pub buttons: u8,
    pub wheel: i8,
}

const BUFFER_SIZE: usize = 256;
pub struct MouseBuffer {
    packets: [MousePacket; BUFFER_SIZE],
    head: usize,
    tail: usize,
}

impl MouseBuffer {
    pub const fn new() -> Self {
        Self {
            packets: [MousePacket { x: 0, y: 0, buttons: 0, wheel: 0 }; BUFFER_SIZE],
            head: 0,
            tail: 0,
        }
    }

    pub fn push(&mut self, packet: MousePacket) {
        let next = (self.head + 1) % BUFFER_SIZE;
        if next != self.tail {
            self.packets[self.head] = packet;
            self.head = next;
        }
    }
}

pub static MOUSE_BUFFER: Mutex<MouseBuffer> = Mutex::new(MouseBuffer::new());

// State for the interrupt handler to track packet assembly
static mut BYTE_CYCLE: u8 = 0;
static mut BYTES: [u8; 4] = [0; 4];
static mut MOUSE_ID: u8 = 0;

pub fn initialize() {
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
        if !ps2::write_mouse_command(0xFF) {
            crate::serial_println!("[MOUSE] Reset command failed (no ACK)");
        } else {
            // After a reset, the mouse sends an ACK (0xFA), then a self-test result (0xAA), and an ID (0x00).
            // The write_mouse_command function consumes the ACK. We must consume the other two bytes.
            if ps2::wait_output_avail() {
                let _ = ps2::read_data(); // Consume self-test result
            } else {
                crate::serial_println!("[MOUSE] No self-test result after reset");
            }
            if ps2::wait_output_avail() {
                let _ = ps2::read_data(); // Consume mouse ID
            } else {
                crate::serial_println!("[MOUSE] No mouse ID after reset");
            }
        }

        // Set Defaults (Note: this often disables extensions, so we do it before the magic sequence)
        if !ps2::write_mouse_command(0xF6) { crate::serial_println!("[MOUSE] Set Defaults failed"); }

        // Set Scaling 1:1 (Command 0xE6) to ensure predictable relative movement
        if !ps2::write_mouse_command(0xE6) { crate::serial_println!("[MOUSE] Set Scaling 1:1 failed"); }

        // Set Resolution (Command 0xE8, Data 0x03 = 8 counts/mm) for higher precision
        if !ps2::write_mouse_command(0xE8) { crate::serial_println!("[MOUSE] Set Resolution command failed"); }
        if !ps2::write_mouse_command(0x03) { crate::serial_println!("[MOUSE] Set Resolution data failed"); }

        // Enable Intellimouse Extensions (Magic Sequence: 200, 100, 80)
        if !ps2::write_mouse_command(0xF3) { crate::serial_println!("[MOUSE] Set Sample Rate (200) command failed"); }
        if !ps2::write_mouse_command(200) { crate::serial_println!("[MOUSE] Set Sample Rate (200) data failed"); }
        if !ps2::write_mouse_command(0xF3) { crate::serial_println!("[MOUSE] Set Sample Rate (100) command failed"); }
        if !ps2::write_mouse_command(100) { crate::serial_println!("[MOUSE] Set Sample Rate (100) data failed"); }
        if !ps2::write_mouse_command(0xF3) { crate::serial_println!("[MOUSE] Set Sample Rate (80) command failed"); }
        if !ps2::write_mouse_command(80) { crate::serial_println!("[MOUSE] Set Sample Rate (80) data failed"); }

        // Get Device ID to verify extension is enabled (should be 3 or 4)
        if !ps2::write_mouse_command(0xF2) { crate::serial_println!("[MOUSE] Get Device ID command failed"); }
        if ps2::wait_output_avail() {
            let id = ps2::read_data();
            MOUSE_ID = id;
            crate::serial_println!("[MOUSE] Device ID: {:#x}", id);
        } else {
            crate::serial_println!("[MOUSE] No Device ID received");
        }

        // Set final sample rate to 100Hz for smoother tracking
        if !ps2::write_mouse_command(0xF3) { crate::serial_println!("[MOUSE] Set Sample Rate (100Hz) command failed"); }
        if !ps2::write_mouse_command(100) { crate::serial_println!("[MOUSE] Set Sample Rate (100Hz) data failed"); }

        if !ps2::write_mouse_command(0xF4) { crate::serial_println!("[MOUSE] Enable Scanning failed"); }

        // 5. Enable the devices
        ps2::write_command(0xAE); // Enable keyboard
        ps2::write_command(0xA8); // Enable mouse
    }
    crate::serial_println!("[MOUSE] PS/2 mouse initialization complete.");
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
                        if MOUSE_ID == 3 || MOUSE_ID == 4 {
                            BYTE_CYCLE = 3;
                        } else {
                            let mut x = BYTES[1] as i16;
                            let mut y = BYTES[2] as i16;
                            
                            // Handle PS/2 9-bit signed values by checking sign bits in Byte 0
                            if (BYTES[0] & 0x10) != 0 { x |= 0xFF00u16 as i16; }
                            if (BYTES[0] & 0x20) != 0 { y |= 0xFF00u16 as i16; }

                            // Packet complete, push to buffer
                            MOUSE_BUFFER.lock().push(MousePacket {
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

                        let wheel = if MOUSE_ID == 3 {
                            byte as i8 // ID 3: standard signed byte
                        } else if MOUSE_ID == 4 {
                            let val = byte & 0x0F; // ID 4: lower 4 bits are wheel
                            if (val & 0x08) != 0 { (val | 0xF0) as i8 } else { val as i8 }
                        } else { 0 };

                        MOUSE_BUFFER.lock().push(MousePacket {
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
    // Disable interrupts to prevent deadlock with mouse interrupt handler
    unsafe { asm!("cli", options(nomem, nostack)); }
    let mut buffer = MOUSE_BUFFER.lock();
    let packet = if buffer.head == buffer.tail {
        None
    } else {
        let packet = buffer.packets[buffer.tail];
        buffer.tail = (buffer.tail + 1) % BUFFER_SIZE;
        Some(packet)
    };
    drop(buffer);
    unsafe { asm!("sti", options(nomem, nostack)); }
    packet
}
