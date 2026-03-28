use core::arch::{asm, global_asm};
use core::mem::size_of;
use super::io;

// --- IDT Structures and functions ---

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    zero: u8,
    type_attr: u8,
    offset_high: u16,
}

impl IdtEntry {
    pub fn new(offset: u32, selector: u16, type_attr: u8) -> Self {
        IdtEntry {
            offset_low: (offset & 0xFFFF) as u16,
            selector,
            zero: 0,
            type_attr,
            offset_high: ((offset >> 16) & 0xFFFF) as u16,
        }
    }
}

// Helper function to set an IDT entry for a task gate
fn set_idt_task_gate(index: usize, tss_selector: u16) {
    unsafe {
        // For a task gate, the offset is not used.
        // The type_attr is 0x85 (Present, DPL=0, Type=5 for Task Gate)
        IDT[index] = IdtEntry::new(0, tss_selector, 0x85);
    }
}

// IDT Pointer


#[repr(C, packed)]
struct IdtPtr {
    limit: u16,
    base: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct InterruptStackFrame {
    pub instruction_pointer: u32,
    pub code_segment: u32,
    pub cpu_flags: u32,
}

// Helper function to set an IDT entry
fn set_idt_entry(index: usize, offset: u32, selector: u16, type_attr: u8) {
    unsafe {
        IDT[index] = IdtEntry::new(offset, selector, type_attr);
    }
}

// Storage for the IDT
//IDT must not be on the stack, so it is static and mutable
//All access to this must be done in an unsafe block
static mut IDT: [IdtEntry; 256] = [IdtEntry {
    offset_low: 0, selector: 0, zero: 0, type_attr: 0, offset_high: 0
}; 256];


// --- Initialization ---


//This initializes the IDT table
//This function is unsafe, since it modifies global state with an unsafe static mutable
//It sets up all the interrupt handlers, but does not enable them, since that requires memory initialization
//Most of the code here is setting up functions to be called.
//These handlers will be dispatched to when the interrupts are fired.
//init_pics is called to set up the interrupt controller
pub fn init() {
    unsafe {
        // 1. Remap the PIC
        init_pics();

        // Exception Handlers
        set_idt_entry(0, crate::kernel::exceptions::divide_by_zero_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(1, crate::kernel::exceptions::debug_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(2, crate::kernel::exceptions::non_maskable_interrupt_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(3, crate::kernel::exceptions::breakpoint_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(4, crate::kernel::exceptions::overflow_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(5, crate::kernel::exceptions::bound_range_exceeded_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(6, crate::kernel::exceptions::invalid_opcode_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(7, crate::kernel::exceptions::device_not_available_handler as *const () as u32, 0x08, 0x8E);
        set_idt_task_gate(8, crate::kernel::gdt::DOUBLE_FAULT_TSS_SELECTOR);
        set_idt_entry(10, crate::kernel::exceptions::invalid_tss_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(11, crate::kernel::exceptions::segment_not_present_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(12, crate::kernel::exceptions::stack_segment_fault_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(13, crate::kernel::exceptions::gpf_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(14, crate::kernel::exceptions::page_fault_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(16, crate::kernel::exceptions::x87_floating_point_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(17, crate::kernel::exceptions::alignment_check_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(18, crate::kernel::exceptions::machine_check_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(19, crate::kernel::exceptions::simd_floating_point_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(20, crate::kernel::exceptions::virtualization_handler as *const () as u32, 0x08, 0x8E);
        set_idt_entry(30, crate::kernel::exceptions::security_exception_handler as *const () as u32, 0x08, 0x8E);

        // 2. Add Timer Handler (IRQ 0 is mapped to index 32)
        let func_addr = timer_handler as *const () as u32;
        // Selector 0x08 is usually the Code Segment in Multiboot
        // Type 0x8E = Present, Ring0, 32-bit Interrupt Gate
        set_idt_entry(32, func_addr, 0x08, 0x8E);

        // 3. Add Keyboard Handler (IRQ 1 is mapped to index 33)
        let func_addr = crate::drivers::keyboard::interrupt_handler as *const () as u32;
        set_idt_entry(33, func_addr, 0x08, 0x8E);

        // 4. Add Mouse Handler (IRQ 12 is mapped to index 44)
        let func_addr = crate::drivers::mouse::interrupt_handler as *const () as u32;
        set_idt_entry(44, func_addr, 0x08, 0x8E);

        // 5. Add Syscall Handler (0x80)
        // Type 0xEE = Present, Ring3 (DPL=3), 32-bit Interrupt Gate (allows calling from userspace)
        set_idt_entry(0x80, crate::kernel::syscall::syscall_handler as *const () as u32, 0x08, 0xEE);

        // 6. Load IDT
        let idt_ptr = IdtPtr {
            limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: core::ptr::addr_of!(IDT) as u32,
        };
        asm!("lidt [{}]", in(reg) &idt_ptr, options(readonly, nostack, preserves_flags));
    }
}

pub fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

// --- 8259 PIC Remapping ---

unsafe fn init_pics() {
    // Save masks
    let _m1 = io::inb(0x21);
    let _m2 = io::inb(0xA1);

    // Start initialization sequence
    io::outb(0x20, 0x11); io::wait();
    io::outb(0xA0, 0x11); io::wait();

    // Define offsets (32 for Master, 40 for Slave)
    io::outb(0x21, 0x20); io::wait();
    io::outb(0xA1, 0x28); io::wait();

    // Tell Master about Slave at IRQ2
    io::outb(0x21, 0x04); io::wait();
    // Tell Slave its cascade identity
    io::outb(0xA1, 0x02); io::wait();

    // 8086 mode
    io::outb(0x21, 0x01); io::wait();
    io::outb(0xA1, 0x01); io::wait();

    // Restore masks (or set to 0 to enable all)
    // Mask all interrupts except for the keyboard (IRQ 1) and the mouse (IRQ 12).
    // The cascade from master to slave (IRQ 2) must also be unmasked.
    io::outb(0x21, 0b11111000); // Master: Unmask IRQ0 (timer), IRQ1 (keyboard), and IRQ2 (cascade)
    io::outb(0xA1, 0b11101111); // Slave: Unmask IRQ12 (mouse)
}

// Naked assembly handler for the Timer Interrupt (IRQ 0)
// We use this instead of x86-interrupt ABI to manually control the stack switching.
global_asm!(
    ".global timer_handler",
    "timer_handler:",
    // 1. Save Context
    "push 0",           // Dummy error code for stack alignment consistency
    
    // Save Segment Registers
    "push ds",
    "push es",
    "push fs",
    "push gs",
    
    "pusha",            // Save General Registers (EDI, ESI, EBP, ESP, EBX, EDX, ECX, EAX)

    // 2. Load Kernel Data Segment (0x10)
    "mov ax, 0x10",
    "mov ds, ax",
    "mov es, ax",
    "mov fs, ax",
    "mov gs, ax",

    // 3. Call Schedule(current_esp)
    "mov eax, esp",     // Pass current ESP as argument
    "push eax",
    "call schedule",    // Returns new ESP in EAX
    "add esp, 4",       // Pop argument

    // 4. Switch Stack
    "mov esp, eax",     // Switch to new task's stack

    // 5. Restore Context
    "popa",             // Restore General Registers
    "pop gs",
    "pop fs",
    "pop es",
    "pop ds",
    "add esp, 4",       // Pop error code
    "iretd"             // Return from interrupt
);