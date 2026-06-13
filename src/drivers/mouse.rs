use core::arch::asm;
use crate::sync::Spinlock;

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
        let status = inb(0x64);
        // Bit 0: Output buffer full. Bit 5: Data is from the mouse (Auxiliary device)
        if (status & 0x01 != 0) && (status & 0x20 != 0) {
            let data = inb(0x60);
            match MOUSE_CYCLE {
                0 => {
                    MOUSE_BYTE[0] = data;
                    if data & 0x08 != 0 { MOUSE_CYCLE = 1; }
                }
                1 => {
                    MOUSE_BYTE[1] = data;
                    MOUSE_CYCLE = 2;
                }
                2 => {
                    MOUSE_BYTE[2] = data;
                    // Process packet
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

pub unsafe fn mouse_wait(command: u8) {
    let mut timeout = 100000;
    if command == 0 {
        // Wait for data to be ready to read
        while (inb(0x64) & 0x01 == 0) && timeout > 0 { timeout -= 1; }
    } else {
        // Wait for controller to be ready to receive command
        while (inb(0x64) & 0x02 != 0) && timeout > 0 { timeout -= 1; }
    }
}

pub unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("edx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

pub unsafe fn inb(port: u16) -> u8 {
    let res: u8;
    asm!("in al, dx", out("al") res, in("edx") port, options(nomem, nostack, preserves_flags));
    res
}

pub fn init_mouse() {
    unsafe {
        mouse_wait(1);
        outb(0x64, 0xA8);
        mouse_wait(1);
        outb(0x64, 0xD4);
        mouse_wait(1);
        outb(0x60, 0xF4);
        mouse_wait(0);
        let _ack = inb(0x60); // Consume ACK (0xFA)
    }
}