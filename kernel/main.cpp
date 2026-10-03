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
#include "../drivers/serial.hpp"
#include "../shell/shell.hpp"
#include "arch/x86/interrupts.hpp"
#include "heap.hpp"
#include "multiboot.hpp"
#include "paging.hpp"
#include "pmm.hpp"
#include "timer.hpp"

namespace {
const unsigned int boot_text = 0x00E6EDF3;
const unsigned int boot_ok = 0x0088E0A0;
const unsigned int boot_error = 0x00FF7777;
const unsigned int status_top = 86;
const unsigned int status_spacing = 22;

// The heap is carved from frames inside the identity mapping, so it must stay
// comfortably below the memory the kernel was told about.
const unsigned int heap_megabytes = 16;
const unsigned int maximum_identity_megabytes = 256;

void show_boot_screen() {
    drivers::graphics::clear(0x00000000);
    drivers::graphics::draw_text(24, 24, "NEBULAOS BOOT", boot_text, 2);
    drivers::graphics::draw_text(24, 48, "INITIALIZING SYSTEM COMPONENTS", boot_text, 1);
}

void show_status(unsigned int row, const char* text, unsigned int color) {
    drivers::graphics::draw_text(24, status_top + row * status_spacing, text, color, 1);
    drivers::serial::write("[boot] ");
    drivers::serial::write_line(text);
    drivers::graphics::present();
}

void write_vga_line(const char* prefix, const char* message) {
    volatile unsigned short* const text_buffer =
        reinterpret_cast<volatile unsigned short*>(0xB8000);
    unsigned int index = 0;
    for (unsigned int letter = 0; prefix[letter] != '\0' && index < 80; ++letter) {
        text_buffer[index] = static_cast<unsigned short>(0x4F00 | prefix[letter]);
        ++index;
    }
    for (unsigned int letter = 0; message[letter] != '\0' && index < 80; ++letter) {
        text_buffer[index] = static_cast<unsigned short>(0x4F00 | message[letter]);
        ++index;
    }
}

[[noreturn]] void halt_with_error(const char* message) {
    const char prefix[] = "NEBULAOS BOOT ERROR: ";
    write_vga_line(prefix, message);
    drivers::serial::write_line(prefix);
    drivers::serial::write_line(message);
    for (;;) {
        asm volatile("cli; hlt");
    }
}

// Identity-map everything the firmware reported, so that every pointer the
// kernel obtains from the page frame allocator stays directly dereferenceable.
unsigned int identity_megabytes_for(unsigned long long detected_bytes) {
    const unsigned long long megabyte = 1024ULL * 1024ULL;
    unsigned long long megabytes = (detected_bytes + megabyte - 1) / megabyte;
    if (megabytes < 1) {
        megabytes = 1;
    }
    if (megabytes > maximum_identity_megabytes) {
        megabytes = maximum_identity_megabytes;
    }
    return static_cast<unsigned int>(megabytes);
}
}

extern "C" void kmain(unsigned int boot_magic, unsigned int multiboot_info_address) {
    drivers::serial::initialize();

    if (boot_magic != kernel::multiboot::handoff_magic) {
        halt_with_error("INVALID MULTIBOOT HANDOFF");
    }

    if (!kernel::memory::pmm::initialize(multiboot_info_address)) {
        halt_with_error("MEMORY MAP UNAVAILABLE");
    }

    const unsigned int identity_megabytes =
        identity_megabytes_for(kernel::memory::pmm::detected_bytes());
    if (!kernel::memory::paging::initialize(identity_megabytes)) {
        halt_with_error("PAGING TABLES UNAVAILABLE");
    }

    // The IDT has to be in place before paging is switched on, otherwise a
    // fault during translation cannot be reported.
    kernel::interrupts::install_handlers();
    if (!kernel::memory::paging::enable()) {
        halt_with_error("PAGING COULD NOT BE ENABLED");
    }

    kernel::memory::heap::initialize(heap_megabytes);
    if (!kernel::memory::heap::is_initialized()) {
        halt_with_error("KERNEL HEAP UNAVAILABLE");
    }

    const char* framebuffer_reason = nullptr;
    if (!drivers::graphics::initialize(multiboot_info_address,
                                       &framebuffer_reason)) {
        halt_with_error(framebuffer_reason == nullptr ? "FRAMEBUFFER UNAVAILABLE"
                                                      : framebuffer_reason);
    }

    kernel::interrupts::initialize();

    show_boot_screen();
    show_status(0, "GRUB MULTIBOOT HANDOFF: READY", boot_ok);
    show_status(1, drivers::graphics::is_double_buffered()
                       ? "FRAMEBUFFER + DOUBLE BUFFER: READY"
                       : "FRAMEBUFFER: READY (NO BACK BUFFER)",
                drivers::graphics::is_double_buffered() ? boot_ok : boot_error);
    show_status(2, "PHYSICAL MEMORY MANAGER: READY", boot_ok);
    show_status(3, "PAGING + KERNEL HEAP: READY", boot_ok);

    show_status(4, "PS/2 KEYBOARD: INITIALIZING", boot_text);
    drivers::keyboard::initialize();
    show_status(4, "PS/2 KEYBOARD: READY", boot_ok);

    show_status(5, "PS/2 MOUSE: INITIALIZING", boot_text);
    const bool mouse_available = drivers::mouse::initialize(
        drivers::graphics::width(), drivers::graphics::height());
    show_status(5, mouse_available ? "PS/2 MOUSE: READY" : "PS/2 MOUSE: NOT FOUND",
                mouse_available ? boot_ok : boot_error);

    show_status(6, "STARTING GRAPHICAL DESKTOP", boot_text);
    kernel::timer::sleep_seconds(1);
    shell::run(mouse_available);
}