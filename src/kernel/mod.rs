use core::arch::asm; 
use crate::drivers::framebuffer::FRAMEBUFFER;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
pub mod boot;
pub mod panic;
pub mod jit;

pub const VERSION: &str = "0.0.3";

pub static TOTAL_MEMORY: AtomicUsize = AtomicUsize::new(0);
pub static CPU_CORES: AtomicUsize = AtomicUsize::new(1);
pub static IS_SAFE_MODE: AtomicBool = AtomicBool::new(false);

/// Executes a buffer of native machine code. 
/// This is the entry point for JIT-compiled architecture-independent logic.
pub unsafe fn execute_jit_code(code: &[u8]) {
    // Ensure the memory is marked as executable in the page tables
    paging::make_executable(code.as_ptr() as usize, code.len());

    let code_ptr = code.as_ptr() as *const ();
    let func: extern "C" fn() = core::mem::transmute(code_ptr);
    func();
}

// Entry point called by boot assembly
#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info_ptr: usize) -> ! {
    // It's crucial to disable interrupts before initializing the allocator
    unsafe { asm!("cli", options(nomem, nostack)); }

    // Initialize Serial Port
    crate::drivers::serial::SERIAL1.lock().init(); 
    crate::serial_println!("NebulaOS is starting up...");

    // Parse Multiboot command line for "safemode"
    unsafe {
        // Pointer math using usize for multiarch safety
        let cmdline_ptr = *((multiboot_info_ptr + 16) as *const usize);
        if cmdline_ptr != 0 {
            let ptr = cmdline_ptr as *const u8;
            let mut len = 0;
            // Scan for null terminator with a safety limit
            while *ptr.add(len) != 0 && len < 256 { len += 1; }
            let cmdline = core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len));
            if cmdline.contains("safemode") {
                IS_SAFE_MODE.store(true, Ordering::Relaxed);
            }
        }
    }

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
            let (width, height) = (fb_info.1, fb_info.2);
            let mut fb = FRAMEBUFFER.lock();
            fb.clear(0x00_050515); // Full clear only once on entry
            boot::draw_boot_screen_content(&mut fb, "NebulaOS is starting up...", 0);
            
            // Immediately present the boot screen content without the slow fade-in animation
            fb.present_rect(width / 2 - 210, (height / 2).saturating_sub(15), 420, 150);
        }
    } else {
        crate::serial_println!("ERROR: No framebuffer information found!");
    }
    
    // Initialize GDT and TSS
    gdt::init();
    boot::add_boot_status("System Core Initializing...", 15);

    // Set PIT frequency for scheduler (e.g., 1000 Hz)
    crate::drivers::pit::set_frequency(1000);

    // Initialize IDT (but do not enable interrupts yet)
    interrupts::init();
    boot::add_boot_status("Setting Up Interrupts...", 25); 

    // Initialize the mouse driver (polls for ACKs, so interrupts must be disabled)
    crate::drivers::mouse::initialize();
    boot::add_boot_status("Mouse Driver Initialized", 40);

    // Initialize the keyboard driver
    crate::drivers::keyboard::init();
    boot::add_boot_status("Keyboard Driver Initialized", 50);

    // Safe Mode Check: If Left Shift is held, enter Safe Mode
    if crate::drivers::keyboard::is_shift_pressed() {
        IS_SAFE_MODE.store(true, Ordering::Relaxed);
        boot::add_boot_status("ENTERING SAFE MODE...", 52);
    }

    // Initialize the brightness driver
    if !IS_SAFE_MODE.load(Ordering::Relaxed) {
        crate::drivers::brightness::BRIGHTNESS.lock().init();
        boot::add_boot_status("Brightness Driver Initialized", 60);
    }

    // Now it is safe to enable interrupts
    interrupts::enable_interrupts();
    boot::add_boot_status("Hardware Signals Active", 70);

    // Calibrate NOP loops while PIT is running for hardware-consistent delays
    cpu::calibrate_delay();

    // Initialize ACPI (Skipped in Safe Mode to prevent power-related hangs)
    if !IS_SAFE_MODE.load(Ordering::Relaxed) {
        acpi::init();
        boot::add_boot_status("Power Management Ready", 80);
    } else {
        boot::add_boot_status("ACPI Bypassed", 80);
    }

    // Initialize CPU Info detection (CPUID)
    cpu::init();
    boot::add_boot_status("System Discovery Complete", 95);

    boot::add_boot_status("Starting Desktop Environment...", 100);

    // Fade out boot screen
    {
        let mut fb = FRAMEBUFFER.lock();
        let (width, height) = match fb.info.as_ref() {
            Some(info) => (info.width, info.height),
            None => (800, 600),
        };
        // Skip the fade-out and present the final boot state before launching the GUI
        boot::BOOT_ANIM_RUNNING.store(false, Ordering::Relaxed); // Freeze the spinner
        boot::draw_boot_screen_content(&mut fb, "Starting Desktop Environment...", 100);
        fb.present_rect(width / 2 - 210, (height / 2).saturating_sub(15), 420, 150);
    }
    
    // Initialize localisation before GUI
    crate::userspace::localisation::init();
    
    crate::userspace::gui::init();
    
    // Notify user if Safe Mode is active via a non-blocking popup
    if IS_SAFE_MODE.load(Ordering::Relaxed) {
        crate::userspace::gui::push_system_error(
            crate::userspace::gui::ErrorLevel::Warning,
            "System Information",
            alloc::string::String::from("Safe Mode active: Power management and SMP disabled.")
        );
    }

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
