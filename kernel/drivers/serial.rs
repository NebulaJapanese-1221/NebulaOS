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

// Serial port driver
#[cfg(target_arch = "x86")]
use crate::sync::SpinLock;
use core::fmt::{self, Write};

struct SerialWriter;

#[cfg(target_arch = "x86")]
static SERIAL_LOCK: SpinLock<()> = SpinLock::new(());

impl Write for SerialWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        write_string(value);
        Ok(())
    }
}

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
pub fn init() {
    port_byte_out(0x3F8 + 3, 0x80);
    port_byte_out(0x3F8 + 0, 0x01);
    port_byte_out(0x3F8 + 1, 0x00);
    port_byte_out(0x3F8 + 3, 0x03);
    port_byte_out(0x3F8 + 2, 0xC7);
    port_byte_out(0x3F8 + 1, 0x00);
}

#[cfg(target_arch = "aarch64")]
pub fn init() {
    // AArch64 serial uses MMIO (PL011 UART); stubbed for now
}

#[cfg(target_arch = "x86")]
pub fn write_byte(byte: u8) {
    let _guard = SERIAL_LOCK.lock();
    write_byte_unlocked(byte);
}

#[cfg(target_arch = "x86")]
fn write_byte_unlocked(byte: u8) {
    while port_byte_in(0x3F8 + 5) & 0x20 == 0 {
        core::hint::spin_loop();
    }
    port_byte_out(0x3F8, byte);
}

#[cfg(target_arch = "aarch64")]
pub fn write_byte(_byte: u8) {
    // Stub - no serial output on aarch64 yet
}

#[cfg(target_arch = "x86")]
pub fn write_string(s: &str) {
    let _guard = SERIAL_LOCK.lock();
    for byte in s.bytes() {
        write_byte_unlocked(byte);
    }
}

#[cfg(target_arch = "aarch64")]
pub fn write_string(_s: &str) {
    // Stub
}

#[cfg(target_arch = "x86")]
pub fn write_dec(mut val: u32) {
    let _guard = SERIAL_LOCK.lock();
    if val == 0 {
        write_byte_unlocked(b'0');
        return;
    }

    let mut buf = [0u8; 10];
    let mut pos = 10;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    for i in pos..10 {
        write_byte_unlocked(buf[i]);
    }
}

#[cfg(target_arch = "aarch64")]
pub fn write_dec(_val: u32) {
    // Stub
}

#[cfg(target_arch = "x86")]
pub fn write_hex(value: u32) {
    let _guard = SERIAL_LOCK.lock();
    for byte in b"0x" {
        write_byte_unlocked(*byte);
    }
    for shift in (0..8).rev() {
        let digit = ((value >> (shift * 4)) & 0xF) as u8;
        write_byte_unlocked(if digit < 10 {
            b'0' + digit
        } else {
            b'A' + digit - 10
        });
    }
}

#[cfg(target_arch = "aarch64")]
pub fn write_hex(_value: u32) {
    // Stub
}

pub fn write_fmt(arguments: fmt::Arguments<'_>) {
    let _ = SerialWriter.write_fmt(arguments);
}
