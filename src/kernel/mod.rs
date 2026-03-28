#[cfg(not(test))]
use core::panic::PanicInfo;
use core::arch::asm; 
use crate::drivers::framebuffer::FRAMEBUFFER;
use crate::userspace::fonts::font;
use crate::userspace::gui::{self, Rect};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::kernel::process::TICKS;
use spin::Mutex;
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
static BOOT_ANIMATION_RUNNING: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
static BOOT_STATUS_LINE: AtomicUsize = AtomicUsize::new(2);
static WATCHDOG_STATE: Mutex<(usize, u64, u32)> = Mutex::new((0, 0, 0));

/// Monitors the health of the kernel heartbeat.
/// If TICKS stays the same while TSC continues to increase significantly, triggers a diagnostic halt.
fn check_watchdog() {
    let current_tick = TICKS.load(Ordering::Relaxed);
    let current_tsc = crate::kernel::cpu::read_tsc();
    
    let mut state = WATCHDOG_STATE.lock();
    if current_tick != state.0 {
        // Heartbeat is healthy
        state.0 = current_tick;
        state.1 = current_tsc;
        state.2 = 0; // Reset retry count
    } else {
        // TICKS is frozen. Check TSC delta (assuming ~2GHz, 5 billion cycles is ~2.5 seconds)
        if current_tsc > state.1.wrapping_add(2_000_000_000) {
            state.2 += 1;
            if state.2 <= 3 {
                crate::serial_println!("[WATCHDOG] Heartbeat frozen. Attempting soft recovery {}/3...", state.2);
                // Attempt to re-initialize PIT frequency
                crate::drivers::pit::set_frequency(1000);
                // Reset TSC anchor to give the new configuration a window to work
                state.1 = current_tsc;
            } else {
                drop(state);
                show_boot_error("Watchdog Timeout: System heartbeat (TICKS) stopped responding after 3 recovery attempts.");
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BootMilestone {
    Interrupts,
    Mouse,
    Keyboard,
    InputActive,
    Acpi,
    CpuInfo,
    Gui,
}

impl BootMilestone {
    pub fn message(&self) -> &'static str {
        match self {
            BootMilestone::Interrupts => "[OK] Interrupts Initialized",
            BootMilestone::Mouse => "[OK] Mouse Driver Initialized",
            BootMilestone::Keyboard => "[OK] Keyboard Driver Initialized",
            BootMilestone::InputActive => "[OK] Input Drivers Active",
            BootMilestone::Acpi => "[OK] ACPI Initialized",
            BootMilestone::CpuInfo => "[OK] CPU Info Detected",
            BootMilestone::Gui => "[OK] Initializing GUI...",
        }
    }
}

fn draw_boot_screen() {   
    let title = "NebulaOS";
    let (width, height) = if let Some(info) = FRAMEBUFFER.lock().info.as_ref() { (info.width, info.height) } else { return };

    {
        let mut fb = FRAMEBUFFER.lock();
        fb.clear(0x00_000033); // Initial dark blue background
        fb.present();
    }

    let x_title = (width / 2).saturating_sub((title.len() * 8) / 2);
    let y_title = (height / 2).saturating_sub(16 / 2);
    let x_version = (width / 2).saturating_sub((VERSION.len() * 8) / 2);
    let y_version = y_title + 20;

    // Fade-in the title at ~60 FPS
    for step in 0..=32u32 {
        let start_tick = TICKS.load(Ordering::Relaxed);
        {
            let mut fb = FRAMEBUFFER.lock();
            let color = gui::interpolate_color(0x00_000033, 0x00_FFFFFF, step, 32);
            let version_color = gui::interpolate_color(0x00_000033, 0x00_888888, step, 32); // Slightly dimmer gray for version
            font::draw_string(&mut fb, x_title as isize, y_title as isize, title, color, None);
            font::draw_string(&mut fb, x_version as isize, y_version as isize, VERSION, version_color, None);
            
            // Present a rectangle covering both strings
            let min_x = x_title.min(x_version);
            let max_w = (title.len().max(VERSION.len())) * 8;
            fb.present_rect(min_x, y_title, max_w, 40);
        }

        // Wait for next frame (1000ms / 60fps = ~16ms)
        let frame_timeout_tsc = crate::kernel::cpu::read_tsc().wrapping_add(10_000_000);
        while TICKS.load(Ordering::Relaxed).wrapping_sub(start_tick) < 16 { 
            unsafe { asm!("pause"); }
            check_watchdog();
            // Soft timeout: If 16ms of ticks don't pass but 10M cycles do, the PIT is likely stuck
            if crate::kernel::cpu::read_tsc() > frame_timeout_tsc { break; }
        }
    }
}

fn add_boot_status(milestone: BootMilestone) {
    let line = BOOT_STATUS_LINE.fetch_add(1, Ordering::Relaxed);
    let mut fb = FRAMEBUFFER.lock();
    let x = 20;
    // Start printing status messages
    let y = 20 + (line * 20);
    font::draw_string(&mut fb, x as isize, y as isize, milestone.message(), 0x00_CCCCCC, None); // Light gray
    fb.present_rect(x, y as usize, 400, 20);
    drop(fb);

    // Intentional delay to make the log readable (50ms per entry) 
    let start_wait = TICKS.load(Ordering::Relaxed);
    let wait_timeout_tsc = crate::kernel::cpu::read_tsc().wrapping_add(20_000_000);
    while TICKS.load(Ordering::Relaxed).wrapping_sub(start_wait) < 50 { 
        unsafe { asm!("pause"); }
        check_watchdog();
        if crate::kernel::cpu::read_tsc() > wait_timeout_tsc { break; }
    }
}

/// Displays a critical error screen during the boot process and halts.
fn show_boot_error(message: &str) -> ! {
    crate::serial_println!("\nBOOT ERROR: {}", message);
    
    // Check if the framebuffer is initialized and has a draw buffer to draw the error screen
    unsafe { FRAMEBUFFER.force_unlock(); }
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
    // Perform fade-out in 32 steps at ~60 FPS
    for step in (0..=32u32).rev() {
        let start_tick = TICKS.load(Ordering::Relaxed);
        {
            let mut fb = FRAMEBUFFER.lock();
            gui::fade_buffer(&mut fb, step, 32);
            fb.present();
        }
        while TICKS.load(Ordering::Relaxed) < start_tick + 16 { unsafe { asm!("pause"); } }
    }
}

/// Background task that handles the animated loading spinner during boot.
pub extern "C" fn boot_animation_task() {
    while BOOT_ANIMATION_RUNNING.load(Ordering::Relaxed) {
        {
            let mut fb = FRAMEBUFFER.lock();
            let dims = fb.info.as_ref().map(|i| (i.width, i.height));
            
            if let Some((width, height)) = dims {
                let sx = (width / 2) as isize;
                let sy = (height / 2) as isize + 60;
                gui::draw_loading_spinner(&mut fb, sx, sy, Rect { x: 0, y: 0, width, height });
                fb.present_rect((sx - 40) as usize, (sy - 40) as usize, 80, 100);
            }
        }

        // Target 30 FPS for the background spinner
        unsafe { asm!("int 0x80", in("eax") 5usize, in("ebx") 33usize); }
    }
    // Task completed, yield forever until reaped
    loop { unsafe { asm!("int 0x80", in("eax") 0usize); } }
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
    } else {
        show_boot_error("No framebuffer information found!");
    }
    
    // Initialize GDT and TSS
    gdt::init();

    // Initialize IDT (but do not enable interrupts yet)
    if let Err(e) = interrupts::init() {
        show_boot_error(e);
    }

    // Enable interrupts now so we can use timer-based FPS for the splash screen
    interrupts::enable_interrupts();
    crate::serial_println!("[KERNEL] Interrupts and Ticks active.");

    // Initialize Watchdog with current state
    {
        let mut state = WATCHDOG_STATE.lock();
        state.0 = TICKS.load(Ordering::Relaxed);
        state.1 = cpu::read_tsc();
        state.2 = 0;
    }

    // Show splash screen with fade-in
    draw_boot_screen();

    // Start the background animation task for the spinner
    BOOT_ANIMATION_RUNNING.store(true, Ordering::Relaxed);
    let boot_task_entry = boot_animation_task as usize;
    crate::kernel::process::SCHEDULER.lock().add_task(boot_task_entry, 15);

    add_boot_status(BootMilestone::Interrupts); 

    // Initialize the mouse driver (polls for ACKs, so interrupts must be disabled)
    if let Err(e) = crate::drivers::mouse::initialize() {
        show_boot_error(e);
    } else {
        add_boot_status(BootMilestone::Mouse);
    }

    // Initialize the keyboard driver
    if let Err(e) = crate::drivers::keyboard::init() {
        show_boot_error(e);
    } else {
        add_boot_status(BootMilestone::Keyboard);
    }
    add_boot_status(BootMilestone::InputActive);

    // Initialize ACPI
    acpi::init();
    add_boot_status(BootMilestone::Acpi);

    // Initialize CPU Info detection (CPUID)
    cpu::init();
    add_boot_status(BootMilestone::CpuInfo);

    // NOTE: The old text-mode GUI has been disabled.
    // The next step is to build a new GUI that draws to the framebuffer.
    add_boot_status(BootMilestone::Gui);
    
    // Initialize localisation before GUI
    crate::userspace::localisation::init();
    
    // Stop the background animation before the final transition
    BOOT_ANIMATION_RUNNING.store(false, Ordering::Relaxed);

    // Perform final transition effect before starting the GUI
    fade_out_boot_screen();

    crate::userspace::gui::init();
    
    // Spawn the dedicated Render Task to offload blitting from this input loop
    let render_entry = crate::userspace::gui::window_manager::WindowManager::render_loop as usize;
    crate::kernel::process::SCHEDULER.lock().add_task(render_entry, 15);

    // Halt loop (The scheduler will hijack execution on the next timer tick)
    loop {
        crate::userspace::gui::update();
        check_watchdog();
        
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
