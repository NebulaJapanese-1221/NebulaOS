#![no_std]
#![no_main]

extern crate alloc;

// Multiboot Header (allows GRUB or QEMU -kernel to load the OS)
core::arch::global_asm!(
    ".section .multiboot, \"a\"", // Ensure the section is allocatable
    ".align 4",
    ".long 0x1BADB002",           // Magic
    // Flags: 
    // bit 0: ALIGN (align modules on page boundaries)
    // bit 1: MEM_INFO (provide memory map)
    // bit 2: VIDEO (request video mode)
    ".long 0x00000007",           
    ".long -(0x1BADB002 + 0x00000007)", // Checksum (magic + flags + checksum = 0)
    ".long 0, 0, 0, 0, 0",        // Header, load, load_end, bss_end, entry addr (unused for ELF)
    ".long 0",                    // Mode type (0 = Linear Framebuffer)
    ".long 1024",                 // Width
    ".long 768",                  // Height
    ".long 32"                    // Depth (32-bit color)
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
use allocator::LinkedHeap;

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

use core::arch::asm;
use alloc::vec;
use gui::{CURSOR_BITMAP, CURSOR_WIDTH, CURSOR_HEIGHT};
use framebuffer::{FRAMEBUFFER};

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
    mem_lower: u32,      // Offset 4
    mem_upper: u32,      // Offset 8
    _ignore1: [u32; 8],  // Offsets 12-44
    mmap_length: u32,    // Offset 44
    mmap_addr: u32,      // Offset 48
    _ignore2: [u32; 9],  // Offsets 52-88
    fb_addr: u64,
    fb_pitch: u32,
    fb_width: u32,
    fb_height: u32,
    fb_bpp: u8,
    fb_type: u8,
}

#[global_allocator]
static ALLOCATOR: LinkedHeap = LinkedHeap::empty();

