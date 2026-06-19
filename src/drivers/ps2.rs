use core::arch::asm;

pub const PS2_DATA_PORT: u16 = 0x60;
pub const PS2_STATUS_PORT: u16 = 0x64;
pub const PS2_COMMAND_PORT: u16 = 0x64;

/// Write a byte to an I/O port
pub unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("edx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

/// Read a byte from an I/O port
pub unsafe fn inb(port: u16) -> u8 {
    let res: u8;
    asm!("in al, dx", out("al") res, in("edx") port, options(nomem, nostack, preserves_flags));
    res
}

/// Wait until the controller is ready to read data (output buffer full)
pub unsafe fn wait_read() -> bool {
    let mut timeout = 100000;
    while (inb(PS2_STATUS_PORT) & 0x01 == 0) && timeout > 0 {
        timeout -= 1;
    }
    timeout > 0
}

/// Wait until the controller is ready to receive command/data (input buffer empty)
pub unsafe fn wait_write() -> bool {
    let mut timeout = 100000;
    while (inb(PS2_STATUS_PORT) & 0x02 != 0) && timeout > 0 {
        timeout -= 1;
    }
    timeout > 0
}

/// Read data from the controller configuration byte
pub unsafe fn read_config() -> u8 {
    wait_write();
    outb(PS2_COMMAND_PORT, 0x20);
    wait_read();
    inb(PS2_DATA_PORT)
}

/// Write data to the controller configuration byte
pub unsafe fn write_config(config: u8) {
    wait_write();
    outb(PS2_COMMAND_PORT, 0x60);
    wait_write();
    outb(PS2_DATA_PORT, config);
}

/// Write command to the mouse (auxiliary device)
pub unsafe fn write_mouse(cmd: u8) -> bool {
    wait_write();
    outb(PS2_COMMAND_PORT, 0xD4); // Signal next byte is for 2nd port (mouse)
    wait_write();
    outb(PS2_DATA_PORT, cmd);
    
    // Wait for ACK
    let ack = read_mouse();
    ack == Some(0xFA)
}

/// Read mouse data with timeout and verify it is from the auxiliary device
pub unsafe fn read_mouse() -> Option<u8> {
    let mut timeout = 100000;
    while timeout > 0 {
        let status = inb(PS2_STATUS_PORT);
        if (status & 0x01) != 0 {
            let val = inb(PS2_DATA_PORT);
            if (status & 0x20) != 0 {
                return Some(val);
            }
        }
        timeout -= 1;
    }
    None
}

/// Flush any pending bytes in the PS/2 controller buffers
pub unsafe fn flush_buffers() {
    let mut timeout = 100000;
    while (inb(PS2_STATUS_PORT) & 0x01 != 0) && timeout > 0 {
        let _ = inb(PS2_DATA_PORT);
        timeout -= 1;
    }
}
