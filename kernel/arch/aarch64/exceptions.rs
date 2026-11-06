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

// AArch64 exception handling
// Sets up the vector table and handles synchronous/asynchronous exceptions

use core::arch::asm;
use crate::drivers::serial;

const ESR_EC_SHIFT: u32 = 26;
const ESR_EC_MASK: u32 = 0x3F << ESR_EC_SHIFT;

// Exception classes
const EC_UNKNOWN: u32 = 0;
const EC_WFE_WFI: u32 = 1;
const EC_MRS_CP15_64: u32 = 7;
const EC_ILLEGAL_STATE: u32 = 11;
const EC_SVC_AARCH64: u32 = 15;
const EC_HVC_AARCH64: u32 = 16;
const EC_SMC_AARCH64: u32 = 17;
const EC_SYSREG_AARCH64: u32 = 18;
const EC_INSTR_ABORT_LOWER: u32 = 20;
const EC_INSTR_ABORT_SAME: u32 = 21;
const EC_PC_ALIGNMENT: u32 = 22;
const EC_DATA_ABORT_LOWER: u32 = 24;
const EC_DATA_ABORT_SAME: u32 = 25;
const EC_SP_ALIGNMENT: u32 = 26;
const EC_FPSIMD: u32 = 28;
const EC_ERROR: u32 = 31;

static mut EXCEPTION_COUNT: u32 = 0;

/// Initialize exception handling.
/// The vector table is set up in boot.S; this just enables relevant traps.
pub fn init() {
    // Clear any pending exception state
    unsafe {
        EXCEPTION_COUNT = 0;
    }
    serial::write_string("AArch64 exceptions initialized.\r\n");
}

/// Get the number of exceptions taken
pub fn exception_count() -> u32 {
    unsafe { EXCEPTION_COUNT }
}

/// Common exception handler entry point (called from assembly vector table)
/// Registers are passed via x0-x30, and the exception syndrome is in ESR_EL1.
#[no_mangle]
pub extern "C" fn handle_exception(esr: u32, elr: u64, spsr: u64, sp: u64) {
    unsafe {
        EXCEPTION_COUNT += 1;
    }

    let ec = (esr >> ESR_EC_SHIFT) & ESR_EC_MASK;
    let iss = esr & 0x03FFFFFF;

    serial::write_string("AArch64 exception: EC=");
    crate::drivers::serial::write_dec(ec as u32);
    serial::write_string(" ISS=");
    crate::drivers::serial::write_hex(iss);
    serial::write_string(" ELR=");
    crate::drivers::serial::write_hex(elr as u32);
    serial::write_string(" SPSR=");
    crate::drivers::serial::write_hex(spsr as u32);
    serial::write_string(" SP=");
    crate::drivers::serial::write_hex(sp as u32);
    serial::write_string("\r\n");

    match ec {
        EC_SVC_AARCH64 => {
            serial::write_string("  -> SVC call\r\n");
        }
        EC_DATA_ABORT_SAME | EC_DATA_ABORT_LOWER => {
            serial::write_string("  -> Data abort\r\n");
        }
        EC_INSTR_ABORT_SAME | EC_INSTR_ABORT_LOWER => {
            serial::write_string("  -> Instruction abort\r\n");
        }
        _ => {
            serial::write_string("  -> Unknown exception\r\n");
        }
    }
}

/// Panic handler entry point
#[no_mangle]
pub extern "C" fn exception_panic() -> ! {
    serial::write_string("AArch64 kernel panic - halting.\r\n");
    loop {
        unsafe {
            asm!("wfe", options(nomem, nostack));
        }
    }
}
