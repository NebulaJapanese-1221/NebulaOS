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
mod process;
mod scheduler;
mod syscalls;
mod panic;
mod exceptions;
mod memory;
mod services;

#[path = "../fs/mod.rs"]
mod fs;

use allocator::ALLOCATOR; 
use core::arch::asm;
use framebuffer::FRAMEBUFFER;
use alloc::vec::Vec;
use alloc::boxed::Box;

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
    // These will survive after initialization (use Option since init happens inside block)
    let mut window_manager: gui::WindowManager;
    let mut start_menu_open: bool = false;
    let mut last_mouse_l: bool = false;
    let mut _last_mouse_r: bool = false;
    let mut login_state: gui::LoginState = gui::LoginState::new();

    {
        serial::SERIAL_PORT.lock().init();
        serial_println!("NebulaOS v0.0.1 started...");

        if magic == 0x2BADB002 {
            let info = unsafe { &*(mb_ptr as *const MultibootInfo) };
            let flags = info.flags;

            if (flags & 0x40) != 0 {
                let mmap_addr = info.mmap_addr;
                let mmap_length = info.mmap_length;
                serial_println!("Memory Map found at 0x{:X}, length: {} bytes", mmap_addr, mmap_length);
                let mut current_addr = mmap_addr;
                let end_addr = mmap_addr + mmap_length;
                while current_addr < end_addr {
                    let mut entry_data: [u8; 24] = [0; 24];
                    let entry_slice = unsafe { core::slice::from_raw_parts(current_addr as *const u8, 24) };
                    entry_data.copy_from_slice(entry_slice);
                    
                    let entry = unsafe { &*(entry_data.as_ptr() as *const MultibootMmapEntry) };
                    
                    let addr = entry.addr;
                    let len = entry.len;
                    let type_ = entry.type_;
                    
                    if type_ == 1 {
                        serial_println!("  Available: 0x{:016X} - 0x{:016X} ({} MB)", addr, addr + len, len / (1024 * 1024));
                    }
                    current_addr += entry.size + 4;
                }
            }

            if (flags & 0x800) != 0 {
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
            let heap_start = 0x1000000;
            let heap_size = 0x100000;
            ALLOCATOR.init(heap_start, heap_size);
            update_progress(50);

            serial_println!("Initializing Paging...");
            memory::paging::init_paging();
            update_progress(55);

            serial_println!("Initializing System Services...");
            services::init();
            update_progress(60);

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
            pit::init(100);
            update_progress(65);

            serial_println!("Initializing Window Manager...");
            window_manager = gui::WindowManager::new();

            let (fb_width, fb_height) = {
                let fb = FRAMEBUFFER.lock();
                (fb.width as u32, fb.height as u32)
            };
    
            window_manager.set_screen_size(fb_width, fb_height);

            let mut vfs = fs::vfs::VFS::new();

            let mut nebula_fs = Box::new(fs::NebulaFS::new("nebula_pool", 4096, 1024 * 1024));
            if nebula_fs.mount().is_ok() {
                if let Err(e) = vfs.mount(nebula_fs, "/") {
                    serial_println!("Failed to mount NebulaFS: {}", e);
                } else {
                    serial_println!("Mounted NebulaFS at /");
                }
            }

            window_manager.set_filesystem(vfs);
            serial_println!("Filesystem initialized and passed to window manager");
            update_progress(75);
            
            serial_println!("Initializing PIC and Exceptions...");
            idt::init_pic();
            exceptions::init();

            idt::set_gate(32, interrupts::timer_handler_asm as *const () as u32, 0x08, 0x8E);
            idt::set_gate(33, interrupts::keyboard_handler_asm as *const () as u32, 0x08, 0x8E);
            idt::set_gate(44, interrupts::mouse_handler_asm as *const () as u32, 0x08, 0x8E);
            idt::set_gate(0x80, interrupts::syscall_handler_asm as *const () as u32, 0x08, 0xEE);
            
            idt::load_idt();
            update_progress(90);
            
            asm!("sti");
            update_progress(100);
        }
    }

    // Spawn a demo user process
    extern "C" fn user_program_entry() -> ! {
        serial_println!("Entering user mode!");
        
        syscalls::syscall_draw_pixel(100, 100, 0x00FF0000);
        serial_println!("User process drew a red pixel.");

        serial_println!("User process sleeping for 1 second...");
        syscalls::syscall_sleep(1000);
        serial_println!("User process woke up.");

        serial_println!("User process exiting.");
        syscalls::syscall_exit();
    }
    
    {
        let mut sched = scheduler::get_scheduler().lock();
        sched.spawn_user_process(
            user_program_entry as *const () as u32,
            4096 * 4,
            4096,
        );
    }

    serial_println!("Desktop environment started. Entering main loop...");

    // ===== MAIN EVENT LOOP =====
    loop {
        // --- 1. Process keyboard input ---
        while let Some(key) = keyboard::KEY_BUFFER.lock().pop() {
            if login_state.logged_in {
                window_manager.handle_keyboard_input(key);
            }
        }

        // --- 2. Process mouse input ---
        {
            let mouse = mouse::MOUSE_STATE.lock();
            let ml = mouse.left_button;
            let mr = mouse.right_button;
            let mx = mouse.x;
            let my = mouse.y;
            drop(mouse);

            if !login_state.logged_in {
                // Login screen mode
                if ml && !last_mouse_l {
                    login_state.handle_click(mx, my);

                    if login_state.shutdown_requested {
                        serial_println!("Shutdown requested. Halting system...");
                        loop { core::hint::spin_loop(); }
                    }

                    if login_state.restart_requested {
                        serial_println!("Restart requested. Triggering restart...");
                        loop { core::hint::spin_loop(); }
                    }
                }
            } else {
                // Desktop mode
                if ml != last_mouse_l || ml {
                    let toggled = window_manager.handle_mouse(mx, my, ml, mr);
                    if toggled {
                        start_menu_open = !start_menu_open;
                    }
                }

                if start_menu_open {
                    let fb_height = {
                        let fb = FRAMEBUFFER.lock();
                        fb.height
                    };
                    gui::start_menu::handle_click(mx, my, fb_height as i32, &mut window_manager, &mut start_menu_open, &mut login_state.logged_in);
                }

                if !login_state.logged_in {
                    window_manager.windows.clear();
                    start_menu_open = false;
                }
            }

            last_mouse_l = ml;
            _last_mouse_r = mr;
        }

        // --- 3. Render the full GUI ---
        {
            let mut fb = FRAMEBUFFER.lock();

            if !login_state.logged_in {
                login_state.draw(&mut fb);
            } else {
                let fb_w = fb.width;
                let fb_h = fb.height;
                fb.draw_rect(0, 0, fb_w, fb_h, 0x00003366);

                window_manager.draw(&mut fb);

                let time = rtc::get_time();

                gui::render_ui(&mut fb, start_menu_open, time.hour, time.minute, time.second, window_manager.windows.as_slice());
            }

            // Draw mouse cursor on top of everything
            {
                let mouse = mouse::MOUSE_STATE.lock();
                let cx = mouse.x as usize;
                let cy = mouse.y as usize;
                drop(mouse);

                let bitmap = gui::window_manager::CURSOR_BITMAP;
                for row in 0..19 {
                    for col in 0..12 {
                        if (bitmap[row] & (0x800 >> col)) != 0 {
                            let px = cx + col;
                            let py = cy + row;
                            if px < fb.width && py < fb.height {
                                fb.draw_pixel(px, py, 0x00FFFFFF);
                            }
                        }
                    }
                }
            }

            fb.present();
        }

        // --- 4. Small delay ---
        for _ in 0..100000 { core::hint::spin_loop(); }
    }
}

