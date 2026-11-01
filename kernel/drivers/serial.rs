// Serial port driver

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn port_byte_in(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port);
    }
    val
}

pub fn init() {
    port_byte_out(0x3F8 + 3, 0x80);
    port_byte_out(0x3F8 + 0, 0x01);
    port_byte_out(0x3F8 + 1, 0x00);
    port_byte_out(0x3F8 + 3, 0x03);
    port_byte_out(0x3F8 + 2, 0xC7);
    port_byte_out(0x3F8 + 1, 0x00);
}

pub fn write_byte(byte: u8) {
    unsafe {
        core::arch::asm!("2: in al, dx", "test al, 0x20", "jz 2b", in("dx") 0x3F8 + 5, options(nostack));
    }
    port_byte_out(0x3F8, byte);
}

pub fn write_string(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

pub fn write_dec(mut val: u32) {
    if val == 0 {
        write_byte(b'0');
        return;
    }
    let mut buf = [0u8; 10];
    let mut pos = 10;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    for i in pos..10 {
        write_byte(buf[i]);
    }
}
