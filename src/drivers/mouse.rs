use crate::sync::Spinlock;
use crate::ps2::{inb, outb, read_config, write_config, write_mouse, flush_buffers, PS2_STATUS_PORT, PS2_DATA_PORT, PS2_COMMAND_PORT};

pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left_button: bool,
    pub right_button: bool,
}

impl MouseState {
    pub const fn new() -> Self {
        Self { x: 0, y: 0, left_button: false, right_button: false }
    }
}

pub static MOUSE_STATE: Spinlock<MouseState> = Spinlock::new(MouseState::new());
static mut MOUSE_CYCLE: u8 = 0;
static mut MOUSE_BYTE: [u8; 3] = [0; 3];

pub fn handle_mouse_interrupt() {
    unsafe {
        let status = inb(PS2_STATUS_PORT);
        // Bit 0: Output buffer full. Bit 5: Data is from the mouse (Auxiliary device)
        if (status & 0x01 != 0) && (status & 0x20 != 0) {
            let data = inb(PS2_DATA_PORT);
            match MOUSE_CYCLE {
                0 => {
                    // Bit 3: Always 1 for standard 3-byte mouse packets
                    if data & 0x08 != 0 {
                        MOUSE_BYTE[0] = data;
                        MOUSE_CYCLE = 1;
                    } else {
                        // Unexpected packet start, reset cycle
                        MOUSE_CYCLE = 0;
                    }
                }
                1 => {
                    MOUSE_BYTE[1] = data;
                    MOUSE_CYCLE = 2;
                }
                2 => {
                    MOUSE_BYTE[2] = data;
                    
                    // Packet Processing:
                    // Byte 0: [Y-overflow][X-overflow][Y-sign][X-sign][Middle][Right][Left][Always 1]
                    // Byte 1: X movement
                    // Byte 2: Y movement
                    
                    let x_offset = if MOUSE_BYTE[0] & 0x10 != 0 { (MOUSE_BYTE[1] as i32) - 256 } else { MOUSE_BYTE[1] as i32 };
                    let y_offset = if MOUSE_BYTE[0] & 0x20 != 0 { (MOUSE_BYTE[2] as i32) - 256 } else { MOUSE_BYTE[2] as i32 };

                    let mut mouse = MOUSE_STATE.lock();
                    mouse.x += x_offset;
                    mouse.y -= y_offset; // Y is inverted in PS/2
                    mouse.left_button = (MOUSE_BYTE[0] & 0x01) != 0;
                    mouse.right_button = (MOUSE_BYTE[0] & 0x02) != 0;

                    MOUSE_CYCLE = 0;
                }
                _ => MOUSE_CYCLE = 0,
            }
        }
    }
}

pub fn init_mouse() {
    unsafe {
        // 1. Flush any leftover data in the PS/2 controller buffers
        flush_buffers();

        // 2. Enable the auxiliary device (mouse port)
        // 0xA8: Enable auxiliary device
        if crate::ps2::wait_write() {
            outb(PS2_COMMAND_PORT, 0xA8);
        }

        // 3. Read, modify, and write Controller Configuration Byte
        let mut status = read_config();
        status |= 0x02;     // Enable mouse interrupt
        status &= !0x20;    // Enable mouse clock (clear disable bit)
        write_config(status);

        // 4. Set defaults on mouse
        // 0xF4: Enable data reporting (streaming)
        // 0xF6: Set defaults
        // We do these specifically in order.
        let _ = write_mouse(0xF6); // Set defaults
        let _ = write_mouse(0xF4); // Enable data reporting

        // 6. Final flush of any leftover bytes
        flush_buffers();
    }
}