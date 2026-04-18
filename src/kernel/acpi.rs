use super::io;
use core::{mem::size_of, slice};
use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};

static EC_PRESENT: AtomicBool = AtomicBool::new(false);
static PM1A_CNT_BLK: AtomicU32 = AtomicU32::new(0);
static SHUTDOWN_CMD: AtomicU16 = AtomicU16::new(0);
static BATTERY_BST_PTR: AtomicU32 = AtomicU32::new(0);
static BATTERY_BIF_PTR: AtomicU32 = AtomicU32::new(0);
static THERMAL_TMP_PTR: AtomicU32 = AtomicU32::new(0);
static DSDT_PTR: AtomicU32 = AtomicU32::new(0);
static DSDT_LEN: AtomicU32 = AtomicU32::new(0);

static EC_REG_BACKLIGHT: AtomicU16 = AtomicU16::new(0);
static EC_REG_BATTERY_STATUS: AtomicU16 = AtomicU16::new(0);

// AML OpCodes
const AML_EXT_OP_PREFIX: u8 = 0x5B;
const AML_BYTE_PREFIX: u8 = 0x0A; // ByteConst
const AML_WORD_PREFIX: u8 = 0x0B; // WordConst
const AML_DWORD_PREFIX: u8 = 0x0C; // DWordConst
const AML_OP_REGION_OP: u8 = 0x80; // ExtOpPrefix + OpRegion
const AML_PACKAGE_OP: u8 = 0x12; // PackageOp
const AML_METHOD_OP: u8 = 0x14; // MethodOp
const AML_RETURN_OP: u8 = 0xA4; // ReturnOp
const AML_FIELD_OP: u8 = 0x81;  // ExtOpPrefix + FieldOp
const AML_ZERO_OP: u8 = 0x00; // ZeroOp
const AML_ONE_OP: u8 = 0x01; // OneOp
const AML_ONES_OP: u8 = 0xFF; // OnesOp

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

/// A basic AML stream reader used to navigate DSDT bytecode.
struct AmlStream<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> AmlStream<'a> {
    fn new(data: &'a [u8], offset: usize) -> Self {
        Self { data, offset }
    }

    fn peek_u8(&self) -> Option<u8> {
        self.data.get(self.offset).copied()
    }

    fn read_u8(&mut self) -> Option<u8> {
        let b = self.data.get(self.offset).copied();
        if b.is_some() { self.offset += 1; }
        b
    }

    /// Reads an AML integer constant (ByteConst, WordConst, DWordConst, etc.).
    fn parse_integer(&mut self) -> Option<u64> {
        match self.read_u8()? {
            AML_ZERO_OP => Some(0),
            AML_ONE_OP => Some(1),
            AML_ONES_OP => Some(0xFFFFFFFFFFFFFFFF),
            AML_BYTE_PREFIX => self.read_u8().map(|v| v as u64),
            AML_WORD_PREFIX => {
                let low = self.read_u8()? as u64;
                let high = self.read_u8()? as u64;
                Some(low | (high << 8))
            }
            AML_DWORD_PREFIX => {
                let mut val = 0u64;
                for i in 0..4 {
                    val |= (self.read_u8()? as u64) << (i * 8);
                }
                Some(val)
            }
            _ => None,
        }
    }

    /// Skips a PkgLength field and returns the calculated length.
    fn skip_pkg_length(&mut self) -> Option<usize> {
        let lead = self.read_u8()?;
        let byte_count = (lead >> 6) as usize;
        let mut pkg_len = (lead & 0x3F) as usize;
        for i in 0..byte_count {
            pkg_len |= (self.read_u8()? as usize) << (6 + i * 8);
        }
        Some(pkg_len)
    }

    /// Reads a 4-character ACPI name.
    fn read_name(&mut self) -> Option<[u8; 4]> {
        Some([self.read_u8()?, self.read_u8()?, self.read_u8()?, self.read_u8()?])
    }

    /// Parses a full NameString and returns the final 4-character NameSeg.
    /// This handles root prefixes, parent prefixes, and multi-part paths.
    fn parse_name_string(&mut self) -> Option<[u8; 4]> {
        let mut b = self.peek_u8()?;
        if b == b'\\' { // Root prefix
            self.read_u8();
            b = self.peek_u8()?;
        }
        while b == b'^' { // Parent prefix
            self.read_u8();
            b = self.peek_u8()?;
        }

        match b {
            0x00 => { self.read_u8(); None } // NullName
            0x2E => { // DualNamePrefix
                self.read_u8();
                self.read_name(); // Skip first segment
                self.read_name()   // Return second segment
            }
            0x2F => { // MultiNamePrefix
                self.read_u8();
                let count = self.read_u8()? as usize;
                let mut last = [0u8; 4];
                for _ in 0..count { last = self.read_name()?; }
                Some(last)
            }
            _ => self.read_name(), // Single NameSeg
        }
    }
}

