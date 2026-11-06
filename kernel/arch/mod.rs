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

// Architecture-specific modules. The x86 modules are compiled for i686,
// the aarch64 modules for arm64.

#[cfg(target_arch = "x86")]
pub mod gdt;
#[cfg(target_arch = "x86")]
pub mod idt;
#[cfg(target_arch = "x86")]
pub mod lapic;
#[cfg(target_arch = "x86")]
pub mod pic;
#[cfg(target_arch = "x86")]
pub mod smp;
#[cfg(target_arch = "x86")]
pub mod tss;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;
#[cfg(target_arch = "aarch64")]
#[path = "aarch64/timer.rs"]
pub mod timer;

// On aarch64, provide dummy x86-compatible module shims so that existing
// code referencing arch::gdt, arch::tss, arch::smp, etc. still compiles.
// These are stubs; full aarch64 implementations are in arch/aarch64/.
#[cfg(target_arch = "aarch64")]
pub mod gdt {
    pub fn init() {}
    pub fn user_data_selector() -> u16 { 0 }
    pub fn user_code_selector() -> u16 { 0 }
    pub fn kernel_code_selector() -> u16 { 0 }
}
#[cfg(target_arch = "aarch64")]
pub mod idt {
    pub fn init() {}
    pub fn load() {}
}
#[cfg(target_arch = "aarch64")]
pub mod lapic {
    pub fn init(_base: u32) -> bool { true }
    pub fn initialize_cpu() -> bool { true }
    pub fn id() -> u8 { 0 }
    pub fn calibrate_timer(_ticks: u32) -> Option<u32> { Some(1000) }
    pub fn start_scheduler_timer() -> bool { true }
    pub fn end_of_interrupt() {}
    pub fn send_init(_id: u8) -> bool { true }
    pub fn send_init_deassert(_id: u8) -> bool { true }
    pub fn send_startup(_id: u8, _vector: u8) -> bool { true }
    pub fn enable_local_apic(_base: u32) -> bool { true }
}
#[cfg(target_arch = "aarch64")]
pub mod pic {
    pub fn remap(_o1: u8, _o2: u8) {}
    pub fn mask_irq(_irq: u8) {}
    pub fn unmask_irq(_irq: u8) {}
    pub fn eoi(_irq: u8) {}
}
#[cfg(target_arch = "aarch64")]
pub mod smp {
    pub const MAX_CPUS: usize = 8;
    pub fn init(_mb: *const u8) {}
    pub fn cpu_count() -> usize { 1 }
    pub fn current_cpu_index() -> usize { 0 }
}
#[cfg(target_arch = "aarch64")]
pub mod tss {
    pub fn init(_stack: u32) {}
    pub fn init_cpu(_cpu: usize, _stack: u32) {}
    pub fn tss_esp0() -> u32 { 0 }
    pub fn set_kernel_stack(_cpu: usize, _stack: u32) {}
}

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;
#[cfg(target_arch = "aarch64")]
pub use aarch64::early_init;
#[cfg(target_arch = "aarch64")]
pub use timer::*;

#[cfg(target_arch = "x86")]
pub use gdt::*;
#[cfg(target_arch = "x86")]
pub use idt::*;
#[cfg(target_arch = "x86")]
pub use lapic::*;
#[cfg(target_arch = "x86")]
pub use pic::*;
#[cfg(target_arch = "x86")]
pub use smp::*;
#[cfg(target_arch = "x86")]
pub use tss::*;
