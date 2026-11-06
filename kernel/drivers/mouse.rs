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

// PS/2 Mouse driver (x86) / stub (aarch64)

#[cfg(target_arch = "x86")]
use crate::sync::SpinLock;

#[cfg(target_arch = "x86")]
const DATA_PORT: u16 = 0x60;
#[cfg(target_arch = "x86")]
const STATUS_PORT: u16 = 0x64;
#[cfg(target_arch = "x86")]
const COMMAND_PORT: u16 = 0x64;
const PACKET_SIZE: usize = 3;

#[derive(Clone, Copy)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

#[cfg(target_arch = "x86")]
static MOUSE_STATE: SpinLock<MouseState> = SpinLock::new(MouseState {
    x: 0,
    y: 0,
    left: false,
    right: false,
    middle: false,
});

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
    value
}

#[cfg(target_arch = "x86")]
pub fn init() -> bool {
    port_byte_out(COMMAND_PORT, 0xA8);
    port_byte_out(COMMAND_PORT, 0xD4);
    port_byte_out(DATA_PORT, 0xF4);
    let _ = port_byte_in(STATUS_PORT);
    true
}

#[cfg(target_arch = "aarch64")]
pub fn init() -> bool {
    false
}

#[cfg(target_arch = "x86")]
pub fn poll() -> Option<MouseState> {
    let mut guard = MOUSE_STATE.lock();
    if port_byte_in(STATUS_PORT) & 0x01 == 0 {
        return None;
    }
    let byte = port_byte_in(DATA_PORT);
    if byte & 0x08 == 0 {
        return None;
    }
    Some(*guard)
}

#[cfg(target_arch = "aarch64")]
pub fn poll() -> Option<MouseState> {
    None
}

#[cfg(target_arch = "x86")]
pub fn interrupts_enabled() -> bool {
    true
}

#[cfg(target_arch = "aarch64")]
pub fn interrupts_enabled() -> bool {
    false
}

pub fn is_available() -> bool {
    #[cfg(target_arch = "x86")]
    {
        true
    }
    #[cfg(target_arch = "aarch64")]
    {
        false
    }
}

pub fn sensitivity() -> u32 {
    #[cfg(target_arch = "x86")]
    {
        1
    }
    #[cfg(target_arch = "aarch64")]
    {
        1
    }
}

pub fn set_sensitivity(_value: i32) {
    #[cfg(target_arch = "x86")]
    {
        // stub
    }
    #[cfg(target_arch = "aarch64")]
    {
        // stub
    }
}

pub fn state() -> MouseState {
    #[cfg(target_arch = "x86")]
    {
        MOUSE_STATE.lock().clone()
    }
    #[cfg(target_arch = "aarch64")]
    {
        MouseState { x: 0, y: 0, left: false, right: false, middle: false }
    }
}
