use core::mem::size_of;
use crate::kernel::paging::{VirtualAddressSpace, FLAG_PRESENT, FLAG_COW, FLAG_USER};

pub const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
pub const PT_LOAD: u32 = 1;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ElfHeader {
    pub ident: [u8; 16],
    pub type_: u16,
    pub machine: u16,
    pub version: u32,
    pub entry: u32,
    pub phoff: u32, 
    pub shoff: u32,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ProgramHeader {
    pub type_: u32,
    pub offset: u32,
    pub vaddr: u32,
    pub paddr: u32,
    pub filesz: u32,
    pub memsz: u32,
    pub flags: u32,
    pub align: u32,
}

/// Verifies if the data is a valid ELF binary (NebulaOS .app format).
pub fn check_header(data: &[u8]) -> bool {
    if data.len() < size_of::<ElfHeader>() {
        return false;
    }
    let header = unsafe { &*(data.as_ptr() as *const ElfHeader) };
    // Check Magic and Class (1 = 32-bit)
    header.ident[0..4] == ELF_MAGIC && header.ident[4] == 1
}

/// Loads the ELF binary into memory and adds it as a task.
/// Returns true if successful.
pub fn load_and_run(data: &[u8]) -> bool {
    if !check_header(data) {
        return false;
    }
    let header = unsafe { &*(data.as_ptr() as *const ElfHeader) };

    // 1. Calculate the memory size needed for the segments
    let ph_offset = header.phoff as usize;
    let ph_count = header.phnum as usize;
    let ph_size = header.phentsize as usize;

    let address_space = match VirtualAddressSpace::new_user() {
        Some(as_space) => as_space,
        None => return false,
    };
    
    // 2. Map Segments using CoW
    for i in 0..ph_count {
        let offset = ph_offset + i * ph_size;
        let ph = unsafe { &*(data.as_ptr().add(offset) as *const ProgramHeader) };
        
        if ph.type_ == PT_LOAD {
            // Physical address is just the offset into the provided data slice
            let phys_addr = unsafe { data.as_ptr().add(ph.offset as usize) as usize };
            
            // We map the segment as Read-Only + CoW + User.
            // The Copy-on-Write mechanism in the page fault handler will handle writes.
            address_space.map_region(
                ph.vaddr as usize,
                phys_addr,
                ph.memsz as usize,
                FLAG_PRESENT | FLAG_COW | FLAG_USER
            );
        }
    }

    // 3. Spawn Task
    // We map the entry point directly. Note: We no longer need to "leak" memory
    // here because the address space handles the mapping to the 'data' buffer.
    // In a production kernel, 'data' should be backed by a persistent Page Cache.
    crate::kernel::process::SCHEDULER.lock().add_task(header.entry as usize, 10, Some(address_space));
    true
}