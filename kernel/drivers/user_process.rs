use crate::drivers::{disk, paging, process};
use crate::memory::frame_allocator;

const USER_IMAGE_BASE: u32 = 0x4000_0000;
const USER_IMAGE_LIMIT: u32 = 0x4001_0000;
const USER_STACK_TOP: u32 = 0xC000_0000;
const PAGE_SIZE: u32 = 4096;
const PT_LOAD: u32 = 1;
const PF_WRITE: u32 = 2;
const PF_EXECUTE: u32 = 1;
const ELF_HEADER_SIZE: usize = 52;
const PROGRAM_HEADER_SIZE: usize = 32;
const MAX_PROGRAM_HEADERS: usize = 16;
const KERNEL_PHYSICAL_LIMIT: usize = 0x0080_0000;
const LAST_OUTPUT_CAPACITY: usize = 128;
const DEMO_FILE_NAME: &[u8] = b"DEMO.ELF";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UserProcessError {
    OutOfMemory,
    InvalidElf,
    NoLoadableSegments,
    InvalidEntryPoint,
    LoadFailed,
    Storage(disk::StorageError),
}

static mut LAST_OUTPUT: [u8; LAST_OUTPUT_CAPACITY] = [0; LAST_OUTPUT_CAPACITY];
static mut LAST_OUTPUT_LENGTH: usize = 0;

unsafe extern "C" {
    #[link_name = "_binary_demo_elf_start"]
    static USER_ELF_START: u8;
    #[link_name = "_binary_demo_elf_end"]
    static USER_ELF_END: u8;
}

pub fn spawn_demo() -> Result<u32, UserProcessError> {
    let mut image = [0u8; 512];
    match disk::read_file(DEMO_FILE_NAME, &mut image) {
        Ok(length) => spawn_image(&image[..length]),
        Err(disk::StorageError::NotFound) => {
            let image_start = core::ptr::addr_of!(USER_ELF_START);
            let image_end = core::ptr::addr_of!(USER_ELF_END);
            let embedded_len = image_end as usize - image_start as usize;
            if embedded_len > image.len() {
                return Err(UserProcessError::LoadFailed);
            }
            let embedded = unsafe { core::slice::from_raw_parts(image_start, embedded_len) };
            disk::write_file(DEMO_FILE_NAME, embedded).map_err(UserProcessError::Storage)?;
            spawn_file(DEMO_FILE_NAME)
        }
        Err(error) => return Err(UserProcessError::Storage(error)),
    }
}

pub fn spawn_file(name: &[u8]) -> Result<u32, UserProcessError> {
    let mut image = [0u8; 512];
    let image_len = disk::read_file(name, &mut image).map_err(UserProcessError::Storage)?;
    spawn_image(&image[..image_len])
}

fn spawn_image(image: &[u8]) -> Result<u32, UserProcessError> {
    let page_directory =
        paging::create_user_address_space().ok_or(UserProcessError::OutOfMemory)?;

    let entry = match load_elf(image, page_directory) {
        Ok(entry) => entry,
        Err(error) => {
            paging::destroy_user_address_space(page_directory);
            return Err(error);
        }
    };

    if paging::map_user_page(page_directory, USER_STACK_TOP - PAGE_SIZE, true).is_none() {
        paging::destroy_user_address_space(page_directory);
        return Err(UserProcessError::OutOfMemory);
    }

    let Some(kernel_stack) = frame_allocator::alloc_frame_below(KERNEL_PHYSICAL_LIMIT) else {
        paging::destroy_user_address_space(page_directory);
        return Err(UserProcessError::OutOfMemory);
    };
    unsafe {
        core::ptr::write_bytes(kernel_stack as *mut u8, 0, PAGE_SIZE as usize);
    }
    let Some(pid) = process::add_user_process(entry, USER_STACK_TOP, page_directory, kernel_stack)
    else {
        paging::destroy_user_address_space(page_directory);
        frame_allocator::free_frame(kernel_stack);
        return Err(UserProcessError::OutOfMemory);
    };
    crate::drivers::serial::write_string("Queued storage-backed ring-3 process PID ");
    crate::drivers::serial::write_dec(pid);
    crate::drivers::serial::write_string(".\r\n");
    Ok(pid)
}

pub fn record_syscall_output(bytes: &[u8]) {
    unsafe {
        let current_length = LAST_OUTPUT_LENGTH;
        let copy_length = bytes.len().min(LAST_OUTPUT_CAPACITY - current_length);
        core::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            core::ptr::addr_of_mut!(LAST_OUTPUT)
                .cast::<u8>()
                .add(current_length),
            copy_length,
        );
        LAST_OUTPUT_LENGTH += copy_length;
    }
}

pub fn copy_last_output(destination: &mut [u8]) -> usize {
    unsafe {
        let length = LAST_OUTPUT_LENGTH.min(destination.len());
        core::ptr::copy_nonoverlapping(
            core::ptr::addr_of!(LAST_OUTPUT).cast::<u8>(),
            destination.as_mut_ptr(),
            length,
        );
        length
    }
}

pub fn exit_from_syscall(_registers: *mut u32, _code: u32) -> bool {
    process::terminate_current_user_process()
}

