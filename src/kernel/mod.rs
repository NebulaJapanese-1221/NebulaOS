#[cfg(not(test))]
use core::panic::PanicInfo;
use core::arch::asm; 
use crate::drivers::framebuffer::FRAMEBUFFER;
use crate::userspace::fonts::font;
use crate::userspace::gui::{self, Rect};
use core::sync::atomic::{AtomicUsize, AtomicU32, Ordering};
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
pub mod symbols;
pub mod paging;

pub const VERSION: &str = "0.0.3-dev2";

// CPU Feature Flags
pub const FEATURE_SSE: u32 = 1 << 0;
pub const FEATURE_SSE2: u32 = 1 << 1;
pub const FEATURE_SSE3: u32 = 1 << 2;
pub const FEATURE_SSSE3: u32 = 1 << 3;
pub const FEATURE_SSE4_1: u32 = 1 << 4;
pub const FEATURE_SSE4_2: u32 = 1 << 5;
pub const FEATURE_AVX: u32 = 1 << 6;
pub const FEATURE_FPU: u32 = 1 << 7;

pub struct KernelConfig {
    pub tsc_frequency: AtomicU32,
    pub total_memory: AtomicUsize,
    pub cpu_cores: AtomicUsize,
    pub features: AtomicU32,
}

impl KernelConfig {
    pub fn has_feature(&self, feature: u32) -> bool {
        (self.features.load(Ordering::Relaxed) & feature) != 0
    }
}

pub static CONFIG: KernelConfig = KernelConfig {
    tsc_frequency: AtomicU32::new(2_000_000_000), // Default 2GHz
    total_memory: AtomicUsize::new(0),
    cpu_cores: AtomicUsize::new(1),
    features: AtomicU32::new(0),
};

static BOOT_ANIMATION_RUNNING: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
static BOOT_STATUS_LINE: AtomicUsize = AtomicUsize::new(2);
/// (last_tick, last_tick_tsc, pit_retries, last_gpu_val, last_gpu_tsc)
static WATCHDOG_STATE: Mutex<(usize, u64, u32, usize, u64)> = Mutex::new((0, 0, 0, 0, 0));

