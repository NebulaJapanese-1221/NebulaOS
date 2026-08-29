use crate::common::io;

#[no_mangle]
pub unsafe extern "C" fn pic_init(master_offset: u8, slave_offset: u8) {
    io::outb(0x20, 0x11);
    io::outb(0x21, master_offset);
    io::outb(0x21, 0x04);
    io::outb(0x21, 0x01);
    io::outb(0xA0, 0x11);
    io::outb(0xA1, slave_offset);
    io::outb(0xA1, 0x02);
    io::outb(0xA1, 0x01);
    io::outb(0x21, 0xFF);
    io::outb(0xA1, 0xFF);
}

pub unsafe fn pic_enable_irq(irq: u8) {
    if irq < 8 {
        let mask = io::inb(0x21) & !(1 << irq);
        io::outb(0x21, mask);
    } else {
        let mask = io::inb(0xA1) & !(1 << (irq - 8));
        io::outb(0xA1, mask);
    }
}

pub unsafe fn pic_disable_irq(irq: u8) {
    if irq < 8 {
        let mask = io::inb(0x21) | (1 << irq);
        io::outb(0x21, mask);
    } else {
        let mask = io::inb(0xA1) | (1 << (irq - 8));
        io::outb(0xA1, mask);
    }
}

pub unsafe fn pic_send_eoi(irq: u8) {
    if irq >= 8 {
        io::outb(0xA0, 0x20);
    }
    io::outb(0x20, 0x20);
}
