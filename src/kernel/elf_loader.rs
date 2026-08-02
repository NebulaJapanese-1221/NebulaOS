// ELF Binary Loader for NebulaOS
// Supports ELF32 and ELF64 executables

use core::mem;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use crate::serial_println;

/// ELF magic number
pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// ELF class (32-bit vs 64-bit)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElfClass {
    Elf32,
    Elf64,
}

/// ELF data encoding (endianness)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElfData {
    LittleEndian,
    BigEndian,
}

/// ELF file type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElfType {
    None,
    Relocatable,
    Executable,
    Shared,
    Core,
    Unknown(u16),
}

/// ELF machine architecture
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElfMachine {
    X86,
    X86_64,
    Unknown(u16),
}

/// ELF segment types (program header)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegmentType {
    Null,
    Load,
    Dynamic,
    Interp,
    Note,
    Tls,
    GnuStack,
    GnuRelro,
    Unknown(u32),
}

/// Loaded ELF program
#[derive(Debug)]
pub struct LoadedProgram {
    pub entry_point: usize,
    pub segments: Vec<LoadedSegment>,
    pub vaddr_start: usize,
    pub vaddr_end: usize,
    pub class: ElfClass,
}

/// A loaded ELF segment in memory
#[derive(Debug)]
pub struct LoadedSegment {
    pub vaddr: usize,
    pub mem_size: usize,
    pub file_size: usize,
    pub flags: SegmentFlags,
    pub data: Vec<u8>,
}

/// Segment flags
#[derive(Debug, Clone, Copy)]
pub struct SegmentFlags {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

impl SegmentFlags {
    fn from_elf(p_flags: u32) -> Self {
        SegmentFlags {
            readable: (p_flags & 0x4) != 0,
            writable: (p_flags & 0x2) != 0,
            executable: (p_flags & 0x1) != 0,
        }
    }
}

/// ELF file header (32-bit)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Elf32Header {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u32,
    e_phoff: u32,
    e_shoff: u32,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

/// ELF program header (32-bit)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Elf32ProgramHeader {
    p_type: u32,
    p_offset: u32,
    p_vaddr: u32,
    p_paddr: u32,
    p_filesz: u32,
    p_memsz: u32,
    p_flags: u32,
    p_align: u32,
}

/// ELF file header (64-bit)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Elf64Header {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

/// ELF program header (64-bit)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Elf64ProgramHeader {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

/// Load an ELF binary from a byte buffer
pub fn load_elf(data: &[u8]) -> Result<LoadedProgram, &'static str> {
    if data.len() < 16 {
        return Err("Data too small for ELF header");
    }

    // Check magic
    if data[0..4] != ELF_MAGIC {
        return Err("Invalid ELF magic");
    }

    // Determine class (32-bit or 64-bit)
    match data[4] {
        1 => load_elf32(data),
        2 => load_elf64(data),
        _ => Err("Invalid ELF class"),
    }
}

