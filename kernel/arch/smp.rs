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

use crate::{arch, drivers, memory::paging, services::process};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub const MAX_CPUS: usize = 4;
const NO_APIC_ID: u32 = u32::MAX;
const AP_STACK_SIZE: usize = 8192;
const AP_STARTUP_VECTOR: u8 = 0x08;
const AP_STARTUP_TIMEOUT_TICKS: u32 = 100;

#[repr(align(4096))]
struct ApStack([u8; AP_STACK_SIZE]);

const EMPTY_STACK: ApStack = ApStack([0; AP_STACK_SIZE]);

static APIC_IDS: [AtomicU32; MAX_CPUS] = [const { AtomicU32::new(NO_APIC_ID) }; MAX_CPUS];
static ONLINE: [AtomicBool; MAX_CPUS] = [const { AtomicBool::new(false) }; MAX_CPUS];
static CPU_COUNT: AtomicUsize = AtomicUsize::new(1);
static APIC_PHYSICAL_BASE: AtomicU32 = AtomicU32::new(0);
static mut AP_STACKS: [ApStack; MAX_CPUS] = [EMPTY_STACK; MAX_CPUS];

unsafe extern "C" {
    static mut ap_trampoline_stack_top: u32;
    static mut ap_trampoline_cr3: u32;
    static mut ap_trampoline_entry: u32;
    static mut ap_trampoline_cpu_index: u32;
}

#[derive(Clone, Copy)]
struct ProcessorTable {
    apic_ids: [u8; 256],
    count: usize,
    local_apic_address: u32,
}

impl ProcessorTable {
    const EMPTY: Self = Self {
        apic_ids: [0; 256],
        count: 0,
        local_apic_address: 0,
    };
}

pub fn init(multiboot_info: *const u8) {
    let Some(mut apic_base) = read_apic_base() else {
        drivers::serial::write_string("Local APIC unavailable; using one CPU.\r\n");
        return;
    };
    let table = find_processor_table(multiboot_info);
    if table.local_apic_address != 0 {
        apic_base = table.local_apic_address;
    }

    if !arch::lapic::init(apic_base) || !arch::lapic::initialize_cpu() {
        drivers::serial::write_string("Local APIC setup failed; using one CPU.\r\n");
        return;
    }

    let bsp_apic_id = arch::lapic::id();
    APIC_PHYSICAL_BASE.store(apic_base, Ordering::Release);
    APIC_IDS[0].store(u32::from(bsp_apic_id), Ordering::Release);
    ONLINE[0].store(true, Ordering::Release);
    process::init_cpu(0, arch::tss::tss_esp0());

    let timer_ticks = arch::lapic::calibrate_timer(10);
    let Some(timer_ticks) = timer_ticks else {
        drivers::serial::write_string("Local APIC timer calibration failed.\r\n");
        return;
    };
    if !arch::lapic::start_scheduler_timer() {
        drivers::serial::write_string("Local APIC timer setup failed; using PIT scheduling.\r\n");
        return;
    }
    let mut ids = ProcessorTable::EMPTY;
    ids.apic_ids[0] = bsp_apic_id;
    ids.count = 1;
    for apic_id in table.apic_ids.iter().take(table.count) {
        if *apic_id != bsp_apic_id && !ids.apic_ids[..ids.count].contains(apic_id) {
            ids.apic_ids[ids.count] = *apic_id;
            ids.count += 1;
            if ids.count == MAX_CPUS {
                break;
            }
        }
    }

    CPU_COUNT.store(ids.count, Ordering::Release);
    for index in 1..ids.count {
        let apic_id = ids.apic_ids[index];
        APIC_IDS[index].store(u32::from(apic_id), Ordering::Release);
        process::init_cpu(index, ap_stack_top(index));
        if !start_processor(index, apic_id) {
            APIC_IDS[index].store(NO_APIC_ID, Ordering::Release);
            drivers::serial::write_string("Failed to start APIC ID ");
            drivers::serial::write_dec(u32::from(apic_id));
            drivers::serial::write_string(".\r\n");
        }
    }

    let online = ONLINE
        .iter()
        .take(ids.count)
        .filter(|cpu| cpu.load(Ordering::Acquire))
        .count();
    CPU_COUNT.store(online, Ordering::Release);
    drivers::serial::write_string("SMP online: ");
    drivers::serial::write_dec(online as u32);
    drivers::serial::write_string(" CPU(s), LAPIC timer ");
    drivers::serial::write_dec(timer_ticks);
    drivers::serial::write_string(" ticks per slice.\r\n");
}

