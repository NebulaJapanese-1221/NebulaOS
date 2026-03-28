#[cfg(not(test))]
use core::panic::PanicInfo;
use core::arch::asm; 
use crate::drivers::framebuffer::FRAMEBUFFER;
use crate::userspace::fonts::font;
use crate::userspace::gui::{self, Rect};
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
pub mod cpu;
pub mod elf;

pub const VERSION: &str = "0.0.3-dev2";

pub static TOTAL_MEMORY: AtomicUsize = AtomicUsize::new(0);
pub static CPU_CORES: AtomicUsize = AtomicUsize::new(1);
static BOOT_STATUS_LINE: AtomicUsize = AtomicUsize::new(2);

fn draw_boot_screen() {   
    let title = "NebulaOS";
    let mut fb = FRAMEBUFFER.lock();
    if let Some(ref fb_info) = fb.info {
        let width = fb_info.width;
        let height = fb_info.height;

        fb.clear(0x00_000033); // Initial dark blue background

        let x_title = (width / 2).saturating_sub((title.len() * 8) / 2);
        let y_title = (height / 2).saturating_sub(16 / 2);

        // Fade-in the title over 100 steps for maximum smoothness
        for step in 0..=100u32 {
            let color = gui::interpolate_color(0x00_000033, 0x00_FFFFFF, step, 100);

            font::draw_string(&mut fb, x_title as isize, y_title as isize, title, color, None);
            fb.present();

            // Small delay loop to make the fade visible
            for _ in 0..2000000 { unsafe { asm!("nop"); } }
        }
    }
}

fn add_boot_status(status: &str) {
    let line = BOOT_STATUS_LINE.fetch_add(1, Ordering::Relaxed);
    let mut fb = FRAMEBUFFER.lock();
    let x = 20;
    // Start printing status messages
    let y = 20 + (line * 20);
    font::draw_string(&mut fb, x as isize, y as isize, status, 0x00_CCCCCC, None); // Light gray
    fb.present();
    // Intentional delay to make the boot process take longer and be readable
    for _ in 0..20000000 { unsafe { asm!("nop"); } }
}

/// Displays a critical error screen during the boot process and halts.
fn show_boot_error(message: &str) -> ! {
    crate::serial_println!("\nBOOT ERROR: {}", message);
    
    // Check if the framebuffer is initialized and has a draw buffer to draw the error screen
    let mut fb_lock = FRAMEBUFFER.lock();
    if fb_lock.info.is_some() && fb_lock.draw_buffer.is_some() {
        fb_lock.clear(0x00_880000); // Dark red background
        
        font::draw_string(&mut fb_lock, 30, 30, "!! BOOT ERROR !!", 0xFFFFFFFF, None);
        font::draw_string(&mut fb_lock, 30, 60, "NebulaOS failed to initialize during boot.", 0xFFFFFFFF, None);
        
        let mut y = 100;
        font::draw_string(&mut fb_lock, 30, y, "Reason:", 0x00_CCCCCC, None);
        y += 20;
        font::draw_string(&mut fb_lock, 30, y, message, 0xFFFFFFFF, None);
        
        y += 40;
        font::draw_string(&mut fb_lock, 30, y, "The system has been halted to prevent damage.", 0x00_CCCCCC, None);
        font::draw_string(&mut fb_lock, 30, y + 20, "Please check the serial log for more details.", 0x00_CCCCCC, None);
        
        fb_lock.present();
    }
    
    loop { unsafe { asm!("cli; hlt", options(nomem, nostack)); } }
}

/// Smoothly fades the entire boot screen to black before transitioning to the desktop.
fn fade_out_boot_screen() {
    let mut fb = FRAMEBUFFER.lock();

    // Perform fade-out in 64 steps for a smoother transition to desktop
    for step in (0..64u32).rev() {
        gui::fade_buffer(&mut fb, step, 64);
        fb.present();
        // Small delay loop to make the fade visible
        for _ in 0..2000000 { unsafe { asm!("nop"); } }
    }
}

fn draw_progress_bar(_progress: usize) {
    let mut fb = FRAMEBUFFER.lock();
    let (width, height) = if let Some(info) = fb.info.as_ref() {
        (info.width, info.height)
    } else { return };

    // Draw the loading spinner below the title (replacing the progress bar)
    let spinner_x = (width / 2) as isize;
    let spinner_y = (height / 2) as isize + 60;
    gui::draw_loading_spinner(&mut fb, spinner_x, spinner_y, Rect { x: 0, y: 0, width, height });

    fb.present();
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

    // 1. Initialize the heap allocator first (required for FB draw buffer allocation)
    if let Some((heap_start, heap_size)) = heap_region {
        unsafe {
            allocator::ALLOCATOR.lock().init(heap_start as *mut u8, heap_size);
            TOTAL_MEMORY.store(heap_size, Ordering::Relaxed);
        }
    } else {
        show_boot_error("Could not find a suitable heap region!");
    }

    // 2. Initialize Framebuffer to show boot progress and potential errors
    if let Some(fb_info) = fb_info_opt {
        crate::drivers::framebuffer::FRAMEBUFFER.lock().init(fb_info);
        // Clear screen and show initial text immediately
        draw_boot_screen(); 
    } else {
        show_boot_error("No framebuffer information found!");
    }
    
    // Initialize GDT and TSS
    gdt::init();

    // Set PIT frequency for scheduler (e.g., 1000 Hz)
    // This must be done before interrupts are enabled.
    crate::drivers::pit::set_frequency(1000);

    // Initialize IDT (but do not enable interrupts yet)
    interrupts::init();
    add_boot_status("[OK] Interrupts Initialized"); 
    draw_progress_bar(30);

    // Initialize the mouse driver (polls for ACKs, so interrupts must be disabled)
    if let Err(e) = crate::drivers::mouse::initialize() {
        show_boot_error(e);
    } else {
        add_boot_status("[OK] Mouse Driver Initialized");
    }
    draw_progress_bar(60);

    // Initialize the keyboard driver
    if let Err(e) = crate::drivers::keyboard::init() {
        show_boot_error(e);
    } else {
        add_boot_status("[OK] Keyboard Driver Initialized");
    }

    // Now it is safe to enable interrupts
    interrupts::enable_interrupts();
    add_boot_status("[OK] Interrupts Enabled");
    draw_progress_bar(90);

    // Initialize ACPI
    acpi::init();

    // Initialize CPU Info detection (CPUID)
    cpu::init();
    add_boot_status("[OK] CPU Info Detected");

    // NOTE: The old text-mode GUI has been disabled.
    // The next step is to build a new GUI that draws to the framebuffer.
    add_boot_status("[OK] Initializing GUI...");
    draw_progress_bar(100);
    
    // Initialize localisation before GUI
    crate::userspace::localisation::init();
    
    // Perform final transition effect before starting the GUI
    fade_out_boot_screen();

    crate::userspace::gui::init();
    
    // Spawn the dedicated Render Task to offload blitting from this input loop
    let render_entry = crate::userspace::gui::window_manager::WindowManager::render_loop as usize;
    crate::kernel::process::SCHEDULER.lock().add_task(render_entry, 15);

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
    // Force unlock the framebuffer to prevent deadlock if the panic happened while drawing
    unsafe { FRAMEBUFFER.force_unlock(); }
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