/// Load a 32-bit ELF
fn load_elf32(data: &[u8]) -> Result<LoadedProgram, &'static str> {
    if data.len() < mem::size_of::<Elf32Header>() {
        return Err("Data too small for ELF32 header");
    }

    let header: &Elf32Header = unsafe { &*(data.as_ptr() as *const Elf32Header) };
    
    let phoff = header.e_phoff as usize;
    let phentsize = header.e_phentsize as usize;
    let phnum = header.e_phnum as usize;
    let entry = header.e_entry as usize;

    if phoff == 0 || phentsize == 0 || phnum == 0 {
        return Err("No program headers");
    }

    if phoff + (phnum * phentsize) > data.len() {
        return Err("Program headers out of bounds");
    }

    let mut segments = Vec::new();
    let mut vaddr_start = usize::MAX;
    let mut vaddr_end = 0;

    for i in 0..phnum {
        let ph_addr = phoff + (i * phentsize);
        let ph: &Elf32ProgramHeader = unsafe { &*(data.as_ptr().add(ph_addr) as *const Elf32ProgramHeader) };

        if ph.p_type != 1 {
            continue; // Only load PT_LOAD segments
        }

        let offset = ph.p_offset as usize;
        let vaddr = ph.p_vaddr as usize;
        let filesz = ph.p_filesz as usize;
        let memsz = ph.p_memsz as usize;

        if offset + filesz > data.len() {
            return Err("Segment data out of bounds");
        }

        // Copy segment data
        let seg_data = if filesz > 0 {
            data[offset..offset + filesz].to_vec()
        } else {
            Vec::new()
        };

        // Note: .bss section is represented by memsz > filesz; zero-fill happens at load time
        if memsz > filesz {
            let mut full_data = seg_data;
            full_data.resize(memsz, 0);
            segments.push(LoadedSegment {
                vaddr,
                mem_size: memsz,
                file_size: filesz,
                flags: SegmentFlags::from_elf(ph.p_flags),
                data: full_data,
            });
        } else {
            segments.push(LoadedSegment {
                vaddr,
                mem_size: memsz,
                file_size: filesz,
                flags: SegmentFlags::from_elf(ph.p_flags),
                data: seg_data,
            });
        }

        if vaddr < vaddr_start {
            vaddr_start = vaddr;
        }
        if vaddr + memsz > vaddr_end {
            vaddr_end = vaddr + memsz;
        }
    }

    if segments.is_empty() {
        return Err("No loadable segments");
    }

    Ok(LoadedProgram {
        entry_point: entry,
        segments,
        vaddr_start,
        vaddr_end,
        class: ElfClass::Elf32,
    })
}

/// Load a 64-bit ELF
fn load_elf64(data: &[u8]) -> Result<LoadedProgram, &'static str> {
    if data.len() < mem::size_of::<Elf64Header>() {
        return Err("Data too small for ELF64 header");
    }

    let header: &Elf64Header = unsafe { &*(data.as_ptr() as *const Elf64Header) };
    
    let phoff = header.e_phoff as usize;
    let phentsize = header.e_phentsize as usize;
    let phnum = header.e_phnum as usize;
    let entry = header.e_entry as usize;

    if phoff == 0 || phentsize == 0 || phnum == 0 {
        return Err("No program headers");
    }

    if phoff + (phnum * phentsize) > data.len() {
        return Err("Program headers out of bounds");
    }

    let mut segments = Vec::new();
    let mut vaddr_start = usize::MAX;
    let mut vaddr_end = 0;

    for i in 0..phnum {
        let ph_addr = phoff + (i * phentsize);
        let ph: &Elf64ProgramHeader = unsafe { &*(data.as_ptr().add(ph_addr) as *const Elf64ProgramHeader) };

        if ph.p_type != 1 {
            continue; // Only load PT_LOAD segments
        }

        let offset = ph.p_offset as usize;
        let vaddr = ph.p_vaddr as usize;
        let filesz = ph.p_filesz as usize;
        let memsz = ph.p_memsz as usize;

        if offset + filesz > data.len() {
            return Err("Segment data out of bounds");
        }

        // Copy segment data
        let seg_data = if filesz > 0 {
            data[offset..offset + filesz].to_vec()
        } else {
            Vec::new()
        };

        if memsz > filesz {
            let mut full_data = seg_data;
            full_data.resize(memsz, 0);
            segments.push(LoadedSegment {
                vaddr,
                mem_size: memsz,
                file_size: filesz,
                flags: SegmentFlags::from_elf(ph.p_flags),
                data: full_data,
            });
        } else {
            segments.push(LoadedSegment {
                vaddr,
                mem_size: memsz,
                file_size: filesz,
                flags: SegmentFlags::from_elf(ph.p_flags),
                data: seg_data,
            });
        }

        if vaddr < vaddr_start {
            vaddr_start = vaddr;
        }
        if vaddr + memsz > vaddr_end {
            vaddr_end = vaddr + memsz;
        }
    }

    if segments.is_empty() {
        return Err("No loadable segments");
    }

    Ok(LoadedProgram {
        entry_point: entry,
        segments,
        vaddr_start,
        vaddr_end,
        class: ElfClass::Elf64,
    })
}

