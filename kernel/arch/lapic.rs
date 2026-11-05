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

use crate::memory::paging;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

const IA32_APIC_BASE: u32 = 0x1B;
const APIC_ENABLE: u64 = 1 << 11;
const APIC_X2APIC_ENABLE: u64 = 1 << 10;

const REG_ID: usize = 0x020;
const REG_TPR: usize = 0x080;
const REG_EOI: usize = 0x0B0;
const REG_SVR: usize = 0x0F0;
const REG_ICR_LOW: usize = 0x300;
const REG_ICR_HIGH: usize = 0x310;
const REG_LVT_TIMER: usize = 0x320;
const REG_TIMER_INITIAL: usize = 0x380;
const REG_TIMER_CURRENT: usize = 0x390;
const REG_TIMER_DIVIDE: usize = 0x3E0;

const TIMER_VECTOR: u32 = 48;
const SPURIOUS_VECTOR: u32 = 0xFF;
const LVT_MASKED: u32 = 1 << 16;
const LVT_PERIODIC: u32 = 1 << 17;

static LAPIC: AtomicUsize = AtomicUsize::new(0);
static TIMER_TICKS_PER_SLICE: AtomicUsize = AtomicUsize::new(0);

pub fn init(physical_base: u32) -> bool {
    let Some(base) = paging::map_mmio(physical_base, 4096) else {
        return false;
    };
    LAPIC.store(base as usize, Ordering::Release);
    enable_local_apic(physical_base)
}

pub fn enable_local_apic(physical_base: u32) -> bool {
    let mut low: u32;
    let mut high: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") IA32_APIC_BASE,
            out("eax") low,
            out("edx") high,
            options(nostack, preserves_flags)
        );
    }
    let value = (u64::from(high) << 32) | u64::from(low);
    if value & APIC_X2APIC_ENABLE != 0 {
        return false;
    }
    let value = (value & !0xFFFF_F000) | u64::from(physical_base & 0xFFFF_F000) | APIC_ENABLE;
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") IA32_APIC_BASE,
            in("eax") value as u32,
            in("edx") (value >> 32) as u32,
            options(nostack, preserves_flags)
        );
    }
    true
}

pub fn initialize_cpu() -> bool {
    if LAPIC.load(Ordering::Acquire) == 0 {
        return false;
    }
    write(REG_TPR, 0);
    write(REG_SVR, 0x100 | SPURIOUS_VECTOR);
    write(REG_TIMER_DIVIDE, 0x3);
    write(REG_LVT_TIMER, TIMER_VECTOR | LVT_MASKED);
    write(REG_TIMER_INITIAL, 0);
    true
}

pub fn id() -> u8 {
    (read(REG_ID) >> 24) as u8
}

pub fn calibrate_timer(ticks: u32) -> Option<u32> {
    if ticks == 0 {
        return None;
    }
    write(REG_TIMER_DIVIDE, 0x3);
    write(REG_LVT_TIMER, TIMER_VECTOR | LVT_MASKED);
    write(REG_TIMER_INITIAL, u32::MAX);
    let start = crate::drivers::pit::ticks();
    while crate::drivers::pit::ticks().wrapping_sub(start) < ticks {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
    let elapsed = u32::MAX.wrapping_sub(read(REG_TIMER_CURRENT));
    let per_tick = elapsed.checked_div(ticks)?.max(1);
    TIMER_TICKS_PER_SLICE.store(per_tick as usize, Ordering::Release);
    Some(per_tick)
}

pub fn start_scheduler_timer() -> bool {
    let ticks = TIMER_TICKS_PER_SLICE.load(Ordering::Acquire) as u32;
    if ticks == 0 {
        return false;
    }
    write(REG_TIMER_DIVIDE, 0x3);
    write(REG_LVT_TIMER, TIMER_VECTOR | LVT_PERIODIC);
    write(REG_TIMER_INITIAL, ticks);
    true
}

pub fn end_of_interrupt() {
    write(REG_EOI, 0);
}

pub fn send_init(apic_id: u8) -> bool {
    send_ipi(apic_id, 0x0000_C500)
}

pub fn send_init_deassert(apic_id: u8) -> bool {
    send_ipi(apic_id, 0x0000_8500)
}

pub fn send_startup(apic_id: u8, vector: u8) -> bool {
    send_ipi(apic_id, 0x0000_0600 | u32::from(vector))
}

fn send_ipi(apic_id: u8, command: u32) -> bool {
    if LAPIC.load(Ordering::Acquire) == 0 {
        return false;
    }
    for _ in 0..100_000 {
        if read(REG_ICR_LOW) & (1 << 12) == 0 {
            break;
        }
        core::hint::spin_loop();
    }
    write(REG_ICR_HIGH, u32::from(apic_id) << 24);
    write(REG_ICR_LOW, command);
    for _ in 0..100_000 {
        if read(REG_ICR_LOW) & (1 << 12) == 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

fn read(offset: usize) -> u32 {
    let base = LAPIC.load(Ordering::Acquire);
    if base == 0 {
        return 0;
    }
    unsafe { ptr::read_volatile((base + offset) as *const u32) }
}

fn write(offset: usize, value: u32) {
    let base = LAPIC.load(Ordering::Acquire);
    if base != 0 {
        unsafe {
            ptr::write_volatile((base + offset) as *mut u32, value);
        }
    }
}
