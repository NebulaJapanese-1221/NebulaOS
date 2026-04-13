#[cfg(not(test))]
use core::panic::PanicInfo;
use core::arch::asm; 
use crate::drivers::framebuffer::FRAMEBUFFER;
use crate::userspace::fonts::font;
use core::sync::atomic::{AtomicUsize, Ordering};
pub mod io;
pub mod interrupts;
pub mod multiboot;
pub mod allocator;
pub mod acpi;
pub mod exceptions;
pub mod power;
pub mod gdt;
pub mod syscall;
pub mod paging; // Make paging module public
pub mod process;
pub mod cpu;
pub mod elf;

pub const VERSION: &str = "0.0.3-dev2";

pub static TOTAL_MEMORY: AtomicUsize = AtomicUsize::new(0);
pub static CPU_CORES: AtomicUsize = AtomicUsize::new(1);
static BOOT_ANIM_FRAME: AtomicUsize = AtomicUsize::new(0);

/// Pre-calculated offsets for a 12-spoke loading wheel (30-degree increments).
const SPOKE_OFFSETS: [(isize, isize); 12] = [
    (0, -20), (10, -17), (17, -10), (20, 0),
    (17, 10), (10, 17), (0, 20), (-10, 17),
    (-17, 10), (-20, 0), (-17, -10), (-10, -17)
];

pub(crate) fn draw_spinner(fb: &mut crate::drivers::framebuffer::Framebuffer, cx: isize, cy: isize) {
    let frame = BOOT_ANIM_FRAME.fetch_add(1, Ordering::Relaxed);
    
    for i in 0..12 {
        let (dx, dy) = SPOKE_OFFSETS[i];
        
        // Calculate brightness: current spoke is brightest, previous ones fade out (tail effect)
        let diff = (i + 12 - (frame % 12)) % 12;
        let (r, g, b) = match diff {
            0 => (255, 255, 255), // Lead (White)
            1 => (100, 200, 255), // Nebula Blue
            2 => (80, 150, 255),
            3 => (60, 100, 200),
            4 => (40, 60, 150),
            5 => (20, 30, 80),
            _ => (15, 15, 40),    // Dim tail
        };
        
        let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);

        // Draw the spoke as a line from the inner hub (radius 8) to outer rim (radius 20)
        for r in 8..=20 {
            let px = cx + (dx * r / 20);
            let py = cy + (dy * r / 20);
            fb.set_pixel(px as usize, py as usize, color);
        }
    }
}

fn draw_boot_screen_content(fb: &mut crate::drivers::framebuffer::Framebuffer, status: &str, progress: usize) {
    let (width, height) = match fb.info.as_ref() {
        Some(info) => (info.width, info.height),
        None => return,
    };

    // Optimized: Only clear the central active area to maintain high FPS
    let clear_rect = crate::userspace::gui::rect::Rect {
        x: (width / 2) as isize - 210,
        y: (height / 2) as isize - 20,
        width: 420,
        height: 160,
    };
    crate::userspace::gui::draw_rect(fb, clear_rect.x, clear_rect.y, clear_rect.width, clear_rect.height, 0x00_050515, None);

    let title = "NebulaOS";
    let x_title = (width / 2).saturating_sub((title.len() * 8) / 2);
    let y_title = (height / 2).saturating_sub(8); // True vertical center for logo
    
    font::draw_string(fb, x_title as isize, y_title as isize, title, 0x00_FFFFFF, None);

    // Optimized Reflection: Draw once with a fixed dim color to save cycles
    font::draw_string(fb, x_title as isize, y_title as isize + 14, title, 0x00_151535, None);

    // Draw bike-spoke spinner below the title
    draw_spinner(fb, (width / 2) as isize, (height / 2) as isize + 45);

    // Draw status message
    let x_status = (width / 2).saturating_sub((status.len() * 8) / 2);
    font::draw_string(fb, x_status as isize, (height / 2) as isize + 85, status, 0x00_CCCCCC, None);

    // Draw progress bar
    draw_progress_bar_internal(fb, progress, width, height);
}

fn draw_boot_screen(status: &str, progress: usize) {
    let mut fb = FRAMEBUFFER.lock();
    let (width, height) = match fb.info.as_ref() {
        Some(info) => (info.width, info.height),
        None => return,
    };

    draw_boot_screen_content(&mut fb, status, progress);
    // Optimized present covering the new vertically shifted group
    fb.present_rect(width / 2 - 210, (height / 2).saturating_sub(15), 420, 150);
}

fn add_boot_status(status: &str, progress: usize) {
    draw_boot_screen(status, progress);
}

fn draw_progress_bar_internal(fb: &mut crate::drivers::framebuffer::Framebuffer, progress: usize, width: usize, height: usize) {
    let info = if let Some(i) = fb.info.as_ref() { i } else { return };
    let buffer = if let Some(b) = fb.draw_buffer.as_mut() { b } else { return };

    let bar_width = 400;
    let bar_height = 4;
    let x = (width / 2).saturating_sub(bar_width / 2);
    let y = (height / 2) + 120;
    
    // Draw border
    for j in -1..(bar_height as isize + 1) {
        let py = (y as isize + j) as usize;
        buffer[py * info.width + x - 1] = 0x00_444444;
        buffer[py * info.width + x + bar_width] = 0x00_444444;
    }
    for i in 0..bar_width {
        buffer[(y - 1) * info.width + x + i] = 0x00_444444;
        buffer[(y + bar_height) * info.width + x + i] = 0x00_444444;
    }

    // Draw background
    for py in y..(y + bar_height) {
        let offset = py * info.width + x;
        buffer[offset..offset + bar_width].fill(0x00_202020);
    }
    
    // Draw progress
    let filled_width = (bar_width * progress) / 100;
    for py in y..(y + bar_height) {
        let offset = py * info.width + x;
        buffer[offset..offset + filled_width].fill(0x00_00AAFF);
    }
}

