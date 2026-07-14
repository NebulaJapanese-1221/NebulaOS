#![no_std]
#![no_main]

extern crate alloc;

core::arch::global_asm!(
    ".section .multiboot, \"a\"", 
    ".align 4",
    ".long 0x1BADB002",           
    ".long 0x00000007",           
    ".long -(0x1BADB002 + 0x00000007)", 
    ".long 0, 0, 0, 0, 0",        
    ".long 0",                    
    ".long 1024",                 
    ".long 768",                  
    ".long 32"                    
);

mod sync;
mod gdt;
mod idt;
mod interrupts;
mod allocator;
mod process; // Modified
mod scheduler; // Modified
mod syscalls;
mod panic;
mod exceptions;
mod memory; // New module for paging

#[path = "../fs/mod.rs"]
mod fs;

use allocator::ALLOCATOR; 
use core::arch::asm;
use framebuffer::FRAMEBUFFER;
use alloc::vec::Vec;
use alloc::format;
use crate::drivers::rtc;

#[path = "../drivers/vga.rs"]
mod vga;

#[path = "../drivers/ps2.rs"]
mod ps2;

#[path = "../drivers/mouse.rs"]
mod mouse;

#[path = "../drivers/keyboard.rs"]
mod keyboard;

#[path = "../drivers/framebuffer.rs"]
mod framebuffer;

#[path = "../drivers/rtc.rs"]
mod rtc;

#[path = "../drivers/serial.rs"]
mod serial;

#[path = "../drivers/pit.rs"]
mod pit;

#[path = "../userspace/apps/mod.rs"]
mod apps;

#[path = "../userspace/gui/mod.rs"]
mod gui;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct MultibootMmapEntry {
    pub size: u32,
    pub addr: u64,
    pub len: u64,
    pub type_: u32,
}

#[repr(C, packed)]
struct MultibootInfo {
    flags: u32,
    mem_lower: u32,      
    mem_upper: u32,      
    _ignore1: [u32; 8],  
    mmap_length: u32,    
    mmap_addr: u32,      
    _ignore2: [u32; 9],  
    fb_addr: u64,
    fb_pitch: u32,
    fb_width: u32,
    fb_height: u32,
    fb_bpp: u8,
    fb_type: u8,
}

core::arch::global_asm!(
    ".global _start",
    "_start:",
    "push ebx",
    "push eax",
    "call kmain",
    "1: jmp 1b"
);

