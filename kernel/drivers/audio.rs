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

// Audio subsystem for x86.
// Provides PC speaker audio (PIT channel 2 + port 0x61) and optional
// AC'97 codec playback via PCI enumeration. PC speaker is always available;
// AC'97 is used when a compatible codec is detected on the PCI bus.

const PIT_CHANNEL2: u16 = 0x42;
const PIT_COMMAND: u16 = 0x43;
const SPEAKER_PORT: u16 = 0x61;

const PIT_BASE_FREQ: u32 = 1_193_182;

const SPEAKER_GATE_BIT: u8 = 0x01;
const SPEAKER_DATA_BIT: u8 = 0x02;

const VOLUME_OFF: u8 = 0x00;
const VOLUME_LOW: u8 = 0x01;
const VOLUME_MED: u8 = 0x02;
const VOLUME_HIGH: u8 = 0x03;

// AC'97 register offsets (from codec base BAR0)
const AC97_RESET: u16 = 0x00;
const AC97_PCM_OUT_VOLUME: u16 = 0x02;
const AC97_PCM_IN_VOLUME: u16 = 0x0A;
const AC97_GENERAL_PURPOSE: u16 = 0x34;
const AC97_PCM_LR_DAC_RATE: u16 = 0x2C;

// PCI vendor/device IDs we probe for.
const AC97_VENDOR_INTEL: u16 = 0x8086;

// Static state.
static mut AUDIO_INITIALIZED: bool = false;
static mut VOLUME_LEVEL: u8 = VOLUME_HIGH;
static mut AC97_AVAILABLE: bool = false;
static mut AC97_CODEC_BASE: u32 = 0;

static mut QUEUED_FREQ: u32 = 0;
static mut QUEUED_DURATION: u32 = 0;
static mut QUEUE_PENDING: bool = false;

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

fn port_word_out(port: u16, data: u16) {
    unsafe {
        core::arch::asm!("out dx, ax", in("dx") port, in("ax") data);
    }
}

fn port_word_in(port: u16) -> u16 {
    let val: u16;
    unsafe {
        core::arch::asm!("in ax, dx", out("ax") val, in("dx") port);
    }
    val
}

// ---------------------------------------------------------------------------
// AC'97 helpers
// ---------------------------------------------------------------------------

fn ac97_read(base: u32, reg: u16) -> u16 {
    unsafe {
        port_word_out((base as u16).wrapping_add(reg), 0);
        port_word_in((base as u16).wrapping_add(reg))
    }
}

fn ac97_write(base: u32, reg: u16, value: u16) {
    unsafe {
        port_word_out((base as u16).wrapping_add(reg), value);
    }
}

fn ac97_detect() -> Option<u32> {
    // Scan PCI for Intel 82801 series AC'97 audio controllers.
    for bus in 0..=255u8 {
        for device in 0..32u8 {
            for function in 0..8u8 {
                let vendor_id = crate::drivers::pci::pci_read_word(bus, device, function, 0x00);
                if vendor_id == 0xFFFF || vendor_id != AC97_VENDOR_INTEL {
                    continue;
                }
                let class = crate::drivers::pci::pci_read_byte(bus, device, function, 0x0B);
                if class != 0x04 {
                    continue;
                }
                let bar0 = crate::drivers::pci::pci_read_word(bus, device, function, 0x14);
                if bar0 != 0 {
                    return Some((bar0 & !1) as u32);
                }
            }
        }
    }
    None
}

fn ac97_init(base: u32) {
    // Software reset.
    ac97_write(base, AC97_RESET, 0);
    let _ = ac97_read(base, AC97_RESET);

    // Set PCM output volume (both channels) to a reasonable level.
    let volume = match unsafe { VOLUME_LEVEL } {
        VOLUME_LOW => 0x0808,
        VOLUME_MED => 0x0404,
        VOLUME_HIGH => 0x0202,
        _ => 0x8000,
    };
    ac97_write(base, AC97_PCM_OUT_VOLUME, volume);

    // Set PCM sample rate to 48 kHz (0xBB80).
    ac97_write(base, AC97_PCM_LR_DAC_RATE, 0xBB80);

    unsafe {
        AC97_AVAILABLE = true;
        AC97_CODEC_BASE = base;
    }
}

pub fn init() {
    unsafe {
        AUDIO_INITIALIZED = true;
        VOLUME_LEVEL = VOLUME_HIGH;
        QUEUED_FREQ = 0;
        QUEUED_DURATION = 0;
        QUEUE_PENDING = false;
    }

    // PC speaker: ensure off initially.
    let mut val = port_byte_in(SPEAKER_PORT);
    val &= !(SPEAKER_GATE_BIT | SPEAKER_DATA_BIT);
    port_byte_out(SPEAKER_PORT, val);

    // Try to detect AC'97 codec.
    if let Some(base) = ac97_detect() {
        ac97_init(base);
    }
}

