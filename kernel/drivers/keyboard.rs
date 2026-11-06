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

// PS/2 Keyboard driver (x86) / stub (aarch64)

#[cfg(target_arch = "x86")]
const KEYBOARD_DATA_PORT: u16 = 0x60;
#[cfg(target_arch = "x86")]
const KEYBOARD_STATUS_PORT: u16 = 0x64;

#[cfg(target_arch = "x86")]
fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

#[cfg(target_arch = "x86")]
fn port_byte_in(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") value, in("dx") port);
    }
    val
}

#[cfg(target_arch = "x86")]
pub fn init() {
    // Enable keyboard interrupts
    port_byte_out(KEYBOARD_STATUS_PORT, 0xAE);
    port_byte_out(0x60, 0xF4);
}

#[cfg(target_arch = "aarch64")]
pub fn init() {
    // Stub - no PS/2 on aarch64
}

#[cfg(target_arch = "x86")]
pub fn poll_key() -> Option<u8> {
    if port_byte_in(KEYBOARD_STATUS_PORT) & 0x01 == 0 {
        return None;
    }
    Some(port_byte_in(KEYBOARD_DATA_PORT))
}

#[cfg(target_arch = "aarch64")]
pub fn poll_key() -> Option<u8> {
    None
}

#[cfg(target_arch = "x86")]
pub fn poll() {
    let _ = poll_key();
}

#[cfg(target_arch = "aarch64")]
pub fn poll() {
    // Stub
}
