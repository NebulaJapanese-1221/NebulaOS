use crate::ps2::{inb, outb};

pub struct Time {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
}

fn read_cmos(reg: u8) -> u8 {
    unsafe {
        outb(0x70, reg);
        inb(0x71)
    }
}

fn bcd_to_bin(bcd: u8) -> u8 {
    (bcd & 0x0F) + ((bcd / 16) * 10)
}

pub fn get_time() -> Time {
    // Wait for RTC update bit to be clear
    while (read_cmos(0x0A) & 0x80) != 0 {}

    let mut second = read_cmos(0x00);
    let mut minute = read_cmos(0x02);
    let mut hour = read_cmos(0x04);

    let register_b = read_cmos(0x0B);

    // Convert BCD to binary if necessary
    if (register_b & 0x04) == 0 {
        second = bcd_to_bin(second);
        minute = bcd_to_bin(minute);
        hour = bcd_to_bin(hour);
    }

    Time { second, minute, hour }
}