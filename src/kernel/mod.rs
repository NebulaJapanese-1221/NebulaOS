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
pub mod process;

pub const VERSION: &str = "0.0.2";

pub static TOTAL_MEMORY: AtomicUsize = AtomicUsize::new(0);

fn draw_boot_screen() {   
    let mut fb = FRAMEBUFFER.lock();
    let (width, height) = {
        if let Some(ref fb_info) = fb.info {
            (fb_info.width, fb_info.height)
        } else {
            return;
        }
    };

    fb.clear(0x00_000033); // Dark blue background

    let title = "NebulaOS";
    let x_title = (width / 2).saturating_sub((title.len() * 8) / 2);
    let y_title = (height / 2).saturating_sub(16 / 2);
    
    font::draw_string(&mut fb, x_title as isize, y_title as isize, title, 0x00_FFFFFF, None);

    fb.present();
}

fn add_boot_status(status: &str, line: usize) {
    let mut fb = FRAMEBUFFER.lock();
    let x = 20;
    // Start printing status messages (line is 1-based index)
    let y = 20 + (line * 20);
    font::draw_string(&mut fb, x as isize, y as isize, status, 0x00_CCCCCC, None); // Light gray
    fb.present();
}

fn draw_progress_bar(progress: usize) {
    let mut fb = FRAMEBUFFER.lock();
    if let Some(info) = fb.info.as_ref() {
        let bar_width = 400;
        let bar_height = 20;
        let x = (info.width / 2) - (bar_width / 2);
        let y = (info.height / 2) + 20;
        
        // Draw background
        for j in 0..bar_height {
            for i in 0..bar_width {
                fb.set_pixel(x + i, y + j, 0x00_404040);
            }
        }
        
        // Draw progress
        let filled_width = (bar_width * progress) / 100;
        for j in 0..bar_height {
            for i in 0..filled_width {
                fb.set_pixel(x + i, y + j, 0x00_00FF00);
            }
        }
        fb.present();
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
        // Clear screen and show initial text immediately
        draw_boot_screen(); 
    } else {
        crate::serial_println!("ERROR: No framebuffer information found!");
    }
    
    // Initialize GDT and TSS
    gdt::init();

    // Set PIT frequency for scheduler (e.g., 1000 Hz)
    // This must be done before interrupts are enabled.
    crate::drivers::pit::set_frequency(1000);

    // Initialize IDT (but do not enable interrupts yet)
    interrupts::init();
    add_boot_status("[OK] Interrupts Initialized", 2); 
    draw_progress_bar(30);

    // Initialize the mouse driver (polls for ACKs, so interrupts must be disabled)
    crate::drivers::mouse::initialize();
    add_boot_status("[OK] Mouse Driver Initialized", 3);
    draw_progress_bar(60);

    // Initialize the keyboard driver
    crate::drivers::keyboard::init();

    // Now it is safe to enable interrupts
    interrupts::enable_interrupts();
    add_boot_status("[OK] Interrupts Enabled", 4);
    draw_progress_bar(90);

    // Initialize ACPI
    acpi::init();

    // NOTE: The old text-mode GUI has been disabled.
    // The next step is to build a new GUI that draws to the framebuffer.
    add_boot_status("[OK] Initializing GUI...", 5);
    draw_progress_bar(100);
    
    // Initialize localisation before GUI
    crate::userspace::localisation::init();
    
    crate::userspace::gui::init();
    
    // Halt loop (The scheduler will hijack execution on the next timer tick)
    loop {
        crate::userspace::gui::update();
        // Halt CPU until next interrupt to save power (and prevent 100% CPU usage)
        unsafe { asm!("hlt") };
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
