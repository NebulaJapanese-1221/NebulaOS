#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[path = "../drivers/mod.rs"]
mod drivers;
#[path = "../memory/mod.rs"]
mod memory;
#[path = "../gdt.rs"]
mod gdt;
#[path = "../tss.rs"]
mod tss;
#[path = "../idt.rs"]
mod idt;

use core::panic::PanicInfo;
use core::arch::asm;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn kernel_entry() -> ! {
    // Initialize GDT and segments
    gdt::init();
    drivers::serial::init();
    drivers::serial::write_string("NebulaOS Kernel Booting...\r\n");

    // Initialize TSS with initial kernel stack
    let initial_stack: u32 = 0x90000;
    tss::init(initial_stack);

    // Initialize memory (frame allocator)
    memory::init();
    drivers::serial::write_string("Memory manager initialized.\r\n");

    // Initialize paging (virtual memory)
    drivers::paging::init();
    drivers::serial::write_string("Paging initialized.\r\n");

    // Initialize heap allocator
    drivers::allocator::init();
    drivers::serial::write_string("Heap allocator initialized.\r\n");

    // Initialize IDT with ISR stubs
    idt::init();
    drivers::serial::write_string("IDT initialized.\r\n");

    // Remap PIC
    drivers::pic::remap(0x20, 0x28);
    drivers::serial::write_string("PIC remapped.\r\n");

    // Initialize PIT timer (100 Hz)
    drivers::pit::init(100);
    drivers::serial::write_string("PIT initialized.\r\n");

    // Initialize VGA text mode
    drivers::vga::init();
    drivers::vga::clear_screen();

    drivers::print("NebulaOS GUI OS v0.1.0\r\n");
    drivers::print("========================================\r\n");

    // Initialize keyboard
    drivers::keyboard::init();
    drivers::serial::write_string("Keyboard initialized.\r\n");

    // Initialize mouse
    drivers::mouse::init();
    drivers::serial::write_string("Mouse initialized.\r\n");

    // Initialize VESA graphics
    if drivers::vesa::init() {
        drivers::serial::write_string("VESA graphics mode initialized.\r\n");
        let fb = drivers::vesa::get_fb().unwrap();
        drivers::serial::write_string("Resolution: ");
        drivers::serial::write_dec(fb.width as u32);
        drivers::serial::write_string("x");
        drivers::serial::write_dec(fb.height as u32);
        drivers::serial::write_string("x");
        drivers::serial::write_dec(fb.bpp as u32);
        drivers::serial::write_string("\r\n");
    } else {
        drivers::serial::write_string("VESA not available, using text mode.\r\n");
    }

    drivers::vesa::clear_screen(0x002040);
    drivers::vesa::draw_rect(50, 50, 200, 150, 0x00FF00);
    drivers::vesa::draw_rect(300, 150, 250, 200, 0x0000FF);
    drivers::vesa::draw_rect(550, 200, 150, 150, 0xFF0000);

    // Unmask timer and keyboard IRQs
    drivers::pic::unmask_irq(0); // Timer
    drivers::pic::unmask_irq(1); // Keyboard

    // Initialize process management
    drivers::process::init();

    // Add init process
    drivers::process::add_process(init_process as u32);

    // Enable scheduler
    drivers::process::enable_scheduler();

    drivers::serial::write_string("GUI ready.\r\n");

    // Enable interrupts
    unsafe {
        asm!("sti");
    }

    // Main loop - the timer IRQ will handle context switching
    loop {
        drivers::mouse::poll();
        drivers::keyboard::poll();

        for _ in 0..100000 {
            core::hint::spin_loop();
        }
    }
}

fn init_process() -> ! {
    drivers::serial::write_string("Init process started\r\n");
    loop {
        core::hint::spin_loop();
    }
}
