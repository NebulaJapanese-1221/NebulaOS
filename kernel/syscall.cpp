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

} // namespace

void initialize() {
    for (unsigned int i = 0; i < 256; ++i) {
        handlers[i] = nullptr;
    }
    handlers[1] = sys_write;   // write
    handlers[4] = sys_write;   // write (Linux compat)
    handlers[60] = sys_exit;   // exit
    handlers[1] = sys_write;   // write
    handlers[20] = sys_getpid; // getpid
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