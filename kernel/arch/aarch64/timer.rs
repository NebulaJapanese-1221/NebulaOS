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

// AArch64 architected timer (CNTPCT_EL0, CNTP_TVAL_EL0, CNTP_CTL_EL0)

use core::arch::asm;
use core::sync::atomic::{AtomicU32, Ordering};

static TICKS: AtomicU32 = AtomicU32::new(0);
static TICKS_PER_SECOND: AtomicU32 = AtomicU32::new(100);

const CNTP_CTL_EL0: u32 = 0;
const CNTP_TVAL_EL0: u32 = 1;
const CNTPCT_EL0: u32 = 2;

/// Read the physical counter (CNTPCT_EL0)
fn cntpct_read() -> u64 {
    let val: u64;
    unsafe {
        asm!("mrs {0}, cntpct_el0", out(reg) val, options(nostack));
    }
    val
}

/// Initialize the architected timer to fire at the given frequency (Hz)
pub fn init() {
    // Set timer frequency to 100 Hz
    let ticks_per_second = 100u32;
    TICKS_PER_SECOND.store(ticks_per_second, Ordering::Release);

    // Calculate timer value for 10ms ticks
    // The counter runs at the system counter frequency; read CNTFRQ_EL0
    let counter_freq: u32;
    unsafe {
        asm!("mrs {0}, cntfrq_el0", out(reg) counter_freq, options(nostack));
    }

    let tick_delta = counter_freq / ticks_per_second;
    if tick_delta == 0 {
        // Fallback: use a reasonable default
        unsafe {
            asm!("msr cntp_tval_el0, {0}", in(reg) 1_000_000u32, options(nostack));
            asm!("msr cntp_ctl_el0, {0}", in(reg) 1u32, options(nostack));
        }
    } else {
        unsafe {
            asm!("msr cntp_tval_el0, {0}", in(reg) tick_delta, options(nostack));
            asm!("msr cntp_ctl_el0, {0}", in(reg) 1u32, options(nostack));
        }
    }
}

/// Get the current tick count
pub fn ticks() -> u32 {
    TICKS.load(Ordering::Acquire)
}

/// Timer interrupt handler - called from the exception vector on IRQ
pub fn timer_tick() {
    TICKS.fetch_add(1, Ordering::Release);

    // Re-arm the timer for the next tick
    let counter_freq: u32;
    unsafe {
        asm!("mrs {0}, cntfrq_el0", out(reg) counter_freq, options(nostack));
    }
    let ticks_per_second = TICKS_PER_SECOND.load(Ordering::Acquire);
    let tick_delta = counter_freq / ticks_per_second;
    if tick_delta != 0 {
        unsafe {
            asm!("msr cntp_tval_el0, {0}", in(reg) tick_delta, options(nostack));
        }
    }
}

/// Sleep for approximately the given number of milliseconds
pub fn sleep_ms(ms: u32) {
    let start = ticks();
    let ticks_needed = (ms as u64 * TICKS_PER_SECOND.load(Ordering::Acquire) as u64 / 1000).max(1);
    while (ticks() as u64) - (start as u64) < ticks_needed {
        unsafe {
            asm!("wfe", options(nomem, nostack));
        }
    }
}
