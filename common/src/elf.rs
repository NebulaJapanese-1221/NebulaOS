pub const EI_MAG0: usize = 0;
pub const EI_MAG1: usize = 1;
pub const EI_MAG2: usize = 2;
pub const EI_MAG3: usize = 3;
pub const EI_CLASS: usize = 4;
pub const EI_DATA: usize = 5;
pub const EI_VERSION: usize = 6;
pub const EI_OSABI: usize = 7;
pub const EI_ABIVERSION: usize = 8;
pub const EI_PAD: usize = 9;
pub const EI_NIDENT: usize = 16;

pub const ELFCLASSNONE: u8 = 0;
pub const ELFCLASS32: u8 = 1;
pub const ELFCLASS64: u8 = 2;

pub const ELFDATANONE: u8 = 0;
pub const ELFDATA2LSB: u8 = 1;
pub const ELFDATA2MSB: u8 = 2;

pub const EV_NONE: u32 = 0;
pub const EV_CURRENT: u32 = 1;

pub const ET_NONE: u16 = 0;
pub const ET_REL: u16 = 1;
pub const ET_EXEC: u16 = 2;
pub const ET_DYN: u16 = 3;
pub const ET_CORE: u16 = 4;

pub const PT_NULL: u32 = 0;
pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_INTERP: u32 = 3;
pub const PT_NOTE: u32 = 4;
pub const PT_SHLIB: u32 = 5;
pub const PT_PHDR: u32 = 6;

pub const ELFMAG0: u8 = 0x7F;
pub const ELFMAG1: u8 = b'E';
pub const ELFMAG2: u8 = b'L';
pub const ELFMAG3: u8 = b'F';

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Elf32Header {
    pub e_ident: [u8; EI_NIDENT],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u32,
    pub e_phoff: u32,
    pub e_shoff: u32,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Elf64Header {
    pub e_ident: [u8; EI_NIDENT],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Elf32ProgramHeader {
    pub p_type: u32,
    pub p_offset: u32,
    pub p_vaddr: u32,
    pub p_paddr: u32,
    pub p_filesz: u32,
    pub p_memsz: u32,
    pub p_flags: u32,
    pub p_align: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Elf64ProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

pub fn elf_validate(image: &[u8], expected_class: u8) -> Option<u64> {
    if expected_class != ELFCLASS32 && expected_class != ELFCLASS64 {
        return None;
    }
    if image.len() < EI_NIDENT
        || image[EI_MAG0] != ELFMAG0
        || image[EI_MAG1] != ELFMAG1
        || image[EI_MAG2] != ELFMAG2
        || image[EI_MAG3] != ELFMAG3
        || image[EI_CLASS] != expected_class
        || image[EI_DATA] != ELFDATA2LSB
        || image[EI_VERSION] != EV_CURRENT as u8
    {
        return None;
    }

    if expected_class == ELFCLASS32 {
        elf_validate32(image)
    } else {
        elf_validate64(image)
    }
}

fn elf_validate32(image: &[u8]) -> Option<u64> {
    const HEADER_SIZE: usize = core::mem::size_of::<Elf32Header>();
    const PROGRAM_HEADER_SIZE: usize = core::mem::size_of::<Elf32ProgramHeader>();
    if image.len() < HEADER_SIZE
        || read_u16(image, 16)? != ET_EXEC
        || read_u16(image, 18)? != 3
        || read_u32(image, 20)? != EV_CURRENT
        || (read_u16(image, 40)? as usize) < HEADER_SIZE
    {
        return None;
    }
    let entry = read_u32(image, 24)? as u64;
    let phoff = read_u32(image, 28)? as usize;
    let phentsize = read_u16(image, 42)? as usize;
    let phnum = read_u16(image, 44)? as usize;
    if phentsize < PROGRAM_HEADER_SIZE
        || phoff.checked_add(phentsize.checked_mul(phnum)?)? > image.len()
    {
        return None;
    }
    validate_program_headers32(image, entry, phoff, phentsize, phnum)
}

fn validate_program_headers32(image: &[u8], entry: u64, phoff: usize, phentsize: usize, phnum: usize) -> Option<u64> {
    let mut executable_entry = false;
    for index in 0..phnum {
        let offset = phoff.checked_add(index.checked_mul(phentsize)?)?;
        if read_u32(image, offset)? != PT_LOAD {
            continue;
        }
        let file_offset = read_u32(image, offset + 4)? as u64;
        let virtual_address = read_u32(image, offset + 8)? as u64;
        let file_size = read_u32(image, offset + 16)? as u64;
        let memory_size = read_u32(image, offset + 20)? as u64;
        let flags = read_u32(image, offset + 24)?;
        if file_size > memory_size
            || file_offset.checked_add(file_size)? > image.len() as u64
            || virtual_address.checked_add(memory_size).is_none()
        {
            return None;
        }
        if flags & 1 != 0 && entry >= virtual_address && entry < virtual_address + memory_size {
            executable_entry = true;
        }
    }
    executable_entry.then_some(entry)
}

fn elf_validate64(image: &[u8]) -> Option<u64> {
    const HEADER_SIZE: usize = core::mem::size_of::<Elf64Header>();
    const PROGRAM_HEADER_SIZE: usize = core::mem::size_of::<Elf64ProgramHeader>();
    if image.len() < HEADER_SIZE
        || read_u16(image, 16)? != ET_EXEC
        || read_u16(image, 18)? != 62
        || read_u32(image, 20)? != EV_CURRENT
        || (read_u16(image, 52)? as usize) < HEADER_SIZE
    {
        return None;
    }
    let entry = read_u64(image, 24)?;
    let phoff = usize::try_from(read_u64(image, 32)?).ok()?;
    let phentsize = read_u16(image, 54)? as usize;
    let phnum = read_u16(image, 56)? as usize;
    if phentsize < PROGRAM_HEADER_SIZE
        || phoff.checked_add(phentsize.checked_mul(phnum)?)? > image.len()
    {
        return None;
    }
    validate_program_headers64(image, entry, phoff, phentsize, phnum)
}

fn validate_program_headers64(image: &[u8], entry: u64, phoff: usize, phentsize: usize, phnum: usize) -> Option<u64> {
    let mut executable_entry = false;
    for index in 0..phnum {
        let offset = phoff.checked_add(index.checked_mul(phentsize)?)?;
        if read_u32(image, offset)? != PT_LOAD {
            continue;
        }
        let flags = read_u32(image, offset + 4)?;
        let file_offset = read_u64(image, offset + 8)?;
        let virtual_address = read_u64(image, offset + 16)?;
        let file_size = read_u64(image, offset + 32)?;
        let memory_size = read_u64(image, offset + 40)?;
        if file_size > memory_size
            || file_offset.checked_add(file_size)? > image.len() as u64
            || virtual_address.checked_add(memory_size).is_none()
        {
            return None;
        }
        if flags & 1 != 0 && entry >= virtual_address && entry < virtual_address + memory_size {
            executable_entry = true;
        }
    }
    executable_entry.then_some(entry)
}

fn read_u16(image: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(image.get(offset..offset.checked_add(2)?)?.try_into().ok()?))
}

fn read_u32(image: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(image.get(offset..offset.checked_add(4)?)?.try_into().ok()?))
}

fn read_u64(image: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(image.get(offset..offset.checked_add(8)?)?.try_into().ok()?))
}

