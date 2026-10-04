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

unsigned int seconds() {
    return ticks() / ticks_per_second;
}

}
