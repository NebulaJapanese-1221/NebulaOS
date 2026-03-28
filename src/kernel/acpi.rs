use super::io;
use core::{mem::size_of, slice};
use core::sync::atomic::{AtomicU16, AtomicU32, Ordering};

static PM1A_CNT_BLK: AtomicU32 = AtomicU32::new(0);
static SHUTDOWN_CMD: AtomicU16 = AtomicU16::new(0);

#[repr(C, packed)]
struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    // ACPI 2.0+ fields
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    _reserved: [u8; 3],
}

impl Rsdp {
    fn is_valid(&self) -> bool {
        // Checksum for Version 1.0 (first 20 bytes)
        let bytes = unsafe { slice::from_raw_parts(self as *const _ as *const u8, 20) };
        let sum = bytes.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        if sum != 0 { return false; }

        // Extended checksum for Version 2.0+ (entire structure)
        if self.revision >= 2 {
            let length = unsafe { core::ptr::addr_of!(self.length).read_unaligned() as usize };
            // Sanity check: RSDP shouldn't be suspiciously large
            if length > 1024 || length < 36 { return false; }
            let bytes = unsafe { slice::from_raw_parts(self as *const _ as *const u8, length) };
            let sum = bytes.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
            if sum != 0 { return false; }
        }
        true
    }
}

#[repr(C, packed)]
pub struct SdtHeader {
    signature: [u8; 4],
    length: u32,
    _revision: u8,
    checksum: u8,
    _oem_id: [u8; 6],
    _oem_table_id: [u8; 8],
    _oem_revision: u32,
    _creator_id: u32,
    _creator_revision: u32,
}

impl SdtHeader {
    fn is_valid(&self) -> bool {
        let length = unsafe { core::ptr::addr_of!(self.length).read_unaligned() as usize };
        // ACPI tables are rarely larger than 1MB; avoid massive OOB reads if corrupted
        if length < size_of::<SdtHeader>() || length > 1024 * 1024 { return false; }
        let bytes = unsafe { slice::from_raw_parts(self as *const _ as *const u8, length) };
        let sum = bytes.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        sum == 0
    }
}

#[repr(C, packed)]
struct Fadt {
    header: SdtHeader,
    _firmware_ctrl: u32,
    dsdt: u32,
    _reserved: u8,
    _preferred_pm_profile: u8,
    _sci_int: u16,
    _smi_cmd: u32,
    _acpi_enable: u8,
    _acpi_disable: u8,
    _s4bios_req: u8,
    _pstate_cnt: u8,
    _pm1a_evt_blk: u32,
    _pm1b_evt_blk: u32,
    pm1a_cnt_blk: u32,
    // There are more fields, but we only need up to here for shutdown.
}

fn find_rsdp() -> Option<*const Rsdp> {
    // 1. Check the first 1KB of the EBDA
    // The EBDA segment address is stored at 0x40E in the BDA
    let ebda_base = unsafe { (*(0x40E as *const u16) as usize) << 4 };
    if ebda_base > 0x400 {
        for addr in (ebda_base..ebda_base + 1024).step_by(16) {
            let rsdp = addr as *const Rsdp;
            unsafe {
                if (*rsdp).signature == *b"RSD PTR " && (*rsdp).is_valid() {
                    return Some(rsdp);
                }
            }
        }
    }

    // 2. Scan the BIOS read-only memory area
    for addr in (0x000E0000..=0x000FFFFF).step_by(16) {
        let rsdp = addr as *const Rsdp;
        unsafe {
            if (*rsdp).signature == *b"RSD PTR " && (*rsdp).is_valid() {
                return Some(rsdp);
            }
        }
    }
    None
}

/// Generically finds an SDT in either an RSDT or XSDT.
fn find_sdt(root: *const SdtHeader, signature: &[u8; 4]) -> Option<*const SdtHeader> {
    if root.is_null() { return None; }
    
    let (entry_size, table_len) = unsafe {
        let header = &*root;
        if !header.is_valid() { return None; }
        
        // RSDT uses 32-bit pointers (4 bytes), XSDT uses 64-bit pointers (8 bytes)
        let is_xsdt = &header.signature == b"XSDT";
        (if is_xsdt { 8 } else { 4 }, header.length as usize)
    };

    let entry_count = (table_len.saturating_sub(size_of::<SdtHeader>())) / entry_size;
    let entries_ptr = unsafe { (root as *const u8).add(size_of::<SdtHeader>()) };

    for i in 0..entry_count {
        let sdt_ptr = unsafe {
            if entry_size == 4 {
                (entries_ptr as *const u32).add(i).read_unaligned() as *const SdtHeader
            } else {
                (entries_ptr as *const u64).add(i).read_unaligned() as *const SdtHeader
            }
        };

        if !sdt_ptr.is_null() {
            let sdt = unsafe { &*sdt_ptr };
            if &sdt.signature == signature && sdt.is_valid() {
                return Some(sdt_ptr);
            }
        }
    }
    None
}


