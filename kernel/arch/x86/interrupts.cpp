// IDT, PIC, and interrupt dispatch for the NebulaOS x86 operating system.
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

#include "interrupts.hpp"
#include "tss.hpp"
#include "../../timer.hpp"
#include "../../../drivers/serial.hpp"
#include "../../../drivers/vga.hpp"

namespace {
struct __attribute__((packed)) IdtEntry {
    unsigned short offset_low;
    unsigned short selector;
    // Packed flags: present, descriptor privilege level, and the gate type.
    unsigned char flags;
    // Interrupt stack table index. The CPU reads the stack for a vector from
    // this slot in the TSS instead of from the interrupted stack.
    unsigned char stack_table;
    unsigned short offset_high;
};

struct __attribute__((packed)) IdtPointer {
    unsigned short limit;
    unsigned int base;
};

// Present, ring zero, 32-bit interrupt gate.
const unsigned char gate_attributes = 0x8E;

IdtEntry idt[256];
extern "C" unsigned int isr_stub_table[48];
extern "C" void isr_spurious();

void write_port(unsigned short port, unsigned char value) {
    asm volatile("outb %0, %1" : : "a"(value), "Nd"(port));
}

unsigned char read_port(unsigned short port) {
    unsigned char value;
    asm volatile("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

void io_wait() {
    write_port(0x80, 0);
}

void set_gate(unsigned int vector, unsigned int address, unsigned int stack_table) {
    idt[vector].offset_low = static_cast<unsigned short>(address & 0xFFFF);
    idt[vector].selector = 0x08;
    idt[vector].flags = gate_attributes;
    idt[vector].stack_table = static_cast<unsigned char>(stack_table & 0x7);
    idt[vector].offset_high = static_cast<unsigned short>(address >> 16);
}

void remap_pic() {
    write_port(0x20, 0x11);
    io_wait();
    write_port(0xA0, 0x11);
    io_wait();
    write_port(0x21, 0x20);
    io_wait();
    write_port(0xA1, 0x28);
    io_wait();
    write_port(0x21, 0x04);
    io_wait();
    write_port(0xA1, 0x02);
    io_wait();
    write_port(0x21, 0x01);
    io_wait();
    write_port(0xA1, 0x01);
    io_wait();

    write_port(0x21, 0xFE);
    write_port(0xA1, 0xFF);
}

void send_end_of_interrupt(bool second_controller) {
    const unsigned char eoi = 0x20;
    if (second_controller) {
        write_port(0xA0, eoi);
    }
    write_port(0x20, eoi);
}

// Vectors that report onto the task state's fault stack rather than the
// interrupted one. A double fault is the case that matters: it means the first
// handler could not run on the stack it was given, so the second one must not
// be handed the same stack. The stack and general protection faults are listed
// because they share that failure mode.
unsigned int stack_table_for(unsigned int vector) {
    if (vector == 8 || vector == 12 || vector == 13 || vector == 14) {
        return 1;
    }
    return 0;
}

// Names are indexed by vector. The reserved slots are named rather than left
// blank so a panic screen never points at an index that is not there.
const char* const exception_names[32] = {
    "Divide error",
    "Debug exception",
    "Non-maskable interrupt",
    "Breakpoint",
    "Overflow",
    "Bound range exceeded",
    "Invalid opcode",
    "Device not available",
    "Double fault",
    "Coprocessor segment overrun",
    "Invalid TSS",
    "Segment not present",
    "Stack segment fault",
    "General protection fault",
    "Page fault",
    "Reserved",
    "x87 floating point",
    "Alignment check",
    "Machine check",
    "SIMD floating point",
    "Virtualization exception",
    "Control protection exception",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Security exception",
    "Reserved",
    "Reserved"
};

// Everything below writes to both output devices. A panic has to be visible on
// whichever one is still working, and the two ports are memory mapped and
// identity mapped, so neither path can fault the way the code that reached the
// panic might have.
void put(char character) {
    drivers::vga::put(character);
    drivers::serial::write(character);
}

void put(const char* text) {
    for (unsigned int index = 0; text[index] != '\0'; ++index) {
        put(text[index]);
    }
}

void put_line(const char* text) {
    put(text);
    put('\n');
}

void put_hex(unsigned int value, unsigned int digits) {
    const char* table = "0123456789ABCDEF";
    for (unsigned int position = digits; position > 0; --position) {
        put(table[(value >> ((position - 1) * 4)) & 0xF]);
    }
}

unsigned int fault_address() {
    unsigned int address = 0;
    asm volatile("mov %%cr2, %0" : "=r"(address));
    return address;
}
}

namespace kernel::interrupts {

void install_handlers() {
    disable();

    // The task state has to be resident before any gate points at its fault
    // stack, otherwise a fault in that window would fault again on the way in.
    kernel::tss::initialize();

    const unsigned int fallback =
        reinterpret_cast<unsigned int>(&isr_spurious);
    for (unsigned int vector = 0; vector < 256; ++vector) {
        set_gate(vector, fallback, 0);
    }
    for (unsigned int vector = 0; vector < 48; ++vector) {
        set_gate(vector, isr_stub_table[vector], stack_table_for(vector));
    }

    const IdtPointer pointer = {
        static_cast<unsigned short>(sizeof(idt) - 1),
        reinterpret_cast<unsigned int>(idt)
    };
    asm volatile("lidt %0" : : "m"(pointer));
}

void initialize() {
    disable();

    install_handlers();
    remap_pic();
    kernel::timer::initialize();
    enable();
}

void enable() {
    asm volatile("sti");
}

void disable() {
    asm volatile("cli");
}

}

namespace {
// Terminal handler for every exception. It never returns: the state that
// produced the fault is not recoverable from inside the fault, so continuing
// would only trip the next one. Halting with interrupts off is what stops that
// escalation, and the dedicated fault stack behind the gates is what got the
// screen drawn at all.
[[noreturn]] void panic(const unsigned int* frame_pointer) {
    kernel::interrupts::disable();

    const unsigned int vector = frame_pointer[0];
    const unsigned int error_code = frame_pointer[1];
    const unsigned int eip = frame_pointer[2];
    const unsigned int cs = frame_pointer[3];
    const unsigned int eflags = frame_pointer[4];

    drivers::vga::clear();
    drivers::serial::write_line("");
    drivers::serial::write_line("");

    put_line("+--------------------------------------------------------------+");
    put_line("|                    NEBULAOS KERNEL PANIC                    |");
    put_line("+--------------------------------------------------------------+");
    put_line("");
    put("Exception: ");
    put_line(vector < 32 ? exception_names[vector] : "Unknown");
    put_line("");

    put("Vector   0x");
    put_hex(vector, 2);
    put_line("");
    put("Error    0x");
    put_hex(error_code, 8);
    put_line("");
    put("EIP      0x");
    put_hex(eip, 8);
    put_line("");
    put("CS:EFLAGS 0x");
    put_hex(cs, 4);
    put(':');
    put_hex(eflags, 8);
    put_line("");

    if (vector == 14) {
        put("Address  0x");
        put_hex(fault_address(), 8);
        put_line("");
        // The low three bits of a page fault error code say what kind of
        // access it was and whether it was user or supervisor, which is usually
        // the difference between a null pointer and a permissions bug.
        put_line("");
        put("Access   ");
        put((error_code & 0x1) != 0 ? "write" : "read");
        put_line("");
        put("User     ");
        put_line((error_code & 0x4) != 0 ? "yes" : "no");
        put_line("");
        const unsigned int cause = (error_code >> 1) & 0x7;
        put("Cause    ");
        switch (cause) {
        case 0: put_line("page not present"); break;
        case 1: put_line("write to read only"); break;
        case 2: put_line("access through a reserved bit"); break;
        case 3: put_line("access through a reserved bit"); break;
        case 4: put_line("instruction fetch"); break;
        case 5: put_line("write to read only, user"); break;
        case 6: put_line("reserved bit, user"); break;
        default: put_line("reserved bit, user"); break;
        }
    }

    put_line("");
    put_line("The system has been halted. Reset to restart.");
    put_line("");

    for (;;) {
        asm volatile("cli; hlt");
    }
}
}

extern "C" void interrupt_dispatch(const void* frame) {
    const unsigned int* const registers = static_cast<const unsigned int*>(frame);
    const unsigned int vector = registers[0];

    // Every exception, including the double fault and the page fault, lands on
    // the panic screen. Only hardware interrupts get past this point.
    if (vector < 32) {
        panic(registers);
    }

    if (vector == 32) {
        kernel::timer::interrupt_tick();
        send_end_of_interrupt(false);
        return;
    }

    if (vector >= 32 && vector < 40) {
        if (vector == 39) {
            // Vector 39 is only raised for a spurious interrupt, which has no
            // in-service bit to acknowledge, so it must not be acknowledged
            // either or the real interrupt behind it is lost.
            const unsigned char read_isr = 0x0B;
            write_port(0x20, read_isr);
            const unsigned char isr = read_port(0x20);
            if ((isr & 0x80) == 0) {
                return;
            }
        }
        send_end_of_interrupt(false);
        return;
    }

    if (vector >= 40 && vector < 48) {
        const unsigned char read_isr = 0x0B;
        write_port(0xA0, read_isr);
        const unsigned char isr = read_port(0xA0);
        if (vector == 47 && (isr & 0x80) == 0) {
            send_end_of_interrupt(false);
            return;
        }
        send_end_of_interrupt(true);
    }
}