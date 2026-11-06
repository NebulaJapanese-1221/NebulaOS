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

// AArch64 MMU helpers - translation control, page table access

use core::arch::asm;
use core::ptr;

// TCR_EL1 bit fields
const TCR_T0SZ_SHIFT: u64 = 0;
const TCR_IRGN0_SHIFT: u64 = 8;
const TCR_ORGN0_SHIFT: u64 = 10;
const TCR_SH0_SHIFT: u64 = 12;

// MAIR_EL1 attribute indices
const MAIR_ATTR_DEVICE: u64 = 0x00;
const MAIR_ATTR_NORMAL_NC: u64 = 0x44;
const MAIR_ATTR_NORMAL_WB: u64 = 0xBB;

// SCTLR_EL1 bits
const SCTLR_M: u64 = 1 << 0;   // MMU enable
const SCTLR_C: u64 = 1 << 2;   // Cache enable
const SCTLR_I: u64 = 1 << 12;  // Instruction cache enable
const SCTLR_A: u64 = 1 << 3;   // Alignment check

/// Read the current TTBR0_EL1 (translation table base register 0)
pub fn ttbr0() -> u64 {
    let val: u64;
    unsafe {
        asm!("mrs {0}, ttbr0_el1", out(reg) val, options(nostack));
    }
    val
}

/// Write TTBR0_EL1
pub fn set_ttbr0(value: u64) {
    unsafe {
        asm!("msr ttbr0_el1, {0}", in(reg) value, options(nostack));
    }
}

/// Read TCR_EL1
pub fn tcr() -> u64 {
    let val: u64;
    unsafe {
        asm!("mrs {0}, tcr_el1", out(reg) val, options(nostack));
    }
    val
}

/// Write TCR_EL1
pub fn set_tcr(value: u64) {
    unsafe {
        asm!("msr tcr_el1, {0}", in(reg) value, options(nostack));
    }
}

/// Read MAIR_EL1
pub fn mair() -> u64 {
    let val: u64;
    unsafe {
        asm!("mrs {0}, mair_el1", out(reg) val, options(nostack));
    }
    val
}

/// Write MAIR_EL1
pub fn set_mair(value: u64) {
    unsafe {
        asm!("msr mair_el1, {0}", in(reg) value, options(nostack));
    }
}

/// Read SCTLR_EL1
pub fn sctlr() -> u64 {
    let val: u64;
    unsafe {
        asm!("mrs {0}, sctlr_el1", out(reg) val, options(nostack));
    }
    val
}

/// Write SCTLR_EL1
pub fn set_sctlr(value: u64) {
    unsafe {
        asm!("msr sctlr_el1, {0}", in(reg) value, options(nostack));
    }
}

/// Enable the MMU (set SCTLR_EL1.M)
pub fn enable_mmu() {
    unsafe {
        let mut val = sctlr();
        val |= SCTLR_M | SCTLR_C | SCTLR_I | SCTLR_A;
        set_sctlr(val);
        // ISB to synchronize
        asm!("isb", options(nostack));
    }
}

/// Disable the MMU
pub fn disable_mmu() {
    unsafe {
        let mut val = sctlr();
        val &= !SCTLR_M;
        set_sctlr(val);
        asm!("isb", options(nostack));
    }
}

/// Perform a TLB invalidate all (ASID=0)
pub fn tlb_invalidate_all() {
    unsafe {
        asm!("tlbi vmalle1is", options(nostack));
    }
}

/// Data memory barrier
pub fn dmb() {
    unsafe {
        asm!("dmb ish", options(nostack));
    }
}

/// Instruction synchronization barrier
pub fn isb() {
    unsafe {
        asm!("isb", options(nostack));
    }
}

/// Read the current exception level
pub fn current_el() -> u64 {
    let val: u64;
    unsafe {
        asm!("mrs {0}, CurrentEL", out(reg) val, options(nostack));
    }
    (val >> 2) & 3
}

/// Kernel page directory physical address (identity-mapped boot table)
pub fn kernel_page_dir() -> u32 {
    // On AArch64, the boot translation table is set up in assembly at tt_l0.
    // Return a placeholder; the actual TTBR0 is already set by boot.S.
    0x4000_0000
}

/// Map a 2MB section (block descriptor) identity mapping
#[allow(dead_code)]
pub fn map_section(virtual_addr: u64, physical_addr: u64, attrs: u64) {
    // This would modify the level-0 table set up in boot.S.
    // Implemented minimally; full page table management is future work.
    let _ = (virtual_addr, physical_addr, attrs);
}

/// Read the current stack pointer
pub fn sp() -> u64 {
    let val: u64;
    unsafe {
        asm!("mov {0}, sp", out(reg) val, options(nostack));
    }
    val
}

/// Read the program counter (via LR after a branch)
pub fn pc() -> u64 {
    let val: u64;
    unsafe {
        asm!("adr {0}, 1f", out(reg) val);
        asm!("1:", options(nostack));
    }
    val
}

/// Read pointer auth register (reserved)
#[allow(dead_code)]
pub fn read_ptr_auth() -> u64 {
    let val: u64;
    unsafe {
        asm!("mrs {0}, ptr_auth_el1", out(reg) val, options(nostack));
    }
    val
}
