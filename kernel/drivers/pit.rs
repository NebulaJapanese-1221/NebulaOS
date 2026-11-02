// Programmable Interval Timer

const PIT_CHANNEL0: u16 = 0x40;
const PIT_COMMAND: u16 = 0x43;

const PIT_SEL_CHANNEL0: u8 = 0x00;
const PIT_ACCESS_LOHI: u8 = 0x30;
const PIT_MODE_RATE_GENERATOR: u8 = 0x04;

const PIT_BASE_FREQ: u32 = 1_193_182;

static mut TICKS: u32 = 0;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

pub fn init(frequency: u32) {
    let divisor = PIT_BASE_FREQ / frequency;
    let mode_byte = PIT_SEL_CHANNEL0 | PIT_ACCESS_LOHI | PIT_MODE_RATE_GENERATOR;

    port_byte_out(PIT_COMMAND, mode_byte);
    port_byte_out(PIT_CHANNEL0, (divisor & 0xFF) as u8);
    port_byte_out(PIT_CHANNEL0, ((divisor >> 8) & 0xFF) as u8);
}

#[no_mangle]
pub extern "C" fn pit_tick() {
    unsafe {
        TICKS += 1;
    }
}

pub fn ticks() -> u32 {
    unsafe { TICKS }
}

pub fn sleep_ticks(duration: u32) {
    let start = unsafe { TICKS };
    while unsafe { TICKS } - start < duration {
        core::hint::spin_loop();
    }
}