const TASKBAR_HEIGHT: u32 = 40;
static mut LAST_MOUSE_X: i32 = 0;
static mut LAST_MOUSE_Y: i32 = 0;

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

            serial_println!("Multiboot flags: 0x{:x}", flags);
            serial_println!("Multiboot MB_PTR: 0x{:x}", mb_ptr);

            // Safely parse Memory Map (Flag bit 6)
            if (flags & 0x40) != 0 {
                let mmap_addr = info.mmap_addr;
                let mmap_length = info.mmap_length;
                serial_println!("Memory Map found at 0x{:X}, length: {} bytes", mmap_addr, mmap_length);

                let mut current_addr = mmap_addr;
                let end_addr = mmap_addr + mmap_length;

                while current_addr < end_addr {
                    let entry = unsafe { &*(current_addr as *const MultibootMmapEntry) };
                    let addr = entry.addr;
                    let len = entry.len;
                    let type_ = entry.type_;
                    let size = entry.size;

                    serial_println!("  Region: 0x{:08X} -> 0x{:08X}, Type: {}", addr, addr + len, type_);
                    current_addr += size + 4;
                }
            }

            if (flags & 0x800) != 0 { // Check for MULTIBOOT_INFO_FRAMEBUFFER_INFO (bit 11)
                let fb_addr = info.fb_addr;
                let fb_width = info.fb_width;
                let fb_height = info.fb_height;
                let fb_pitch = info.fb_pitch;

                let mut fb = FRAMEBUFFER.lock();
                fb.init(fb_addr as *mut u32, fb_width as usize, fb_height as usize, fb_pitch as usize);
                serial_println!("Framebuffer initialized at 0x{:X} ({}x{})", fb_addr, fb_width, fb_height);
            }
        }
        
        // Helper to update boot progress bar
        let update_progress = |progress: usize| {
            {
                let mut fb = FRAMEBUFFER.lock();
                let bar_width = 400;
                let bar_height = 12;
                let x = (fb.width - bar_width) / 2;
                let y = fb.height - 150;
                
                fb.draw_rect(x, y, bar_width, bar_height, 0x00111111); // Track background
                fb.draw_rect(x, y, (bar_width * progress) / 100, bar_height, 0x0000AAFF); // Blue progress
                fb.present();
            }
            // Delay reduced and moved outside the lock to prevent the appearance of a freeze
            for _ in 0..500000 { core::hint::spin_loop(); } 
        };

        serial_println!("Initializing Framebuffer...");
        {
            let mut fb = FRAMEBUFFER.lock();
            // Use the text as the primary logo, centered on screen, scaled and purple
            let scale = 8;
            let string_width = 8 * 8 * scale; // 8 characters * 8 pixels * scale
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
            ALLOCATOR.init(0x1000000, 0x1000000);
            update_progress(50);

            // Initialize hardware that requires interrupts to be OFF
            mouse::init_mouse();

            let (width, height) = {
                let fb = FRAMEBUFFER.lock();
                (fb.width, fb.height)
            };

            // Initialize mouse position to screen center BEFORE interrupts start
            {
                let mut m = mouse::MOUSE_STATE.lock();
                m.x = (width / 2) as i32;
                m.y = (height / 2) as i32;
            }

            serial_println!("Initializing PIT...");
            pit::init(100);
            update_progress(65);

            serial_println!("Initializing PIC and Exceptions...");
            idt::init_pic();
            exceptions::init();
            
            idt::set_gate(32, interrupts::timer_handler_asm as *const () as u32, 0x08, 0x8E);
            idt::set_gate(44, interrupts::mouse_handler_asm as *const () as u32, 0x08, 0x8E);
            idt::set_gate(33, interrupts::keyboard_handler_asm as *const () as u32, 0x08, 0x8E);
            idt::set_gate(0x80, interrupts::syscall_handler_asm as *const () as u32, 0x08, 0xEE);
            
            idt::load_idt();
            update_progress(90);
            
            asm!("sti");
            update_progress(100);
        }
    }

    let (width, height) = {
        let fb = FRAMEBUFFER.lock();
        (fb.width, fb.height)
    };

    // Allocate backbuffer based on detected resolution
    let mut backbuffer = vec![0u32; width * height];
    {
        let mut fb = FRAMEBUFFER.lock();
        fb.backbuffer = backbuffer.as_mut_ptr();
    }
    
    syscalls::test_syscall();

    let mut start_menu_open = false;
    let mut wm = gui::WindowManager::new();
    // All windows closed on boot; Nebula Explorer removed.
    loop {
        let (width, height) = {
            let fb = FRAMEBUFFER.lock();
            (fb.width, fb.height)
        };
        wm.set_screen_size(width as u32, height as u32);

        let mut fb = FRAMEBUFFER.lock();

        // 3. Handle Input & Cursor
        let (mx, my, ml, mr) = {
            let mut m = mouse::MOUSE_STATE.lock();
            // Clamp logical coordinates to screen dimensions to prevent "drifting" off-screen
            m.x = m.x.clamp(0, width as i32);
            m.y = m.y.clamp(0, height as i32);
            (m.x, m.y, m.left_button, m.right_button)
        };

        // Mark the OLD cursor position as dirty so it gets erased from the LFB
        unsafe {
            fb.mark_dirty(LAST_MOUSE_X as u32, LAST_MOUSE_Y as u32, CURSOR_WIDTH as u32, CURSOR_HEIGHT as u32);
        }

        // Handle mouse input through the WindowManager
        if wm.handle_mouse(mx, my, ml, mr) {
            // Dispatch to start menu logic
            gui::start_menu::handle_click(mx, my, height as i32, &mut wm, &mut start_menu_open);
        }

        // 1. Render UI Components (Desktop + Taskbar)
        let time = rtc::get_time();
        gui::render_ui(&mut fb, start_menu_open, time.hour, time.minute, time.second, wm.windows.as_slice());
        
        // 2. Render Windows
        wm.draw(&mut fb);
        // 3.1 Process keyboard input
        while let Some(_c) = keyboard::KEY_BUFFER.lock().pop() {
            wm.handle_keyboard_input(_c);
        }

        // Clamp coordinates to prevent wrap-around when casting to usize
        let cursor_x = mx.clamp(0, width as i32 - CURSOR_WIDTH as i32) as usize;
        let cursor_y = my.clamp(0, height as i32 - CURSOR_HEIGHT as i32) as usize;

        // Draw a shadow/outline first to remove the "extra background" bloom effect
        fb.draw_bitmap(cursor_x + 1, cursor_y + 1, CURSOR_WIDTH, CURSOR_HEIGHT, &CURSOR_BITMAP, 0x00000000);
        
        // Draw the main cursor
        fb.draw_bitmap(cursor_x, cursor_y, CURSOR_WIDTH, CURSOR_HEIGHT, &CURSOR_BITMAP, 0x00FFFFFF);
        
        unsafe {
            LAST_MOUSE_X = cursor_x as i32;
            LAST_MOUSE_Y = cursor_y as i32;
        }
        
        // 4. Swap buffers
        fb.present();

        // 5. Halt the CPU until the next interrupt (PIT, Mouse, or Keyboard)
        // This prevents the OS from laggy 'infinite' re-renders and saves CPU.
        unsafe { asm!("hlt"); }
    }
}
