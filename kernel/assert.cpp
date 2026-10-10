// Kernel assertions and panic handling implementation for NebulaOS.
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

#include "assert.hpp"
#include "log.hpp"
#include "../drivers/serial.hpp"
#include <cstdarg>
#include <cstring>

namespace kernel::assert {

namespace {

AssertHandler g_assert_handler = default_assert_handler;

void print_registers(const RegisterState* regs) {
    drivers::serial::write_line("+--------------------------------------------------------------+");
    drivers::serial::write_line("|                    REGISTER DUMP                            |");
    drivers::serial::write_line("+--------------------------------------------------------------+");
    
    char buf[128];
    
    drivers::serial::write("EAX: 0x");
    drivers::serial::write_hex(regs->eax);
    drivers::serial::write("  EBX: 0x");
    drivers::serial::write_hex(regs->ebx);
    drivers::serial::write("  ECX: 0x");
    drivers::serial::write_hex(regs->ecx);
    drivers::serial::write("  EDX: 0x");
    drivers::serial::write_hex(regs->edx);
    drivers::serial::write_newline();
    
    drivers::serial::write("ESI: 0x");
    drivers::serial::write_hex(regs->esi);
    drivers::serial::write("  EDI: 0x");
    drivers::serial::write_hex(regs->edi);
    drivers::serial::write("  EBP: 0x");
    drivers::serial::write_hex(regs->ebp);
    drivers::serial::write("  ESP: 0x");
    drivers::serial::write_hex(regs->esp);
    drivers::serial::write_newline();
    
    drivers::serial::write("EIP: 0x");
    drivers::serial::write_hex(regs->eip);
    drivers::serial::write("  EFLAGS: 0x");
    drivers::serial::write_hex(regs->eflags);
    drivers::serial::write("  CS: 0x");
    drivers::serial::write_hex(regs->cs);
    drivers::serial::write("  SS: 0x");
    drivers::serial::write_hex(regs->user_ss);
    drivers::serial::write_newline();
    
    // CR2 - page fault address
    std::uint32_t cr2;
    asm volatile("mov %%cr2, %0" : "=r"(cr2));
    drivers::serial::write("CR2 (fault addr): 0x");
    drivers::serial::write_hex(cr2);
    drivers::serial::write_newline();
    
    // CR0
    std::uint32_t cr0;
    asm volatile("mov %%cr0, %0" : "=r"(cr0));
    drivers::serial::write("CR0: 0x");
    drivers::serial::write_hex(cr0);
    drivers::serial::write_newline();
    
    // CR3
    std::uint32_t cr3;
    asm volatile("mov %%cr3, %0" : "=r"(cr3));
    drivers::serial::write("CR3 (page dir): 0x");
    drivers::serial::write_hex(cr3);
    drivers::serial::write_newline();
}

void print_stack_trace_impl() {
    drivers::serial::write_line("+--------------------------------------------------------------+");
    drivers::serial::write_line("|                    STACK TRACE                              |");
    drivers::serial::write_line("+--------------------------------------------------------------+");
    
    std::uint32_t frames[32];
    int count = backtrace(frames, 32);
    
    for (int i = 0; i < count; ++i) {
        char buf[64];
        drivers::serial::write("  #");
        drivers::serial::write_decimal(i);
        drivers::serial::write("  0x");
        drivers::serial::write_hex(frames[i]);
        drivers::serial::write_newline();
    }
    
    if (count == 0) {
        drivers::serial::write_line("  (no frames available)");
    }
}

} // namespace

void set_assert_handler(AssertHandler handler) {
    g_assert_handler = handler ? handler : default_assert_handler;
}

void default_assert_handler(const char* expr, const char* file, int line,
                            const char* func, const char* msg) {
    // Disable interrupts
    asm volatile("cli");
    
    // Log to serial
    drivers::serial::write_line("");
    drivers::serial::write_line("+==============================================================+");
    drivers::serial::write_line("|                    KERNEL ASSERTION FAILED                  |");
    drivers::serial::write_line("+==============================================================+");
    drivers::serial::write_line("");
    
    drivers::serial::write("Expression: ");
    drivers::serial::write_line(expr);
    
    drivers::serial::write("File: ");
    drivers::serial::write_line(file);
    
    drivers::serial::write("Line: ");
    char line_buf[16];
    int n = line;
    int i = 0;
    if (n == 0) {
        line_buf[i++] = '0';
    } else {
        char tmp[16];
        int j = 0;
        while (n > 0) {
            tmp[j++] = '0' + (n % 10);
            n /= 10;
        }
        while (j > 0) {
            line_buf[i++] = tmp[--j];
        }
    }
    line_buf[i] = '\0';
    drivers::serial::write_line(line_buf);
    
    if (func != nullptr) {
        drivers::serial::write("Function: ");
        drivers::serial::write_line(func);
    }
    
    if (msg != nullptr && msg[0] != '\0') {
        drivers::serial::write("Message: ");
        drivers::serial::write_line(msg);
    }
    
    drivers::serial::write_line("");
    
    // Print stack trace
    print_stack_trace_impl();
    
    drivers::serial::write_line("");
    drivers::serial::write_line("System halted.");
    drivers::serial::write_line("+==============================================================+");
    
    // Also log to kernel log buffer
    char log_buf[256];
    int pos = 0;
    pos += kernel::log::write(log_buf + pos, sizeof(log_buf) - pos, "ASSERT: %s at %s:%d", expr, file, line);
    if (func) {
        pos += kernel::log::write(log_buf + pos, sizeof(log_buf) - pos, " in %s", func);
    }
    if (msg && msg[0]) {
        pos += kernel::log::write(log_buf + pos, sizeof(log_buf) - pos, ": %s", msg);
    }
    kernel::log::err(log_buf);
    
    // Halt
    for (;;) {
        asm volatile("cli; hlt");
    }
}

[[noreturn]] void panic(const char* fmt, ...) {
    asm volatile("cli");
    
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    
    // Simple vsnprintf
    int pos = 0;
    const char* p = fmt;
    while (*p && pos < (int)sizeof(buf) - 1) {
        if (*p == '%') {
            ++p;
            if (*p == 's') {
                const char* s = va_arg(ap, const char*);
                while (*s && pos < (int)sizeof(buf) - 1) {
                    buf[pos++] = *s++;
                }
            } else if (*p == 'd' || *p == 'x') {
                unsigned int val = va_arg(ap, unsigned int);
                char tmp[16];
                int len = 0;
                if (*p == 'd') {
                    // decimal
                    int is_neg = 0;
                    if ((int)val < 0) {
                        is_neg = 1;
                        val = -val;
                    }
                    do {
                        tmp[len++] = '0' + (val % 10);
                        val /= 10;
                    } while (val > 0);
                    if (is_neg) tmp[len++] = '-';
                } else {
                    // hex
                    do {
                        int d = val & 0xF;
                        tmp[len++] = (d < 10) ? '0' + d : 'A' + (d - 10);
                        val >>= 4;
                    } while (val > 0);
                    tmp[len++] = 'x';
                    tmp[len++] = '0';
                }
                while (len > 0 && pos < (int)sizeof(buf) - 1) {
                    buf[pos++] = tmp[--len];
                }
            } else if (*p == 'c') {
                buf[pos++] = (char)va_arg(ap, int);
            } else {
                buf[pos++] = *p;
            }
        } else {
            buf[pos++] = *p;
        }
        ++p;
    }
    buf[pos] = '\0';
    va_end(ap);
    
    drivers::serial::write_line("");
    drivers::serial::write_line("+==============================================================+");
    drivers::serial::write_line("|                    KERNEL PANIC                             |");
    drivers::serial::write_line("+==============================================================+");
    drivers::serial::write_line("");
    drivers::serial::write_line(buf);
    drivers::serial::write_line("");
    
    print_stack_trace_impl();
    
    drivers::serial::write_line("");
    drivers::serial::write_line("System halted.");
    
    for (;;) {
        asm volatile("cli; hlt");
    }
}

[[noreturn]] void panic_with_regs(const RegisterState* regs, const char* msg) {
    asm volatile("cli");
    
    drivers::serial::write_line("");
    drivers::serial::write_line("+==============================================================+");
    drivers::serial::write_line("|                    KERNEL PANIC                             |");
    drivers::serial::write_line("+==============================================================+");
    drivers::serial::write_line("");
    
    if (msg) {
        drivers::serial::write_line(msg);
        drivers::serial::write_line("");
    }
    
    print_registers(regs);
    drivers::serial::write_line("");
    print_stack_trace_impl();
    
    drivers::serial::write_line("");
    drivers::serial::write_line("System halted.");
    
    for (;;) {
        asm volatile("cli; hlt");
    }
}

int backtrace(std::uint32_t* frames, int max_frames) noexcept {
    if (frames == nullptr || max_frames <= 0) {
        return 0;
    }
    
    std::uint32_t ebp;
    asm volatile("mov %%ebp, %0" : "=r"(ebp));
    
    int count = 0;
    std::uint32_t* frame = reinterpret_cast<std::uint32_t*>(ebp);
    
    while (count < max_frames && frame != nullptr) {
        // Frame layout: [prev_ebp, return_address, ...]
        std::uint32_t return_addr = frame[1];
        
        // Validate return address (must be in kernel space)
        if (return_addr >= 0xC0000000 && return_addr < 0xFFFFFFFF) {
            frames[count++] = return_addr;
        }
        
        // Move to previous frame
        std::uint32_t prev_ebp = frame[0];
        if (prev_ebp <= (std::uint32_t)frame || prev_ebp >= 0xFFFFF000) {
            break;  // Invalid frame pointer
        }
        frame = reinterpret_cast<std::uint32_t*>(prev_ebp);
    }
    
    return count;
}

void print_stack_trace() noexcept {
    print_stack_trace_impl();
}

} // namespace kernel::assert