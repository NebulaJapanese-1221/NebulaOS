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
#include "../../timer.hpp"
#include "../../../drivers/serial.hpp"
#include "../../../drivers/vga.hpp"

namespace {
struct __attribute__((packed)) IdtEntry {
    unsigned short offset_low;
    unsigned short selector;
    unsigned char zero;
    unsigned char attributes;
    unsigned short offset_high;
};

struct __attribute__((packed)) IdtPointer {
    unsigned short limit;
    unsigned int base;
};

IdtEntry idt[256];
extern "C" unsigned int isr_stub_table[48];
extern "C" void isr_spurious();

void write_port(unsigned short port, unsigned char value) {
    asm volatile("outb %0, %1" : : "a"(value), "Nd"(port));
}

void io_wait() {
    write_port(0x80, 0);
}

void set_gate(unsigned int vector, unsigned int address) {
    idt[vector].offset_low = static_cast<unsigned short>(address & 0xFFFF);
    idt[vector].selector = 0x08;
    idt[vector].zero = 0;
    idt[vector].attributes = 0x8E;
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

const char* exception_names[32] = {
    "Division by Zero",
    "Debug",
    "Non-Maskable Interrupt",
    "Breakpoint",
    "Overflow",
    "Bound Range Exceeded",
    "Invalid Opcode",
    "Device Not Available",
    "Double Fault",
    "Coprocessor Segment Overrun",
    "Invalid TSS",
    "Segment Not Present",
    "Stack Segment Fault",
    "General Protection Fault",
    "Page Fault",
    "Reserved",
    "x87 FPU Error",
    "Alignment Check",
    "Machine Check",
    "SIMD FPU Error",
    "Virtualization Exception",
    "Control Protection Exception",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Security Exception"
};

void write_hex_32(unsigned int value) {
    const char* hex = "0123456789ABCDEF";
    char buffer[9];
    for (int i = 7; i >= 0; --i) {
        buffer[7 - i] = hex[value & 0xF];
        value >>= 4;
    }
    buffer[8] = '\0';
    drivers::vga::write(buffer);
    drivers::serial::write(buffer);
}

void write_dec(unsigned int value) {
    if (value == 0) {
        drivers::vga::put('0');
        drivers::serial::write("0");
        return;
    }
    char buffer[11];
    int pos = 10;
    buffer[pos] = '\0';
    while (value > 0) {
        buffer[--pos] = '0' + (value % 10);
        value /= 10;
    }
    drivers::vga::write(&buffer[pos]);
    drivers::serial::write(&buffer[pos]);
}

void error_screen(unsigned int vector, unsigned int error_code, unsigned int eip, unsigned int cs, unsigned int eflags, unsigned int esp, unsigned int ss) {
    asm volatile("cli");
    
    drivers::vga::clear();
    
    drivers::vga::write_line("========================================");
    drivers::vga::write_line("         NEBULAOS KERNEL PANIC          ");
    drivers::vga::write_line("========================================");
    drivers::vga::write_line("");
    
    if (vector < 32) {
        drivers::vga::write("Exception: ");
        drivers::vga::write(exception_names[vector]);
        drivers::vga::write(" (Vector ");
        write_dec(vector);
        drivers::vga::write(")");
        drivers::vga::write_line("");
    } else {
        drivers::vga::write("Interrupt: Vector ");
        write_dec(vector);
        drivers::vga::write_line("");
    }
    
    drivers::vga::write_line("");
    drivers::vga::write("Error Code: 0x");
    write_hex_32(error_code);
    drivers::vga::write_line("");
    
    drivers::vga::write("EIP: 0x");
    write_hex_32(eip);
    drivers::vga::write_line("");
    
    drivers::vga::write("CS:  0x");
    write_hex_32(cs);
    drivers::vga::write_line("");
    
    drivers::vga::write("EFLAGS: 0x");
    write_hex_32(eflags);
    drivers::vga::write_line("");
    
    drivers::vga::write("ESP: 0x");
    write_hex_32(esp);
    drivers::vga::write_line("");
    
    drivers::vga::write("SS:  0x");
    write_hex_32(ss);
    drivers::vga::write_line("");
    
    if (vector == 14) {
        unsigned int cr2 = 0;
        asm volatile("mov %%cr2, %0" : "=r"(cr2));
        drivers::vga::write_line("");
        drivers::vga::write("CR2 (Fault Address): 0x");
        write_hex_32(cr2);
        drivers::vga::write_line("");
    }
    
    drivers::vga::write_line("");
    drivers::vga::write_line("========================================");
    drivers::vga::write_line("System halted. Press reset to restart.");
    drivers::vga::write_line("========================================");
    
    drivers::serial::write_line("");
    drivers::serial::write_line("========================================");
    drivers::serial::write_line("         NEBULAOS KERNEL PANIC          ");
    drivers::serial::write_line("========================================");
    drivers::serial::write_line("");
    
    if (vector < 32) {
        drivers::serial::write("Exception: ");
        drivers::serial::write(exception_names[vector]);
        drivers::serial::write(" (Vector ");
        char vec_str[4];
        vec_str[0] = '0' + (vector / 10);
        vec_str[1] = '0' + (vector % 10);
        vec_str[2] = ')';
        vec_str[3] = '\0';
        drivers::serial::write(vec_str);
        drivers::serial::write_newline();
    } else {
        drivers::serial::write("Interrupt: Vector ");
        drivers::serial::write_hex(vector);
        drivers::serial::write_newline();
    }
    
    drivers::serial::write_line("");
    drivers::serial::write("Error Code: 0x");
    drivers::serial::write_hex(error_code);
    drivers::serial::write_newline();
    
    drivers::serial::write("EIP: 0x");
    drivers::serial::write_hex(eip);
    drivers::serial::write_newline();
    
    drivers::serial::write("CS: 0x");
    drivers::serial::write_hex(cs);
    drivers::serial::write_newline();
    
    drivers::serial::write("EFLAGS: 0x");
    drivers::serial::write_hex(eflags);
    drivers::serial::write_newline();
    
    drivers::serial::write("ESP: 0x");
    drivers::serial::write_hex(esp);
    drivers::serial::write_newline();
    
    drivers::serial::write("SS: 0x");
    drivers::serial::write_hex(ss);
    drivers::serial::write_newline();
    
    if (vector == 14) {
        unsigned int cr2 = 0;
        asm volatile("mov %%cr2, %0" : "=r"(cr2));
        drivers::serial::write("CR2 (Fault Address): 0x");
        drivers::serial::write_hex(cr2);
        drivers::serial::write_newline();
    }
    
    drivers::serial::write_line("");
    drivers::serial::write_line("========================================");
    drivers::serial::write_line("System halted. Press reset to restart.");
    drivers::serial::write_line("========================================");
    
    for (;;) {
        asm volatile("cli; hlt");
    }
}
}

namespace kernel::interrupts {

void install_handlers() {
    disable();

    const unsigned int fallback =
        reinterpret_cast<unsigned int>(&isr_spurious);
    for (unsigned int vector = 0; vector < 256; ++vector) {
        set_gate(vector, fallback);
    }
    for (unsigned int vector = 0; vector < 48; ++vector) {
        set_gate(vector, isr_stub_table[vector]);
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

extern "C" void interrupt_dispatch(unsigned int vector, unsigned int error_code, unsigned int eip, unsigned int cs, unsigned int eflags, unsigned int esp, unsigned int ss) {
    if (vector == 14) {
        kernel::interrupts::disable();
        error_screen(vector, error_code, eip, cs, eflags, esp, ss);
    }

    if (vector < 32) {
        kernel::interrupts::disable();
        error_screen(vector, error_code, eip, cs, eflags, esp, ss);
    }

    if (vector == 32) {
        kernel::timer::interrupt_tick();
    }

    if (vector >= 40 && vector < 48) {
        unsigned char isr = 0;
        const unsigned char read_isr = 0x0B;
        asm volatile("outb %0, %1" : : "a"(read_isr), "Nd"(0xA0));
        asm volatile("inb %1, %0" : "=a"(isr) : "Nd"(0xA0));
        if (vector == 47 && (isr & 0x80) == 0) {
            const unsigned char eoi = 0x20;
            asm volatile("outb %0, %1" : : "a"(eoi), "Nd"(0x20));
            return;
        }
        const unsigned char eoi = 0x20;
        asm volatile("outb %0, %1" : : "a"(eoi), "Nd"(0xA0));
        asm volatile("outb %0, %1" : : "a"(eoi), "Nd"(0x20));
    }
    if (vector >= 32 && vector < 40) {
        if (vector == 39) {
            unsigned char isr = 0;
            const unsigned char read_isr = 0x0B;
            asm volatile("outb %0, %1" : : "a"(read_isr), "Nd"(0x20));
            asm volatile("inb %1, %0" : "=a"(isr) : "Nd"(0x20));
            if ((isr & 0x80) == 0) {
                return;
            }
        }
        const unsigned char eoi = 0x20;
        asm volatile("outb %0, %1" : : "a"(eoi), "Nd"(0x20));
    }
}