pub unsafe fn elf_load(image: *const u8, entry_point: *mut *mut u8) -> i32 {
    let ident = image as *const [u8; EI_NIDENT];
    if (*ident)[EI_MAG0] != ELFMAG0 || (*ident)[EI_MAG1] != ELFMAG1 || (*ident)[EI_MAG2] != ELFMAG2 || (*ident)[EI_MAG3] != ELFMAG3 {
        return -1;
    }
    if (*ident)[EI_CLASS] == ELFCLASS64 {
        let header = image as *const Elf64Header;
        *entry_point = (*header).e_entry as *mut u8;
    } else if (*ident)[EI_CLASS] == ELFCLASS32 {
        let header = image as *const Elf32Header;
        *entry_point = (*header).e_entry as *mut u8;
    } else {
        return -1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_elf64() -> [u8; 128] {
        let mut image = [0u8; 128];
        image[0..4].copy_from_slice(b"\x7FELF");
        image[EI_CLASS] = ELFCLASS64;
        image[EI_DATA] = ELFDATA2LSB;
        image[EI_VERSION] = EV_CURRENT as u8;
        image[16..18].copy_from_slice(&ET_EXEC.to_le_bytes());
        image[18..20].copy_from_slice(&62u16.to_le_bytes());
        image[20..24].copy_from_slice(&EV_CURRENT.to_le_bytes());
        image[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
        image[32..40].copy_from_slice(&64u64.to_le_bytes());
        image[52..54].copy_from_slice(&(core::mem::size_of::<Elf64Header>() as u16).to_le_bytes());
        image[54..56].copy_from_slice(&(core::mem::size_of::<Elf64ProgramHeader>() as u16).to_le_bytes());
        image[56..58].copy_from_slice(&1u16.to_le_bytes());
        image[64..68].copy_from_slice(&PT_LOAD.to_le_bytes());
        image[68..72].copy_from_slice(&5u32.to_le_bytes());
        image[80..88].copy_from_slice(&0x400000u64.to_le_bytes());
        image[88..96].copy_from_slice(&0x400000u64.to_le_bytes());
        image[104..112].copy_from_slice(&0x1000u64.to_le_bytes());
        image
    }

    #[test]
    fn validates_executable_elf64_and_rejects_bad_segment_bounds() {
        let mut image = valid_elf64();
        assert_eq!(elf_validate(&image, ELFCLASS64), Some(0x400000));
        image[72..80].copy_from_slice(&127u64.to_le_bytes());
        image[96..104].copy_from_slice(&8u64.to_le_bytes());
        assert_eq!(elf_validate(&image, ELFCLASS64), None);
        assert_eq!(elf_validate(&image, ELFCLASS32), None);
    }
}