pub fn cpu_count() -> usize {
    CPU_COUNT.load(Ordering::Acquire)
}

pub fn current_cpu_index() -> usize {
    let apic_id = u32::from(arch::lapic::id());
    APIC_IDS
        .iter()
        .position(|id| id.load(Ordering::Acquire) == apic_id)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn ap_entry(cpu_index: u32) -> ! {
    let index = cpu_index as usize;
    if index == 0 || index >= MAX_CPUS {
        stop_cpu();
    }
    let stack_top = ap_stack_top(index);
    if !arch::lapic::enable_local_apic(APIC_PHYSICAL_BASE.load(Ordering::Acquire))
        || !arch::lapic::initialize_cpu()
    {
        stop_cpu();
    }
    arch::gdt::load();
    arch::tss::init_cpu(index, stack_top);
    arch::idt::load();
    process::init_cpu(index, stack_top);
    if !arch::lapic::start_scheduler_timer() {
        stop_cpu();
    }
    ONLINE[index].store(true, Ordering::Release);
    loop {
        unsafe {
            core::arch::asm!("sti", "hlt", options(nomem, nostack));
        }
    }
}

fn start_processor(index: usize, apic_id: u8) -> bool {
    let stack_top = ap_stack_top(index);
    unsafe {
        ptr::addr_of_mut!(ap_trampoline_stack_top).write_volatile(stack_top);
        ptr::addr_of_mut!(ap_trampoline_cr3).write_volatile(paging::kernel_page_dir());
        ptr::addr_of_mut!(ap_trampoline_entry).write_volatile(ap_entry as usize as u32);
        ptr::addr_of_mut!(ap_trampoline_cpu_index).write_volatile(index as u32);
    }

    if !arch::lapic::send_init(apic_id) {
        return false;
    }
    wait_ticks(1);
    let _ = arch::lapic::send_init_deassert(apic_id);
    wait_ticks(1);
    if !arch::lapic::send_startup(apic_id, AP_STARTUP_VECTOR) {
        return false;
    }
    wait_ticks(1);
    if !ONLINE[index].load(Ordering::Acquire) {
        let _ = arch::lapic::send_startup(apic_id, AP_STARTUP_VECTOR);
    }

    let start = drivers::pit::ticks();
    while drivers::pit::ticks().wrapping_sub(start) < AP_STARTUP_TIMEOUT_TICKS {
        if ONLINE[index].load(Ordering::Acquire) {
            return true;
        }
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
    ONLINE[index].load(Ordering::Acquire)
}

fn wait_ticks(ticks: u32) {
    let start = drivers::pit::ticks();
    while drivers::pit::ticks().wrapping_sub(start) < ticks {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

fn ap_stack_top(index: usize) -> u32 {
    unsafe {
        let stack = ptr::addr_of_mut!(AP_STACKS).cast::<ApStack>().add(index);
        (stack as usize + core::mem::size_of::<ApStack>()) as u32
    }
}

fn read_apic_base() -> Option<u32> {
    let mut low: u32;
    let mut high: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") 0x1B_u32,
            out("eax") low,
            out("edx") high,
            options(nostack, preserves_flags)
        );
    }
    let value = (u64::from(high) << 32) | u64::from(low);
    if value & (1 << 10) != 0 {
        return None;
    }
    Some((value as u32) & 0xFFFF_F000)
}

fn find_processor_table(multiboot_info: *const u8) -> ProcessorTable {
    if let Some(table) = find_acpi_processor_table(multiboot_info) {
        return table;
    }
    if let Some(pointer) = find_mp_floating_pointer() {
        let config_address = unsafe { read_u32(pointer.add(4)) };
        if config_address != 0 {
            if let Some(table) = parse_mp_config(config_address as *const u8) {
                return table;
            }
        }
    }
    ProcessorTable::EMPTY
}

fn find_acpi_processor_table(multiboot_info: *const u8) -> Option<ProcessorTable> {
    if multiboot_info.is_null() {
        return None;
    }
    let total_size = unsafe { read_u32(multiboot_info) } as usize;
    if !(16..=1024 * 1024).contains(&total_size) {
        return None;
    }
    let mut tag_offset = 8usize;
    let mut rsdp_v1 = None;
    let mut rsdp_v2 = None;
    while tag_offset.checked_add(8)? <= total_size {
        let tag = unsafe { multiboot_info.add(tag_offset) };
        let tag_type = unsafe { read_u32(tag) };
        let tag_size = unsafe { read_u32(tag.add(4)) } as usize;
        if tag_size < 8 || tag_offset.checked_add(tag_size)? > total_size {
            return None;
        }
        match tag_type {
            14 => rsdp_v1 = Some(unsafe { tag.add(8) }),
            15 => rsdp_v2 = Some(unsafe { tag.add(8) }),
            0 => break,
            _ => {}
        }
        tag_offset = tag_offset.checked_add(tag_size.checked_add(7)? & !7)?;
    }

    let rsdp = rsdp_v2.or(rsdp_v1)?;
    if unsafe { read_u64(rsdp) } != u64::from_le_bytes(*b"RSD PTR ") {
        return None;
    }
    if !checksum_valid(rsdp, 20) {
        return None;
    }
    let rsdt_address = unsafe { read_u32(rsdp.add(16)) };
    if rsdt_address == 0 {
        return None;
    }
    let rsdt = map_acpi_table(rsdt_address)?;
    if unsafe { read_u32(rsdt) } != u32::from_le_bytes(*b"RSDT") {
        return None;
    }
    let rsdt_length = unsafe { read_u32(rsdt.add(4)) } as usize;
    if rsdt_length < 36 || rsdt_length > 1024 * 1024 || !checksum_valid(rsdt, rsdt_length) {
        return None;
    }
    for offset in (36..rsdt_length).step_by(4) {
        if offset + 4 > rsdt_length {
            break;
        }
        let table_address = unsafe { read_u32(rsdt.add(offset)) };
        let Some(header) = map_acpi_table(table_address) else {
            continue;
        };
        if unsafe { read_u32(header) } != u32::from_le_bytes(*b"APIC") {
            continue;
        }
        let length = unsafe { read_u32(header.add(4)) } as usize;
        if length < 44 || length > 1024 * 1024 || !checksum_valid(header, length) {
            continue;
        }
        return parse_madt(header, length);
    }
    None
}

fn map_acpi_table(physical_address: u32) -> Option<*const u8> {
    let header = paging::map_mmio(physical_address, 36)?;
    let length = unsafe { read_u32(header.add(4)) } as usize;
    if !(36..=1024 * 1024).contains(&length) {
        return None;
    }
    paging::map_mmio(physical_address, length).map(|table| table as *const u8)
}

fn parse_madt(madt: *const u8, length: usize) -> Option<ProcessorTable> {
    let mut table = ProcessorTable {
        local_apic_address: unsafe { read_u32(madt.add(36)) },
        ..ProcessorTable::EMPTY
    };
    let mut offset = 44usize;
    while offset.checked_add(2)? <= length {
        let entry_type = unsafe { ptr::read(madt.add(offset)) };
        let entry_length = unsafe { ptr::read(madt.add(offset + 1)) } as usize;
        if entry_length < 2 || offset.checked_add(entry_length)? > length {
            return None;
        }
        match entry_type {
            0 if entry_length >= 8 => {
                let apic_id = unsafe { ptr::read(madt.add(offset + 3)) };
                let flags = unsafe { read_u32(madt.add(offset + 4)) };
                if flags & 1 != 0 && table.count < table.apic_ids.len() {
                    table.apic_ids[table.count] = apic_id;
                    table.count += 1;
                }
            }
            5 if entry_length >= 12 => {
                let address = unsafe { read_u64(madt.add(offset + 4)) };
                if address <= u32::MAX as u64 {
                    table.local_apic_address = address as u32;
                }
            }
            9 if entry_length >= 16 => {
                let apic_id = unsafe { read_u32(madt.add(offset + 4)) };
                let flags = unsafe { read_u32(madt.add(offset + 8)) };
                if flags & 1 != 0 && apic_id <= u8::MAX as u32 && table.count < 256 {
                    table.apic_ids[table.count] = apic_id as u8;
                    table.count += 1;
                }
            }
            _ => {}
        }
        offset += entry_length;
    }
    Some(table)
}

fn checksum_valid(address: *const u8, length: usize) -> bool {
    unsafe {
        core::slice::from_raw_parts(address, length)
            .iter()
            .fold(0u8, |sum, value| sum.wrapping_add(*value))
            == 0
    }
}

fn find_mp_floating_pointer() -> Option<*const u8> {
    let ebda_segment = unsafe { read_u16(0x40E as *const u8) };
    let base_memory_kb = unsafe { read_u16(0x413 as *const u8) };
    let ebda = u32::from(ebda_segment) << 4;
    if ebda >= 0x80000 {
        if let Some(pointer) = scan_mp_region(ebda as *const u8, 1024) {
            return Some(pointer);
        }
    }
    let base_end = u32::from(base_memory_kb) * 1024;
    if base_end >= 1024 {
        if let Some(pointer) = scan_mp_region((base_end - 1024) as *const u8, 1024) {
            return Some(pointer);
        }
    }
    scan_mp_region(0xF0000 as *const u8, 0x10000)
}

fn scan_mp_region(start: *const u8, length: usize) -> Option<*const u8> {
    let end = (start as usize).checked_add(length)?;
    let mut address = start as usize;
    while address.checked_add(16)? <= end {
        let candidate = address as *const u8;
        if unsafe { read_u32(candidate) } == u32::from_le_bytes(*b"_MP_") {
            let size = unsafe { ptr::read(candidate.add(8)) as usize }.checked_mul(16)?;
            if size >= 16 && address.checked_add(size)? <= end {
                let checksum = unsafe {
                    core::slice::from_raw_parts(candidate, size)
                        .iter()
                        .fold(0u8, |sum, value| sum.wrapping_add(*value))
                };
                if checksum == 0 {
                    return Some(candidate);
                }
            }
        }
        address += 16;
    }
    None
}

fn parse_mp_config(address: *const u8) -> Option<ProcessorTable> {
    if unsafe { read_u32(address) } != u32::from_le_bytes(*b"PCMP") {
        return None;
    }
    let length = usize::from(unsafe { read_u16(address.add(4)) });
    let entry_count = usize::from(unsafe { read_u16(address.add(34)) });
    let local_apic_address = unsafe { read_u32(address.add(36)) };
    if length < 44 || length > 64 * 1024 {
        return None;
    }
    let checksum = unsafe {
        core::slice::from_raw_parts(address, length)
            .iter()
            .fold(0u8, |sum, value| sum.wrapping_add(*value))
    };
    if checksum != 0 {
        return None;
    }

    let mut table = ProcessorTable {
        local_apic_address,
        ..ProcessorTable::EMPTY
    };
    let mut offset = 44usize;
    for _ in 0..entry_count {
        if offset >= length {
            return None;
        }
        let entry_type = unsafe { ptr::read(address.add(offset)) };
        let entry_size = if entry_type == 0 { 20 } else { 8 };
        if offset.checked_add(entry_size)? > length {
            return None;
        }
        if entry_type == 0 {
            let apic_id = unsafe { ptr::read(address.add(offset + 1)) };
            let flags = unsafe { ptr::read(address.add(offset + 3)) };
            if flags & 1 != 0 && table.count < table.apic_ids.len() {
                table.apic_ids[table.count] = apic_id;
                table.count += 1;
            }
        }
        offset += entry_size;
    }
    Some(table)
}

unsafe fn read_u16(address: *const u8) -> u16 {
    ptr::read_unaligned(address.cast::<u16>())
}

unsafe fn read_u32(address: *const u8) -> u32 {
    ptr::read_unaligned(address.cast::<u32>())
}

unsafe fn read_u64(address: *const u8) -> u64 {
    ptr::read_unaligned(address.cast::<u64>())
}

fn stop_cpu() -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}