/// Searches the DSDT for a specific AML object name.
fn find_aml_object_bytecode(dsdt_data: &[u8], object_name: &[u8; 4]) -> Option<u32> {
    dsdt_data.windows(4).position(|window| window == object_name).map(|pos| pos as u32)
}






unsafe fn get_s5_val(dsdt_ptr: *const SdtHeader) -> Option<u8> {
    let dsdt_len = core::ptr::addr_of!((*dsdt_ptr).length).read_unaligned() as usize;
    if dsdt_len < size_of::<SdtHeader>() {
        return None;
    }
    let data_ptr = (dsdt_ptr as *const u8).add(size_of::<SdtHeader>());
    let data = slice::from_raw_parts(data_ptr, dsdt_len - size_of::<SdtHeader>());

    if let Some(s5_offset) = find_aml_object_bytecode(data, b"_S5_") {
        if data[s5_offset as usize + 4] == AML_BYTE_PREFIX { // Check for ByteConst
            return Some(data[s5_offset as usize + 5]); // Value of ByteConst
        }
    }
    None
}

unsafe fn parse_madt(madt_ptr: *const SdtHeader) {
    // MADT Structure: Header (44 bytes) + Interrupt Controller Structures
    let len = core::ptr::addr_of!((*madt_ptr).length).read_unaligned() as usize;
    if len < 44 { return; }
    
    // Skip the standard header (36 bytes) + Local APIC Address (4) + Flags (4) = 44 bytes
    let mut current_ptr = (madt_ptr as *const u8).add(44);
    let end_ptr = (madt_ptr as *const u8).add(len);

    let mut cores = 0;

    while current_ptr < end_ptr {
        let entry_type = *current_ptr;
        let entry_len = *current_ptr.add(1);

        if entry_type == 0 { // Processor Local APIC
            // Offset 4 is the Flags field (u32). Bit 0 is 'Processor Enabled'.
            let flags = (current_ptr.add(4) as *const u32).read_unaligned();
            if (flags & 1) == 1 {
                cores += 1;
            }
        }

        current_ptr = current_ptr.add(entry_len as usize);
    }

    if cores > 0 {
        crate::kernel::CPU_CORES.store(cores, Ordering::Relaxed);
    }
}

pub fn init() {
    if let Some(rsdp) = find_rsdp() {
        let rsdt_ptr = unsafe { core::ptr::addr_of!((*rsdp).rsdt_address).read_unaligned() } as *const SdtHeader;
        if let Some(fadt_ptr) = find_sdt_in_rsdt(rsdt_ptr, b"FACP") {
            let fadt = fadt_ptr as *const Fadt;
            let pm1a_cnt_port = unsafe { core::ptr::addr_of!((*fadt).pm1a_cnt_blk).read_unaligned() } as u16;
            let dsdt_ptr = unsafe { core::ptr::addr_of!((*fadt).dsdt).read_unaligned() } as *const SdtHeader;
            let dsdt_len = unsafe { core::ptr::addr_of!((*dsdt_ptr).length).read_unaligned() as usize };

            if pm1a_cnt_port == 0 || dsdt_ptr.is_null() {
                return;
            }

            // Fragile DSDT parsing to find the _S5_ value
            let s5_val = unsafe { get_s5_val(dsdt_ptr).unwrap_or(0) };

            // Write SLP_TYPa << 10 | SLP_EN to the PM1a control port
            let shutdown_val = (s5_val as u16) << 10 | 0x2000;
            
            PM1A_CNT_BLK.store(pm1a_cnt_port as u32, Ordering::Relaxed);
            SHUTDOWN_CMD.store(shutdown_val, Ordering::Relaxed);

            // Cache DSDT for efficient periodic updates
            DSDT_PTR.store(dsdt_ptr as u32, Ordering::Relaxed);
            DSDT_LEN.store(dsdt_len as u32, Ordering::Relaxed);

            // Use the DSDT pointer found in the FADT to detect power devices
                let dsdt_data = unsafe { slice::from_raw_parts(dsdt_ptr as *const u8, dsdt_len) };
                crate::drivers::ec::init(); // Initialize EC driver
                detect_ec(dsdt_data);
                discover_ec_offsets(dsdt_data);
            detect_battery(dsdt_data);
            detect_thermal(dsdt_data);
        }

        // Parse MADT for core count
        if let Some(madt_ptr) = find_sdt_in_rsdt(rsdt_ptr, b"APIC") {
            unsafe { parse_madt(madt_ptr); }
        }
        
        // Initialize brightness driver with default level
        crate::drivers::brightness::BRIGHTNESS.lock().init();
    }
}

