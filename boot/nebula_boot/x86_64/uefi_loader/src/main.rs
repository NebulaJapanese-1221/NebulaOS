#![no_std]
#![no_main]

use core::arch::asm;
use core::ffi::c_void;
use core::ptr;

type Handle = *mut c_void;
type Status = usize;

const EFI_SUCCESS: Status = 0;
const EFI_LOADER_DATA: u32 = 2;
const EFI_ALLOCATE_ADDRESS: u32 = 2;
const EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL: u32 = 1;
const EFI_CONVENTIONAL_MEMORY: u32 = 7;
const PAGE_SIZE: u64 = 4096;
const MAX_MEMORY_REGIONS: usize = 1024;
const RAW_MAP_SIZE: usize = 65536;
const FILE_INFO_SIZE: usize = 4096;

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryRegion {
    base: u64,
    length: u64,
    kind: u32,
    reserved: u32,
}

#[repr(C)]
struct BootInfo {
    framebuffer: u64,
    width: u32,
    height: u32,
    stride: u32,
    bits_per_pixel: u32,
    red_size: u32,
    red_position: u32,
    green_size: u32,
    green_position: u32,
    blue_size: u32,
    blue_position: u32,
    memory_map: *const MemoryRegion,
    memory_region_count: u32,
    reserved: u32,
}

#[repr(C)]
struct GraphicsMode {
    max_mode: u32,
    mode: u32,
    info: *mut GraphicsModeInfo,
    info_size: usize,
    framebuffer: u64,
    framebuffer_size: usize,
}

#[repr(C)]
struct GraphicsModeInfo {
    version: u32,
    width: u32,
    height: u32,
    pixel_format: u32,
    pixel_information: [u32; 4],
    pixels_per_scanline: u32,
}

