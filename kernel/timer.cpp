// PIT clock and delays for the NebulaOS x86 operating system.
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
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// See LICENCE for the full license text.

#include "timer.hpp"

namespace {
volatile unsigned int system_ticks = 0;

void write_port(unsigned short port, unsigned char value) {
    asm volatile("outb %0, %1" : : "a"(value), "Nd"(port));
}

unsigned char read_port(unsigned short port) {
    unsigned char value;
    asm volatile("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

// The PIT is clocked at 1.193182 MHz and its counter is sixteen bits wide, so
// one count of channel 2 covers at most this many milliseconds before it wraps.
const unsigned int maximum_count_milliseconds = 54;

// Programs channel 2 as a one shot and waits for it to finish. Bit 5 of port
// 0x61 reads high while the channel is counting and low once it reaches
// terminal count, which is the only synchronisation needed: no interrupt, and
// therefore no dependency on the interrupt controller being installed.
void wait_ticks(unsigned int count) {
    // Bit 0 restarts the counter and bit 1 is the speaker gate, which has to be
    // driven for the channel to count at all.
    unsigned char gate = read_port(0x61);
    write_port(0x61, static_cast<unsigned char>((gate & ~0x02) | 0x01));

    // Channel 2, access mode low byte then high byte, mode 0.
    write_port(0x43, 0xB0);
    write_port(0x42, static_cast<unsigned char>(count & 0xFF));
    write_port(0x42, static_cast<unsigned char>((count >> 8) & 0xFF));

    // The reload bit has to be seen low before it is seen high, otherwise the
    // counter starts from whatever it was left at.
    const unsigned char held = read_port(0x61);
    write_port(0x61, static_cast<unsigned char>(held & ~0x01));
    write_port(0x61, static_cast<unsigned char>(held | 0x01));
    write_port(0x61, static_cast<unsigned char>((held & ~0x02) | 0x02));

    while ((read_port(0x61) & 0x20) != 0) {
        asm volatile("nop");
    }
}
}

namespace kernel::timer {

void initialize() {
    const unsigned short divisor = 11932;
    write_port(0x43, 0x36);
    write_port(0x40, static_cast<unsigned char>(divisor & 0xFF));
    write_port(0x40, static_cast<unsigned char>(divisor >> 8));
    system_ticks = 0;
}

void interrupt_tick() {
    ++system_ticks;
}

unsigned int ticks() {
    const unsigned int flags = []() {
        unsigned int saved_flags;
        asm volatile("pushf; pop %0; cli" : "=r"(saved_flags) : : "memory");
        return saved_flags;
    }();
    const unsigned int current_ticks = system_ticks;
    if ((flags & (1U << 9)) != 0) {
        asm volatile("sti" : : : "memory");
    }
    return current_ticks;
}

void sleep_seconds(unsigned int seconds) {
    const unsigned int duration = seconds * ticks_per_second;
    const unsigned int start = ticks();
    while (static_cast<unsigned int>(ticks() - start) < duration) {
        asm volatile("sti; hlt" : : : "memory");
    }
}

void delay_ms(unsigned int milliseconds) {
    // Split rather than scale once, so the multiply stays inside a thirty two
    // bit word even for the long waits the boot sequence uses.
    while (milliseconds != 0) {
        const unsigned int chunk = milliseconds > maximum_count_milliseconds
                                       ? maximum_count_milliseconds
                                       : milliseconds;
        wait_ticks(1193182U * chunk / 1000U);
        milliseconds -= chunk;
    }
}

unsigned int seconds() {
    return ticks() / ticks_per_second;
}

}