/// Allocate memory for loaded ELF segments and copy them
/// Returns the entry point
pub fn load_elf_to_memory(program: &LoadedProgram) -> Result<usize, &'static str> {
    let total_size = program.vaddr_end - program.vaddr_start;
    if total_size == 0 {
        return Err("Empty program");
    }

    serial_println!("ELF Loader: Loading program (entry=0x{:x}, vaddr=0x{:x}-0x{:x}, size={})",
        program.entry_point, program.vaddr_start, program.vaddr_end, total_size);

    // In a full implementation, we would:
    // 1. Allocate pages for each segment
    // 2. Map them at the correct virtual addresses
    // 3. Copy segment data
    // 4. Set page permissions (R/W/X)
    //
    // For now, we simulate the allocation and return the entry point.

    for segment in &program.segments {
        serial_println!("  Segment: vaddr=0x{:x}, size={}, flags={}{}{}",
            segment.vaddr,
            segment.mem_size,
            if segment.flags.readable { 'R' } else { '-' },
            if segment.flags.writable { 'W' } else { '-' },
            if segment.flags.executable { 'X' } else { '-' },
        );

        // In a real implementation, we'd map pages here
        // map_pages(segment.vaddr, segment.mem_size, segment.flags);
        // copy segment.data to segment.vaddr
    }

    serial_println!("ELF Loader: Load complete, entry point at 0x{:x}", program.entry_point);
    Ok(program.entry_point)
}

/// Parse an ELF binary and get its entry point
pub fn get_elf_entry(data: &[u8]) -> Result<usize, &'static str> {
    if data.len() < 16 || data[0..4] != ELF_MAGIC {
        return Err("Invalid ELF");
    }

    match data[4] {
        1 => {
            if data.len() < mem::size_of::<Elf32Header>() {
                return Err("Data too small");
            }
            let header: &Elf32Header = unsafe { &*(data.as_ptr() as *const Elf32Header) };
            Ok(header.e_entry as usize)
        }
        2 => {
            if data.len() < mem::size_of::<Elf64Header>() {
                return Err("Data too small");
            }
            let header: &Elf64Header = unsafe { &*(data.as_ptr() as *const Elf64Header) };
            Ok(header.e_entry as usize)
        }
        _ => Err("Invalid ELF class"),
    }
}

/// Verify that a buffer contains a valid ELF binary
pub fn verify_elf(data: &[u8]) -> Result<(ElfClass, ElfType, ElfMachine), &'static str> {
    if data.len() < 16 || data[0..4] != ELF_MAGIC {
        return Err("Invalid ELF magic");
    }

    let class = match data[4] {
        1 => ElfClass::Elf32,
        2 => ElfClass::Elf64,
        _ => return Err("Invalid class"),
    };

    let data_encoding = match data[5] {
        1 => ElfData::LittleEndian,
        2 => ElfData::BigEndian,
        _ => return Err("Invalid data encoding"),
    };

    if data_encoding != ElfData::LittleEndian {
        return Err("Big-endian ELF not supported");
    }

    match class {
        ElfClass::Elf32 => {
            let header: &Elf32Header = unsafe { &*(data.as_ptr() as *const Elf32Header) };
            let e_type = match header.e_type {
                0 => ElfType::None,
                1 => ElfType::Relocatable,
                2 => ElfType::Executable,
                3 => ElfType::Shared,
                4 => ElfType::Core,
                t => ElfType::Unknown(t),
            };
            let e_machine = match header.e_machine {
                3 => ElfMachine::X86,
                m => ElfMachine::Unknown(m),
            };
            Ok((class, e_type, e_machine))
        }
        ElfClass::Elf64 => {
            let header: &Elf64Header = unsafe { &*(data.as_ptr() as *const Elf64Header) };
            let e_type = match header.e_type {
                0 => ElfType::None,
                1 => ElfType::Relocatable,
                2 => ElfType::Executable,
                3 => ElfType::Shared,
                4 => ElfType::Core,
                t => ElfType::Unknown(t),
            };
            let e_machine = match header.e_machine {
                0x3E => ElfMachine::X86_64,
                m => ElfMachine::Unknown(m),
            };
            Ok((class, e_type, e_machine))
        }
    }
}