/// Detects the presence of an Embedded Controller (EC) by scanning the DSDT.
fn detect_ec(dsdt_data: &[u8]) {
    if dsdt_data.windows(4).any(|w| w == b"EC0_" || w == b"EC__") {
        EC_PRESENT.store(true, Ordering::Relaxed);
        crate::serial_println!("[ACPI] Embedded Controller detected in DSDT.");
    }
}

/// Dynamically discovers register offsets within the EC OperationRegion.
fn discover_ec_offsets(dsdt_data: &[u8]) {
    let mut ec_region_name: Option<[u8; 4]> = None;
    let mut stream = AmlStream::new(dsdt_data, 0);

    // 1. Find OperationRegion(..., EmbeddedControl, ...)
    while stream.offset < dsdt_data.len() - 10 {
        if stream.read_u8() == Some(AML_EXT_OP_PREFIX) && stream.peek_u8() == Some(AML_OP_REGION_OP) {
            stream.read_u8(); // Consume OpRegion opcode
            let name = stream.parse_name_string();
            let region_space = stream.read_u8();
            if region_space == Some(0x03) { // 0x03 = EmbeddedControl
                ec_region_name = name;
                break;
            }
        }
    }

    let region_name = if let Some(n) = ec_region_name { n } else { return; };

    // 2. Find Field(RegionName, ...)
    stream.offset = 0; // Restart scan
    while stream.offset < dsdt_data.len() - 10 {
        if stream.read_u8() == Some(AML_EXT_OP_PREFIX) && stream.peek_u8() == Some(AML_FIELD_OP) {
            stream.read_u8(); // Consume FieldOp
            let _pkg_len = stream.skip_pkg_length();
            let target_name = stream.parse_name_string();
            
            if target_name == Some(region_name) {
                stream.read_u8(); // AccessType
                let mut bit_offset = 0;
                
                // Simple walk of the field list (4-byte names or special offsets)
                while stream.offset < dsdt_data.len() {
                    match stream.read_u8() {
                        Some(0x00) => { // ReservedField
                            bit_offset += stream.skip_pkg_length().unwrap_or(0);
                        },
                        Some(0x01) => { // AccessField
                            stream.read_u8(); stream.read_u8(); // Skip access type/attrib
                        },
                        Some(c) if c >= b'A' && c <= b'Z' || c == b'_' => {
                            // It's a named field (4 characters)
                            stream.offset -= 1;
                            let name = stream.read_name().unwrap();
                            let bit_length = stream.skip_pkg_length().unwrap_or(0);
                            
                            let byte_addr = (bit_offset / 8) as u16;
                            
                            // Map known names used by Dell/Standard laptops
                            if &name == b"BRIT" || &name == b"BCM_" {
                                EC_REG_BACKLIGHT.store(byte_addr, Ordering::Relaxed);
                                crate::serial_println!("[ACPI] Found Backlight EC Reg: {:#x}", byte_addr);
                            } else if &name == b"BSTS" {
                                EC_REG_BATTERY_STATUS.store(byte_addr, Ordering::Relaxed);
                                crate::serial_println!("[ACPI] Found Battery Status EC Reg: {:#x}", byte_addr);
                            }
                            
                            bit_offset += bit_length;
                        },
                        _ => break,
                    }
                }
            }
        }
    }
}

