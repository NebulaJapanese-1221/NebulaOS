const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
const COMMAND_PORT: u16 = 0x64;
const PACKET_SIZE: usize = 3;

#[derive(Clone, Copy)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

static mut MOUSE: MouseState = MouseState {
    x: 0,
    y: 0,
    left: false,
    right: false,
    middle: false,
};
static mut PACKET: [u8; PACKET_SIZE] = [0; PACKET_SIZE];
static mut PACKET_INDEX: usize = 0;
static mut SENSITIVITY: i32 = 2;

pub fn set_sensitivity(value: i32) {
    unsafe { SENSITIVITY = value.clamp(1, 4); }
}

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn port_byte_in(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") value, in("dx") port);
    }
    value
}

fn wait_write() -> bool {
    for _ in 0..100_000 {
        if port_byte_in(STATUS_PORT) & 0x02 == 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

fn wait_read() -> bool {
    for _ in 0..100_000 {
        if port_byte_in(STATUS_PORT) & 0x01 != 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

pub fn init() -> bool {
    if !wait_write() {
        return false;
    }
    port_byte_out(COMMAND_PORT, 0xA8);
    if !wait_write() {
        return false;
    }
    port_byte_out(COMMAND_PORT, 0xD4);
    if !wait_write() {
        return false;
    }
    port_byte_out(DATA_PORT, 0xF4);
    if !wait_read() {
        return false;
    }
    let enabled = port_byte_in(DATA_PORT) == 0xFA;
    if enabled {
        unsafe {
            MOUSE.x = crate::drivers::framebuffer::get_width() as i32 / 2;
            MOUSE.y = crate::drivers::framebuffer::get_height() as i32 / 2;
        }
    }
    enabled
}

pub fn poll() -> Option<MouseState> {
    unsafe {
        let status = port_byte_in(STATUS_PORT);
        if status & 0x21 != 0x21 {
            return None;
        }

        let byte = port_byte_in(DATA_PORT);
        if PACKET_INDEX == 0 && byte & 0x08 == 0 {
            return None;
        }
        PACKET[PACKET_INDEX] = byte;
        PACKET_INDEX += 1;
        if PACKET_INDEX < PACKET_SIZE {
            return None;
        }
        PACKET_INDEX = 0;

        let fb_width = crate::drivers::framebuffer::get_width() as i32;
        let fb_height = crate::drivers::framebuffer::get_height() as i32;
        let sensitivity = SENSITIVITY;
        let dx = PACKET[1] as i8 as i32 * sensitivity / 2;
        let dy = PACKET[2] as i8 as i32 * sensitivity / 2;
        MOUSE.x = (MOUSE.x + dx).clamp(0, fb_width.saturating_sub(1));
        MOUSE.y = (MOUSE.y - dy).clamp(0, fb_height.saturating_sub(1));
        MOUSE.left = PACKET[0] & 0x01 != 0;
        MOUSE.right = PACKET[0] & 0x02 != 0;
        MOUSE.middle = PACKET[0] & 0x04 != 0;
        Some(MOUSE)
    }
}