fn get_s5_val(dsdt_ptr: *const SdtHeader) -> Option<u8> {
    if dsdt_ptr.is_null() { return None; }
    let dsdt_len = unsafe { core::ptr::addr_of!((*dsdt_ptr).length).read_unaligned() as usize };
    if dsdt_len < size_of::<SdtHeader>() {
        return None;
    }
    let data = unsafe { 
        let ptr = (dsdt_ptr as *const u8).add(size_of::<SdtHeader>());
        slice::from_raw_parts(ptr, dsdt_len - size_of::<SdtHeader>())
    };

    // AML encoding for Name(_S5, Package...) usually looks like:
    // 08 (NameOp) 5F 53 35 5F (_S5_) 12 (PackageOp) ...
    // We scan for the NameOp + Signature to ensure we are at the start of the object.
    for i in 0..data.len().saturating_sub(5) {
        if data[i] == 0x08 && &data[i+1..i+5] == b"_S5_" {
            // Search for the PackageOp (0x12) within a reasonable window after the name
            for j in (i + 5)..(i + 15).min(data.len()) {
                if data[j] == 0x12 {
                    let mut p = j + 1;
                    if p >= data.len() { break; }

                    // Skip PkgLength (AML encoding: 1-4 bytes depending on top 2 bits)
                    let pkg_lead = data[p];
                    p += if pkg_lead < 0x40 { 1 } else { ((pkg_lead >> 6) & 0x03) as usize + 1 };
                    
                    // Skip NumElements byte (usually 1 for _S5)
                    p += 1;
                    if p >= data.len() { break; }

                    // Parse the actual value (SLP_TYPa). It may have a BytePrefix (0x0A) 
                    // or be a small integer OpCode (0x00-0x09).
                    return match data[p] {
                        0x0A => data.get(p + 1).copied(),
                        val if val <= 0x09 => Some(val),
                        _ => Some(data[p]),
                    };
                }
            }
        }
    }
    None
}

fn parse_madt(madt_ptr: *const SdtHeader) {
    if madt_ptr.is_null() { return; }
    // MADT Structure: Header (44 bytes) + Interrupt Controller Structures
    let len = unsafe { core::ptr::addr_of!((*madt_ptr).length).read_unaligned() as usize };
    if len < 44 { return; }
    
    let data = unsafe { slice::from_raw_parts(madt_ptr as *const u8, len) };
    let mut pos = 44; // Skip standard header (36) + LAPIC Addr (4) + Flags (4)
    let mut cores = 0;

    while pos + 2 <= len {
        let entry_type = data[pos];
        let entry_len = data[pos + 1] as usize;

        if entry_len < 2 || pos + entry_len > len {
            break;
        }

        if entry_type == 0 { // Processor Local APIC
            // Local APIC Structure: Type(0), Length(8), ProcID(1), APIC_ID(1), Flags(4)
            if entry_len >= 8 {
                let flags = u32::from_le_bytes([
                    data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]
                ]);
                if (flags & 1) == 1 {
                    cores += 1;
                }
            }
        }
        pos += entry_len;
    }

    if cores > 0 {
        crate::kernel::CPU_CORES.store(cores, Ordering::Relaxed);
    }
}

pub fn init() {
    if let Some(rsdp) = find_rsdp() {
        let root_table = unsafe {
            if (*rsdp).revision >= 2 && (*rsdp).xsdt_address != 0 {
                (*rsdp).xsdt_address as *const SdtHeader
            } else {
                core::ptr::addr_of!((*rsdp).rsdt_address).read_unaligned() as *const SdtHeader
            }
        };

        let fadt_ptr = find_sdt(root_table, b"FACP");

        if let Some(fadt_ptr) = fadt_ptr {
            let fadt = fadt_ptr as *const Fadt;
            let pm1a_cnt_port = unsafe { core::ptr::addr_of!((*fadt).pm1a_cnt_blk).read_unaligned() } as u16;
            let dsdt_ptr = unsafe { core::ptr::addr_of!((*fadt).dsdt).read_unaligned() } as *const SdtHeader;

            if pm1a_cnt_port == 0 || dsdt_ptr.is_null() {
                return;
            }

            // Fragile DSDT parsing to find the _S5_ value
            let s5_val = get_s5_val(dsdt_ptr).unwrap_or(0);

            // Write SLP_TYPa << 10 | SLP_EN to the PM1a control port
            let shutdown_val = (s5_val as u16) << 10 | 0x2000;
            
            PM1A_CNT_BLK.store(pm1a_cnt_port as u32, Ordering::Relaxed);
            SHUTDOWN_CMD.store(shutdown_val, Ordering::Relaxed);
        }

        let madt_ptr = find_sdt(root_table, b"APIC");

        if let Some(madt_ptr) = madt_ptr {
            parse_madt(madt_ptr);
        }
    }
}

pub fn acpi_shutdown() {
    let port = PM1A_CNT_BLK.load(Ordering::Relaxed) as u16;
    let cmd = SHUTDOWN_CMD.load(Ordering::Relaxed);
    if port != 0 {
        unsafe { io::outw(port, cmd); }
    }
}