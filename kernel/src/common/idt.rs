pub unsafe fn init_idt64() {
    IDT_PTR.limit = (core::mem::size_of::<[IdtEntry; IDT_MAX_DESCRIPTORS]>() - 1) as u16;
    IDT_PTR.base = &mut IDT as *mut _ as u32;

    for i in 0..IDT_MAX_DESCRIPTORS {
        INTERRUPT_HANDLERS[i] = Some(default_interrupt_handler);
    }

    for i in 0..32 {
        set_gate64(i, isr_table[i] as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
    }
    for i in 0..16 {
        set_gate64(IRQ0 as usize + i, irq_table[i] as u64, 0x08, 0, IDT_FLAG_PRESENT | IDT_FLAG_INTERRUPT);
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
