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
            let bytes = unsafe { slice::from_raw_parts(self as *const _ as *const u8, self.length as usize) };
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
        let bytes = unsafe { slice::from_raw_parts(self as *const _ as *const u8, self.length as usize) };
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

fn find_sdt_in_rsdt(rsdt: *const SdtHeader, signature: &[u8; 4]) -> Option<*const SdtHeader> {
    unsafe {
        // Use read_unaligned to safely read the length field
        let rsdt_len = core::ptr::addr_of!((*rsdt).length).read_unaligned() as usize;

        if rsdt_len < size_of::<SdtHeader>() {
            return None;
        }
        let entry_count = (rsdt_len - size_of::<SdtHeader>()) / 4;
        let entries_ptr = (rsdt as *const u8).add(size_of::<SdtHeader>()) as *const u32;

        for i in 0..entry_count {
            // Use read_unaligned to safely read the pointer from the list
            let sdt_ptr = entries_ptr.add(i).read_unaligned() as *const SdtHeader;
            if (*sdt_ptr).signature == *signature && (*sdt_ptr).is_valid() {
                return Some(sdt_ptr);
            }
        }
    }
    None
}

fn find_sdt_in_xsdt(xsdt: *const SdtHeader, signature: &[u8; 4]) -> Option<*const SdtHeader> {
    unsafe {
        let xsdt_len = core::ptr::addr_of!((*xsdt).length).read_unaligned() as usize;
        if xsdt_len < size_of::<SdtHeader>() {
            return None;
        }
        
        let entry_count = (xsdt_len - size_of::<SdtHeader>()) / 8;
        let entries_ptr = (xsdt as *const u8).add(size_of::<SdtHeader>()) as *const u64;

        for i in 0..entry_count {
            let sdt_ptr = entries_ptr.add(i).read_unaligned() as *const SdtHeader;
            if !sdt_ptr.is_null() && (*sdt_ptr).signature == *signature && (*sdt_ptr).is_valid() {
                return Some(sdt_ptr);
            }
        }
    }
    None
}

fn find_signature_in_slice(data: &[u8], signature: &[u8; 4]) -> Option<*const u8> {
    //let sig = data.windows(4).position(|window| window == signature);
    if data.len() < 4 {
        return None;
    }
    for i in 0..(data.len() - 4) {
        if &data[i..i + 4] == signature {
            unsafe {
                return Some(data.as_ptr().add(i));
            }
        }
    }
    None
}






unsafe fn get_s5_val(dsdt_ptr: *const SdtHeader) -> Option<u8> {
    let dsdt_len = core::ptr::addr_of!((*dsdt_ptr).length).read_unaligned() as usize;
    if dsdt_len < size_of::<SdtHeader>() {
        return None;
    }
    let data_ptr = (dsdt_ptr as *const u8).add(size_of::<SdtHeader>());
    let data = slice::from_raw_parts(data_ptr, dsdt_len - size_of::<SdtHeader>());

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

unsafe fn parse_madt(madt_ptr: *const SdtHeader) {
    // MADT Structure: Header (44 bytes) + Interrupt Controller Structures
    let len = core::ptr::addr_of!((*madt_ptr).length).read_unaligned() as usize;
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
        // Prefer XSDT on ACPI 2.0+ systems, fallback to RSDT
        let (root_table, is_xsdt) = unsafe {
            if (*rsdp).revision >= 2 && (*rsdp).xsdt_address != 0 {
                ((*rsdp).xsdt_address as *const SdtHeader, true)
            } else {
                (core::ptr::addr_of!((*rsdp).rsdt_address).read_unaligned() as *const SdtHeader, false)
            }
        };

        let fadt_ptr = if is_xsdt {
            find_sdt_in_xsdt(root_table, b"FACP")
        } else {
            find_sdt_in_rsdt(root_table, b"FACP")
        };

        if let Some(fadt_ptr) = fadt_ptr {
            let fadt = fadt_ptr as *const Fadt;
            let pm1a_cnt_port = unsafe { core::ptr::addr_of!((*fadt).pm1a_cnt_blk).read_unaligned() } as u16;
            let dsdt_ptr = unsafe { core::ptr::addr_of!((*fadt).dsdt).read_unaligned() } as *const SdtHeader;

            if pm1a_cnt_port == 0 || dsdt_ptr.is_null() {
                return;
            }

            // Fragile DSDT parsing to find the _S5_ value
            let s5_val = unsafe { get_s5_val(dsdt_ptr).unwrap_or(0) };

            // Write SLP_TYPa << 10 | SLP_EN to the PM1a control port
            let shutdown_val = (s5_val as u16) << 10 | 0x2000;
            
            PM1A_CNT_BLK.store(pm1a_cnt_port as u32, Ordering::Relaxed);
            SHUTDOWN_CMD.store(shutdown_val, Ordering::Relaxed);
        }

        let madt_ptr = if is_xsdt {
            find_sdt_in_xsdt(root_table, b"APIC")
        } else {
            find_sdt_in_rsdt(root_table, b"APIC")
        };

        if let Some(madt_ptr) = madt_ptr {
            unsafe { parse_madt(madt_ptr); }
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