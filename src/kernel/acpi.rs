use super::io;
use core::{mem::size_of, slice};
use core::sync::atomic::{AtomicU16, AtomicU32, Ordering};

static PM1A_CNT_BLK: AtomicU32 = AtomicU32::new(0);
static SHUTDOWN_CMD: AtomicU16 = AtomicU16::new(0);

#[repr(C, packed)]
struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    _oem_id: [u8; 6],
    _revision: u8,
    rsdt_address: u32,
}

impl Rsdp {
    fn is_valid(&self) -> bool {
        let bytes = unsafe { slice::from_raw_parts(self as *const _ as *const u8, 20) };
        let sum = bytes.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        sum == 0
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
    // Scan the BIOS read-only memory area for the RSDP signature
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
        let rsdt_len = (*rsdt).length as usize;
        if rsdt_len < size_of::<SdtHeader>() {
            return None;
        }
        let entry_count = (rsdt_len - size_of::<SdtHeader>()) / 4;
        let entries_ptr = (rsdt as *const u8).add(size_of::<SdtHeader>()) as *const u32;

        for i in 0..entry_count {
            let sdt_ptr = *entries_ptr.add(i) as *const SdtHeader;
            if (*sdt_ptr).signature == *signature && (*sdt_ptr).is_valid() {
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
    let dsdt_len = (*dsdt_ptr).length as usize;
    if dsdt_len < size_of::<SdtHeader>() {
        return None;
    }
    let data_ptr = (dsdt_ptr as *const u8).add(size_of::<SdtHeader>());
    let data = slice::from_raw_parts(data_ptr, dsdt_len - size_of::<SdtHeader>());

    if let Some(s5_ptr) = find_signature_in_slice(data, b"_S5_") {
        if *s5_ptr.offset(4) == 0x12 {
            return Some(*s5_ptr.offset(8));
        }
    }
    None
}

pub fn init() {
    if let Some(rsdp) = find_rsdp() {
        let rsdt_ptr = unsafe { (*rsdp).rsdt_address } as *const SdtHeader;
        if let Some(fadt_ptr) = find_sdt_in_rsdt(rsdt_ptr, b"FACP") {
            let fadt = unsafe { &*(fadt_ptr as *const Fadt) };
            let pm1a_cnt_port = fadt.pm1a_cnt_blk as u16;
            let dsdt_ptr = fadt.dsdt as *const SdtHeader;

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
    }
}

pub fn acpi_shutdown() {
    let port = PM1A_CNT_BLK.load(Ordering::Relaxed) as u16;
    let cmd = SHUTDOWN_CMD.load(Ordering::Relaxed);
    if port != 0 {
        unsafe { io::outw(port, cmd); }
    }
}