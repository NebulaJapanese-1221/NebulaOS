// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

//! AArch64 architecture-specific code for NebulaOS.
//! Provides exception handling, architectural timer, MMU helpers,
//! and CPU bring-up analogous to the x86 GDT/IDT/Local APIC modules.

use core::arch::asm;
use crate::drivers::serial;

#[path = "exceptions.rs"]
mod exceptions;
#[path = "timer.rs"]
mod timer;
#[path = "mmu.rs"]
mod mmu;

pub use exceptions::init as init_exceptions;
pub use timer::init as init_timer;
pub use timer::ticks as timer_ticks;
pub use mmu::kernel_page_dir;

// Maximum number of CPUs
pub const MAX_CPUS: usize = 8;

// Read the current MPIDR (affinity level)
pub fn mpidr() -> u64 {
    let mpidr: u64;
    unsafe {
        asm!("mrs {0}, mpidr_el1", out(reg) mpidr, options(nostack));
    }
    mpidr & 0xFFFF
}

// Current CPU index (low 8 bits of MPIDR)
pub fn current_cpu() -> usize {
    (mpidr() & 0xFF) as usize
}

// Total CPU count (from MPIDR, limited)
pub fn cpu_count() -> usize {
    // On QEMU virt, we can read the CPU count from the device tree or use a fixed max.
    // For now, return 1 (primary CPU) unless SMP bring-up is implemented.
    1
}

// Early initialization - called from kernel_entry before Rust heap is set up
pub fn early_init() {
    // Exception vectors are set up by boot.S; just initialize the timer
    timer::init();
    serial::write_string("AArch64 early init complete.\r\n");
}

// Full initialization - called after heap and paging are ready
pub fn init() {
    serial::write_string("AArch64 SMP: ");
    serial::write_dec(cpu_count() as u32);
    serial::write_string(" CPU(s)\r\n");
}

// Send an IPI to another CPU (stub - GIC not yet implemented)
pub fn send_ipi(_cpu: usize) {
    // Not implemented yet
}

// Halt the current CPU
pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfe", options(nomem, nostack));
        }
    }
}
