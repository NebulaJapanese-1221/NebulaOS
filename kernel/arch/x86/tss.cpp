// Task state segment for the NebulaOS x86 operating system.
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

#include "tss.hpp"

#include <cstdint>

namespace {
struct __attribute__((packed)) TaskStateSegment {
    std::uint32_t back_link;      // 0x00: previous task link
    std::uint32_t esp0;           // 0x04: kernel stack pointer (ring 0)
    std::uint32_t ss0;            // 0x08: kernel stack segment (ring 0)
    std::uint32_t esp1;           // 0x0C: unused
    std::uint32_t ss1;            // 0x10: unused
    std::uint32_t esp2;           // 0x14: unused
    std::uint32_t ss2;            // 0x18: unused
    std::uint32_t cr3;            // 0x1C: page directory base
    std::uint32_t eip;            // 0x20: instruction pointer
    std::uint32_t eflags;         // 0x24: flags
    std::uint32_t eax;            // 0x28: general purpose
    std::uint32_t ecx;            // 0x2C: general purpose
    std::uint32_t edx;            // 0x30: general purpose
    std::uint32_t ebx;            // 0x34: general purpose
    std::uint32_t esp;            // 0x38: stack pointer
    std::uint32_t ebp;            // 0x3C: base pointer
    std::uint32_t esi;            // 0x40: source index
    std::uint32_t edi;            // 0x44: destination index
    std::uint32_t es;             // 0x48: extra segment
    std::uint32_t cs;             // 0x4C: code segment
    std::uint32_t ss;             // 0x50: stack segment
    std::uint32_t ds;             // 0x54: data segment
    std::uint32_t fs;             // 0x58: F segment
    std::uint32_t gs;             // 0x5C: G segment
    std::uint32_t ldt;            // 0x60: LDT selector
    std::uint32_t trace_trap;     // 0x64: trace/trap bits
    std::uint16_t iomap_base;     // 0x68: I/O map base offset
};

// The stack the faulting vectors run on. It is separate from the kernel stack
// on purpose: a double fault is most often a symptom of that stack having run
// out of room, so the handler cannot borrow it.
__attribute__((aligned(16))) unsigned char fault_stack[16384] __attribute__((unused));

// Kernel stack for syscalls and interrupt handlers that need to run from user mode.
// This is the esp0/ss0 that the CPU loads when transitioning from ring 3 to ring 0.
__attribute__((aligned(16))) unsigned char syscall_stack[8192];

// Reserved by the GDT in entry.asm, three entries past the null descriptor.
extern "C" unsigned char gdt_tss_descriptor[8];

const unsigned short tss_selector = 0x18;

TaskStateSegment task_state;
bool loaded = false;

// The kernel data selector, used for every stack the task state names.
const unsigned short kernel_data_selector = 0x10;
const unsigned short kernel_code_selector = 0x08;
}

namespace kernel::tss {

void reset_for_reinit() {
    loaded = false;
}

void initialize() {
    if (loaded) {
        return;
    }

    unsigned char* const syscall_stack_top = syscall_stack + sizeof(syscall_stack);

    task_state.back_link = 0;
    task_state.esp0 = reinterpret_cast<std::uintptr_t>(syscall_stack_top);
    task_state.ss0 = kernel_data_selector;
    task_state.esp1 = 0;
    task_state.ss1 = 0;
    task_state.esp2 = 0;
    task_state.ss2 = 0;
    task_state.cr3 = 0;
    task_state.eip = 0;
    task_state.eflags = 0;
    task_state.eax = 0;
    task_state.ecx = 0;
    task_state.edx = 0;
    task_state.ebx = 0;
    task_state.esp = 0;
    task_state.ebp = 0;
    task_state.esi = 0;
    task_state.edi = 0;
    task_state.es = kernel_data_selector;
    task_state.cs = kernel_code_selector;
    task_state.ss = kernel_data_selector;
    task_state.ds = kernel_data_selector;
    task_state.fs = kernel_data_selector;
    task_state.gs = kernel_data_selector;
    task_state.ldt = 0;
    task_state.trace_trap = 0;
    task_state.iomap_base = static_cast<std::uint16_t>(sizeof(TaskStateSegment));

    // Fill the GDT entry the task register will point at. The descriptor is a
    // 32-bit available TSS: present, ring zero, and it occupies one entry.
    const unsigned int base = reinterpret_cast<unsigned int>(&task_state);
    const unsigned int limit = sizeof(TaskStateSegment) - 1;
    for (unsigned int index = 0; index < 8; ++index) {
        gdt_tss_descriptor[index] = 0;
    }
    gdt_tss_descriptor[0] = static_cast<unsigned char>(limit & 0xFF);
    gdt_tss_descriptor[1] = static_cast<unsigned char>((limit >> 8) & 0xFF);
    gdt_tss_descriptor[2] = static_cast<unsigned char>(base & 0xFF);
    gdt_tss_descriptor[3] = static_cast<unsigned char>((base >> 8) & 0xFF);
    gdt_tss_descriptor[4] = static_cast<unsigned char>((base >> 16) & 0xFF);
    gdt_tss_descriptor[5] = 0x89;
    gdt_tss_descriptor[6] = 0;
    gdt_tss_descriptor[7] = static_cast<unsigned char>((base >> 24) & 0xFF);

    const unsigned short selector = tss_selector;
    asm volatile("ltr %0" : : "r"(selector));

    loaded = true;
}

void update_descriptor() {
    const unsigned int base = reinterpret_cast<unsigned int>(&task_state);
    const unsigned int limit = sizeof(TaskStateSegment) - 1;
    for (unsigned int index = 0; index < 8; ++index) {
        gdt_tss_descriptor[index] = 0;
    }
    gdt_tss_descriptor[0] = static_cast<unsigned char>(limit & 0xFF);
    gdt_tss_descriptor[1] = static_cast<unsigned char>((limit >> 8) & 0xFF);
    gdt_tss_descriptor[2] = static_cast<unsigned char>(base & 0xFF);
    gdt_tss_descriptor[3] = static_cast<unsigned char>((base >> 8) & 0xFF);
    gdt_tss_descriptor[4] = static_cast<unsigned char>((base >> 16) & 0xFF);
    gdt_tss_descriptor[5] = 0x89;
    gdt_tss_descriptor[6] = 0;
    gdt_tss_descriptor[7] = static_cast<unsigned char>((base >> 24) & 0xFF);

    unsigned char* const syscall_stack_top = syscall_stack + sizeof(syscall_stack);
    task_state.esp0 = reinterpret_cast<std::uintptr_t>(syscall_stack_top);
    task_state.iomap_base = static_cast<std::uint16_t>(sizeof(TaskStateSegment));

    asm volatile("ltr %0" : : "r"(tss_selector));
}

}