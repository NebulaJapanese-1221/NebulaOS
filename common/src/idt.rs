#![allow(unused_imports)]
use core::arch::asm;
use crate::stdint::*;
use crate::io;

pub const IDT_MAX_DESCRIPTORS: usize = 256;
pub const IRQ0: u32 = 32;
pub const IRQ8: u32 = 40;
pub const IRQ15: u32 = 47;

pub const IDT_FLAG_PRESENT: u8 = 0x80;
pub const IDT_FLAG_INTERRUPT: u8 = 0x0E;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    pub base_low: u16,
    pub sel: u16,
    pub ist: u8,
    pub flags: u8,
    pub base_mid: u16,
    pub base_high: u32,
    pub reserved: u32,
}

#[repr(C, packed)]
pub struct IdtPtr {
    pub limit: u16,
    pub base: u32,
}

pub type InterruptHandler = unsafe extern "C" fn(*mut Registers);

#[repr(C)]
pub struct Registers {
    pub int_no: u32,
    pub error_code: u32,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

pub static mut IDT: [IdtEntry; IDT_MAX_DESCRIPTORS] = [IdtEntry { base_low: 0, sel: 0, ist: 0, flags: 0, base_mid: 0, base_high: 0, reserved: 0 }; IDT_MAX_DESCRIPTORS];
pub static mut IDT_PTR: IdtPtr = IdtPtr { limit: 0, base: 0 };
pub static mut INTERRUPT_HANDLERS: [Option<InterruptHandler>; IDT_MAX_DESCRIPTORS] = [None; IDT_MAX_DESCRIPTORS];

extern "C" {
    static mut isr_table: [unsafe extern "C" fn(); 32];
    static mut irq_table: [unsafe extern "C" fn(); 16];
    static mut isr_double_fault: unsafe extern "C" fn();
    static mut isr_gpf: unsafe extern "C" fn();
    static mut isr_page_fault: unsafe extern "C" fn();
}

pub unsafe fn init_idt64() {
    IDT_PTR.limit = (core::mem::size_of::<[IdtEntry; IDT_MAX_DESCRIPTORS]>() - 1) as u16;
    IDT_PTR.base = &mut IDT as *mut _ as u32;

    for i in 0..IDT_MAX_DESCRIPTORS {
        INTERRUPT_HANDLERS[i] = Some(default_interrupt_handler);
    }

    // Standard ISRs (0-7, 9, 15-31)
    for i in 0..8 {
        set_gate64(i, isr_table[i] as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    }
    set_gate64(8, isr_double_fault as u64, 0x08, 1, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);  // Double fault with IST
    set_gate64(9, isr_table[9] as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    for i in 10..14 {
        set_gate64(i, isr_table[i] as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    }
    set_gate64(13, isr_gpf as u64, 0x08, 2, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);  // GPF with IST
    set_gate64(14, isr_page_fault as u64, 0x08, 3, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);  // Page fault with IST
    for i in 15..32 {
        set_gate64(i, isr_table[i] as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    }
    
    for i in 0..16 {
        set_gate64((IRQ0 as usize) + i, irq_table[i] as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    }
    for i in 48..IDT_MAX_DESCRIPTORS {
        set_gate64(i, default_interrupt_handler as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    }

    idt_flush64();
}

pub unsafe fn set_gate64(num: usize, base: u64, sel: u16, ist: u8, flags: u8) {
    IDT[num].base_low = (base & 0xFFFF) as u16;
    IDT[num].sel = sel;
    IDT[num].ist = ist;
    IDT[num].flags = flags;
    IDT[num].base_mid = ((base >> 16) & 0xFFFF) as u16;
    IDT[num].base_high = ((base >> 32) & 0xFFFF) as u32;
    IDT[num].reserved = 0;
}

pub unsafe fn idt_flush64() {
    asm!("lidt [{0}]", in(reg) &IDT_PTR, options(nomem, nostack));
}

pub unsafe fn register_interrupt_handler(n: usize, handler: Option<InterruptHandler>) {
    if n < IDT_MAX_DESCRIPTORS {
        INTERRUPT_HANDLERS[n] = handler;
    }
}

pub unsafe fn interrupt_handler(regs: *mut Registers) {
    let int_no = (*regs).int_no as usize;
    if int_no < IDT_MAX_DESCRIPTORS {
        if let Some(handler) = INTERRUPT_HANDLERS[int_no] {
            handler(regs);
        }
    }
}

pub unsafe extern "C" fn default_interrupt_handler(regs: *mut Registers) {
    let int_no = (*regs).int_no;
    if int_no >= IRQ0 as u32 && int_no <= IRQ15 as u32 {
        io::outb(0x20, 0x20);
        if int_no >= IRQ8 as u32 {
            io::outb(0xA0, 0x20);
        }
    }
}

// Exception handlers with error screen
pub unsafe extern "C" fn handle_double_fault(regs: *mut Registers) {
    show_error_screen(8, "DOUBLE FAULT", "A double fault occurred. The system will halt.", (*regs).error_code);
}

pub unsafe extern "C" fn handle_gpf(regs: *mut Registers) {
    show_error_screen(13, "GENERAL PROTECTION FAULT", "A general protection fault occurred.", (*regs).error_code);
}

pub unsafe extern "C" fn handle_page_fault(regs: *mut Registers) {
    let cr2: u64;
    asm!("mov {}, cr2", out(reg) cr2);
    show_page_fault_screen((*regs).error_code, cr2);
}

fn show_error_screen(vector: u32, title: &str, message: &str, error_code: u32) {
    use crate::vga;
    
    // Disable interrupts
    asm!("cli");
    
    // Set up error screen
    vga::init();
    vga::set_color(vga::VGA_COLOR_WHITE);
    vga::set_bg_color(vga::VGA_COLOR_RED);
    vga::clear();
    
    // Draw border
    for i in 0..80 {
        vga::putc(i, 0, b'=' as u8);
        vga::putc(i, 24, b'=' as u8);
    }
    for i in 0..25 {
        vga::putc(0, i, b'|' as u8);
        vga::putc(79, i, b'|' as u8);
    }
    
    // Print title
    vga::set_color(vga::VGA_COLOR_YELLOW);
    vga::set_bg_color(vga::VGA_COLOR_RED);
    vga::puts_at(10, 2, b"*** NEBULAOS EXCEPTION ***\0" as *const u8 as *const u8);
    
    vga::set_color(vga::VGA_COLOR_WHITE);
    vga::puts_at(10, 4, title.as_ptr() as *const u8);
    
    vga::puts_at(10, 6, message.as_ptr() as *const u8);
    
    // Print error code
    vga::puts_at(10, 8, b"Exception Vector: \0" as *const u8 as *const u8);
    print_hex_at(28, 8, vector as u64);
    
    vga::puts_at(10, 9, b"Error Code:       \0" as *const u8 as *const u8);
    print_hex_at(28, 9, error_code as u64);
    
    // Print register dump
    vga::puts_at(10, 11, b"Register Dump:\0" as *const u8 as *const u8);
    
    // Halt system
    vga::puts_at(10, 20, b"System halted. Press any key to reboot...\0" as *const u8 as *const u8);
    
    loop {
        asm!("hlt");
    }
}

fn show_page_fault_screen(error_code: u32, cr2: u64) {
    use crate::vga;
    
    asm!("cli");
    
    vga::init();
    vga::set_color(vga::VGA_COLOR_WHITE);
    vga::set_bg_color(vga::VGA_COLOR_RED);
    vga::clear();
    
    for i in 0..80 {
        vga::putc(i, 0, b'=' as u8);
        vga::putc(i, 24, b'=' as u8);
    }
    for i in 0..25 {
        vga::putc(0, i, b'|' as u8);
        vga::putc(79, i, b'|' as u8);
    }
    
    vga::set_color(vga::VGA_COLOR_YELLOW);
    vga::set_bg_color(vga::VGA_COLOR_RED);
    vga::puts_at(10, 2, b"*** NEBULAOS PAGE FAULT ***\0" as *const u8 as *const u8);
    
    vga::set_color(vga::VGA_COLOR_WHITE);
    vga::puts_at(10, 4, b"A page fault occurred.\0" as *const u8 as *const u8);
    
    vga::puts_at(10, 6, b"Faulting Address: \0" as *const u8 as *const u8);
    print_hex_at(28, 6, cr2);
    
    vga::puts_at(10, 7, b"Error Code:       \0" as *const u8 as *const u8);
    print_hex_at(28, 7, error_code as u64);
    
    // Decode error code
    vga::puts_at(10, 9, b"Error Details:\0" as *const u8 as *const u8);
    if error_code & 1 != 0 {
        vga::puts_at(12, 10, b"- Page not present\0" as *const u8 as *const u8);
    } else {
        vga::puts_at(12, 10, b"- Page protection violation\0" as *const u8 as *const u8);
    }
    if error_code & 2 != 0 {
        vga::puts_at(12, 11, b"- Write access\0" as *const u8 as *const u8);
    } else {
        vga::puts_at(12, 11, b"- Read access\0" as *const u8 as *const u8);
    }
    if error_code & 4 != 0 {
        vga::puts_at(12, 12, b"- User mode\0" as *const u8 as *const u8);
    } else {
        vga::puts_at(12, 12, b"- Kernel mode\0" as *const u8 as *const u8);
    }
    if error_code & 8 != 0 {
        vga::puts_at(12, 13, b"- Reserved bit set\0" as *const u8 as *const u8);
    }
    if error_code & 16 != 0 {
        vga::puts_at(12, 14, b"- Instruction fetch\0" as *const u8 as *const u8);
    }
    
    vga::puts_at(10, 20, b"System halted. Press any key to reboot...\0" as *const u8 as *const u8);
    
    loop {
        asm!("hlt");
    }
}

fn print_hex_at(x: usize, y: usize, mut val: u64) {
    use crate::vga;
    let mut buf = [0u8; 18];
    buf[0] = b'0';
    buf[1] = b'x';
    for i in (2..18).rev() {
        let digit = (val & 0xF) as u8;
        buf[i] = if digit < 10 { b'0' + digit } else { b'A' + (digit - 10) };
        val >>= 4;
    }
    vga::puts_at(x, y, buf.as_ptr());
}
