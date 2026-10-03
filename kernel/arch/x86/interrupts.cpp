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

namespace {
unsigned int fault_address() {
    unsigned int address = 0;
    asm volatile("mov %%cr2, %0" : "=r"(address));
    return address;
}

void report_page_fault(unsigned int error_code) {
    const unsigned int address = fault_address();

    volatile unsigned short* const text =
        reinterpret_cast<volatile unsigned short*>(0xB8000);
    const char message[] = "NEBULAOS PAGE FAULT";
    for (unsigned int index = 0; message[index] != '\0'; ++index) {
        text[index] = static_cast<unsigned short>(0x4F00 | message[index]);
    }

    drivers::serial::write_line("NEBULAOS PAGE FAULT");
    drivers::serial::write("address ");
    drivers::serial::write_hex(address);
    drivers::serial::write(" error ");
    drivers::serial::write_hex(error_code);
    drivers::serial::write_newline();
}
}

extern "C" void interrupt_dispatch(unsigned int vector, unsigned int error_code) {
    if (vector == 14) {
        kernel::interrupts::disable();
        report_page_fault(error_code);
        for (;;) {
            asm volatile("cli; hlt");
        }
    }

    if (vector < 32) {
        kernel::interrupts::disable();
        volatile unsigned short* const text =
            reinterpret_cast<volatile unsigned short*>(0xB8000);
        const char message[] = "NEBULAOS CPU EXCEPTION";
        for (unsigned int index = 0; message[index] != '\0'; ++index) {
            text[index] = static_cast<unsigned short>(0x4F00 | message[index]);
        }
        text[24] = static_cast<unsigned short>(0x4F00 | ('0' + (vector / 10)));
        text[25] = static_cast<unsigned short>(0x4F00 | ('0' + (vector % 10)));
        text[27] = static_cast<unsigned short>(0x4F00 | 'E');
        text[28] = static_cast<unsigned short>(0x4F00 | 'R');
        text[29] = static_cast<unsigned short>(0x4F00 | 'R');
        text[30] = static_cast<unsigned short>(0x4F00 | ' ');
        text[31] = static_cast<unsigned short>(0x4F00 | ('0' + ((error_code / 1000) % 10)));
        text[32] = static_cast<unsigned short>(0x4F00 | ('0' + ((error_code / 100) % 10)));
        text[33] = static_cast<unsigned short>(0x4F00 | ('0' + ((error_code / 10) % 10)));
        text[34] = static_cast<unsigned short>(0x4F00 | ('0' + (error_code % 10)));
        drivers::serial::write_line("NEBULAOS CPU EXCEPTION");
        drivers::serial::write("vector ");
        drivers::serial::write_hex(vector);
        drivers::serial::write(" error ");
        drivers::serial::write_hex(error_code);
        drivers::serial::write_newline();
        for (;;) {
            asm volatile("cli; hlt");
        }
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