#[no_mangle]
pub extern "C" fn kmain(magic: u32, mb_ptr: u32) -> ! {
    {
        serial::SERIAL_PORT.lock().init();
        serial_println!("NebulaOS v0.0.1 started...");

        if magic == 0x2BADB002 {
            let info = unsafe { &*(mb_ptr as *const MultibootInfo) };
            let flags = info.flags;

            if (flags & 0x40) != 0 { // Memory map info
                let mmap_addr = info.mmap_addr;
                let mmap_length = info.mmap_length;
                serial_println!("Memory Map found at 0x{:X}, length: {} bytes", mmap_addr, mmap_length);
                let mut current_addr = mmap_addr;
                let end_addr = mmap_addr + mmap_length;
                while current_addr < end_addr {
                    // Safely copy entry data to avoid potential unaligned access errors
                    let mut entry_data: [u8; 24] = [0; 24]; // Max size of MmapEntry is 20, but size field is variable. Use max.
                    let entry_slice = unsafe { core::slice::from_raw_parts(current_addr as *const u8, 24) };
                    entry_data.copy_from_slice(entry_slice);
                    
                    let entry = unsafe { &*(entry_data.as_ptr() as *const MultibootMmapEntry) };
                    
                    let addr = entry.addr;
                    let len = entry.len;
                    let type_ = entry.type_;
                    
                    if type_ == 1 { // Available memory
                        serial_println!("  Available: 0x{:016X} - 0x{:016X} ({} MB)", addr, addr + len, len / (1024 * 1024));
                    }
                    current_addr += entry.size + 4; // Move to the next entry
                }
            }

            if (flags & 0x800) != 0 { // Framebuffer info
                let fb_addr = info.fb_addr;
                let fb_width = info.fb_width;
                let fb_height = info.fb_height;
                let fb_pitch = info.fb_pitch;

                let mut fb = FRAMEBUFFER.lock();
                fb.init(fb_addr as *mut u32, fb_width as usize, fb_height as usize, fb_pitch as usize);
                serial_println!("Framebuffer initialized at 0x{:X} ({}x{})", fb_addr, fb_width, fb_height);
            }
        }
        
        let update_progress = |progress: usize| {
            {
                let mut fb = FRAMEBUFFER.lock();
                let bar_width = 400;
                let bar_height = 12;
                let x = (fb.width - bar_width) / 2;
                let y = fb.height - 150;
                
                fb.draw_rect(x, y, bar_width, bar_height, 0x00111111); 
                fb.draw_rect(x, y, (bar_width * progress) / 100, bar_height, 0x0000AAFF); 
                fb.present();
            }
            for _ in 0..500000 { core::hint::spin_loop(); } 
        };

        serial_println!("Initializing Framebuffer...");
        {
            let mut fb = FRAMEBUFFER.lock();
            let scale = 8;
            let string_width = 8 * 8 * scale; 
            let string_height = 8 * scale;
            let x = (fb.width / 2) - (string_width / 2);
            let y = (fb.height / 2) - (string_height / 2);
            gui::draw_large_string(&mut fb, x, y, "NebulaOS", 0x00800080, scale);
        }

        update_progress(10);

        unsafe {
            serial_println!("Initializing GDT...");
            gdt::init();
            update_progress(30);

            serial_println!("Initializing Heap...");
            let heap_start = 0x1000000; // Example: start heap at 16MB
            let heap_size = 0x100000;   // 1MB heap size
            ALLOCATOR.init(heap_start, heap_size);
            update_progress(50);

            serial_println!("Initializing Paging...");
            memory::paging::init_paging(); // Initialize paging for kernel
            update_progress(55);

            mouse::init_mouse();

            let (width, height) = {
                let fb = FRAMEBUFFER.lock();
                (fb.width, fb.height)
            };

            {
                let mut m = mouse::MOUSE_STATE.lock();
                m.x = (width / 2) as i32;
                m.y = (height / 2) as i32;
            }

            serial_println!("Initializing PIT...");
            pit::init(100); // Initialize PIT for 100Hz interrupts
            update_progress(65);

            serial_println!("Initializing Window Manager...");
            let mut window_manager = gui::WindowManager::new();

            // Get framebuffer dimensions
            let (fb_width, fb_height) = {
                let fb = FRAMEBUFFER.lock();
                (fb.width as u32, fb.height as u32)
            };
    
            window_manager.set_screen_size(fb_width, fb_height);

            // Initialize VFS and mount file systems
            let mut vfs = fs::vfs::VFS::new();

            // Mount NebulaFS as the root file system
            let mut nebula_fs = Box::new(fs::NebulaFS::new("nebula_pool", 4096, 1024 * 1024));
            if nebula_fs.mount("/").is_ok() {
                if let Err(e) = vfs.mount(nebula_fs, "/") {
                    serial_println!("Failed to mount NebulaFS: {}", e);
                } else {
                    serial_println!("Mounted NebulaFS at /");
                }
            }

            // Pass VFS to window manager
            window_manager.set_filesystem(vfs);
            serial_println!("Filesystem initialized and passed to window manager");
            update_progress(75);
            
            serial_println!("Initializing PIC and Exceptions...");
            idt::init_pic();
            exceptions::init();

            idt::set_gate(32, interrupts::timer_handler_asm as *const () as u32, 0x08, 0x8E); // IRQ 0 -> Vector 32
            idt::set_gate(33, interrupts::keyboard_handler_asm as *const () as u32, 0x08, 0x8E); // IRQ 1 -> Vector 33
            idt::set_gate(44, interrupts::mouse_handler_asm as *const () as u32, 0x08, 0x8E); // IRQ 12 -> Vector 44
            idt::set_gate(0x80, interrupts::syscall_handler_asm as *const () as u32, 0x08, 0xEE); // Syscall -> Vector 0x80 (DPL=3)
            
            idt::load_idt();
            update_progress(90);
            
            asm!("sti"); // Enable interrupts
            update_progress(100);
        }
    }

    // --- Launching the first user process ---
    extern "C" fn user_program_entry() -> ! {
        serial_println!("Entering user mode!");
        
        syscalls::syscall_draw_pixel(100, 100, 0x00FF0000); // Red pixel at (100, 100)
        serial_println!("User process drew a red pixel.");

        serial_println!("User process sleeping for 1 second...");
        syscalls::syscall_sleep(1000); // Sleep for 1000ms
        serial_println!("User process woke up.");

        serial_println!("User process exiting.");
        syscalls::syscall_exit(); // Exit the process
    }
    
    let user_kernel_stack_size = 4096;
    let user_stack_size = 4096 * 4; // 16KB user stack

    let entry_point_virtual_addr = user_program_entry as *const () as u32;
    
        let new_pid = {
        let mut sched = scheduler::SCHEDULER.lock();
        sched.spawn_user_process(
            entry_point_virtual_addr,
            user_stack_size,
                user_kernel_stack_size,
        )
    };
    
    unsafe {
        // Retrieve the process from the scheduler's list using the obtained PID.
        // Keep the scheduler lock guard in scope so the borrowed reference remains valid.
        let sched_guard = scheduler::SCHEDULER.lock();
        let process = match sched_guard.processes.get(new_pid) {
            Some(proc) => proc,
            None => {
                serial_println!("Error: Could not find process with PID {} after spawning.", new_pid);
                // Handle error: perhaps halt or panic.
                loop { asm!("hlt"); }
            }
        };
        
        gdt::set_kernel_stack(process.kernel_stack_ptr);
        asm!("mov cr3, {}", in(reg) process.page_directory_phys_addr);
        
        asm!(
            "cli",
            "push {ss_kern}",
            "push {esp_kern}",
            "pushfd",
            "or dword ptr [esp], 0x200",
            "push {cs_user}",
            "push {eip_user}",
            "iretd",

            ss_kern = in(reg) 0x10u32,
            esp_kern = in(reg) process.kernel_stack_ptr,
            cs_user = in(reg) 0x1Bu32,
            eip_user = in(reg) process.user_eip,

            options(noreturn)
        );
    }
}

fn test_filesystem(fs: &mut fs::NebulaFS) -> Result<(), &'static str> {
    // Test creating a file
    let file_inode = fs.create_file(2, "test_file.txt")?;
    serial_println!("Created file with inode: {}", file_inode);

    // Test writing to the file
    let test_data = b"Hello, NebulaFS!";
    let bytes_written = fs.write(file_inode, 0, test_data)?;
    serial_println!("Wrote {} bytes to file", bytes_written);

    // Test reading from the file
    let mut read_buffer = Vec::with_capacity(test_data.len());
    unsafe { read_buffer.set_len(test_data.len()); }
    let bytes_read = fs.read(file_inode, 0, &mut read_buffer)?;
    serial_println!("Read {} bytes from file", bytes_read);

    // Verify the data
    if &read_buffer[..bytes_read] == test_data {
        serial_println!("Data verification successful!");
    } else {
        return Err("Data verification failed");
    }

    // Test creating a directory
    let dir_inode = fs.create_dir(2, "test_dir")?;
    serial_println!("Created directory with inode: {}", dir_inode);

    // Test creating a snapshot
    fs.snapshot("test_snapshot")?;
    serial_println!("Created snapshot: test_snapshot");

    Ok(())
}

