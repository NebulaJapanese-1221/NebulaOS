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

// PS/2 Keyboard driver for x86.

const KEYBOARD_DATA_PORT: u16 = 0x60;
const KEYBOARD_STATUS_PORT: u16 = 0x64;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn port_byte_in(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") value, in("dx") port);
    }
    value
}

pub fn init() {
    // Enable keyboard interrupts
    port_byte_out(KEYBOARD_STATUS_PORT, 0xAE);
    port_byte_out(0x60, 0xF4);
}

pub fn poll_key() -> Option<u8> {
    if port_byte_in(KEYBOARD_STATUS_PORT) & 0x01 == 0 {
        return None;
    }
    Some(port_byte_in(KEYBOARD_DATA_PORT))
}

/// Drain the keyboard buffer and return true if F12 (scan code 0x58) was pressed.
pub fn f12_pressed() -> bool {
    let mut found = false;
    while port_byte_in(KEYBOARD_STATUS_PORT) & 0x01 != 0 {
        let scan = port_byte_in(KEYBOARD_DATA_PORT);
        if scan == 0x58 {
            found = true;
        }
    }
    found
}

pub fn poll() {
    let _ = poll_key();
}