// Entry point called by boot assembly
#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info_ptr: usize) -> ! {
    // It's crucial to disable interrupts before initializing the allocator
    unsafe { asm!("cli", options(nomem, nostack)); }

    // Initialize Serial Port
    crate::drivers::serial::SERIAL1.lock().init(); 

    // --- Early hardware discovery and initialization ---
    let heap_region = allocator::find_heap_region(multiboot_info_ptr);
    let fb_info_opt = multiboot::framebuffer_info(multiboot_info_ptr);

    // Initialize the heap allocator.
    if let Some((heap_start, heap_size)) = heap_region {
        unsafe {
            allocator::ALLOCATOR.lock().init(heap_start as *mut u8, heap_size);
            TOTAL_MEMORY.store(heap_size, Ordering::Relaxed);
        }
    } else {
        crate::serial_println!("ERROR: Could not find a suitable heap region!");
    }

    // Initialize Framebuffer first to show boot status text
    if let Some(fb_info) = fb_info_opt {
        crate::drivers::framebuffer::FRAMEBUFFER.lock().init(fb_info);
        
        // Enable hardware paging now that we have memory map and FB info
        paging::init(fb_info_opt);

        // Setup the initial frame in the backbuffer without presenting it (prevents flash)
        {
            let mut fb = FRAMEBUFFER.lock();
            draw_boot_screen_content(&mut fb, "Starting NebulaOS...", 0);
            fb.clear(0x00_050515); // Full clear only once on entry
            fb.apply_fade(10, 10); // Capture to scratch, using 10 steps for speed
            for i in 0..=10 { 
                fb.apply_fade(i, 10); // Fade from black to full
                fb.present(); // Full screen present required for fade
                for _ in 0..40000 { unsafe { asm!("nop") } } // Calibrated delay for smoothness
            }
        }
    } else {
        crate::serial_println!("ERROR: No framebuffer information found!");
    }
    
    // Initialize GDT and TSS
    gdt::init();
    add_boot_status("Initializing GDT...", 10);

    // Set PIT frequency for scheduler (e.g., 1000 Hz)
    crate::drivers::pit::set_frequency(1000);

    // Initialize IDT (but do not enable interrupts yet)
    interrupts::init();
    add_boot_status("Interrupts Initialized", 20); 

    // Initialize the mouse driver (polls for ACKs, so interrupts must be disabled)
    crate::drivers::mouse::initialize();
    add_boot_status("Mouse Driver Initialized", 40);

    // Initialize the keyboard driver
    crate::drivers::keyboard::init();
    add_boot_status("Keyboard Driver Initialized", 50);

    // Initialize the brightness driver
    crate::drivers::brightness::BRIGHTNESS.lock().init();
    add_boot_status("Brightness Driver Initialized", 60);

    // Now it is safe to enable interrupts
    interrupts::enable_interrupts();
    add_boot_status("Interrupts Enabled", 70);

    // Initialize ACPI
    acpi::init();
    add_boot_status("ACPI Subsystem Ready", 80);

    // Initialize CPU Info detection (CPUID)
    cpu::init();
    add_boot_status("CPU Topology Detected", 90);

    add_boot_status("Launching Desktop Environment...", 100);

    // Fade out boot screen
    {
        let mut fb = FRAMEBUFFER.lock();
        fb.apply_fade(10, 10); // Sync scratch buffer
        for i in (0..=10).rev() { 
            fb.apply_fade(i, 10);
            fb.present();
            for _ in 0..40000 { unsafe { asm!("nop") } }
        }
    }
    
    // Initialize localisation before GUI
    crate::userspace::localisation::init();
    
    crate::userspace::gui::init();
    
    // Halt loop (The scheduler will hijack execution on the next timer tick)
    loop {
        crate::userspace::gui::update();
        
        // Mark as idle right before halting
        crate::kernel::cpu::IS_IDLE.store(true, Ordering::Relaxed);
        // Halt CPU until next interrupt to save power (and prevent 100% CPU usage)
        unsafe { asm!("hlt") };
        // Mark as active immediately after waking up
        crate::kernel::cpu::IS_IDLE.store(false, Ordering::Relaxed);
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {    
    // Disable interrupts to prevent further issues during panic
    unsafe { core::arch::asm!("cli") };

    crate::serial_println!("\nKERNEL PANIC\n{}", info);
    unsafe { exceptions::print_stack_trace(); }

    // Draw to screen
    let mut fb = FRAMEBUFFER.lock();
    fb.clear(0x00_CC0000); // Red (RSOD)
    
    font::draw_string(&mut fb, 30, 30, ":(", 0xFFFFFFFF, None);
    font::draw_string(&mut fb, 30, 60, "NebulaOS ran into a problem and needs to restart.", 0xFFFFFFFF, None);

    let mut writer = exceptions::PanicWriter::new(&mut fb, 30, 90);
    use core::fmt::Write;
    let _ = writeln!(writer, "Stop Code: KERNEL_PANIC");
    let _ = writeln!(writer, "Details: {}", info);
    let _ = writeln!(writer, "\nTechnical Information:\n----------------------");
    unsafe { exceptions::print_stack_trace_to(&mut writer); }
    
    fb.present();

    loop {
        // Halt the CPU to prevent further execution
        unsafe { asm!("cli; hlt", options(nomem, nostack)); }
    }
}
