const DATA: u16 = 0x1F0;
const ERROR: u16 = 0x1F1;
const SECTOR_COUNT: u16 = 0x1F2;
const LBA_LOW: u16 = 0x1F3;
const LBA_MID: u16 = 0x1F4;
const LBA_HIGH: u16 = 0x1F5;
const DRIVE: u16 = 0x1F6;
const STATUS: u16 = 0x1F7;
const COMMAND: u16 = 0x1F7;
const DIRECTORY_SECTOR: u32 = 1;
const DATA_START: u32 = 2;
const FILE_SLOTS: usize = 16;
const MAGIC: &[u8; 8] = b"NEBFS001";

#[derive(Clone, Copy)]
pub struct FileInfo {
    pub name: [u8; 16],
    pub length: u16,
}

impl FileInfo {
    const EMPTY: Self = Self { name: [0; 16], length: 0 };
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    NoDisk,
    Io,
    NotFormatted,
    NotBlank,
    InvalidName,
    FileTooLarge,
    DirectoryFull,
    NotFound,
}

static mut SECTORS: u32 = 0;
static mut DISK_READY: bool = false;
static mut FORMATTED: bool = false;

fn out8(port: u16, value: u8) {
    unsafe { core::arch::asm!("out dx, al", in("dx") port, in("al") value); }
}

fn in8(port: u16) -> u8 {
    let value: u8;
    unsafe { core::arch::asm!("in al, dx", in("dx") port, out("al") value); }
    value
}

fn in16(port: u16) -> u16 {
    let value: u16;
    unsafe { core::arch::asm!("in ax, dx", in("dx") port, out("ax") value); }
    value
}

fn out16(port: u16, value: u16) {
    unsafe { core::arch::asm!("out dx, ax", in("dx") port, in("ax") value); }
}

fn wait_ready(require_data: bool) -> Result<u8, StorageError> {
    for _ in 0..1_000_000 {
        let status = in8(STATUS);
        if status == 0 || status == 0xFF {
            return Err(StorageError::NoDisk);
        }
        if status & 0x80 == 0 {
            if status & 0x21 != 0 {
                let _ = in8(ERROR);
                return Err(StorageError::Io);
            }
            if !require_data || status & 0x08 != 0 {
                return Ok(status);
            }
        }
        core::hint::spin_loop();
    }
    Err(StorageError::Io)
}

fn select_sector(lba: u32) -> Result<(), StorageError> {
    let sectors = unsafe { SECTORS };
    if lba >= sectors || lba >= (1 << 28) {
        return Err(StorageError::Io);
    }
    out8(DRIVE, 0xE0 | ((lba >> 24) as u8 & 0x0F));
    for _ in 0..4 { let _ = in8(STATUS); }
    out8(SECTOR_COUNT, 1);
    out8(LBA_LOW, lba as u8);
    out8(LBA_MID, (lba >> 8) as u8);
    out8(LBA_HIGH, (lba >> 16) as u8);
    Ok(())
}

fn read_sector(lba: u32, buffer: &mut [u8; 512]) -> Result<(), StorageError> {
    select_sector(lba)?;
    out8(COMMAND, 0x20);
    wait_ready(true)?;
    for word in 0..256 {
        let value = in16(DATA).to_le_bytes();
        buffer[word * 2] = value[0];
        buffer[word * 2 + 1] = value[1];
    }
    wait_ready(false)?;
    Ok(())
}

fn write_sector(lba: u32, buffer: &[u8; 512]) -> Result<(), StorageError> {
    select_sector(lba)?;
    out8(COMMAND, 0x30);
    wait_ready(true)?;
    for word in 0..256 {
        out16(DATA, u16::from_le_bytes([buffer[word * 2], buffer[word * 2 + 1]]));
    }
    out8(0x3F6, 0);
    wait_ready(false)?;
    out8(DRIVE, 0xE0 | ((lba >> 24) as u8 & 0x0F));
    out8(COMMAND, 0xE7);
    wait_ready(false)?;
    Ok(())
}

pub fn init() -> Result<(), StorageError> {
    out8(DRIVE, 0xA0);
    for _ in 0..4 { let _ = in8(STATUS); }
    out8(SECTOR_COUNT, 0);
    out8(LBA_LOW, 0);
    out8(LBA_MID, 0);
    out8(LBA_HIGH, 0);
    out8(COMMAND, 0xEC);
    let status = in8(STATUS);
    if status == 0 || status == 0xFF {
        return Err(StorageError::NoDisk);
    }
    wait_ready(true)?;
    let mut sectors = 0u32;
    for index in 0..256 {
        let word = in16(DATA);
        if index == 60 { sectors = word as u32; }
        if index == 61 { sectors |= (word as u32) << 16; }
    }
    if sectors <= DATA_START + FILE_SLOTS as u32 {
        return Err(StorageError::Io);
    }
    unsafe {
        SECTORS = sectors;
        DISK_READY = true;
    }
    let mut header = [0; 512];
    read_sector(0, &mut header)?;
    unsafe { FORMATTED = &header[..8] == MAGIC; }
    Ok(())
}

pub fn is_available() -> bool { unsafe { DISK_READY } }
pub fn is_formatted() -> bool { unsafe { FORMATTED } }

