use crate::kernel::io;
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

const CMOS_ADDRESS: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

// Global storage for the current time and update flag
pub static CURRENT_DATETIME: Mutex<DateTime> = Mutex::new(DateTime {
    second: 0, minute: 0, hour: 0, day: 0, month: 0, year: 0
});
pub static TIME_NEEDS_UPDATE: AtomicBool = AtomicBool::new(false);
pub static TICK_COUNT: Mutex<u32> = Mutex::new(0);

fn read_register(reg: u8) -> u8 {
    unsafe {
        io::outb(CMOS_ADDRESS, reg);
        io::inb(CMOS_DATA)
    }
}

fn bcd_to_binary(bcd: u8) -> u8 {
    (bcd & 0x0F) + ((bcd >> 4) * 10)
}

fn get_update_in_progress_flag() -> bool {
    (read_register(0x0A) & 0x80) != 0
}

pub fn read_time() -> DateTime {
    // Wait until the update in progress flag is clear to avoid reading inconsistent values
    while get_update_in_progress_flag() {}

    let mut second = read_register(0x00);
    let mut minute = read_register(0x02);
    let mut hour = read_register(0x04);
    let mut day = read_register(0x07);
    let mut month = read_register(0x08);
    let mut year = read_register(0x09);

    let register_b = read_register(0x0B);

    // Convert BCD to binary if necessary
    if (register_b & 0x04) == 0 {
        second = bcd_to_binary(second);
        minute = bcd_to_binary(minute);
        // Hour is special: mask out 12-hour AM/PM bit before converting
        hour = bcd_to_binary(hour & 0x7F);
        day = bcd_to_binary(day);
        month = bcd_to_binary(month);
        year = bcd_to_binary(year);
    }

    // Convert 12 hour clock to 24 hour clock if necessary
    if (register_b & 0x02) == 0 && (read_register(0x04) & 0x80) != 0 {
        hour = ((hour & 0x7F) + 12) % 24;
    }

    // Assuming the current century is 21st.
    // This is a simplification; a full RTC driver would read the century from CMOS register 0x32
    // and handle year 2000+ correctly.
    let full_year = 2000 + year as u16;

    DateTime {
        second,
        minute,
        hour,
        day,
        month,
        year: full_year,
    }
}

pub fn handle_timer_tick() {
    // Increment tick count
    let mut tick_count = TICK_COUNT.lock();
    *tick_count += 1;

    // Assuming a 1000 Hz PIT frequency, 1000 ticks = 1 second.
    // This is now tied to the PIT frequency set in kernel_main.
    if *tick_count >= 1000 {
        *tick_count = 0; // Reset tick count
        TIME_NEEDS_UPDATE.store(true, Ordering::Relaxed); // Signal that time needs update
    }
}