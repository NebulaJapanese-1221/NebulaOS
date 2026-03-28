use core::mem::size_of;

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

/// Returns the entry point address if valid.
pub fn get_entry_point(data: &[u8]) -> Option<u32> {
    if !check_header(data) {
        return None;
    }
    let header = unsafe { &*(data.as_ptr() as *const ElfHeader) };
    Some(header.entry)
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
    let mut max_vaddr = 0;

    for i in 0..ph_count {
        let offset = ph_offset + i * ph_size;
        if offset + size_of::<ProgramHeader>() > data.len() { continue; }
        let ph = unsafe { &*(data.as_ptr().add(offset) as *const ProgramHeader) };
        if ph.type_ == PT_LOAD {
            let end = ph.vaddr + ph.memsz;
            if end > max_vaddr { max_vaddr = end; }
        }
    }

    if max_vaddr == 0 { return false; }

    // 2. Allocate memory (Simulating a process address space by leaking a Vec)
    // We allocate a bit extra for safety/alignment
    let mut memory = alloc::vec![0u8; max_vaddr as usize + 4096];
    let base_addr = memory.as_ptr() as usize;

    // 3. Load Segments
    for i in 0..ph_count {
        let offset = ph_offset + i * ph_size;
        let ph = unsafe { &*(data.as_ptr().add(offset) as *const ProgramHeader) };
        if ph.type_ == PT_LOAD {
            let segment_end = ph.vaddr as usize + ph.memsz as usize;
            if segment_end <= memory.len() && (ph.offset as usize + ph.filesz as usize) <= data.len() {
                unsafe {
                    let dest_ptr = memory.as_mut_ptr().add(ph.vaddr as usize);
                    let src_ptr = data.as_ptr().add(ph.offset as usize);
                    
                    core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, ph.filesz as usize);
                    
                    if ph.memsz > ph.filesz {
                        core::ptr::write_bytes(dest_ptr.add(ph.filesz as usize), 0, (ph.memsz - ph.filesz) as usize);
                    }
                }
            }
        }
    }

    // 4. Calculate Entry Point and Spawn
    // Assuming the ELF is linked to 0 or is Position Independent, we add base_addr.
    // If it's statically linked to a specific high address, this loader would fail without paging,
    // but for "apps created externally" in this context, we assume they are compatible.
    let entry = base_addr + header.entry as usize;

    // Prevent the memory from being freed when `memory` goes out of scope
    core::mem::forget(memory);

    crate::kernel::process::SCHEDULER.lock().add_task(entry, 10);
    true
}