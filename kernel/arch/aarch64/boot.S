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

// AArch64 boot entry point for NebulaOS
// Boots from QEMU -kernel, sets up MMU, drops to EL1, jumps to Rust

.section .text.boot
.global _start
.global _start_aarch64

#define PAGE_SIZE   4096
#define SECTION_SIZE (2 * 1024 * 1024)   // 2MB section
#define IDENTITY_MAP_SIZE (1 * 1024 * 1024 * 1024) // 1GB identity map

#define MAIR_DEVICE 0x00
#define MAIR_NORMAL 0x44

#define TCR_T0SZ_1GB  25   // 64 - 30 = 25, 1GB = 2^30
#define TCR_IRGN0_WB  1    // Inner WB
#define TCR_ORGN0_WB  1    // Outer WB
#define TCR_SH0_IS    2    // Inner Shareable

_start:
_start_aarch64:
    // Disable all interrupts
    msr daifclr, #0xf
    msr daifset, #0xf

    // Set up stack pointer (top of a reserved area)
    adr x1, stack_top
    mov sp, x1

    // Clear BSS
    adr x0, __bss_start
    adr x1, __bss_end
1:
    cmp x0, x1
    b.hs 2f
    stp xzr, xzr, [x0], #16
    b 1b
2:

    // Set up MMU translation tables (identity map)
    bl setup_mmu

    // Enable the MMU
    bl enable_mmu

    // Jump to the Rust kernel entry
    adr x0, kernel_entry
    br x0

    // Should never reach here
3:
    wfe
    b 3b

// ---------------------------------------------------------------------------
// setup_mmu - Set up stage-1 translation tables for identity mapping
// Maps 1GB of physical memory 1:1 starting at address 0
// ---------------------------------------------------------------------------
setup_mmu:
    // TTBR0 points to the level-0 table at 0x40000000 (in reserved mem)
    // We use a single level-0 table with 2MB sections covering 1GB
    adr x0, tt_l0
    mov x1, #0
    msr ttbr0_el1, x0

    // MAIR: Attr0 = Device, Attr1 = Normal WB
    mov x0, #(MAIR_NORMAL << 8 | MAIR_DEVICE)
    msr mair_el1, x0

    // TCR: T0SZ=25 (1GB), IRGN0=WB, ORGN0=WB, SH0=Inner Shareable, EPD0=0
    mov x0, #(TCR_T0SZ_1GB << 0 | TCR_IRGN0_WB << 8 | TCR_ORGN0_WB << 10 | TCR_SH0_IS << 12)
    msr tcr_el1, x0

    // SCTLR_EL1: enable MMU, cache, alignment, etc.
    mrs x0, sctlr_el1
    orr x0, x0, #(1 << 0)   // M - MMU enable
    orr x0, x0, #(1 << 2)   // C - Cache enable
    orr x0, x0, #(1 << 3)   // A - Alignment check
    orr x0, x0, #(1 << 12)  // I - Instruction cache
    orr x0, x0, #(1 << 11)  // Z - Branch prediction
    and x0, x0, #~(1 << 1)  // A1 - No strict alignment
    msr sctlr_el1, x0

    isb
    ret

// ---------------------------------------------------------------------------
// enable_mmu - Enable the MMU and branch to new translation regime
// ---------------------------------------------------------------------------
enable_mmu:
    mrs x0, sctlr_el1
    orr x0, x0, #(1 << 0)   // M = MMU enable
    msr sctlr_el1, x0
    isb
    ret

// ---------------------------------------------------------------------------
// Translation tables (1 level, 2MB sections)
// ---------------------------------------------------------------------------
.section .data.mmu
.align 12
tt_l0:
// 512 entries * 2MB = 1GB identity map
// Entry format: [31:20]=PA[39:28], [19]=nG, [18]=AF, [17]=SH[1:1]... 
// For simplicity, use a repeatable macro

.rept 512
    .quad ((\.) - tt_l0) * 8 | 0x0000000000000001 | (0x1 << 10) | (0x1 << 8) | (0x1 << 6) | (0x1 << 5)
.endr

// Stack (aligned)
.section .bss.stack
.align 12
stack:
    .skip 4096
stack_top:

// ---------------------------------------------------------------------------
// Exception vectors (minimal - loop on any exception)
// ---------------------------------------------------------------------------
.section .text.vectors
.align 12
vectors:
// Current EL0 with SP0
    b .hang
// Current EL0 with SPx
    b .hang
// Current EL1 with SP0
    b .hang
// Current EL1 with SPx
    b .hang
// Lower EL AArch64
    b .hang
// Lower EL AArch32
    b .hang
// Other exceptions
    b .hang
// SError
    b .hang

.hang:
    wfe
    b .hang
