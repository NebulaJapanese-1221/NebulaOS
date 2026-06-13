use core::arch::asm;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    zero: u8,
    type_attr: u8,
    offset_high: u16,
}

#[repr(C, packed)]
pub struct IdtPtr {
    limit: u16,
    base: u32,
}

static mut IDT: [IdtEntry; 256] = [IdtEntry {
    offset_low: 0,
    selector: 0,
    zero: 0,
    type_attr: 0,
    offset_high: 0,
}; 256];

impl IdtEntry {
    pub fn new(offset: u32, selector: u16, flags: u8) -> Self {
        Self {
            offset_low: (offset & 0xFFFF) as u16,
            selector,
            zero: 0,
            type_attr: flags,
            offset_high: ((offset >> 16) & 0xFFFF) as u16,
        }
    }
}

pub unsafe fn set_gate(num: u8, base: u32, sel: u16, flags: u8) {
    IDT[num as usize] = IdtEntry::new(base, sel, flags);
}

pub unsafe fn load_idt() {
    let idt_ptr = IdtPtr {
        limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
        base: &raw const IDT as u32,
    };
    asm!("lidt [{}]", in(reg) &idt_ptr);
}

pub unsafe fn init_pic() {
    // Remap PIC: IRQs 0-15 to Interrupts 32-47
    super::mouse::outb(0x20, 0x11); // Initialize Master
    super::mouse::outb(0xA0, 0x11); // Initialize Slave
    super::mouse::outb(0x21, 0x20); // Master offset (32)
    super::mouse::outb(0xA1, 0x28); // Slave offset (40)
    super::mouse::outb(0x21, 0x04); // Slave at IRQ2
    super::mouse::outb(0xA1, 0x02); // Cascade identity
    super::mouse::outb(0x21, 0x01); // 8086 mode
    super::mouse::outb(0xA1, 0x01);

    // Enable PS/2 Mouse in the Controller (8042)
    super::mouse::outb(0x64, 0xA8); // Command: Enable 2nd PS/2 port (Mouse)
    
    // Enable Mouse Interrupts in the 8042 Command Byte
    super::mouse::outb(0x64, 0x20); // Command: Read Command Byte
    let status = (super::mouse::inb(0x60) | 0x02) & !0x20; // Bit 1 = Mouse IRQ, Bit 5 = Mouse Clock Enable
    super::mouse::outb(0x64, 0x60); // Command: Write Command Byte
    super::mouse::outb(0x60, status);

    // Unmask Timer (IRQ0), Keyboard (IRQ1), Cascade (IRQ2), and Mouse (IRQ12)
    // IRQ0 is bit 0, IRQ1 is bit 1, IRQ12 is bit 4 of slave PIC.
    super::mouse::outb(0x21, 0xF8); // Master PIC: Unmask IRQ0, IRQ1, IRQ2
    super::mouse::outb(0xA1, 0xEF);
}