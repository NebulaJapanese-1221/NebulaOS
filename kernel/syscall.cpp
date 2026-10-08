// Syscall dispatch for the NebulaOS x86 operating system.
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

#include "syscall.hpp"
#include "../drivers/serial.hpp"
#include "framebuffer.hpp"
#include "timer.hpp"

namespace kernel::syscall {

namespace {
struct RegisterState {
    unsigned int edi, esi, ebp, esp, ebx, edx, ecx, eax;
    unsigned int vector, error_code;
    unsigned int eip, cs, eflags, user_esp, user_ss;
};

using SyscallHandler = unsigned int (*)(RegisterState*);

SyscallHandler handlers[256] = {};

unsigned int sys_write(RegisterState* regs) {
    unsigned int fd = regs->ebx;
    const char* buf = reinterpret_cast<const char*>(regs->ecx);
    unsigned int count = regs->edx;

    if (fd == 1 || fd == 2) {
        for (unsigned int i = 0; i < count; ++i) {
            drivers::serial::write(buf[i]);
        }
        return count;
    }
    return 0xFFFFFFFF;
}

unsigned int sys_exit(RegisterState* regs) {
    (void)regs;
    for (;;) {
        asm volatile("cli; hlt");
    }
    return 0;
}

unsigned int sys_getpid(RegisterState* regs) {
    (void)regs;
    return 1;
}

unsigned int sys_open_device(RegisterState* regs) {
    const char* name = reinterpret_cast<const char*>(regs->ecx);
    unsigned int buffer = regs->edx;
    unsigned int length = regs->esi;

    // fd 3 is the framebuffer, fd 4 is the serial console.
    if (name == nullptr) {
        return 0xFFFFFFFF;
    }
    if (name[0] == 'f' && name[1] == 'b' && name[2] == '\0') {
        return 3;
    }
    if (name[0] == 's' && name[1] == 'e' && name[2] == 'r' && name[3] == '\0') {
        return 4;
    }
    return 0xFFFFFFFF;
}

unsigned int sys_get_framebuffer_info(RegisterState* regs) {
    struct __attribute__((packed)) Info {
        unsigned int buffer;
        unsigned int width;
        unsigned int height;
        unsigned int pitch;
        unsigned char bpp;
        unsigned char type;
        unsigned char red_position;
        unsigned char red_mask_size;
        unsigned char green_position;
        unsigned char green_mask_size;
        unsigned char blue_position;
        unsigned char blue_mask_size;
    };

    Info* info = reinterpret_cast<Info*>(regs->ecx);
    if (info == nullptr) {
        return 0;
    }
    info->buffer = reinterpret_cast<unsigned int>(kernel::framebuffer::buffer());
    info->width = kernel::framebuffer::width();
    info->height = kernel::framebuffer::height();
    info->pitch = 0;
    info->bpp = 32;
    info->type = 0;
    info->red_position = 16;
    info->red_mask_size = 8;
    info->green_position = 8;
    info->green_mask_size = 8;
    info->blue_position = 0;
    info->blue_mask_size = 8;
    return 1;
}

unsigned int sys_get_time(RegisterState* regs) {
    (void)regs;
    return static_cast<unsigned int>(kernel::timer::seconds());
}

unsigned int sys_get_ticks(RegisterState* regs) {
    (void)regs;
    return kernel::timer::ticks();
}

} // namespace

void initialize() {
    for (unsigned int i = 0; i < 256; ++i) {
        handlers[i] = nullptr;
    }
    handlers[1] = sys_write;   // write
    handlers[4] = sys_write;   // write (Linux compat)
    handlers[60] = sys_exit;   // exit
    handlers[20] = sys_getpid; // getpid
    handlers[200] = sys_open_device;          // open_device
    handlers[201] = sys_get_framebuffer_info; // get_framebuffer_info
    handlers[202] = sys_get_time;             // get_time_seconds
    handlers[203] = sys_get_ticks;            // get_ticks
}

extern "C" unsigned int syscall_dispatch(void* register_state) {
    RegisterState* regs = static_cast<RegisterState*>(register_state);
    unsigned int syscall_num = regs->eax;

    if (syscall_num < 256 && handlers[syscall_num] != nullptr) {
        return handlers[syscall_num](regs);
    }

    drivers::serial::write("[syscall] unknown syscall: ");
    drivers::serial::write_decimal(syscall_num);
    drivers::serial::write_newline();

    return 0xFFFFFFFF;
}

} // namespace kernel::syscall