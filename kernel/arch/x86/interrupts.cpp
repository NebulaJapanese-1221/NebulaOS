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
#include "../../assert.hpp"

namespace {

struct __attribute__((packed)) IdtEntry {
    unsigned short offset_low;
    unsigned short selector;
    // Packed flags: present, descriptor privilege level, and the gate type.
    unsigned char flags;
    // Interrupt stack table index. The CPU reads the stack for a
    // vector from this slot in the TSS instead of from the
    // interrupted stack.
    unsigned char stack_table;
    unsigned short offset_high;
    unsigned short reserved;
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
    idt[vector].reserved = 0;
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

void send_pic_eoi(bool second_controller) {
    const unsigned char eoi = 0x20;
    if (second_controller) {
        write_port(0xA0, eoi);
    }
    write_port(0x20, eoi);
}

// Vectors that report onto the task state's fault stack rather than the
// interrupted one. A double fault is the case that matters: it means the
// first handler could not run on the stack it was given, so the second
// one must not be handed the same stack. The stack and general protection
// faults are listed because they share that failure mode.
unsigned int stack_table_for(unsigned int vector) {
    if (vector == 8 || vector == 12 || vector == 13 || vector == 14) {
        return 1;
    }
    return 0;
}

// Names are indexed by vector. The reserved slots are named rather than
// left blank so a panic screen never points at an index that is not there.
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
    "Security exception",
    "Reserved",
    "Reserved"
};

// Everything below writes to both output devices. A panic has to be
// visible on whichever one is still working, and the two ports are
// memory mapped and identity mapped, so neither path can fault the
// way the code that reached the panic might have.
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

// ---- IRQ handler table ----

struct IrqSlot {
    kernel::interrupts::IrqHandler handler;
    void* context;
    bool in_use;
};

// One slot per IRQ line. Chaining is supported by walking the
// registered handlers until one reports that it handled the
// interrupt, which is how the PCI bus shares its lines.
IrqSlot irq_handlers[16] = {};

bool irq_line_masked[16] = {};

// ---- Interrupt statistics ----

std::uint64_t stat_total = 0;
std::uint64_t stat_exceptions = 0;
std::uint64_t stat_spurious = 0;
std::uint64_t stat_syscalls = 0;
std::uint64_t stat_irq[16] = {};

// ---- Local APIC ----

bool lapic_active = false;
std::uint32_t lapic_base = 0;

volatile unsigned char* lapic_reg(unsigned int offset) {
    return reinterpret_cast<volatile unsigned char*>(lapic_base + offset);
}

std::uint64_t read_msr_impl(std::uint32_t msr) {
    std::uint32_t low = 0;
    std::uint32_t high = 0;
    asm volatile("rdmsr" : "=a"(low), "=d"(high) : "c"(msr));
    return (static_cast<std::uint64_t>(high) << 32) | low;
}

void write_msr_impl(std::uint32_t msr, std::uint64_t value) {
    const std::uint32_t low = static_cast<std::uint32_t>(value & 0xFFFFFFFF);
    const std::uint32_t high = static_cast<std::uint32_t>(value >> 32);
    asm volatile("wrmsr" : : "a"(low), "d"(high), "c"(msr));
}

} // namespace