/// Monitors the health of the kernel heartbeat.
/// If TICKS stays the same while TSC continues to increase significantly, triggers a diagnostic halt.
fn check_watchdog() {
    let current_tick = TICKS.load(Ordering::Relaxed);
    let current_gpu = crate::drivers::framebuffer::GPU_HEARTBEAT.load(Ordering::Relaxed);
    let current_tsc = crate::kernel::cpu::read_tsc();
    let tsc_freq = CONFIG.tsc_frequency.load(Ordering::Relaxed) as u64;
    
    let mut state = WATCHDOG_STATE.lock();
    
    // If the watchdog hasn't been initialized with a real TSC value yet, do it now
    if state.1 == 0 {
        state.0 = current_tick;
        state.1 = current_tsc;
        return;
    }

    if current_tick != state.0 {
        // Heartbeat is healthy
        state.0 = current_tick;
        state.1 = current_tsc;
        state.2 = 0; // Reset retry count
    } else {
        // TICKS is frozen. Check TSC delta (1 second threshold)
        if current_tsc > state.1.wrapping_add(tsc_freq) {
            state.2 += 1;
            if state.2 <= 3 {
                crate::log_warn!("Watchdog: Heartbeat frozen. Attempting soft recovery {}/3...", state.2);
                // Handle the PIT Result to ensure hardware reset was successful
                if let Err(e) = crate::drivers::pit::set_frequency(1000) {
                    crate::log_error!("Watchdog: Critical Error: PIT Recovery failed: {}", e);
                }
                // Reset TSC anchor to give the new configuration a window to work
                state.1 = current_tsc;
            } else {
                drop(state);
                show_boot_error("Watchdog Timeout: System heartbeat (TICKS) stopped responding after 3 recovery attempts.");
            }
        }
    }

    // GPU Render Task Watchdog
    if crate::drivers::framebuffer::RENDER_TASK_ACTIVE.load(Ordering::Relaxed) {
        if state.4 == 0 {
            // Initialize GPU tracking state
            state.3 = current_gpu;
            state.4 = current_tsc;
        } else if current_gpu != state.3 {
            state.3 = current_gpu;
            state.4 = current_tsc;
        } else if current_tsc > state.4.wrapping_add(tsc_freq) {
            // RSOD if GPU task hasn't checked in for ~1 second while marked active
            drop(state);
            show_boot_error("GPU Watchdog: The background rendering task has stopped responding (Heartbeat Timeout).");
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

    let x_title = (width / 2).saturating_sub((title.len() * 8) / 2);
    let y_title = (height / 2).saturating_sub(16 / 2);
    let x_version = (width / 2).saturating_sub((VERSION.len() * 8) / 2);
    let y_version = y_title + 20;

    let min_x = x_title.min(x_version);
    let max_w = (title.len().max(VERSION.len())) * 8;

    // Fade-in the title at ~60 FPS
    for step in 0..=32u32 {
        let start_tick = TICKS.load(Ordering::Relaxed);
        {
            let mut fb = FRAMEBUFFER.lock();
            // Clear only the title area to the background blue to prevent pixel accumulation
            gui::draw_rect(&mut fb, min_x as isize, y_title as isize, max_w, 40, 0x00_000033, None);

            let color = gui::interpolate_color(0xFF_000033, 0xFF_FFFFFF, step, 32);
            let version_color = gui::interpolate_color(0xFF_000033, 0xFF_888888, step, 32);
            font::draw_string(&mut fb, x_title as isize, y_title as isize, title, color, None);
            font::draw_string(&mut fb, x_version as isize, y_version as isize, VERSION, version_color, None);
            
            fb.present_rect(min_x, y_title, max_w, 40);
        }

        // Dynamic wait: Base 8ms + measured VRAM latency
        let frame_timeout_tsc = crate::kernel::cpu::read_tsc().wrapping_add(CONFIG.tsc_frequency.load(Ordering::Relaxed) as u64 / 100);
        let latency_ms = crate::drivers::framebuffer::BLIT_LATENCY.load(Ordering::Relaxed) / 2_000_000;
        let wait_ticks = (8 + latency_ms).min(32);

        while TICKS.load(Ordering::Relaxed).wrapping_sub(start_tick) < wait_ticks { 
            unsafe { asm!("int 0x80", in("eax") 0usize); } // Yield to allow blit task to run
            check_watchdog();
            if crate::kernel::cpu::read_tsc() > frame_timeout_tsc { break; }
        }
    }
}

fn add_boot_status(milestone: BootMilestone) {
    crate::log_info!("Boot Status: {}", milestone.message());
    let line = BOOT_STATUS_LINE.fetch_add(1, Ordering::Relaxed);
    let mut fb = FRAMEBUFFER.lock();
    let x = 20;
    // Start printing status messages
    let y = 20 + (line * 20);
    font::draw_string(&mut fb, x as isize, y as isize, milestone.message(), 0xFF_CCCCCC, None); 
    fb.present_rect(x, y as usize, 400, 20);
    drop(fb);

    // Intentional delay to make the log readable (10ms per entry) 
    let start_wait = TICKS.load(Ordering::Relaxed);
    let wait_timeout_tsc = crate::kernel::cpu::read_tsc().wrapping_add(CONFIG.tsc_frequency.load(Ordering::Relaxed) as u64 / 50);
    while TICKS.load(Ordering::Relaxed).wrapping_sub(start_wait) < 10 { 
        unsafe { asm!("int 0x80", in("eax") 0usize); } // Yield
        check_watchdog();
        if crate::kernel::cpu::read_tsc() > wait_timeout_tsc { break; }
    }
}

/// Displays a critical error screen during the boot process and halts.
fn show_boot_error(message: &str) -> ! {
    // Revert to direct writes for the error report
    crate::drivers::serial::SERIAL_TASK_ACTIVE.store(false, Ordering::SeqCst);
    crate::drivers::framebuffer::RENDER_TASK_ACTIVE.store(false, Ordering::SeqCst);

    crate::log_error!("BOOT ERROR: {}", message);
    process::print_kernel_trace();
    
    // Force flush the serial queue directly to hardware since the background task is likely not running
    while let Some(b) = crate::drivers::serial::SERIAL_OUT_QUEUE.pop_byte() {
        crate::drivers::serial::SERIAL1.lock().write_byte(b);
    }

    
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
        let timeout = crate::kernel::cpu::read_tsc().wrapping_add(10_000_000);
        while TICKS.load(Ordering::Relaxed).wrapping_sub(start_tick) < 16 { 
            // Yield to allow the blit task (Task 1) to actually perform the fade
            unsafe { asm!("int 0x80", in("eax") 0usize); } 
            if crate::kernel::cpu::read_tsc() > timeout { break; }
        }
    }
}

/// Background task that flushes the framebuffer intermediate buffer to VRAM.
pub extern "C" fn framebuffer_blit_task() {
    // Signal that the background task is now active
    crate::drivers::framebuffer::RENDER_TASK_ACTIVE.store(true, Ordering::SeqCst);

    let mut frame_count = 0;
    let mut last_fps_check = crate::kernel::cpu::read_tsc();

    loop {
        // Update heartbeat to signal this task is alive
        crate::drivers::framebuffer::GPU_HEARTBEAT.fetch_add(1, Ordering::Relaxed);

        // Kernel Watchdog: Verify VRAM metadata sanity before attempting a blit
        {
            let fb = crate::drivers::framebuffer::FRAMEBUFFER.lock();
            if let Some(info) = fb.info {
                // Detect corruption: Address shouldn't be null, and dimensions shouldn't be impossible
                if info.address == 0 || info.width == 0 || info.height == 0 || info.width > 8192 {
                    show_boot_error("VRAM Watchdog: Framebuffer metadata is invalid or corrupted.");
                }
            } else {
                show_boot_error("VRAM Watchdog: Framebuffer info is missing during active render task.");
            }
        }

        let start = crate::kernel::cpu::read_tsc();
        let blitted = crate::drivers::framebuffer::FRAMEBUFFER.lock().blit_to_vram();
        let end = crate::kernel::cpu::read_tsc();

        if blitted {
            let latency = end.wrapping_sub(start) as usize;
            frame_count += 1;
            crate::drivers::framebuffer::BLIT_LATENCY.store(latency, Ordering::Relaxed);

            // Performance Watchdog: Detect Graphics Bus Stalls
            // If a single blit takes > 1 second, the VRAM is unresponsive.
            if latency as u64 > CONFIG.tsc_frequency.load(Ordering::Relaxed) as u64 {
                show_boot_error("VRAM Unresponsive: Hardware bus timeout detected during VRAM blit operation.");
            }
        }

        let now = crate::kernel::cpu::read_tsc();
        // Update FPS counter every ~1 second
        if now.wrapping_sub(last_fps_check) >= CONFIG.tsc_frequency.load(Ordering::Relaxed) as u64 {
            crate::drivers::framebuffer::FPS.store(frame_count, Ordering::Relaxed);
            frame_count = 0;
            last_fps_check = now;
        }

        // Yield to allow other tasks to perform drawing operations
        unsafe { asm!("int 0x80", in("eax") 0usize); }
    }
}

/// Background task that flushes the serial buffer to the actual hardware.
pub extern "C" fn serial_output_task() {
    // Signal that the background task is now active and consuming the queue
    crate::drivers::serial::SERIAL_TASK_ACTIVE.store(true, Ordering::SeqCst);

    loop {
        let byte = crate::drivers::serial::SERIAL_OUT_QUEUE.pop_byte();
        if let Some(b) = byte {
            crate::drivers::serial::SERIAL1.lock().write_byte(b);
        } else {
            // No data to flush, yield to other tasks
            // Using Syscall 5 (Sleep 1ms) to prevent pinning the CPU in a tight loop
            unsafe { asm!("int 0x80", in("eax") 5usize, in("ebx") 1usize); }
        }
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

        // Adjust spin speed based on measured frequency
        let latency_cycles = crate::drivers::framebuffer::BLIT_LATENCY.load(Ordering::Relaxed);
        let freq = CONFIG.tsc_frequency.load(Ordering::Relaxed) as usize;
        let latency_ms = if freq > 0 { latency_cycles / (freq / 1000) } else { 0 };

        // Target 30 FPS (33ms) + current hardware overhead. Cap at 200ms (5 FPS).
        let dynamic_sleep = (33 + latency_ms).min(200);

        unsafe { asm!("int 0x80", in("eax") 5usize, in("ebx") dynamic_sleep); }
    }
}

// Entry point called by boot assembly
#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info_ptr: usize) -> ! {
    // PHASE 1: Minimal Environment
    unsafe { asm!("cli", options(nomem, nostack)); }
    crate::drivers::serial::SERIAL1.lock().init(); 

    // PHASE 2: Core Memory & Graphics
    let heap_region = allocator::find_heap_region(multiboot_info_ptr);
    let fb_info_opt = multiboot::framebuffer_info(multiboot_info_ptr);

    if let Some((heap_start, heap_size)) = heap_region {
        unsafe { allocator::ALLOCATOR.lock().init(heap_start as *mut u8, heap_size); }
        CONFIG.total_memory.store(heap_size, Ordering::Relaxed);
        
        // Initialize Paging after the heap is ready
        paging::init();

        // Register the current execution as the Boot Task before adding others
        let current_esp: usize;
        unsafe { core::arch::asm!("mov {}, esp", out(reg) current_esp); }
        process::SCHEDULER.lock().init_boot_task(current_esp);
    } else { show_boot_error("Could not find a suitable heap region!"); }

    if let Some(fb_info) = fb_info_opt {
        let mut fb = crate::drivers::framebuffer::FRAMEBUFFER.lock();
        fb.init(fb_info);
        fb.clear(0x00_000033); // Establish the boot background immediately
        fb.present();
    } else {
        show_boot_error("No framebuffer information found!");
    }
    
    // PHASE 3: System Tables & Interrupts
    gdt::init();
    if let Err(e) = interrupts::init() { show_boot_error(e); }
    
    // Start critical background services before showing the logo.
    // This ensures that VRAM blitting and serial logging are non-blocking.
    crate::kernel::process::SCHEDULER.lock().add_task(framebuffer_blit_task as *const () as usize, 20, None); // High priority
    crate::kernel::process::SCHEDULER.lock().add_task(serial_output_task as *const () as usize, 20, None); // Boosted for boot

    // Enable interrupts: Background tasks and PIT heartbeat (TICKS) start now.
    interrupts::enable_interrupts();
    cpu::calibrate_tsc();
    add_boot_status(BootMilestone::Interrupts); 

    // PHASE 3.5: Initialize Idle Task
    let idle_entry = crate::kernel::process::idle_task as *const () as usize;
    crate::kernel::process::SCHEDULER.lock().add_task(idle_entry, 0, None);

    // PHASE 4: Visual Startup
    draw_boot_screen();
    BOOT_ANIMATION_RUNNING.store(true, Ordering::Relaxed);
    crate::kernel::process::SCHEDULER.lock().add_task(boot_animation_task as *const () as usize, 20, None);

    // PHASE 5: Peripheral Drivers
    if let Err(e) = crate::drivers::mouse::initialize() { show_boot_error(e); }
    else { add_boot_status(BootMilestone::Mouse); }

    if let Err(e) = crate::drivers::keyboard::init() { show_boot_error(e); }
    else { add_boot_status(BootMilestone::Keyboard); }

    add_boot_status(BootMilestone::InputActive);

    // PHASE 6: Subsystems
    acpi::init();
    add_boot_status(BootMilestone::Acpi);

    cpu::init();
    add_boot_status(BootMilestone::CpuInfo);

    add_boot_status(BootMilestone::Gui);
    crate::userspace::localisation::init();

    // Access localization via CURRENT_LOCALE lock
    let locale_guard = crate::userspace::localisation::CURRENT_LOCALE.lock();
    let loc = locale_guard.as_ref().unwrap();
    crate::log_info!("Localisation: {}: NebulaOS {}", loc.info_kernel(), VERSION);
    crate::log_info!("Localisation: {}: i386-unknown-none", loc.info_target());
    
    // PHASE 7: Desktop Transition
    BOOT_ANIMATION_RUNNING.store(false, Ordering::Relaxed);
    fade_out_boot_screen();

    // Transition scheduler to allow User-level tasks (Priority < 20) to run
    process::SCHEDULER.lock().mode = process::SchedulerMode::Running;

    // Lower boot task priority now that background services are stable
    process::SCHEDULER.lock().set_task_priority(0, 10);

    crate::userspace::gui::init();
    
    let render_entry = crate::userspace::gui::window_manager::WindowManager::render_loop as *const () as usize;
    crate::kernel::process::SCHEDULER.lock().add_task(render_entry, 15, None);

    // PHASE 8: Main Event Loop
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
    // Ensure panic messages bypass the buffer and go straight to hardware
    crate::drivers::serial::SERIAL_TASK_ACTIVE.store(false, Ordering::SeqCst);
    crate::drivers::framebuffer::RENDER_TASK_ACTIVE.store(false, Ordering::SeqCst);

    // Disable interrupts to prevent further issues during panic
    unsafe { core::arch::asm!("cli") };

    crate::log_error!("KERNEL PANIC: {}", info);
    unsafe { exceptions::print_stack_trace(); }
    process::print_kernel_trace();

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
