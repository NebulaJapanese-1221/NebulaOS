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

// PC speaker audio via PIT channel 2 and port 0x61

const PIT_CHANNEL2: u16 = 0x42;
const PIT_COMMAND: u16 = 0x43;
const SPEAKER_PORT: u16 = 0x61;

const PIT_BASE_FREQ: u32 = 1_193_182;

const SPEAKER_ENABLE: u8 = 0x03;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn port_byte_in(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port);
    }
    val
}

static mut AUDIO_ENABLED: bool = false;

pub fn init() {
    unsafe {
        AUDIO_ENABLED = true;
    }
    // Ensure speaker is off initially
    let val = port_byte_in(SPEAKER_PORT) & !SPEAKER_ENABLE;
    port_byte_out(SPEAKER_PORT, val);
}

pub fn is_available() -> bool {
    unsafe { AUDIO_ENABLED }
}

pub fn play_tone(frequency_hz: u32, duration_ms: u32) {
    if !unsafe { AUDIO_ENABLED } || frequency_hz == 0 {
        return;
    }

    let divisor = PIT_BASE_FREQ / frequency_hz;
    if divisor == 0 || divisor > 0xFFFF {
        return;
    }

    unsafe {
        core::arch::asm!("out dx, al", in("dx") PIT_COMMAND, in("al") 0xB6u8);
        core::arch::asm!("out dx, al", in("dx") PIT_CHANNEL2, in("al") (divisor & 0xFF) as u8);
        core::arch::asm!("out dx, al", in("dx") PIT_CHANNEL2, in("al") ((divisor >> 8) & 0xFF) as u8);

        let mut val = port_byte_in(SPEAKER_PORT);
        val |= SPEAKER_ENABLE;
        port_byte_out(SPEAKER_PORT, val);
    }

    sleep_ms(duration_ms);

    stop();
}

pub fn beep() {
    play_tone(440, 200);
}

pub fn stop() {
    if !unsafe { AUDIO_ENABLED } {
        return;
    }
    unsafe {
        let mut val = port_byte_in(SPEAKER_PORT);
        val &= !SPEAKER_ENABLE;
        port_byte_out(SPEAKER_PORT, val);
    }
}

fn sleep_ms(ms: u32) {
    // Approximate delay using PIT ticks (assumes ~100 Hz from pit::init)
    let ticks_needed = ms / 10;
    let start = crate::drivers::pit::ticks();
    while crate::drivers::pit::ticks().wrapping_sub(start) < ticks_needed {
        core::hint::spin_loop();
    }
}