namespace kernel::interrupts {

void install_handlers() {
    disable();

    // The task state has to be resident before any gate points at
    // its fault stack, otherwise a fault in that window would fault
    // again on the way in.
    tss::initialize();

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
    timer::initialize();
    initialize_lapic();
    
    // Enable FPU: Clear CR0.EM (bit 2) and CR0.TS (bit 3)
    // CR0.EM = 1 means FPU emulation, CR0.TS = 1 means task switched
    asm volatile(
        "mov %%cr0, %%eax\n"
        "and $~0xC, %%eax\n"  ; Clear bits 2 (EM) and 3 (TS)
        "mov %%eax, %%cr0\n"
        : : : "eax", "memory"
    );
    
    // Initialize FPU with fninit
    asm volatile("fninit" : : : "memory");
    
    enable();
}

void enable() {
    asm volatile("sti");
}

void disable() {
    asm volatile("cli");
}

bool enabled() {
    unsigned int flags = 0;
    asm volatile("pushf; pop %0" : "=r"(flags));
    return (flags & (1U << 9)) != 0;
}

void enable_irq(unsigned int irq) {
    if (irq >= 16) {
        return;
    }
    irq_line_masked[irq] = false;

    const unsigned short port = irq < 8 ? 0x21 : 0xA1;
    const unsigned char bit = 1U << (irq % 8);
    unsigned char mask = read_port(port);
    mask &= static_cast<unsigned char>(~bit);
    write_port(port, mask);
}

void disable_irq(unsigned int irq) {
    if (irq >= 16) {
        return;
    }
    irq_line_masked[irq] = true;

    const unsigned short port = irq < 8 ? 0x21 : 0xA1;
    const unsigned char bit = 1U << (irq % 8);
    unsigned char mask = read_port(port);
    mask |= bit;
    write_port(port, mask);
}

bool irq_enabled(unsigned int irq) {
    if (irq >= 16) {
        return false;
    }
    return !irq_line_masked[irq];
}

void send_eoi(unsigned int irq) {
    if (irq >= 16) {
        return;
    }
    if (lapic_active) {
        send_lapic_eoi();
        return;
    }
    send_pic_eoi(irq >= 8);
}

unsigned char read_pic(unsigned int controller, unsigned char register_index) {
    const unsigned short port = controller == 1 ? 0xA0 : 0x20;
    write_port(port, register_index);
    return read_port(port);
}

int register_irq(unsigned int irq, IrqHandler handler, void* context) {
    if (irq >= 16 || handler == nullptr) {
        return -1;
    }
    for (unsigned int index = 0; index < 16; ++index) {
        if (!irq_handlers[irq].in_use) {
            (void)index;
        }
    }
    // Single handler per line, matching the original dispatch model.
    // A driver that needs chaining registers here and walks the PCI
    // device list itself from within its own handler.
    irq_handlers[irq].handler = handler;
    irq_handlers[irq].context = context;
    irq_handlers[irq].in_use = true;
    enable_irq(irq);
    return static_cast<int>(irq);
}

void unregister_irq(int handle) {
    if (handle < 0 || handle >= 16) {
        return;
    }
    irq_handlers[handle].handler = nullptr;
    irq_handlers[handle].context = nullptr;
    irq_handlers[handle].in_use = false;
}

void unregister_irq_all(unsigned int irq) {
    if (irq >= 16) {
        return;
    }
    unregister_irq(static_cast<int>(irq));
}

InterruptStats stats() {
    InterruptStats result = {};
    result.total_interrupts = stat_total;
    result.exception_count = stat_exceptions;
    result.spurious_count = stat_spurious;
    result.syscall_count = stat_syscalls;
    for (unsigned int irq = 0; irq < 16; ++irq) {
        result.irq_counts[irq] = stat_irq[irq];
    }
    return result;
}

void reset_stats() {
    stat_total = 0;
    stat_exceptions = 0;
    stat_spurious = 0;
    stat_syscalls = 0;
    for (unsigned int irq = 0; irq < 16; ++irq) {
        stat_irq[irq] = 0;
    }
}

bool initialize_lapic() {
    // The APIC is enabled by the BIOS unless it was explicitly
    // disabled, so the base address in the MSR is checked rather
    // than assumed. Bit 11 of the base is the enable bit.
    const std::uint64_t base = read_msr_impl(0x1B);
    lapic_base = static_cast<std::uint32_t>(base & 0xFFFFF000);
    if (lapic_base == 0 || (base & (1ULL << 11)) == 0) {
        lapic_active = false;
        return false;
    }

    // The spurious interrupt vector register holds the enable bit
    // in bit 8. Setting it turns the local APIC on without
    // disturbing the vector it reports on.
    volatile unsigned char* spurious = lapic_reg(0xF0);
    *spurious = static_cast<unsigned char>(*spurious | 0x100);

    lapic_active = true;
    return true;
}

bool lapic_present() {
    return lapic_active;
}

std::uint64_t read_msr(std::uint32_t msr) {
    return read_msr_impl(msr);
}

void write_msr(std::uint32_t msr, std::uint64_t value) {
    write_msr_impl(msr, value);
}

void send_lapic_eoi() {
    if (!lapic_active) {
        return;
    }
    // The EOI register sits at offset 0xB0. Writing any value
    // clears the current task priority.
    *lapic_reg(0xB0) = 0;
}

} // namespace kernel::interrupts

