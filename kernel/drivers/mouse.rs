// Mouse driver

const MOUSE_DATA_PORT: u16 = 0x60;
const MOUSE_STATUS_PORT: u16 = 0x64;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn port_byte_in(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port);
    }
    val
}

#[derive(Clone, Copy)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

static mut MOUSE: MouseState = MouseState {
    x: 320,
    y: 240,
    left: false,
    right: false,
    middle: false,
};

pub fn init() {
    unsafe {
        port_byte_out(0x64, 0xD4);
        port_byte_out(0x60, 0xF4);
    }
}

pub fn mouse_state() -> MouseState {
    unsafe { MOUSE }
}

pub fn poll() {
    unsafe {
        let status = port_byte_in(0x64);
        if status & 0x01 != 0 {
            let _byte = port_byte_in(0x60);
        }
    }
}