static LOADED_IMAGE_GUID: Guid = Guid {
    a: 0x5B1B31A1,
    b: 0x9562,
    c: 0x11D2,
    d: [0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
};
static SIMPLE_FILE_SYSTEM_GUID: Guid = Guid {
    a: 0x964E5B22,
    b: 0x6459,
    c: 0x11D2,
    d: [0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
};
static FILE_INFO_GUID: Guid = Guid {
    a: 0x09576E92,
    b: 0x6D3F,
    c: 0x11D2,
    d: [0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
};
static GRAPHICS_OUTPUT_GUID: Guid = Guid {
    a: 0x9042A9DE,
    b: 0x23DC,
    c: 0x4A38,
    d: [0x96, 0xFB, 0x7A, 0xDE, 0xD0, 0x80, 0x51, 0x6A],
};

static mut BOOT_INFO: BootInfo = BootInfo {
    framebuffer: 0,
    width: 0,
    height: 0,
    stride: 0,
    bits_per_pixel: 0,
    red_size: 0,
    red_position: 0,
    green_size: 0,
    green_position: 0,
    blue_size: 0,
    blue_position: 0,
    memory_map: ptr::null(),
    memory_region_count: 0,
    reserved: 0,
};
static mut MEMORY_REGIONS: [MemoryRegion; MAX_MEMORY_REGIONS] = [MemoryRegion {
    base: 0,
    length: 0,
    kind: 0,
    reserved: 0,
}; MAX_MEMORY_REGIONS];
static mut RAW_MEMORY_MAP: [u8; RAW_MAP_SIZE] = [0; RAW_MAP_SIZE];
static mut FILE_INFO: [u8; FILE_INFO_SIZE] = [0; FILE_INFO_SIZE];

#[no_mangle]
pub extern "efiapi" fn efi_main(image_handle: Handle, system_table: *mut u8) -> Status {
    unsafe {
        let con_out = read_ptr(system_table, 64);
        print(con_out, b"NebulaBoot - NebulaOS UEFI loader\r\n\0");
        print(con_out, b"Loading kernel and preparing graphics...\r\n\0");

        let boot_services = read_ptr(system_table, 96);
        let file = match open_kernel(boot_services, image_handle) {
            Some(file) => file,
            None => return error_status(),
        };
        let (kernel_image, kernel_size) = match read_kernel(boot_services, file) {
            Some(image) => image,
            None => return error_status(),
        };
        let entry = match load_elf(boot_services, kernel_image, kernel_size) {
            Some(entry) => entry,
            None => return error_status(),
        };
        if !init_graphics(boot_services) {
            return error_status();
        }
        let map_key = match collect_memory_map(boot_services) {
            Some(key) => key,
            None => return error_status(),
        };
        let exit_boot_services: unsafe extern "efiapi" fn(Handle, usize) -> Status =
            boot_fn(boot_services, 232);
        if exit_boot_services(image_handle, map_key) != EFI_SUCCESS {
            return error_status();
        }

        let boot_info = ptr::addr_of!(BOOT_INFO);
        asm!(
            "jmp rax",
            in("rax") entry,
            in("rdi") boot_info,
            options(noreturn)
        );
    }
}

unsafe fn open_kernel(boot_services: *mut u8, image_handle: Handle) -> Option<*mut u8> {
    type OpenProtocol = unsafe extern "efiapi" fn(
        Handle,
        *const Guid,
        *mut *mut c_void,
        Handle,
        Handle,
        u32,
    ) -> Status;
    let open_protocol: OpenProtocol = boot_fn(boot_services, 280);
    let mut loaded_image = ptr::null_mut();
    if open_protocol(
        image_handle,
        &LOADED_IMAGE_GUID,
        &mut loaded_image,
        image_handle,
        ptr::null_mut(),
        EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL,
    ) != EFI_SUCCESS
    {
        return None;
    }
    let device_handle = read_ptr(loaded_image as *mut u8, 24);
    let mut filesystem = ptr::null_mut();
    if open_protocol(
        device_handle,
        &SIMPLE_FILE_SYSTEM_GUID,
        &mut filesystem,
        image_handle,
        ptr::null_mut(),
        EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL,
    ) != EFI_SUCCESS
    {
        return None;
    }

    type OpenVolume = unsafe extern "efiapi" fn(*mut c_void, *mut *mut c_void) -> Status;
    let open_volume: OpenVolume = read_fn(filesystem as *mut u8, 8);
    let mut root = ptr::null_mut();
    if open_volume(filesystem, &mut root) != EFI_SUCCESS {
        return None;
    }
    type FileOpen = unsafe extern "efiapi" fn(
        *mut c_void,
        *mut *mut c_void,
        *const u16,
        u64,
        u64,
    ) -> Status;
    let file_open: FileOpen = read_fn(root as *mut u8, 8);
    let path = [
        b'\\' as u16, b'E' as u16, b'F' as u16, b'I' as u16, b'\\' as u16,
        b'N' as u16, b'E' as u16, b'B' as u16, b'U' as u16, b'L' as u16, b'A' as u16,
        b'\\' as u16, b'n' as u16, b'e' as u16, b'b' as u16, b'u' as u16,
        b'l' as u16, b'a' as u16, b'o' as u16, b's' as u16, b'_' as u16,
        b'x' as u16, b'8' as u16, b'6' as u16, b'_' as u16, b'6' as u16,
        b'4' as u16, b'.' as u16, b'e' as u16, b'l' as u16, b'f' as u16, 0,
    ];
    let mut file = ptr::null_mut();
    if file_open(root, &mut file, path.as_ptr(), 1, 0) != EFI_SUCCESS {
        return None;
    }
    Some(file as *mut u8)
}

unsafe fn read_kernel(boot_services: *mut u8, file: *mut u8) -> Option<(*mut u8, usize)> {
    type GetInfo = unsafe extern "efiapi" fn(
        *mut c_void,
        *const Guid,
        *mut usize,
        *mut c_void,
    ) -> Status;
    let get_info: GetInfo = read_fn(file, 64);
    let mut info_size = FILE_INFO_SIZE;
    if get_info(
        file as *mut c_void,
        &FILE_INFO_GUID,
        &mut info_size,
        ptr::addr_of_mut!(FILE_INFO) as *mut c_void,
    ) != EFI_SUCCESS
    {
        return None;
    }
    let file_size = ptr::read_unaligned(ptr::addr_of!(FILE_INFO).cast::<u64>().add(1)) as usize;
    if file_size < 64 || file_size > 64 * 1024 * 1024 {
        return None;
    }
    type AllocatePool = unsafe extern "efiapi" fn(u32, usize, *mut *mut c_void) -> Status;
    let allocate_pool: AllocatePool = boot_fn(boot_services, 64);
    let mut buffer = ptr::null_mut();
    if allocate_pool(EFI_LOADER_DATA, file_size, &mut buffer) != EFI_SUCCESS {
        return None;
    }
    type FileRead = unsafe extern "efiapi" fn(*mut c_void, *mut usize, *mut c_void) -> Status;
    let file_read: FileRead = read_fn(file, 32);
    let mut bytes_read = file_size;
    if file_read(file as *mut c_void, &mut bytes_read, buffer) != EFI_SUCCESS || bytes_read != file_size {
        return None;
    }
    Some((buffer as *mut u8, file_size))
}

unsafe fn load_elf(boot_services: *mut u8, image: *const u8, image_size: usize) -> Option<usize> {
    if image_size < 64
        || ptr::read_unaligned(image.cast::<u32>()) != 0x464C457F
        || *image.add(4) != 2
        || *image.add(5) != 1
        || read_u16(image, 16)? != 2
        || read_u16(image, 18)? != 62
    {
        return None;
    }
    let entry = read_u64(image, 24)? as usize;
    let table_offset = usize::try_from(read_u64(image, 32)?).ok()?;
    let table_stride = read_u16(image, 54)? as usize;
    let table_count = read_u16(image, 56)? as usize;
    if table_stride < 56
        || table_offset.checked_add(table_stride.checked_mul(table_count)?)? > image_size
    {
        return None;
    }

    type AllocatePages = unsafe extern "efiapi" fn(u32, u32, usize, *mut u64) -> Status;
    let allocate_pages: AllocatePages = boot_fn(boot_services, 40);
    for index in 0..table_count {
        let ph = image.add(table_offset + index * table_stride);
        if ptr::read_unaligned(ph.cast::<u32>()) != 1 {
            continue;
        }
        let offset = read_u64(ph, 8)?;
        let physical = read_u64(ph, 24)?;
        let file_size = read_u64(ph, 32)?;
        let memory_size = read_u64(ph, 40)?;
        if file_size > memory_size || offset.checked_add(file_size)? > image_size as u64 {
            return None;
        }
        if memory_size == 0 {
            continue;
        }
        let pages = usize::try_from(memory_size.checked_add(PAGE_SIZE - 1)? / PAGE_SIZE).ok()?;
        let mut address = if physical == 0 { read_u64(ph, 16)? } else { physical };
        if allocate_pages(EFI_ALLOCATE_ADDRESS, EFI_LOADER_DATA, pages, &mut address) != EFI_SUCCESS {
            return None;
        }
        ptr::copy_nonoverlapping(image.add(offset as usize), address as *mut u8, file_size as usize);
        ptr::write_bytes(
            (address as *mut u8).add(file_size as usize),
            0,
            (memory_size - file_size) as usize,
        );
    }
    Some(entry)
}

unsafe fn init_graphics(boot_services: *mut u8) -> bool {
    type LocateProtocol = unsafe extern "efiapi" fn(
        *const Guid,
        *mut c_void,
        *mut *mut c_void,
    ) -> Status;
    let locate: LocateProtocol = boot_fn(boot_services, 320);
    let mut protocol = ptr::null_mut();
    if locate(&GRAPHICS_OUTPUT_GUID, ptr::null_mut(), &mut protocol) != EFI_SUCCESS {
        return false;
    }
    let mode = read_ptr(protocol as *mut u8, 24) as *mut u8;
    if mode.is_null() {
        return false;
    }
    let mode_number = ptr::read_unaligned(mode.add(4).cast::<u32>());
    type SetMode = unsafe extern "efiapi" fn(*mut c_void, u32) -> Status;
    let set_mode: SetMode = read_fn(protocol as *mut u8, 8);
    if set_mode(protocol, mode_number) != EFI_SUCCESS {
        return false;
    }
    let mode = read_ptr(protocol as *mut u8, 24) as *mut u8;
    let info = read_ptr(mode, 8) as *mut u8;
    let framebuffer = ptr::read_unaligned(mode.add(24).cast::<u64>());
    let width = ptr::read_unaligned(info.add(4).cast::<u32>());
    let height = ptr::read_unaligned(info.add(8).cast::<u32>());
    let pixel_format = ptr::read_unaligned(info.add(12).cast::<u32>());
    let stride = ptr::read_unaligned(info.add(32).cast::<u32>()).checked_mul(4);
    if width == 0 || height == 0 || framebuffer == 0 {
        return false;
    }
    let Some(stride) = stride else { return false };
    let masks = match pixel_format {
        0 => [8, 0, 8, 8, 8, 16],
        1 => [8, 16, 8, 8, 8, 0],
        _ => return false,
    };
    let boot = ptr::addr_of_mut!(BOOT_INFO);
    (*boot).framebuffer = framebuffer;
    (*boot).width = width;
    (*boot).height = height;
    (*boot).stride = stride;
    (*boot).bits_per_pixel = 32;
    (*boot).red_size = masks[0];
    (*boot).red_position = masks[1];
    (*boot).green_size = masks[2];
    (*boot).green_position = masks[3];
    (*boot).blue_size = masks[4];
    (*boot).blue_position = masks[5];
    true
}

unsafe fn collect_memory_map(boot_services: *mut u8) -> Option<usize> {
    type GetMemoryMap = unsafe extern "efiapi" fn(
        *mut usize,
        *mut c_void,
        *mut usize,
        *mut usize,
        *mut u32,
    ) -> Status;
    let get_map: GetMemoryMap = boot_fn(boot_services, 56);
    let mut size = RAW_MAP_SIZE;
    let mut key = 0usize;
    let mut descriptor_size = 0usize;
    let mut version = 0u32;
    if get_map(
        &mut size,
        ptr::addr_of_mut!(RAW_MEMORY_MAP) as *mut c_void,
        &mut key,
        &mut descriptor_size,
        &mut version,
    ) != EFI_SUCCESS
        || descriptor_size < 40
        || size % descriptor_size != 0
    {
        return None;
    }
    let descriptor_count = size / descriptor_size;
    let mut output_count = 0usize;
    for index in 0..descriptor_count {
        let descriptor = (ptr::addr_of!(RAW_MEMORY_MAP) as *const u8).add(index * descriptor_size);
        if ptr::read_unaligned(descriptor.cast::<u32>()) != EFI_CONVENTIONAL_MEMORY {
            continue;
        }
        if output_count == MAX_MEMORY_REGIONS {
            return None;
        }
        let region = ptr::addr_of_mut!(MEMORY_REGIONS).cast::<MemoryRegion>().add(output_count);
        (*region).base = ptr::read_unaligned(descriptor.add(8).cast::<u64>());
        (*region).length = ptr::read_unaligned(descriptor.add(24).cast::<u64>()).checked_mul(PAGE_SIZE)?;
        (*region).kind = 1;
        (*region).reserved = 0;
        output_count += 1;
    }
    let boot = ptr::addr_of_mut!(BOOT_INFO);
    (*boot).memory_map = ptr::addr_of!(MEMORY_REGIONS).cast::<MemoryRegion>();
    (*boot).memory_region_count = output_count as u32;
    Some(key)
}

unsafe fn print(console: *mut u8, text: &'static [u8]) {
    if console.is_null() {
        return;
    }
    let mut wide = [0u16; 96];
    let mut count = 0usize;
    while count < wide.len() - 1 && text[count] != 0 {
        wide[count] = text[count] as u16;
        count += 1;
    }
    type OutputString = unsafe extern "efiapi" fn(*mut c_void, *const u16) -> Status;
    let output: OutputString = read_fn(console, 8);
    let _ = output(console as *mut c_void, wide.as_ptr());
}

unsafe fn read_ptr(base: *mut u8, offset: usize) -> *mut u8 {
    ptr::read_unaligned(base.add(offset).cast::<*mut u8>())
}

unsafe fn boot_fn<T: Copy>(boot_services: *mut u8, offset: usize) -> T {
    ptr::read_unaligned(boot_services.add(offset).cast::<T>())
}

unsafe fn read_fn<T: Copy>(base: *mut u8, offset: usize) -> T {
    ptr::read_unaligned(base.add(offset).cast::<T>())
}

unsafe fn read_u16(base: *const u8, offset: usize) -> Option<u16> {
    Some(u16::from_le(ptr::read_unaligned(base.add(offset).cast::<u16>())))
}

unsafe fn read_u64(base: *const u8, offset: usize) -> Option<u64> {
    Some(u64::from_le(ptr::read_unaligned(base.add(offset).cast::<u64>())))
}

fn error_status() -> Status {
    (1usize << (usize::BITS - 1)) | 1
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt", options(nomem, nostack)) }
    }
}