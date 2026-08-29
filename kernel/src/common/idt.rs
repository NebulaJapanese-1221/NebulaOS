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

pub unsafe fn default_interrupt_handler(regs: *mut Registers) {
    let int_no = (*regs).int_no;
    if int_no >= IRQ0 as u32 && int_no <= IRQ15 as u32 {
        io::outb(0x20, 0x20);
        if int_no >= IRQ8 as u32 {
            io::outb(0xA0, 0x20);
        }
    }
}

extern "C" {
    static mut isr_table: [unsafe extern "C" fn(); 32];
    static mut irq_table: [unsafe extern "C" fn(); 16];
}
