// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

// PS/2 Mouse driver for x86.

use crate::sync::SpinLock;

const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
const COMMAND_PORT: u16 = 0x64;
const PACKET_SIZE: usize = 3;

const SCREEN_WIDTH: i32 = 1024;
const SCREEN_HEIGHT: i32 = 768;
const SENSITIVITY_DEFAULT: i32 = 2;

#[derive(Clone, Copy)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

static MOUSE_STATE: SpinLock<MouseState> = SpinLock::new(MouseState {
    x: SCREEN_WIDTH / 2,
    y: SCREEN_HEIGHT / 2,
    left: false,
    right: false,
    middle: false,
});

static MOUSE_SENSITIVITY: SpinLock<i32> = SpinLock::new(SENSITIVITY_DEFAULT);
static MOUSE_PACKET: SpinLock<[u8; PACKET_SIZE]> = SpinLock::new([0; PACKET_SIZE]);
static MOUSE_PACKET_INDEX: SpinLock<usize> = SpinLock::new(0);

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

pub fn init() -> bool {
    port_byte_out(COMMAND_PORT, 0xA8); // enable mouse device
    port_byte_out(COMMAND_PORT, 0xD4); // write to mouse
    port_byte_out(DATA_PORT, 0xF4);    // enable streaming
    let _ = port_byte_in(STATUS_PORT);
    true
}

pub fn poll() -> Option<MouseState> {
    if port_byte_in(STATUS_PORT) & 0x01 == 0 {
        return None;
    }
    let byte = port_byte_in(DATA_PORT);

    let mut packet = MOUSE_PACKET.lock();
    let mut index = MOUSE_PACKET_INDEX.lock();

    // First byte must have bit 3 set (synchronization)
    if *index == 0 && byte & 0x08 == 0 {
        return None;
    }

    packet[*index] = byte;
    *index += 1;

    if *index < PACKET_SIZE {
        return None;
    }

    // Full packet received
    let b0 = packet[0];
    let b1 = packet[1] as i8 as i32;
    let b2 = packet[2] as i8 as i32;

    *index = 0;

    let mut state = MOUSE_STATE.lock();
    let sensitivity = *MOUSE_SENSITIVITY.lock();

    // Overflow check
    if b0 & 0xC0 != 0 {
        return Some(*state);
    }

    let mut new_x = state.x + b1 * sensitivity;
    let mut new_y = state.y - b2 * sensitivity; // y is inverted on screen

    new_x = new_x.clamp(0, SCREEN_WIDTH - 1);
    new_y = new_y.clamp(0, SCREEN_HEIGHT - 1);

    state.x = new_x;
    state.y = new_y;
    state.left = b0 & 0x01 != 0;
    state.right = b0 & 0x02 != 0;
    state.middle = b0 & 0x04 != 0;

    Some(*state)
}

pub fn interrupts_enabled() -> bool {
    true
}

pub fn is_available() -> bool {
    true
}

pub fn sensitivity() -> u32 {
    *MOUSE_SENSITIVITY.lock() as u32
}

pub fn set_sensitivity(value: i32) {
    *MOUSE_SENSITIVITY.lock() = value.clamp(1, 4);
}

pub fn state() -> MouseState {
    MOUSE_STATE.lock().clone()
}
