use crate::kernel::io;

pub const CMD_PORT: u16 = 0x64;
pub const DATA_PORT: u16 = 0x60;

pub const STATUS_OUTPUT_BUFFER: u8 = 1 << 0;
pub const STATUS_INPUT_BUFFER: u8 = 1 << 1;
pub const STATUS_MOUSE_DATA: u8 = 1 << 5; // AUX data

/// Reads the Status Register.
pub unsafe fn read_status() -> u8 {
    io::inb(CMD_PORT)
}

/// Reads the Data Port.
pub unsafe fn read_data() -> u8 {
    io::inb(DATA_PORT)
}

/// Writes a command to the Command Port, waiting for the input buffer to be clear.
pub unsafe fn write_command(cmd: u8) {
    wait_input_clear();
    io::outb(CMD_PORT, cmd);
}

/// Writes data to the Data Port, waiting for the input buffer to be clear.
pub unsafe fn write_data(data: u8) {
    wait_input_clear();
    io::outb(DATA_PORT, data);
}

/// Waits until the Input Buffer is empty (ready for CPU to write).
unsafe fn wait_input_clear() {
    let mut timeout = 100000;
    while (read_status() & STATUS_INPUT_BUFFER) != 0 {
        timeout -= 1;
        if timeout == 0 { break; }
    }
}

/// Waits for data to be available in the Output Buffer.
pub unsafe fn wait_output_avail() -> bool {
    let mut timeout = 100000;
    while (read_status() & STATUS_OUTPUT_BUFFER) == 0 {
        timeout -= 1;
        if timeout == 0 { return false; }
    }
    true
}

/// Sends a command to the mouse (Aux device) and waits for ACK.
/// This handles the protocol of writing 0xD4 to port 0x64 first.
pub unsafe fn write_mouse_command(byte: u8) -> bool {
    write_command(0xD4); // Write to Auxiliary Device
    write_data(byte);
    
    // Wait for ACK
    if wait_output_avail() {
        let ack = read_data();
        return ack == 0xFA;
    }
    false
}