use crate::common::io;
use crate::common::idt;

static mut MOUSE_STATE_X: i32 = 0;
static mut MOUSE_STATE_Y: i32 = 0;
static mut MOUSE_STATE_BUTTONS: u8 = 0;
static mut MOUSE_CALLBACK: Option<unsafe extern "C" fn(i32, i32, u8, i32)> = None;
static mut PACKET: [u8; 3] = [0; 3];
static mut PACKET_INDEX: u8 = 0;

#[no_mangle]
pub unsafe extern "C" fn mouse_init() {
    MOUSE_STATE_X = 0;
    MOUSE_STATE_Y = 0;
    MOUSE_STATE_BUTTONS = 0;
    PACKET_INDEX = 0;
    io::outb(0x64, 0xA8);
    if mouse_send_command(0xF6) && mouse_send_command(0xF4) {
        crate::pic::pic_enable_irq(12);
    }
    idt::register_interrupt_handler(44, Some(mouse_irq_handler));
}

unsafe fn mouse_send_command(cmd: u8) -> bool {
    for _ in 0..100_000 {
        if io::inb(0x64) & 0x02 == 0 {
            break;
        }
    }
    io::outb(0x64, 0xD4);
    for _ in 0..100_000 {
        if io::inb(0x64) & 0x02 == 0 {
            break;
        }
    }
    io::outb(0x60, cmd);
    for _ in 0..100_000 {
        if io::inb(0x64) & 0x01 != 0 {
            return io::inb(0x60) == 0xFA;
        }
    }
    false
}

pub unsafe extern "C" fn mouse_irq_handler(_regs: *mut idt::Registers) {
    let data = io::inb(0x60);
    if PACKET_INDEX == 0 && data & 0x08 == 0 {
        crate::pic::pic_send_eoi(12);
        return;
    }
    PACKET[PACKET_INDEX as usize] = data;
    PACKET_INDEX += 1;
    if PACKET_INDEX == 3 {
        PACKET_INDEX = 0;
        let buttons = PACKET[0] & 0x07;
        let dx = PACKET[1] as i8 as i16;
        let dy = PACKET[2] as i8 as i16;
        MOUSE_STATE_X += dx as i32;
        MOUSE_STATE_Y -= dy as i32;
        MOUSE_STATE_BUTTONS = buttons;
        crate::common::input::push(crate::common::input::InputEvent::MouseMotion { dx, dy, buttons });
        if let Some(callback) = MOUSE_CALLBACK {
            callback(MOUSE_STATE_X, MOUSE_STATE_Y, buttons, 0);
        }
    }
    crate::pic::pic_send_eoi(12);
}

pub unsafe fn mouse_set_handler(handler: Option<unsafe extern "C" fn(i32, i32, u8, i32)>) {
    MOUSE_CALLBACK = handler;
}

pub unsafe fn mouse_get_state_x() -> i32 { MOUSE_STATE_X }
pub unsafe fn mouse_get_state_y() -> i32 { MOUSE_STATE_Y }
pub unsafe fn mouse_get_buttons() -> u8 { MOUSE_STATE_BUTTONS }
