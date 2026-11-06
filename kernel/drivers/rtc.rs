// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

// CMOS Real-Time Clock for wall-clock time (x86) / stub (aarch64)

#[cfg(target_arch = "x86")]
const CMOS_ADDRESS: u16 = 0x70;
#[cfg(target_arch = "x86")]
const CMOS_DATA: u16 = 0x71;

#[cfg(target_arch = "x86")]
const RTC_SECONDS: u8 = 0x00;
#[cfg(target_arch = "x86")]
const RTC_MINUTES: u8 = 0x02;
#[cfg(target_arch = "x86")]
const RTC_HOURS: u8 = 0x04;
#[cfg(target_arch = "x86")]
const RTC_DAY: u8 = 0x07;
#[cfg(target_arch = "x86")]
const RTC_MONTH: u8 = 0x08;
#[cfg(target_arch = "x86")]
const RTC_YEAR: u8 = 0x09;
#[cfg(target_arch = "x86")]
const RTC_REGISTER_B: u8 = 0x0B;
#[cfg(target_arch = "x86")]
const RTC_REGISTER_C: u8 = 0x0C;
#[cfg(target_arch = "x86")]
const RTC_REGISTER_D: u8 = 0x0D;
#[cfg(target_arch = "x86")]
const RTC_CENTURY: u8 = 0x32;

static mut RTC_READY: bool = false;

#[cfg(target_arch = "x86")]
fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

#[cfg(target_arch = "x86")]
fn port_byte_in(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port);
    }
    val
}

#[cfg(target_arch = "x86")]
fn cmos_read(reg: u8) -> u8 {
    port_byte_out(CMOS_ADDRESS, reg | 0x80);
    port_byte_in(CMOS_DATA)
}

#[cfg(target_arch = "x86")]
fn cmos_wait_update() {
    loop {
        let status = cmos_read(RTC_REGISTER_D);
        if status & 0x80 == 0 {
            break;
        }
        core::hint::spin_loop();
    }
}

#[cfg(target_arch = "x86")]
fn bcd_to_decimal(bcd: u8) -> u8 {
    (bcd & 0x0F) + ((bcd >> 4) * 10)
}

pub fn init() {
    unsafe {
        RTC_READY = true;
    }
}

pub fn is_available() -> bool {
    unsafe { RTC_READY }
}

#[derive(Clone, Copy, Debug)]
pub struct RtcTime {
    pub year: u32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[cfg(target_arch = "x86")]
pub fn read_time() -> Option<RtcTime> {
    if !unsafe { RTC_READY } {
        return None;
    }

    cmos_wait_update();

    let mut second = cmos_read(RTC_SECONDS);
    let mut minute = cmos_read(RTC_MINUTES);
    let mut hour = cmos_read(RTC_HOURS);
    let mut day = cmos_read(RTC_DAY);
    let mut month = cmos_read(RTC_MONTH);
    let mut year = cmos_read(RTC_YEAR);
    let century = cmos_read(RTC_CENTURY);

    let register_b = cmos_read(RTC_REGISTER_B);
    let bcd_mode = register_b & 0x04 == 0;

    if bcd_mode {
        second = bcd_to_decimal(second);
        minute = bcd_to_decimal(minute);
        hour = bcd_to_decimal(hour);
        day = bcd_to_decimal(day);
        month = bcd_to_decimal(month);
        year = bcd_to_decimal(year);
    }

    let full_year = if century != 0 && century != 0xFF {
        let century_val = if bcd_mode { bcd_to_decimal(century) } else { century };
        (u32::from(century_val) * 100) + u32::from(year)
    } else {
        u32::from(year) + 2000
    };

    Some(RtcTime {
        year: full_year,
        month,
        day,
        hour,
        minute,
        second,
    })
}

#[cfg(target_arch = "aarch64")]
pub fn read_time() -> Option<RtcTime> {
    None
}

#[cfg(target_arch = "x86")]
pub fn read_register_d() -> u8 {
    cmos_read(RTC_REGISTER_D)
}

#[cfg(target_arch = "aarch64")]
pub fn read_register_d() -> u8 {
    0
}
