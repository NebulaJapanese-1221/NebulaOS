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

namespace {
struct __attribute__((packed)) TaskStateSegment {
    unsigned int previous_task_link;
    unsigned short esp0;
    unsigned short ss0;
    unsigned char reserved0;
    unsigned char reserved1;
    unsigned short reserved2;
    unsigned int eip;
    unsigned short cs;
    unsigned short reserved3;
    unsigned int eflags;
    unsigned int esp;
    unsigned short ss;
    unsigned short reserved4;
    unsigned int ist1_sp;
    unsigned short ist1_ss;
    unsigned short reserved5;
};

// The stack the faulting vectors run on. It is separate from the kernel stack
// on purpose: a double fault is most often a symptom of that stack having run
// out of room, so the handler cannot borrow it.
__attribute__((aligned(16))) unsigned char fault_stack[16384];

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
}

namespace kernel::tss {

void reset_for_reinit() {
    loaded = false;
}

void initialize() {
    if (loaded) {
        return;
    }

    unsigned char* const fault_stack_top = fault_stack + sizeof(fault_stack);
    unsigned char* const syscall_stack_top = syscall_stack + sizeof(syscall_stack);
    task_state.previous_task_link = 0;
    task_state.esp0 = static_cast<unsigned short>(reinterpret_cast<unsigned int>(syscall_stack_top) & 0xFFFF);
    task_state.ss0 = kernel_data_selector;
    task_state.ist1_sp = reinterpret_cast<unsigned int>(fault_stack_top);
    task_state.ist1_ss = kernel_data_selector;

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

    unsigned char* const fault_stack_top = fault_stack + sizeof(fault_stack);
    unsigned char* const syscall_stack_top = syscall_stack + sizeof(syscall_stack);
    task_state.esp0 = static_cast<unsigned short>(reinterpret_cast<unsigned int>(syscall_stack_top) & 0xFFFF);
    task_state.ist1_sp = reinterpret_cast<unsigned int>(fault_stack_top);

    asm volatile("ltr %0" : : "r"(tss_selector));
}

}