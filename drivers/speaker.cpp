// PC speaker output for the NebulaOS x86 operating system.
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
// See LICENCE for the full license text.

#include "speaker.hpp"

namespace {
const unsigned short speaker_port = 0x61;
const unsigned short pit_channel2_port = 0x42;
const unsigned short pit_mode_port = 0x43;

const std::uint8_t speaker_enable = 0x03;   // bits 0 and 1
const std::uint8_t speaker_disable = ~0x03;

std::uint8_t read_port(std::uint16_t port) {
    std::uint8_t value;
    asm volatile("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

void write_port(std::uint16_t port, std::uint8_t value) {
    asm volatile("outb %0, %1" : : "a"(value), "Nd"(port));
}

// Polls PIT channel 2 for the given count, then returns. No interrupts used,
// so this works before the interrupt controller is installed.
void wait_channel2(std::uint32_t count) {
    std::uint8_t gate = read_port(speaker_port);
    write_port(speaker_port, static_cast<std::uint8_t>((gate & ~0x02) | 0x01));

    write_port(pit_mode_port, 0xB0);
    write_port(pit_channel2_port, static_cast<std::uint8_t>(count & 0xFF));
    write_port(pit_channel2_port, static_cast<std::uint8_t>((count >> 8) & 0xFF));

    std::uint8_t held = read_port(speaker_port);
    write_port(speaker_port, static_cast<std::uint8_t>(held & ~0x01));
    write_port(speaker_port, static_cast<std::uint8_t>(held | 0x01));
    write_port(speaker_port, static_cast<std::uint8_t>((held & ~0x02) | 0x02));

    while ((read_port(speaker_port) & 0x20) != 0) {
        asm volatile("nop");
    }
}
} // namespace

namespace drivers::speaker {

void initialize() {
    // Ensure the speaker starts silent.
    silence();
}

void beep(unsigned int frequency_hz, unsigned int duration_ms) {
    if (frequency_hz == 0) {
        silence();
        return;
    }

    // PIT channel 2, mode 0, divisor = 1193182 / frequency.
    const std::uint32_t divisor = 1193182U / frequency_hz;
    write_port(pit_mode_port, 0xB0);
    write_port(pit_channel2_port, static_cast<std::uint8_t>(divisor & 0xFF));
    write_port(pit_channel2_port, static_cast<std::uint8_t>((divisor >> 8) & 0xFF));

    // Turn the speaker on.
    std::uint8_t port61 = read_port(speaker_port);
    write_port(speaker_port, static_cast<std::uint8_t>(port61 | speaker_enable));

    // Wait for the duration.
    const std::uint32_t wait_count = 1193182U * duration_ms / 1000U;
    wait_channel2(wait_count);

    // Turn the speaker off.
    silence();
}

void silence() {
    std::uint8_t port61 = read_port(speaker_port);
    write_port(speaker_port, static_cast<std::uint8_t>(port61 & speaker_disable));
}

} // namespace drivers::speaker