pub fn format_if_blank() -> Result<(), StorageError> {
    if !unsafe { DISK_READY } { return Err(StorageError::NoDisk); }
    let mut header = [0; 512];
    read_sector(0, &mut header)?;
    if header.iter().any(|byte| *byte != 0 && *byte != 0xFF) {
        return Err(StorageError::NotBlank);
    }
    header[..8].copy_from_slice(MAGIC);
    header[8..12].copy_from_slice(&1u32.to_le_bytes());
    write_sector(0, &header)?;
    write_sector(DIRECTORY_SECTOR, &[0; 512])?;
    unsafe { FORMATTED = true; }
    Ok(())
}

pub fn list_files() -> Result<[FileInfo; FILE_SLOTS], StorageError> {
    require_formatted()?;
    let mut directory = [0; 512];
    read_sector(DIRECTORY_SECTOR, &mut directory)?;
    let mut files = [FileInfo::EMPTY; FILE_SLOTS];
    for (index, file) in files.iter_mut().enumerate() {
        let entry = &directory[index * 32..index * 32 + 32];
        if entry[19] == 1 {
            file.name.copy_from_slice(&entry[..16]);
            file.length = u16::from_le_bytes([entry[16], entry[17]]);
        }
    }
    Ok(files)
}

pub fn read_file(name: &[u8], output: &mut [u8; 512]) -> Result<usize, StorageError> {
    let (directory, slot) = find_entry(name)?;
    let entry = &directory[slot * 32..slot * 32 + 32];
    let length = u16::from_le_bytes([entry[16], entry[17]]) as usize;
    let data_sector = entry[18] as u32;
    read_sector(DATA_START + data_sector, output)?;
    Ok(length)
}

pub fn write_file(name: &[u8], content: &[u8]) -> Result<(), StorageError> {
    if name.is_empty() || name.len() > 15 || name.iter().any(|b| !b.is_ascii_alphanumeric() && *b != b'.' && *b != b'_' && *b != b'-') {
        return Err(StorageError::InvalidName);
    }
    if content.len() > 512 { return Err(StorageError::FileTooLarge); }
    require_formatted()?;
    let mut directory = [0; 512];
    read_sector(DIRECTORY_SECTOR, &mut directory)?;
    let existing = (0..FILE_SLOTS).find(|index| {
        let entry = &directory[index * 32..index * 32 + 32];
        let stored_len = entry[..16].iter().position(|byte| *byte == 0).unwrap_or(16);
        entry[19] == 1 && &entry[..stored_len] == name
    });
    let slot = existing.or_else(|| (0..FILE_SLOTS).find(|index| directory[index * 32 + 19] != 1))
        .ok_or(StorageError::DirectoryFull)?;
    let mut data = [0; 512];
    data[..content.len()].copy_from_slice(content);
    write_sector(DATA_START + slot as u32, &data)?;
    let entry = &mut directory[slot * 32..slot * 32 + 32];
    entry[..32].fill(0);
    entry[..name.len()].copy_from_slice(name);
    entry[16..18].copy_from_slice(&(content.len() as u16).to_le_bytes());
    entry[18] = slot as u8;
    entry[19] = 1;
    write_sector(DIRECTORY_SECTOR, &directory)
}

pub fn delete_file(name: &[u8]) -> Result<(), StorageError> {
    let (mut directory, slot) = find_entry(name)?;
    directory[slot * 32..slot * 32 + 32].fill(0);
    write_sector(DIRECTORY_SECTOR, &directory)
}

pub fn rename_file(old_name: &[u8], new_name: &[u8]) -> Result<(), StorageError> {
    if new_name.is_empty() || new_name.len() > 15 || new_name.iter().any(|b| !b.is_ascii_alphanumeric() && *b != b'.' && *b != b'_' && *b != b'-') {
        return Err(StorageError::InvalidName);
    }
    let (mut directory, slot) = find_entry(old_name)?;
    for index in 0..FILE_SLOTS {
        if index == slot { continue; }
        let entry = &directory[index * 32..index * 32 + 32];
        let stored_len = entry[..16].iter().position(|byte| *byte == 0).unwrap_or(16);
        if entry[19] == 1 && &entry[..stored_len] == new_name {
            return Err(StorageError::InvalidName);
        }
    }
    let entry = &mut directory[slot * 32..slot * 32 + 32];
    entry[..16].fill(0);
    entry[..new_name.len()].copy_from_slice(new_name);
    write_sector(DIRECTORY_SECTOR, &directory)
}

fn require_formatted() -> Result<(), StorageError> {
    if !unsafe { DISK_READY } { return Err(StorageError::NoDisk); }
    if !unsafe { FORMATTED } { return Err(StorageError::NotFormatted); }
    Ok(())
}

fn find_entry(name: &[u8]) -> Result<([u8; 512], usize), StorageError> {
    require_formatted()?;
    let mut directory = [0; 512];
    read_sector(DIRECTORY_SECTOR, &mut directory)?;
    for index in 0..FILE_SLOTS {
        let entry = &directory[index * 32..index * 32 + 32];
        let stored_len = entry[..16].iter().position(|byte| *byte == 0).unwrap_or(16);
        if entry[19] == 1 && &entry[..stored_len] == name {
            return Ok((directory, index));
        }
    }
    Err(StorageError::NotFound)
}
