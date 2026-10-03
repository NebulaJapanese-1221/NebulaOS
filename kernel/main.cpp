// Kernel initialization for the NebulaOS x86 operating system.
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

#include "../drivers/graphics.hpp"
#include "../drivers/keyboard.hpp"
#include "../drivers/mouse.hpp"
#include "../shell/shell.hpp"

namespace {
const unsigned int boot_text = 0x00E6EDF3;
const unsigned int boot_ok = 0x0088E0A0;
const unsigned int boot_error = 0x00FF7777;

void write_port(unsigned short port, unsigned char value) {
    asm volatile("outb %0, %1" : : "a"(value), "Nd"(port));
}

unsigned short read_pit_counter() {
    write_port(0x43, 0x00);
    unsigned char low;
    unsigned char high;
    asm volatile("inb %1, %0" : "=a"(low) : "Nd"(0x40));
    asm volatile("inb %1, %0" : "=a"(high) : "Nd"(0x40));
    return static_cast<unsigned short>(low | (high << 8));
}

void initialize_boot_timer() {
    const unsigned short divisor = 11932;
    write_port(0x43, 0x34);
    write_port(0x40, static_cast<unsigned char>(divisor & 0xFF));
    write_port(0x40, static_cast<unsigned char>(divisor >> 8));
}

void delay_seconds(unsigned int seconds) {
    unsigned short previous = read_pit_counter();
    unsigned int ticks_remaining = seconds * 100;
    while (ticks_remaining != 0) {
        const unsigned short current = read_pit_counter();
        if (current > previous) {
            --ticks_remaining;
        }
        previous = current;
        asm volatile("pause");
    }
}

void show_boot_screen() {
    drivers::graphics::clear(0x00000000);
    drivers::graphics::draw_text(24, 24, "NEBULAOS TEXT MODE BOOT", boot_text, 2);
    drivers::graphics::draw_text(24, 48, "INITIALIZING SYSTEM COMPONENTS", boot_text, 1);
}

void show_status(unsigned int row, const char* text, unsigned int color) {
    drivers::graphics::draw_text(24, 86 + row * 24, text, color, 1);
}

[[noreturn]] void halt_with_error(const char* message) {
    volatile unsigned short* const text_buffer =
        reinterpret_cast<volatile unsigned short*>(0xB8000);
    const char prefix[] = "NEBULAOS BOOT ERROR: ";
    unsigned int index = 0;
    for (unsigned int letter = 0; prefix[letter] != '\0' && index < 80; ++letter) {
        text_buffer[index] = static_cast<unsigned short>(0x4F00 | prefix[letter]);
        ++index;
    }
    for (unsigned int letter = 0; message[letter] != '\0' && index < 80; ++letter) {
        text_buffer[index] = static_cast<unsigned short>(0x4F00 | message[letter]);
        ++index;
    }
    for (;;) {
        asm volatile("cli; hlt");
    }
}
}

extern "C" void kmain(unsigned int boot_magic, unsigned int multiboot_info_address) {
    if (boot_magic != 0x2BADB002) {
        halt_with_error("INVALID MULTIBOOT HANDOFF");
    }
    if (!drivers::graphics::initialize(multiboot_info_address)) {
        halt_with_error("FRAMEBUFFER UNAVAILABLE");
    }

    initialize_boot_timer();
    show_boot_screen();
    show_status(0, "GRUB MULTIBOOT HANDOFF: READY", boot_ok);
    delay_seconds(2);

    show_status(1, "FRAMEBUFFER: READY", boot_ok);
    delay_seconds(2);

    show_status(2, "PS/2 KEYBOARD: INITIALIZING", boot_text);
    drivers::keyboard::initialize();
    show_status(2, "PS/2 KEYBOARD: READY", boot_ok);
    delay_seconds(2);

    show_status(3, "PS/2 MOUSE: INITIALIZING", boot_text);
    const bool mouse_available = drivers::mouse::initialize(
        drivers::graphics::width(), drivers::graphics::height());
    show_status(3, mouse_available ? "PS/2 MOUSE: READY" : "PS/2 MOUSE: NOT FOUND",
                mouse_available ? boot_ok : boot_error);
    delay_seconds(2);

    show_status(4, "STARTING GRAPHICAL DESKTOP", boot_text);
    delay_seconds(1);
    shell::run(mouse_available);
}
