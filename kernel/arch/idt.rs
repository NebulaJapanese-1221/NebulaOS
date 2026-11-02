// Interrupt Descriptor Table

use crate::arch::gdt;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    _ist: u8,
    type_attr: u8,
    offset_mid: u16,
}

#[repr(C, packed)]
pub struct IdtPtr {
    limit: u16,
    base: u32,
}

const IDT_ENTRIES: usize = 256;

static mut IDT: [IdtEntry; IDT_ENTRIES] = [IdtEntry {
    offset_low: 0,
    selector: 0,
    _ist: 0,
    type_attr: 0,
    offset_mid: 0,
}; IDT_ENTRIES];

static mut IDT_PTR: IdtPtr = IdtPtr { limit: 0, base: 0 };

const IDT_PRESENT: u8 = 0x80;
const IDT_RING0: u8 = 0x00;
const IDT_RING3: u8 = 0x60;
const IDT_32BIT_INTERRUPT: u8 = 0x0E;
const IDT_32BIT_TRAP: u8 = 0x0F;

fn idt_set_entry(index: usize, offset: u32, selector: u16, type_attr: u8) {
    unsafe {
        IDT[index].offset_low = (offset & 0xFFFF) as u16;
        IDT[index].selector = selector;
        IDT[index]._ist = 0;
        IDT[index].type_attr = IDT_PRESENT | IDT_RING0 | type_attr;
        IDT[index].offset_mid = ((offset >> 16) & 0xFFFF) as u16;
    }
}

extern "C" {
    fn idt_load(ptr: *const IdtPtr);
    static isr_stub_table: [u32; 256];
}

pub fn init() {
    unsafe {
        IDT_PTR.limit = (IDT_ENTRIES * core::mem::size_of::<IdtEntry>() - 1) as u16;
        IDT_PTR.base = core::ptr::addr_of!(IDT) as u32;

        for i in 0..IDT_ENTRIES {
            let offset = isr_stub_table[i];
            let type_attr = if i == 128 {
                IDT_RING3 | IDT_32BIT_TRAP
            } else {
                IDT_RING0 | IDT_32BIT_INTERRUPT
            };
            idt_set_entry(i, offset, gdt::kernel_code_selector(), type_attr);
        }

        idt_load(core::ptr::addr_of!(IDT_PTR));
    }
}
