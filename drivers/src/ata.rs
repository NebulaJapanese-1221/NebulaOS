use crate::common::io;
use crate::common::vga;

static mut ATA_DEVICES: i32 = 0;

#[no_mangle]
pub unsafe extern "C" fn ata_init() -> i32 {
    ATA_DEVICES = 0;
    for bus in 0..2 {
        let base = if bus == 0 { 0x1F0 } else { 0x170 };
        for slave in 0..2 {
            let select = if slave == 0 { 0x00 } else { 0x10 };
            io::outb(base + 0x06, select | 0xA0);
            for _ in 0..4 {
                io::inb(base + 0x0C);
            }
            let status = io::inb(base + 0x07);
            if status == 0xFF {
                continue;
            }
            let cl = io::inb(base + 0x04);
            let ch = io::inb(base + 0x05);
            if cl == 0x00 && ch == 0x00 {
                continue;
            }
            if cl == 0x00 && ch == 0xFF {
                continue;
            }
            ATA_DEVICES += 1;
        }
    }
    ATA_DEVICES
}

#[no_mangle]
pub unsafe extern "C" fn ata_read_sectors(lba: u32, count: u8, buffer: *mut u8) -> i32 {
    if buffer.is_null() || count == 0 || count > 128 {
        return -1;
    }
    let base = 0x1F0;
    let select = 0x00;
    for _ in 0..1000000 {
        let status = io::inb(base + 0x07);
        if !(status & 0x80) != 0 && (status & 0x08 != 0 || status & 0x01 != 0) {
            break;
        }
    }
    io::outb(base + 0x06, (select | 0xE0 | ((lba >> 24) & 0x0F)) as u8);
    io::outb(base + 0x01, 0x00);
    io::outb(base + 0x02, count);
    io::outb(base + 0x03, (lba & 0xFF) as u8);
    io::outb(base + 0x04, ((lba >> 8) & 0xFF) as u8);
    io::outb(base + 0x05, ((lba >> 16) & 0xFF) as u8);
    io::outb(base + 0x07, 0x20);

    let dst = buffer as *mut u16;
    for s in 0..count {
        for i in 0..256 {
            *dst.add((s as usize) * 256 + i) = io::inw(base);
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn ata_write_sectors(_lba: u32, _count: u8, _buffer: *const u8) -> i32 {
    -1
}

#[no_mangle]
pub unsafe extern "C" fn ata_get_device_count() -> i32 {
    ATA_DEVICES
}
