use core::arch::asm;

/// Reads a byte from the specified I/O port.
pub unsafe fn inb(port: u16) -> u8 {
    let mut value: u8;
    asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags));
    value
}

/// Writes a byte to the specified I/O port.
pub unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

/// Writes a word (16 bits) to the specified I/O port.
pub unsafe fn outw(port: u16, value: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
}

/// Reads a dword (32 bits) from the specified I/O port.
pub unsafe fn inl(port: u16) -> u32 {
    let mut value: u32;
    asm!("in eax, dx", in("dx") port, out("eax") value, options(nomem, nostack, preserves_flags));
    value
}

/// Writes a dword (32 bits) to the specified I/O port.
pub unsafe fn outl(port: u16, value: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
}

/// A short delay. Useful after I/O operations.
pub unsafe fn wait() {
    // This is a simple "I/O delay" by writing to a dummy port.
    outb(0x80, 0);
}