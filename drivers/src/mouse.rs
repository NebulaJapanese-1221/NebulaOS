use crate::common::io;
use crate::common::idt;

static mut MOUSE_STATE_X: i32 = 0;
static mut MOUSE_STATE_Y: i32 = 0;
static mut MOUSE_STATE_BUTTONS: u8 = 0;
static mut MOUSE_CALLBACK: Option<unsafe extern "C" fn(i32, i32, u8, i32)> = None;

#[no_mangle]
pub unsafe extern "C" fn mouse_init() {
    MOUSE_STATE_X = 0;
    MOUSE_STATE_Y = 0;
    MOUSE_STATE_BUTTONS = 0;
    io::outb(0x64, 0xA8);
    io::outb(0x64, 0xD4);
    io::outb(0x60, 0xFF);
    io::outb(0x64, 0xD4);
    io::outb(0x60, 0xF6);
    io::outb(0x64, 0xD4);
    io::outb(0x60, 0xF4);
    io::outb(0x64, 0xD4);
    io::outb(0x60, 0x64);
    io::outb(0x64, 0xD4);
    io::outb(0x60, 0x00);
    idt::register_interrupt_handler(44, Some(mouse_irq_handler));
}

unsafe fn mouse_send_command(cmd: u8) -> bool {
    io::outb(0x64, 0xD4);
    io::outb(0x60, cmd);
    io::inb(0x60) == 0xFA
}

pub unsafe extern "C" fn mouse_irq_handler(_regs: *mut idt::Registers) {
    let data = io::inb(0x60);
    if data & 0x08 == 0 {
        io::outb(0x20, 0x20);
        io::outb(0xA0, 0x20);
        return;
    }
    let buttons = data & 0x07;
    MOUSE_STATE_BUTTONS = buttons;
    if let Some(callback) = MOUSE_CALLBACK {
        callback(MOUSE_STATE_X, MOUSE_STATE_Y, MOUSE_STATE_BUTTONS, 0);
    }
    io::outb(0x20, 0x20);
    io::outb(0xA0, 0x20);
}

pub unsafe fn mouse_set_handler(handler: Option<unsafe extern "C" fn(i32, i32, u8, i32)>) {
    MOUSE_CALLBACK = handler;
}

pub unsafe fn mouse_get_state_x() -> i32 { MOUSE_STATE_X }
pub unsafe fn mouse_get_state_y() -> i32 { MOUSE_STATE_Y }
pub unsafe fn mouse_get_buttons() -> u8 { MOUSE_STATE_BUTTONS }
