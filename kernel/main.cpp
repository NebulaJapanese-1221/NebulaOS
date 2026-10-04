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
#include "../drivers/vga.hpp"
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

// Every initialisation stage holds the screen this long. The waits go through a
// polled PIT channel rather than the clock interrupt, because most of the
// sequence runs before the interrupt controller is installed and waiting on
// clock ticks there would never return.
const unsigned int stage_delay_ms = 3000;

void show_text_boot_screen() {
    drivers::vga::clear();
    drivers::vga::write_line("==============================================");
    drivers::vga::write_line("            N E B U L A O S                   ");
    drivers::vga::write_line("==============================================");
    drivers::vga::write_line("");
    drivers::vga::write_line("   32 bit protected mode kernel, booting...");
    drivers::vga::write_line("");
}

void show_text_stage(const char* label, const char* state, unsigned char attribute) {
    const char prefix[] = "  [";
    drivers::vga::write(prefix);
    drivers::vga::set_attribute(attribute);
    drivers::vga::write("....");
    drivers::vga::set_attribute(0x07);
    drivers::vga::write("] ");
    drivers::vga::write(label);
    drivers::vga::write(" ");
    drivers::vga::write_line(state);
    drivers::vga::set_attribute(0x07);

    drivers::serial::write("  ");
    drivers::serial::write_line(label);
    drivers::serial::write("    ");
    drivers::serial::write_line(state);
}

// Holds the boot screen on a stage for stage_delay_ms so each step is legible
// before the next one paints over it.
void run_stage(const char* label, const char* state, unsigned char attribute) {
    show_text_stage(label, state, attribute);
    kernel::timer::delay_ms(stage_delay_ms);
}

void show_graphics_boot_screen() {
    drivers::graphics::clear(0x00000000);
    drivers::graphics::draw_text(24, 24, "NEBULAOS BOOT", boot_text, 2);
    drivers::graphics::draw_text(24, 48, "INITIALIZING SYSTEM COMPONENTS", boot_text, 1);
    drivers::graphics::present();
}

void show_graphics_status(unsigned int row, const char* text, unsigned int color) {
    drivers::graphics::draw_text(24, status_top + row * status_spacing, text, color, 1);
    drivers::serial::write("[boot] ");
    drivers::serial::write_line(text);
    drivers::graphics::present();
}

// The VGA text buffer is the only output that exists before the framebuffer is
// up, so fatal errors are mirrored there as well as to the serial port.
[[noreturn]] void halt_with_error(const char* message) {
    const char prefix[] = "NEBULAOS BOOT ERROR: ";
    drivers::vga::write_line(prefix);
    drivers::vga::write_line(message);
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
    drivers::vga::initialize();
    show_text_boot_screen();
    kernel::timer::delay_ms(stage_delay_ms);

    // Text mode carries the whole of the hardware bring up, because it is the
    // only output that exists before a framebuffer has been handed over.
    drivers::serial::initialize();
    run_stage("SERIAL PORT", "OK", 0x0A);

    if (boot_magic != kernel::multiboot::handoff_magic) {
        halt_with_error("INVALID MULTIBOOT HANDOFF");
    }
    run_stage("MULTIBOOT HANDOFF", "OK", 0x0A);

    if (!kernel::memory::pmm::initialize(multiboot_info_address)) {
        halt_with_error("MEMORY MAP UNAVAILABLE");
    }
    run_stage("PHYSICAL MEMORY MANAGER", "OK", 0x0A);

    drivers::serial::write("memory detected ");
    drivers::serial::write_decimal(
        static_cast<unsigned int>(kernel::memory::pmm::detected_bytes() / (1024 * 1024)));
    drivers::serial::write(" MB, free frames ");
    drivers::serial::write_decimal(kernel::memory::pmm::free_frames());
    drivers::serial::write_newline();

    const unsigned int identity_megabytes =
        identity_megabytes_for(kernel::memory::pmm::detected_bytes());
    if (!kernel::memory::paging::initialize(identity_megabytes)) {
        halt_with_error("PAGING TABLES UNAVAILABLE");
    }
    run_stage("PAGING TABLES", "OK", 0x0A);

    // The IDT has to be in place before paging is switched on, otherwise a
    // fault during translation cannot be reported.
    kernel::interrupts::install_handlers();
    if (!kernel::memory::paging::enable()) {
        halt_with_error("PAGING COULD NOT BE ENABLED");
    }
    run_stage("PAGING + FAULT HANDLERS", "OK", 0x0A);

    kernel::memory::heap::initialize(heap_megabytes);
    if (!kernel::memory::heap::is_initialized()) {
        halt_with_error("KERNEL HEAP UNAVAILABLE");
    }
    run_stage("KERNEL HEAP", "OK", 0x0A);

    drivers::serial::write("identity map ");
    drivers::serial::write_decimal(identity_megabytes);
    drivers::serial::write(" MB, heap ");
    drivers::serial::write_decimal(kernel::memory::heap::total_bytes() / 1024);
    drivers::serial::write(" KB, free frames ");
    drivers::serial::write_decimal(kernel::memory::pmm::free_frames());
    drivers::serial::write_newline();

    const char* framebuffer_reason = nullptr;
    if (!drivers::graphics::initialize(multiboot_info_address,
                                       &framebuffer_reason)) {
        halt_with_error(framebuffer_reason == nullptr ? "FRAMEBUFFER UNAVAILABLE"
                                                      : framebuffer_reason);
    }
    run_stage("FRAMEBUFFER", "OK", 0x0A);

    kernel::interrupts::initialize();
    run_stage("INTERRUPT CONTROLLER", "OK", 0x0A);

    // From here the desktop can be drawn, so the remaining stages report into
    // the framebuffer instead of the text buffer.
    show_graphics_boot_screen();
    kernel::timer::delay_ms(stage_delay_ms);

    show_graphics_status(0, "GRUB MULTIBOOT HANDOFF: READY", boot_ok);
    kernel::timer::delay_ms(stage_delay_ms);
    show_graphics_status(1, drivers::graphics::is_double_buffered()
                       ? "FRAMEBUFFER + DOUBLE BUFFER: READY"
                       : "FRAMEBUFFER: READY (NO BACK BUFFER)",
               drivers::graphics::is_double_buffered() ? boot_ok : boot_error);
    kernel::timer::delay_ms(stage_delay_ms);
    show_graphics_status(2, "PHYSICAL MEMORY MANAGER: READY", boot_ok);
    kernel::timer::delay_ms(stage_delay_ms);
    show_graphics_status(3, "PAGING + KERNEL HEAP: READY", boot_ok);
    kernel::timer::delay_ms(stage_delay_ms);

    show_graphics_status(4, "PS/2 KEYBOARD: INITIALIZING", boot_text);
    drivers::keyboard::initialize();
    show_graphics_status(4, "PS/2 KEYBOARD: READY", boot_ok);
    kernel::timer::delay_ms(stage_delay_ms);

    show_graphics_status(5, "PS/2 MOUSE: INITIALIZING", boot_text);
    const bool mouse_available = drivers::mouse::initialize(
        drivers::graphics::width(), drivers::graphics::height());
    show_graphics_status(5, mouse_available ? "PS/2 MOUSE: READY" : "PS/2 MOUSE: NOT FOUND",
                mouse_available ? boot_ok : boot_error);
    kernel::timer::delay_ms(stage_delay_ms);

    show_graphics_status(6, "STARTING GRAPHICAL DESKTOP", boot_text);
    kernel::timer::delay_ms(stage_delay_ms);
    shell::run();
}