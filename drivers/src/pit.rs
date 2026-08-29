use crate::common::io;
use crate::common::idt;

static mut TIMER_TICKS: u32 = 0;
static mut TIMER_CALLBACK: Option<unsafe extern "C" fn(u32)> = None;

#[no_mangle]
pub unsafe extern "C" fn pit_init(frequency: u32) {
    let divisor = 1193182 / frequency;
    io::outb(0x43, 0x36);
    io::outb(0x40, (divisor & 0xFF) as u8);
    io::outb(0x40, ((divisor >> 8) & 0xFF) as u8);
    idt::register_interrupt_handler(32, Some(pit_irq_handler));
}

pub unsafe extern "C" fn pit_irq_handler(_regs: *mut idt::Registers) {
    TIMER_TICKS += 1;
    if let Some(callback) = TIMER_CALLBACK {
        callback(TIMER_TICKS);
    }
    io::outb(0x20, 0x20);
}

pub unsafe fn timer_set_handler(handler: Option<unsafe extern "C" fn(u32)>) {
    TIMER_CALLBACK = handler;
}

pub unsafe fn timer_get_ticks() -> u32 {
    TIMER_TICKS
}
