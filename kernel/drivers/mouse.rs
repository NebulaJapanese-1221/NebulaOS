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

use crate::sync::SpinLock;

const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
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

static mut MOUSE: MouseState = MouseState {
    x: 0,
    y: 0,
    left: false,
    right: false,
    middle: false,
};
static mut PACKET: [u8; PACKET_SIZE] = [0; PACKET_SIZE];
static mut PACKET_INDEX: usize = 0;
static mut SENSITIVITY: i32 = 2;
static mut REMAINDER_X: i32 = 0;
static mut REMAINDER_Y: i32 = 0;
static mut AVAILABLE: bool = false;
static mut INTERRUPTS_ENABLED: bool = false;
static MOUSE_LOCK: SpinLock<()> = SpinLock::new(());

pub fn set_sensitivity(value: i32) {
    let _guard = MOUSE_LOCK.lock();
    unsafe {
        SENSITIVITY = value.clamp(1, 4);
        REMAINDER_X = 0;
        REMAINDER_Y = 0;
    }
}

pub fn sensitivity() -> u32 {
    let _guard = MOUSE_LOCK.lock();
    unsafe { SENSITIVITY as u32 }
}

pub fn state() -> MouseState {
    let _guard = MOUSE_LOCK.lock();
    unsafe { MOUSE }
}

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

fn wait_write() -> bool {
    for _ in 0..100_000 {
        if port_byte_in(STATUS_PORT) & 0x02 == 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

fn wait_read() -> bool {
    for _ in 0..100_000 {
        if port_byte_in(STATUS_PORT) & 0x01 != 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

fn enable_mouse_interrupts() -> bool {
    if !wait_write() {
        return false;
    }
    port_byte_out(COMMAND_PORT, 0x20);
    if !wait_read() {
        return false;
    }
    let command_byte = port_byte_in(DATA_PORT) | 0x02;
    if !wait_write() {
        return false;
    }
    port_byte_out(COMMAND_PORT, 0x60);
    if !wait_write() {
        return false;
    }
    port_byte_out(DATA_PORT, command_byte);
    true
}

pub fn init() -> bool {
    if !wait_write() {
        return false;
    }
    port_byte_out(COMMAND_PORT, 0xA8);
    if !wait_write() {
        return false;
    }
    port_byte_out(COMMAND_PORT, 0xD4);
    if !wait_write() {
        return false;
    }
    port_byte_out(DATA_PORT, 0xF4);
    if !wait_read() {
        return false;
    }
    let enabled = port_byte_in(DATA_PORT) == 0xFA;
    let interrupts_enabled = enabled && enable_mouse_interrupts();
    unsafe {
        AVAILABLE = enabled;
        INTERRUPTS_ENABLED = interrupts_enabled;
    }
    if enabled {
        unsafe {
            MOUSE.x = crate::drivers::framebuffer::get_width() as i32 / 2;
            MOUSE.y = crate::drivers::framebuffer::get_height() as i32 / 2;
        }
    }
    enabled
}

pub fn is_available() -> bool {
    unsafe { AVAILABLE }
}

pub fn interrupts_enabled() -> bool {
    unsafe { INTERRUPTS_ENABLED }
}

pub fn poll() -> Option<MouseState> {
    let _guard = MOUSE_LOCK.lock();
    unsafe {
        let fb_width = crate::drivers::framebuffer::get_width() as i32;
        let fb_height = crate::drivers::framebuffer::get_height() as i32;
        let mut updated = false;
        for _ in 0..24 {
            let status = port_byte_in(STATUS_PORT);
            if status & 0x21 != 0x21 {
                break;
            }

            let byte = port_byte_in(DATA_PORT);
            if PACKET_INDEX == 0 && byte & 0x08 == 0 {
                continue;
            }
            PACKET[PACKET_INDEX] = byte;
            PACKET_INDEX += 1;
            if PACKET_INDEX < PACKET_SIZE {
                continue;
            }
            PACKET_INDEX = 0;

            if PACKET[0] & 0xC0 == 0 {
                let scale_twice = SENSITIVITY + 1;
                let scaled_x = PACKET[1] as i8 as i32 * scale_twice + REMAINDER_X;
                let scaled_y = -(PACKET[2] as i8 as i32) * scale_twice + REMAINDER_Y;
                let dx = scaled_x / 2;
                let dy = scaled_y / 2;
                let next_x = (MOUSE.x + dx).clamp(0, fb_width.saturating_sub(1));
                let next_y = (MOUSE.y + dy).clamp(0, fb_height.saturating_sub(1));
                REMAINDER_X = if next_x == MOUSE.x + dx {
                    scaled_x % 2
                } else {
                    0
                };
                REMAINDER_Y = if next_y == MOUSE.y + dy {
                    scaled_y % 2
                } else {
                    0
                };
                MOUSE.x = next_x;
                MOUSE.y = next_y;
            }
            MOUSE.left = PACKET[0] & 0x01 != 0;
            MOUSE.right = PACKET[0] & 0x02 != 0;
            MOUSE.middle = PACKET[0] & 0x04 != 0;
            updated = true;
        }
        if updated {
            Some(MOUSE)
        } else {
            None
        }
    }
}