fn detect_battery(dsdt_data: &[u8]) {
    // This is a simplified approach. A full AML parser would build a namespace tree.
    // We're looking for the "PNP0C0A" HID within the DSDT.
    // Once found, we then search for _BST and _BIF methods relative to that device.
    
    // Search for "PNP0C0A" (Control Method Battery)
    if let Some(pnp_offset) = dsdt_data.windows(7).position(|window| window == b"PNP0C0A") {
        crate::serial_println!("[ACPI] Battery device (PNP0C0A) detected in DSDT.");

        // Now, search for _BST and _BIF methods *after* this PNP0C0A entry.
        // This is still a heuristic, but often works for simple DSDTs.
        let search_start = pnp_offset + 7; // Start searching after the HID
        let remaining_dsdt = &dsdt_data[search_start..];

        if let Some(bst_offset) = find_aml_object_bytecode(remaining_dsdt, b"_BST") {
            BATTERY_BST_PTR.store((search_start + bst_offset as usize) as u32, Ordering::Relaxed);
            crate::serial_println!("[ACPI] _BST method found at offset {:#x}", search_start + bst_offset as usize);
        }
        if let Some(bif_offset) = find_aml_object_bytecode(remaining_dsdt, b"_BIF") {
            BATTERY_BIF_PTR.store((search_start + bif_offset as usize) as u32, Ordering::Relaxed);
            crate::serial_println!("[ACPI] _BIF method found at offset {:#x}", search_start + bif_offset as usize);
        }

        // Only notify the driver if at least the status method was found
        if BATTERY_BST_PTR.load(Ordering::Relaxed) != 0 {
            crate::drivers::battery::BATTERY.lock().update_status(0, false, None, None, None);
        }
    }
}

fn detect_thermal(dsdt_data: &[u8]) {
    // Search for _TMP (Temperature) method or object
    if let Some(tmp_offset) = find_aml_object_bytecode(dsdt_data, b"_TMP") {
        THERMAL_TMP_PTR.store(tmp_offset, Ordering::Relaxed);
        crate::serial_println!("[ACPI] Thermal monitoring (_TMP) found at offset {:#x}", tmp_offset);
    }
}

/// Evaluates a simple AML method that returns a Package of integers.
/// Writes results into `out_buffer` and returns the number of elements parsed.
fn evaluate_aml_method(method_bytecode_ptr: u32, dsdt_data: &[u8], out_buffer: &mut [u64]) -> Option<usize> {
    if method_bytecode_ptr == 0 { return None; }

    let method_offset = method_bytecode_ptr as usize;
    if method_offset >= dsdt_data.len() { return None; }

    let mut stream = AmlStream::new(dsdt_data, method_offset);

    // Robustly skip the NameString
    stream.parse_name_string()?;

    // Skip MethodOp and PkgLength, NumArgs
    if stream.read_u8()? != AML_METHOD_OP { return None; }
    let pkg_start = stream.offset;
    let pkg_len = stream.skip_pkg_length()?;
    let scope_end = (pkg_start + pkg_len).min(dsdt_data.len());
    stream.read_u8()?; // NumArgs

    // Look for ReturnOp (0xA4) followed by PackageOp (0x12) within the method scope
    while stream.offset < scope_end {
        if stream.read_u8()? == AML_RETURN_OP {
            if stream.read_u8()? == AML_PACKAGE_OP {
                stream.skip_pkg_length()?; // Skip package length
                let num_elements = stream.read_u8()? as usize;
                let to_parse = num_elements.min(out_buffer.len());
                for i in 0..to_parse {
                    // Only parse as long as we find valid integers. 
                    // This allows us to skip trailing strings in _BIF.
                    if let Some(val) = stream.parse_integer() {
                        out_buffer[i] = val;
                    } else {
                        return Some(i);
                    }
                }
                return Some(to_parse);
            }
        }
    }
    None
}

