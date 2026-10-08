// Interrupt controller interface for the NebulaOS x86 kernel.
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

#pragma once

#include <cstdint>
#include <cstddef>

namespace kernel::interrupts {

// Loads the IDT with interrupts still disabled. Paging must be enabled only
// after this runs, otherwise a fault during translation cannot be reported.
void install_handlers();
void initialize();
void enable();
void disable();

// Interrupt state helpers. These are used by the scheduler and the
// atomic sections in the memory and IPC code, where the previous
// state has to be restored rather than assumed.
bool enabled();

// Enables or disables a specific IRQ line on the PIC. IRQs 0-15 map
// to the master and slave controllers; the slave lines are routed
// through the master's line 2.
void enable_irq(unsigned int irq);
void disable_irq(unsigned int irq);

// Whether an IRQ line is currently unmasked.
bool irq_enabled(unsigned int irq);

// Sends an end of interrupt for the given IRQ. The slave controller
// must be acknowledged before the master for IRQs 8-15.
void send_eoi(unsigned int irq);

// Reads the interrupt request register (in-service or interrupt
// request) of the master controller. Register 0x0A is the ISR and
// 0x0B is the IRR.
unsigned char read_pic(unsigned int controller, unsigned char register_index);

// ---- IRQ handler registration ----

// Handler result. Handlers report whether they handled the interrupt
// so that chained devices on the same IRQ line can be walked.
enum class IrqResult : unsigned char {
    NotHandled = 0,
    Handled = 1,
};

// IRQ handler signature. The handler receives the IRQ number and an
// optional context pointer.
using IrqHandler = IrqResult (*)(unsigned int irq, void* context);

// Registers a handler for an IRQ line. Returns a handle that can be
// passed to unregister_irq, or -1 if the line has no free slot.
int register_irq(unsigned int irq, IrqHandler handler, void* context);

// Unregisters a handler previously returned by register_irq.
void unregister_irq(int handle);

// Unregisters every handler for an IRQ line.
void unregister_irq_all(unsigned int irq);

// ---- Interrupt statistics ----

struct InterruptStats {
    std::uint64_t total_interrupts;
    std::uint64_t exception_count;
    std::uint64_t spurious_count;
    std::uint64_t irq_counts[16];
    std::uint64_t syscall_count;
};

// Snapshot of the interrupt counters.
InterruptStats stats();

// Resets the interrupt counters.
void reset_stats();

// ---- Advanced programmable interrupt controller ----

// Detects and, where present, enables the local APIC. The IO APIC is
// left for the driver layer because its redirection table depends on
// the board layout, but the local APIC timer and the EOI register are
// safe to bring up here.
bool initialize_lapic();

// Whether the local APIC was detected and enabled.
bool lapic_present();

// Reads a model-specific register. These hold the APIC base address
// and the feature flags.
std::uint64_t read_msr(std::uint32_t msr);
void write_msr(std::uint32_t msr, std::uint64_t value);

// Sends an end of interrupt through the local APIC, which is required
// on systems where the APIC is enabled even if the PIC is emulated.
void send_lapic_eoi();

} // namespace kernel::interrupts