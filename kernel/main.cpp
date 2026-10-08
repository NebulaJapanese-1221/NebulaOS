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
#include "arch/x86/tss.hpp"
#include "cpio.hpp"
#include "heap.hpp"
#include "multiboot.hpp"
#include "paging.hpp"
#include "pmm.hpp"
#include "scheduler.hpp"
#include "process.hpp"
#include "ipc.hpp"
#include "compositor.hpp"
#include "wm.hpp"
#include "elf.hpp"
#include "pci.hpp"
#include "vfs.hpp"
#include "fs/initramfs.hpp"
#include "syscall.hpp"
#include "timer.hpp"

// Simple ELF32 loader for userspace programs.
struct __attribute__((packed)) Elf32_Ehdr {
    unsigned char ident[16];
    unsigned short type;
    unsigned short machine;
    unsigned int version;
    unsigned int entry;
    unsigned int phoff;
    unsigned int shoff;
    unsigned int flags;
    unsigned short ehsize;
    unsigned short phentsize;
    unsigned short phnum;
    unsigned short shentsize;
    unsigned short shnum;
    unsigned short shstrndx;
};

struct __attribute__((packed)) Elf32_Phdr {
    unsigned int type;
    unsigned int offset;
    unsigned int vaddr;
    unsigned int paddr;
    unsigned int filesz;
    unsigned int memsz;
    unsigned int flags;
    unsigned int align;
};

const unsigned int PT_LOAD = 1;
const unsigned int PT_DYNAMIC = 2;
const unsigned int PT_INTERP = 3;

const unsigned int ELF_MAGIC = 0x464C457F;

const unsigned int USER_BASE = 0x08048000;
const unsigned int USER_STACK_TOP = 0xBFFFFFFF;

// Allocates a frame and maps it at the given virtual address with the
// given flags. Returns false when the allocator is out of frames.
bool map_frame_at(unsigned int vaddr, std::uint32_t flags) {
    const kernel::memory::pmm::FrameResult frame =
        kernel::memory::pmm::allocate_frame(
            kernel::memory::pmm::FrameFlags::ZEROED);
    if (!frame) {
        return false;
    }
    return kernel::memory::paging::map_page(vaddr, frame.value, flags);
}

bool load_elf(const unsigned char* data, unsigned int size,
              unsigned int* entry_out, unsigned int* stack_top_out) {
    if (size < sizeof(Elf32_Ehdr)) {
        return false;
    }
    const Elf32_Ehdr* ehdr = reinterpret_cast<const Elf32_Ehdr*>(data);
    if (ehdr->ident[0] != 0x7F || ehdr->ident[1] != 'E' ||
        ehdr->ident[2] != 'L' || ehdr->ident[3] != 'F') {
        return false;
    }
    if (ehdr->type != 2 || ehdr->machine != 3) {
        return false;
    }

    const std::uint32_t user_flags =
        kernel::memory::paging::PAGE_PRESENT |
        kernel::memory::paging::PAGE_WRITABLE |
        kernel::memory::paging::PAGE_USER;

    unsigned int max_vaddr = 0;
    for (unsigned int i = 0; i < ehdr->phnum; ++i) {
        const Elf32_Phdr* phdr = reinterpret_cast<const Elf32_Phdr*>(
            data + ehdr->phoff + i * ehdr->phentsize);
        if (phdr->type == PT_LOAD) {
            if (phdr->vaddr + phdr->memsz > max_vaddr) {
                max_vaddr = phdr->vaddr + phdr->memsz;
            }
            // Map and copy the file-backed part of the segment.
            for (unsigned int offset = 0; offset < phdr->filesz;
                 offset += 4096) {
                unsigned int vaddr = phdr->vaddr + offset;
                if (!map_frame_at(vaddr & 0xFFFFF000, user_flags)) {
                    return false;
                }
                unsigned int copy_size =
                    (offset + 4096 < phdr->filesz)
                        ? 4096 : (phdr->filesz - offset);
                for (unsigned int j = 0; j < copy_size; ++j) {
                    *reinterpret_cast<unsigned char*>(vaddr + j) =
                        data[phdr->offset + offset + j];
                }
            }
            // Zero-fill the rest of memsz. The frames are already
            // zeroed by the ZEROED flag, so only the pages that were
            // not touched above need to be mapped here.
            for (unsigned int offset = phdr->filesz;
                 offset < phdr->memsz; offset += 4096) {
                unsigned int vaddr = phdr->vaddr + offset;
                if (!map_frame_at(vaddr & 0xFFFFF000, user_flags)) {
                    return false;
                }
            }
        }
    }

    // Map user stack
    unsigned int stack_pages = 16;
    for (unsigned int i = 0; i < stack_pages; ++i) {
        unsigned int vaddr = USER_STACK_TOP - (i + 1) * 4096 + 1;
        if (!map_frame_at(vaddr & 0xFFFFF000, user_flags)) {
            return false;
        }
    }

    *entry_out = ehdr->entry;
    *stack_top_out = USER_STACK_TOP;
    return true;
}

