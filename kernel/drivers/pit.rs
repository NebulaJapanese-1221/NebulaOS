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

// Programmable Interval Timer

const PIT_CHANNEL0: u16 = 0x40;
const PIT_COMMAND: u16 = 0x43;

const PIT_SEL_CHANNEL0: u8 = 0x00;
const PIT_ACCESS_LOHI: u8 = 0x30;
const PIT_MODE_RATE_GENERATOR: u8 = 0x04;

const PIT_BASE_FREQ: u32 = 1_193_182;

use core::sync::atomic::{AtomicU32, Ordering};

static TICKS: AtomicU32 = AtomicU32::new(0);

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
    TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn ticks() -> u32 {
    TICKS.load(Ordering::Relaxed)
}

pub fn sleep_ticks(duration: u32) {
    let start = ticks();
    while ticks().wrapping_sub(start) < duration {
        core::hint::spin_loop();
    }
}