fn load_elf(image: &[u8], page_directory: u32) -> Result<u32, UserProcessError> {
    if image.len() < ELF_HEADER_SIZE
        || &image[..4] != b"\x7fELF"
        || image[4] != 1
        || image[5] != 1
        || image[6] != 1
        || read_u16(image, 16)? != 2
        || read_u16(image, 18)? != 3
        || read_u32(image, 20)? != 1
        || read_u16(image, 40)? as usize != ELF_HEADER_SIZE
        || read_u16(image, 42)? as usize != PROGRAM_HEADER_SIZE
    {
        return Err(UserProcessError::InvalidElf);
    }

    let entry = read_u32(image, 24)?;
    let header_offset = read_u32(image, 28)? as usize;
    let header_count = read_u16(image, 44)? as usize;
    if header_count == 0 || header_count > MAX_PROGRAM_HEADERS {
        return Err(UserProcessError::InvalidElf);
    }
    let headers_len = header_count
        .checked_mul(PROGRAM_HEADER_SIZE)
        .and_then(|length| header_offset.checked_add(length))
        .filter(|end| *end <= image.len())
        .ok_or(UserProcessError::InvalidElf)?;

    let mut loadable_segments = 0;
    let mut entry_is_executable = false;
    for header_offset in (header_offset..headers_len).step_by(PROGRAM_HEADER_SIZE) {
        if read_u32(image, header_offset)? != PT_LOAD {
            continue;
        }
        let file_offset = read_u32(image, header_offset + 4)? as usize;
        let virtual_address = read_u32(image, header_offset + 8)?;
        let file_size = read_u32(image, header_offset + 16)? as usize;
        let memory_size = read_u32(image, header_offset + 20)? as usize;
        let flags = read_u32(image, header_offset + 24)?;

        if file_size > memory_size || memory_size == 0 {
            return Err(UserProcessError::InvalidElf);
        }
        let file_end = file_offset
            .checked_add(file_size)
            .filter(|end| *end <= image.len())
            .ok_or(UserProcessError::InvalidElf)?;
        let segment_end = (virtual_address as usize)
            .checked_add(memory_size)
            .filter(|end| virtual_address >= USER_IMAGE_BASE && *end <= USER_IMAGE_LIMIT as usize)
            .ok_or(UserProcessError::InvalidElf)?;
        if flags & PF_EXECUTE != 0 && entry >= virtual_address && (entry as usize) < segment_end {
            entry_is_executable = true;
        }
        load_segment(
            image,
            page_directory,
            file_offset,
            virtual_address,
            file_end,
            virtual_address
                .checked_add(file_size as u32)
                .ok_or(UserProcessError::InvalidElf)?,
            segment_end as u32,
            flags & PF_WRITE != 0,
        )?;
        loadable_segments += 1;
    }

    if loadable_segments == 0 {
        return Err(UserProcessError::NoLoadableSegments);
    }
    if !entry_is_executable {
        return Err(UserProcessError::InvalidEntryPoint);
    }
    Ok(entry)
}

fn load_segment(
    image: &[u8],
    page_directory: u32,
    file_offset: usize,
    virtual_address: u32,
    file_end: usize,
    file_memory_end: u32,
    memory_end: u32,
    writable: bool,
) -> Result<(), UserProcessError> {
    let memory_start = virtual_address as usize;
    let aligned_start = virtual_address & !(PAGE_SIZE - 1);
    let aligned_end = memory_end
        .checked_add(PAGE_SIZE - 1)
        .map(|end| end & !(PAGE_SIZE - 1))
        .ok_or(UserProcessError::InvalidElf)?;

    let mut page = aligned_start;
    while page < aligned_end {
        if paging::user_page_frame(page_directory, page).is_none()
            && paging::map_user_page(page_directory, page, writable).is_none()
        {
            return Err(UserProcessError::OutOfMemory);
        }
        if writable && !paging::set_user_page_writable(page_directory, page) {
            return Err(UserProcessError::LoadFailed);
        }
        page = page
            .checked_add(PAGE_SIZE)
            .ok_or(UserProcessError::InvalidElf)?;
    }

    let file_size = file_end - file_offset;
    let mut copied = 0;
    while copied < file_size {
        let destination = memory_start + copied;
        let page_address = (destination as u32) & !(PAGE_SIZE - 1);
        let page_offset = destination & (PAGE_SIZE as usize - 1);
        let copy_len = (PAGE_SIZE as usize - page_offset).min(file_size - copied);
        let frame = paging::user_page_frame(page_directory, page_address)
            .ok_or(UserProcessError::LoadFailed)?;
        unsafe {
            core::ptr::copy_nonoverlapping(
                image.as_ptr().add(file_offset + copied),
                (frame as usize + page_offset) as *mut u8,
                copy_len,
            );
        }
        copied += copy_len;
    }
    let mut cleared = file_memory_end;
    while cleared < memory_end {
        let page_address = cleared & !(PAGE_SIZE - 1);
        let page_offset = (cleared - page_address) as usize;
        let clear_len = (PAGE_SIZE as usize - page_offset).min((memory_end - cleared) as usize);
        let frame = paging::user_page_frame(page_directory, page_address)
            .ok_or(UserProcessError::LoadFailed)?;
        unsafe {
            core::ptr::write_bytes((frame as usize + page_offset) as *mut u8, 0, clear_len);
        }
        cleared += clear_len as u32;
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, UserProcessError> {
    let value = bytes
        .get(offset..offset.checked_add(2).ok_or(UserProcessError::InvalidElf)?)
        .ok_or(UserProcessError::InvalidElf)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, UserProcessError> {
    let value = bytes
        .get(offset..offset.checked_add(4).ok_or(UserProcessError::InvalidElf)?)
        .ok_or(UserProcessError::InvalidElf)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}