bool extract_initrd(const unsigned int multiboot_info_address,
                    const unsigned char** initrd_data, unsigned int* initrd_size) {
    const kernel::multiboot::Information* info =
        kernel::multiboot::information(multiboot_info_address);
    if (info == nullptr || info->modules_count == 0) {
        return false;
    }
    const kernel::multiboot::Module* modules =
        reinterpret_cast<const kernel::multiboot::Module*>(info->modules_address);
    for (unsigned int i = 0; i < info->modules_count; ++i) {
        unsigned int mod_start = modules[i].mod_start;
        unsigned int mod_end = modules[i].mod_end;
        if (mod_end > mod_start) {
            *initrd_data = reinterpret_cast<const unsigned char*>(mod_start);
            *initrd_size = mod_end - mod_start;
            return true;
        }
    }
    return false;
}

bool find_file_in_cpio(const unsigned char* cpio_data, unsigned int cpio_size,
                       const char* filename,
                       const unsigned char** file_data, unsigned int* file_size) {
    const unsigned char* data = kernel::cpio::find_file(cpio_data, cpio_size, filename, file_size);
    if (data == nullptr) {
        return false;
    }
    *file_data = data;
    return true;
}

void launch_userspace_program(const unsigned int multiboot_info_address) {
    const unsigned char* initrd_data;
    unsigned int initrd_size;
    if (!extract_initrd(multiboot_info_address, &initrd_data, &initrd_size)) {
        drivers::serial::write_line("[init] No initrd module found");
        return;
    }
    drivers::serial::write("[init] Found initrd: ");
    drivers::serial::write_decimal(initrd_size);
    drivers::serial::write(" bytes\n");

    const unsigned char* program_data;
    unsigned int program_size;
    if (!find_file_in_cpio(initrd_data, initrd_size, "console",
                           &program_data, &program_size)) {
        drivers::serial::write_line("[init] No ELF program found in initrd");
        return;
    }
    drivers::serial::write("[init] Found ELF program: ");
    drivers::serial::write_decimal(program_size);
    drivers::serial::write(" bytes\n");

    unsigned int entry, stack_top;
    if (!load_elf(program_data, program_size, &entry, &stack_top)) {
        drivers::serial::write_line("[init] Failed to load ELF");
        return;
    }

    drivers::serial::write("[init] Launching userspace program at entry 0x");
    drivers::serial::write_decimal(entry);
    drivers::serial::write(" with stack 0x");
    drivers::serial::write_decimal(stack_top);
    drivers::serial::write("\n");

    // Copy the program name onto the user stack so the argv pointer is
    // valid in user space. The kernel can write here because load_elf already
    // mapped the user stack pages into the kernel page directory.
    const char prog_name[] = "console";
    const unsigned int prog_name_len = sizeof(prog_name);
    unsigned int name_addr = stack_top - 64;
    for (unsigned int i = 0; i < prog_name_len; ++i) {
        *reinterpret_cast<unsigned char*>(name_addr + i) = prog_name[i];
    }

    // Push argc, argv[0], and the NULL argv terminator onto the user stack,
    // then switch to user mode via iret. The crt0 entry point expects argc at
    // [esp] and argv at [esp+4].
    unsigned int user_esp;
    asm volatile(
        "mov %1, %%esp\n"           // switch to the user stack
        "pushl $0\n"                // NULL argv terminator
        "pushl %2\n"                // argv[0] = program name address
        "pushl %3\n"                // argc = 1
        "mov %%esp, %0\n"           // capture user esp (points to argc)
        "pushl %4\n"                // user ss
        "pushl %0\n"                // user esp
        "pushfl\n"                  // eflags
        "pushl %5\n"                // user cs
        "pushl %6\n"                // entry (eip)
        "iret\n"
        : "=r"(user_esp)
        : "r"(stack_top), "r"(name_addr), "r"(1),
          "r"(kernel::tss::user_data_selector), "r"(kernel::tss::user_code_selector),
          "r"(entry)
        : "memory"
    );
}

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

    // Enable paging with the new page directory first, then install IDT/TSS
    // so they use virtual addresses valid in the new page directory.
    if (!kernel::memory::paging::enable()) {
        halt_with_error("PAGING COULD NOT BE ENABLED");
    }
    kernel::tss::reset_for_reinit();
    kernel::interrupts::install_handlers();
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

    kernel::syscall::initialize();
    run_stage("SYSCALL INTERFACE", "OK", 0x0A);

    // The virtual filesystem has to be up before the initramfs is
    // mounted, and the initramfs has to be mounted before the first
    // program is loaded, because the loader now resolves its files
    // through the VFS.
    if (!kernel::vfs::initialize()) {
        halt_with_error("VIRTUAL FILE SYSTEM UNAVAILABLE");
    }
    run_stage("VIRTUAL FILE SYSTEM", "OK", 0x0A);

    const unsigned char* initrd_data = nullptr;
    unsigned int initrd_size = 0;
    if (extract_initrd(multiboot_info_address, &initrd_data,
                           &initrd_size)) {
        if (kernel::initramfs::mount(initrd_data, initrd_size, "/") == 0) {
            run_stage("INITRAMFS MOUNTED", "OK", 0x0A);
        } else {
            run_stage("INITRAMFS MOUNT FAILED", "WARN", 0x0E);
        }
    } else {
        run_stage("NO INITRAMFS", "WARN", 0x0E);
    }

    // The dynamic linker, the scheduler, and the process table are
    // brought up before the first program runs, so that exec can
    // create a process and the scheduler can run it.
    kernel::elf::initialize();
    run_stage("DYNAMIC LINKER", "OK", 0x0A);

    kernel::scheduler::initialize();
    run_stage("SCHEDULER", "OK", 0x0A);

    kernel::process::initialize();
    run_stage("PROCESS MANAGEMENT", "OK", 0x0A);

    kernel::pci::initialize();
    run_stage("PCI BUS", "OK", 0x0A);

    launch_userspace_program(multiboot_info_address);

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

    // The compositor and the window manager own the desktop, so
    // they are brought up before the shell draws into it. Both
    // are configured from the framebuffer the boot stages
    // already set up.
    kernel::compositor::Config compositor_config = {};
    compositor_config.screen_width = drivers::graphics::width();
    compositor_config.screen_height = drivers::graphics::height();
    compositor_config.screen_bpp = 32;
    compositor_config.background_color = 0x00000000;
    compositor_config.double_buffering = true;
    compositor_config.vsync = false;
    compositor_config.damage_tracking = true;
    compositor_config.alpha_blending = true;
    compositor_config.shadows = false;
    compositor_config.cursor_x =
        drivers::graphics::width() / 2;
    compositor_config.cursor_y =
        drivers::graphics::height() / 2;
    compositor_config.cursor_visible = true;
    compositor_config.cursor_surface = nullptr;
    kernel::compositor::initialize(compositor_config);

    kernel::wm::Config wm_config = {};
    wm_config.active_border_color = 0x00E6EDF3;
    wm_config.inactive_border_color = 0x007A8A99;
    wm_config.active_titlebar_color = 0x002A3A4A;
    wm_config.inactive_titlebar_color = 0x001A2A3A;
    wm_config.active_titlebar_text_color = 0x00FFFFFF;
    wm_config.inactive_titlebar_text_color = 0x00A0A0A0;
    wm_config.close_button_color = 0x00FF7777;
    wm_config.maximize_button_color = 0x0088E0A0;
    wm_config.minimize_button_color = 0x00E6E060;
    wm_config.button_text_color = 0x00FFFFFF;
    wm_config.background_color = 0x00000000;
    wm_config.border_width = 2;
    wm_config.titlebar_height = 20;
    wm_config.button_size = 14;
    wm_config.resize_handle_size = 6;
    wm_config.min_window_width = 64;
    wm_config.min_window_height = 48;
    wm_config.focus_follows_mouse = true;
    wm_config.raise_on_focus = true;
    wm_config.snap_to_edges = false;
    wm_config.snap_distance = 10;
    wm_config.double_click_maximize = true;
    wm_config.double_click_time = 400;
    kernel::wm::initialize(wm_config);

    shell::run();
}