// x86_64 (64-bit) IDT implementation
// 64-bit IDT entries are 16 bytes each

use core::arch::asm;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist_index: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

#[repr(C, packed(2))]
pub struct IdtPtr {
    limit: u16,
    base: u64,
}

impl IdtEntry {
    pub const fn missing() -> Self {
        IdtEntry {
            offset_low: 0,
            selector: 0,
            ist_index: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    pub fn new(offset: u64, selector: u16, flags: u8) -> Self {
        IdtEntry {
            offset_low: (offset & 0xFFFF) as u16,
            selector,
            ist_index: 0,
            type_attr: flags,
            offset_mid: ((offset >> 16) & 0xFFFF) as u16,
            offset_high: ((offset >> 32) & 0xFFFFFFFF) as u32,
            zero: 0,
        }
    }
}

static mut IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];

pub unsafe fn set_gate(num: u8, base: u64, sel: u16, flags: u8) {
    IDT[num as usize] = IdtEntry::new(base, sel, flags);
}

pub unsafe fn load_idt() {
    let idt_ptr = IdtPtr {
        limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
        base: &raw const IDT as u64,
    };
    asm!("lidt [{}]", in(reg) &idt_ptr);
}

pub unsafe fn init_pic() {
    // Remap PIC: IRQs 0-15 to Interrupts 32-47
    use crate::arch::x86_64::io::outb;

    outb(0x20, 0x11);
    outb(0xA0, 0x11);
    outb(0x21, 0x20);
    outb(0xA1, 0x28);
    outb(0x21, 0x04);
    outb(0xA1, 0x02);
    outb(0x21, 0x01);
    outb(0xA1, 0x01);

    outb(0x21, 0xFF);
    outb(0xA1, 0xFF);
}