namespace {

struct PanicRegs {
    std::uint32_t vector;
    std::uint32_t error_code;
    std::uint32_t eip;
    std::uint32_t cs;
    std::uint32_t eflags;
    std::uint32_t user_esp;
    std::uint32_t user_ss;
};

// Terminal handler for every exception. It never returns: the state
// that produced the fault is not recoverable from inside the fault,
// so continuing would only trip the next one. Halting with interrupts
// off is what stops that escalation, and the dedicated fault stack
// behind the gates is what got the screen drawn at all.
[[noreturn]] void panic(const unsigned int* frame_pointer) {
    kernel::interrupts::disable();

    const unsigned int vector = frame_pointer[0];
    const unsigned int error_code = frame_pointer[1];
    const unsigned int eip = frame_pointer[2];
    const unsigned int cs = frame_pointer[3];
    const unsigned int eflags = frame_pointer[4];

    // Use the new assert/panic with register dump and stack trace
    kernel::assert::RegisterState regs;
    regs.eax = 0;
    regs.ebx = 0;
    regs.ecx = 0;
    regs.edx = 0;
    regs.esi = 0;
    regs.edi = 0;
    regs.ebp = 0;
    regs.esp = 0;
    regs.eip = eip;
    regs.cs = cs;
    regs.eflags = eflags;
    regs.user_esp = frame_pointer[5];
    regs.user_ss = frame_pointer[6];

    // Build panic message
    char msg[256];
    char* p = msg;
    const char* prefix = "Exception: ";
    while (*prefix) *p++ = *prefix++;
    const char* name = vector < 32 ? exception_names[vector] : "Unknown";
    while (*name) *p++ = *name++;
    *p++ = '\n';
    *p++ = '\n';
    *p = '\0';

    kernel::assert::panic_with_regs(&regs, msg);
}

// Walks the registered handlers for an IRQ line. The first handler
// that reports it handled the interrupt ends the walk, which is the
// same rule the PCI bus uses for its shared lines.
bool dispatch_irq_handlers(unsigned int irq) {
    if (irq >= 16 || !irq_handlers[irq].in_use) {
        return false;
    }
    const IrqResult result = irq_handlers[irq].handler(irq, irq_handlers[irq].context);
    return result == IrqResult::Handled;
}

} // namespace

extern "C" void interrupt_dispatch(const void* frame) {
    const unsigned int* const registers = static_cast<const unsigned int*>(frame);
    const unsigned int vector = registers[0];

    ++stat_total;

    // Every exception, including the double fault and the page fault,
    // lands on the panic screen. Only hardware interrupts get past
    // this point.
    if (vector < 32) {
        ++stat_exceptions;
        panic(registers);
    }

    if (vector == 32) {
        ++stat_irq[0];
        kernel::timer::interrupt_tick();
        kernel::interrupts::send_eoi(0);
        return;
    }

    if (vector >= 32 && vector < 40) {
        const unsigned int irq = vector - 32;
        ++stat_irq[irq];

        // A registered handler gets first chance at the line, so a
        // driver can own its IRQ without the dispatcher guessing.
        if (dispatch_irq_handlers(irq)) {
            kernel::interrupts::send_eoi(irq);
            return;
        }

        if (vector == 39) {
            // Vector 39 is only raised for a spurious interrupt,
            // which has no in-service bit to acknowledge, so it must
            // not be acknowledged either or the real interrupt
            // behind it is lost.
            ++stat_spurious;
            const unsigned char read_isr = 0x0B;
            write_port(0x20, read_isr);
            const unsigned char isr = read_port(0x20);
            if ((isr & 0x80) == 0) {
                return;
            }
        }
        kernel::interrupts::send_eoi(irq);
        return;
    }

    if (vector >= 40 && vector < 48) {
        const unsigned int irq = vector - 32;
        ++stat_irq[irq];

        if (dispatch_irq_handlers(irq)) {
            kernel::interrupts::send_eoi(irq);
            return;
        }

        const unsigned char read_isr = 0x0B;
        write_port(0xA0, read_isr);
        const unsigned char isr = read_port(0xA0);
        if (vector == 47 && (isr & 0x80) == 0) {
            ++stat_spurious;
            kernel::interrupts::send_eoi(0);
            return;
        }
        kernel::interrupts::send_eoi(irq);
    }
}

extern "C" void syscall_count_increment() {
    ++stat_syscalls;
}