pub fn is_available() -> bool {
    unsafe { AUDIO_INITIALIZED }
}

pub fn ac97_is_available() -> bool {
    unsafe { AC97_AVAILABLE }
}

pub fn set_volume(level: u8) {
    unsafe {
        VOLUME_LEVEL = level.min(VOLUME_HIGH);
    }
    if unsafe { AC97_AVAILABLE } {
        let volume = match level {
            VOLUME_LOW => 0x0808,
            VOLUME_MED => 0x0404,
            VOLUME_HIGH => 0x0202,
            _ => 0x8000,
        };
        ac97_write(unsafe { AC97_CODEC_BASE }, AC97_PCM_OUT_VOLUME, volume);
    }
}

pub fn volume() -> u8 {
    unsafe { VOLUME_LEVEL }
}

/// Play a tone at `frequency_hz` for `duration_ms` milliseconds via PC speaker.
/// Blocks the caller for the duration of the tone.
pub fn play_tone(frequency_hz: u32, duration_ms: u32) {
    if !unsafe { AUDIO_INITIALIZED } || frequency_hz == 0 {
        return;
    }

    let divisor = PIT_BASE_FREQ / frequency_hz;
    if divisor == 0 || divisor > 0xFFFF {
        return;
    }

    unsafe {
        core::arch::asm!("out dx, al", in("dx") PIT_COMMAND, in("al") 0xB6u8);
        core::arch::asm!(
            "out dx, al",
            in("dx") PIT_CHANNEL2,
            in("al") (divisor & 0xFF) as u8
        );
        core::arch::asm!(
            "out dx, al",
            in("dx") PIT_CHANNEL2,
            in("al") ((divisor >> 8) & 0xFF) as u8
        );
    }

    let mut val = port_byte_in(SPEAKER_PORT);
    val &= !(SPEAKER_GATE_BIT | SPEAKER_DATA_BIT);
    if unsafe { VOLUME_LEVEL } > VOLUME_OFF {
        val |= SPEAKER_GATE_BIT | SPEAKER_DATA_BIT;
    }
    port_byte_out(SPEAKER_PORT, val);

    sleep_ms(duration_ms);

    stop();
}

/// Play a short beep at 440 Hz (A4) for 200 ms.
pub fn beep() {
    play_tone(440, 200);
}

/// Play a higher-pitched alert beep at 880 Hz (A5) for 100 ms.
pub fn alert() {
    play_tone(880, 100);
}

/// Stop any tone currently playing by disabling the speaker gate and data bits.
pub fn stop() {
    if !unsafe { AUDIO_INITIALIZED } {
        return;
    }
    let mut val = port_byte_in(SPEAKER_PORT);
    val &= !(SPEAKER_GATE_BIT | SPEAKER_DATA_BIT);
    port_byte_out(SPEAKER_PORT, val);
}

/// Play a sequence of (frequency_hz, duration_ms) tone pairs with a brief
/// gap between notes. Useful for simple melodies and startup sounds.
pub fn play_melody(notes: &[(u32, u32)]) {
    for &(freq, dur) in notes {
        play_tone(freq, dur);
        sleep_ms(30);
    }
}

/// Play the NebulaOS startup chime (ascending major arpeggio).
pub fn startup_chime() {
    play_melody(&[
        (523, 150), // C5
        (659, 150), // E5
        (784, 150), // G5
        (1047, 300), // C6
    ]);
}

/// Play an error sound (descending minor triad).
pub fn error_sound() {
    play_melody(&[(440, 150), (370, 150), (330, 300)]); // A4 -> F#4 -> E4
}

/// Play a success sound (ascending major triad).
pub fn success_sound() {
    play_melody(&[(523, 120), (659, 120), (784, 250)]); // C5 -> E5 -> G5
}

/// Queue a tone for non-blocking playback. Call `poll()` regularly to advance.
pub fn queue_tone(frequency_hz: u32, duration_ms: u32) {
    if !unsafe { AUDIO_INITIALIZED } || frequency_hz == 0 {
        return;
    }
    unsafe {
        QUEUED_FREQ = frequency_hz;
        QUEUED_DURATION = duration_ms;
        QUEUE_PENDING = true;
    }
}

/// Advance queued tone playback. Must be called from a timer tick or main loop.
/// Returns true if a tone is currently playing or queued.
pub fn poll() -> bool {
    if !unsafe { AUDIO_INITIALIZED } {
        return false;
    }
    if unsafe { QUEUE_PENDING } {
        let freq = unsafe { QUEUED_FREQ };
        let dur = unsafe { QUEUED_DURATION };
        unsafe { QUEUE_PENDING = false; }
        play_tone(freq, dur);
        true
    } else {
        false
    }
}

fn sleep_ms(ms: u32) {
    let ticks_needed = ms / 10;
    let start = crate::drivers::pit::ticks();
    while crate::drivers::pit::ticks().wrapping_sub(start) < ticks_needed {
        core::hint::spin_loop();
    }
}