fn test_filesystem(fs: &mut fs::NebulaFS) -> Result<(), &'static str> {
    use crate::fs::FileSystemOps;
    let file_inode = FileSystemOps::create_file(fs, 2, "test_file.txt")?;
    serial_println!("Created file with inode: {}", file_inode);

    let test_data = b"Hello, NebulaFS!";
    let bytes_written = FileSystemOps::write(fs, file_inode, 0, test_data)?;
    serial_println!("Wrote {} bytes to file", bytes_written);

    let mut read_buffer: Vec<u8> = Vec::with_capacity(test_data.len());
    unsafe { read_buffer.set_len(test_data.len()); }
    let read_slice = read_buffer.as_mut_slice();
    let bytes_read = FileSystemOps::read(fs, file_inode, 0, read_slice)?;
    serial_println!("Read {} bytes from file", bytes_read);

    if &read_buffer.as_slice()[..bytes_read] == test_data {
        serial_println!("Data verification successful!");
    } else {
        return Err("Data failed");
    }

    let dir_inode = FileSystemOps::create_dir(fs, 2, "test_dir")?;
    serial_println!("Created directory with inode: {}", dir_inode);

    fs.snapshot("test_snapshot")?;
    serial_println!("Created snapshot: test_snapshot");

    Ok(())
}