/// Evaluates an AML method that returns a single integer.
fn evaluate_aml_integer_method(method_bytecode_ptr: u32, dsdt_data: &[u8]) -> Option<u64> {
    if method_bytecode_ptr == 0 { return None; }
    let method_offset = method_bytecode_ptr as usize;
    if method_offset >= dsdt_data.len() { return None; }

    let mut stream = AmlStream::new(dsdt_data, method_offset);
    
    stream.parse_name_string()?; // Skip name

    // If it's a Name object instead of a Method, it might just be the integer prefix
    if let Some(prefix) = stream.peek_u8() {
        if prefix == AML_BYTE_PREFIX || prefix == AML_WORD_PREFIX || prefix == AML_DWORD_PREFIX || prefix == AML_ZERO_OP || prefix == AML_ONE_OP {
            return stream.parse_integer();
        }
    }

    if stream.read_u8()? != AML_METHOD_OP { return None; }
    let pkg_start = stream.offset;
    let pkg_len = stream.skip_pkg_length()?;
    let scope_end = (pkg_start + pkg_len).min(dsdt_data.len());
    stream.read_u8()?; // NumArgs

    while stream.offset < scope_end {
        if stream.read_u8()? == AML_RETURN_OP {
            return stream.parse_integer();
        }
    }
    None
}

/// Periodically called to synchronize ACPI state with kernel drivers.
pub fn update_power_status() {
    let bst_ptr = BATTERY_BST_PTR.load(Ordering::Relaxed);
    let bif_ptr = BATTERY_BIF_PTR.load(Ordering::Relaxed);
    let tmp_ptr = THERMAL_TMP_PTR.load(Ordering::Relaxed);
    let dsdt_addr = DSDT_PTR.load(Ordering::Relaxed);
    let dsdt_len = DSDT_LEN.load(Ordering::Relaxed);

    // DSDT not cached yet or no power-related devices were discovered
    if dsdt_addr == 0 || (bst_ptr == 0 && bif_ptr == 0 && tmp_ptr == 0) {
        return;
    }

    let dsdt_data = unsafe { slice::from_raw_parts(dsdt_addr as *const u8, dsdt_len as usize) };

    // 1. Evaluate _BIF (Battery Information) if not already done or if needed
    let mut design_capacity: Option<u32> = None;
    let mut full_charge_capacity: Option<u32> = None;
    let mut cycle_count: Option<u32> = None;

    let mut aml_buffer = [0u64; 16]; // Fixed-size stack buffer to avoid heap allocations

    if bif_ptr != 0 {
        if let Some(count) = evaluate_aml_method(bif_ptr, dsdt_data, &mut aml_buffer) {
            if count >= 2 {
                design_capacity = Some(aml_buffer[1] as u32);
            }
            if count >= 3 {
                full_charge_capacity = Some(aml_buffer[2] as u32);
            }
            // Some implementations append Cycle Count to _BIF, or this may be a _BIX package
            if count >= 9 {
                cycle_count = Some(aml_buffer[8] as u32);
            }
        }
    }

    // 2. Evaluate _BST (Battery Status)
    if bst_ptr != 0 {
        if let Some(count) = evaluate_aml_method(bst_ptr, dsdt_data, &mut aml_buffer) {
            if count >= 4 {
                let state_flags = aml_buffer[0] as u32;
                let remaining_capacity = aml_buffer[2] as u32; // mWh
                
                let is_charging = (state_flags & 0x02) != 0;
                crate::drivers::battery::BATTERY.lock().update_status(remaining_capacity, is_charging, design_capacity, full_charge_capacity, cycle_count);
            }
        }
    }

    // 3. Evaluate Temperature
    if tmp_ptr != 0 {
        if let Some(decikelvins) = evaluate_aml_integer_method(tmp_ptr, dsdt_data) {
            // Decikelvins to Celsius: (K / 10) - 273.15
            let celsius = (decikelvins as i64 / 10) - 273;
            crate::kernel::cpu::CPU_TEMP.store(celsius.max(0) as usize, Ordering::Relaxed);
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

/// Sets the backlight level by writing to the discovered EC register.
pub fn acpi_set_backlight_level(level: u8) {
    if EC_PRESENT.load(Ordering::Relaxed) && crate::drivers::ec::is_initialized() {
        let reg = EC_REG_BACKLIGHT.load(Ordering::Relaxed);
        if reg != 0 {
            // Scale 0-100 level to 0-255 for the EC register (common range for Dell/Standard laptops)
            let ec_value = (level as u16 * 255 / 100) as u8;
            crate::drivers::ec::ec_write_byte(reg as u8, ec_value);
            crate::serial_println!("[ACPI] EC: Set backlight level to {}% via Reg {:#x}", level, reg);
        }
    }
}