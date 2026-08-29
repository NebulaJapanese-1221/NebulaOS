use crate::common::stdint::{u16, u8};

#[inline(always)]
pub unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!("outb %0, %1", in("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
}

#[inline(always)]
pub unsafe fn outw(port: u16, value: u16) {
    unsafe {
        asm!("outw %0, %1", in("ax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
}

#[inline(always)]
pub unsafe fn outl(port: u16, value: u32) {
    unsafe {
        asm!("outl %0, %1", in("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
}

#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!("inb %1, %0", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline(always)]
pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    unsafe {
        asm!("inw %1, %0", out("ax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline(always)]
pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    unsafe {
        asm!("inl %1, %0", out("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}
