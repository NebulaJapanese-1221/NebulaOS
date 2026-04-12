//! Driver for the ACPI Embedded Controller (EC).
//! The EC is a microcontroller that handles various platform-specific tasks,
//! including battery management, thermal control, and backlight adjustment.
//! Communication is typically done via I/O ports 0x62 (command/status) and 0x66 (data).

use crate::kernel::io;

// EC I/O Ports
const EC_COMMAND_PORT: u16 = 0x66;
const EC_DATA_PORT: u16 = 0x62;

// EC Status Register Bits (read from EC_COMMAND_PORT)
const EC_STATUS_IBF: u8 = 0x02; // Input Buffer Full (EC is busy, cannot accept command/data)
const EC_STATUS_OBF: u8 = 0x01; // Output Buffer Full (EC has data ready to be read)

// EC Commands (written to EC_COMMAND_PORT)
const EC_CMD_READ: u8 = 0x80;
const EC_CMD_WRITE: u8 = 0x81;
const EC_CMD_QUERY: u8 = 0x84;

/// Waits until the EC's input buffer is empty, meaning it's ready to accept a command or data.
fn ec_wait_for_ibf() {
    let mut timeout = 1_000_000; // Prevent infinite loop
    while unsafe { io::inb(EC_COMMAND_PORT) } & EC_STATUS_IBF != 0 {
        unsafe { io::wait(); }
        timeout -= 1;
        if timeout == 0 {
            crate::serial_println!("[EC] Timeout waiting for IBF clear.");
            return;
        }
    }
}

/// Waits until the EC's output buffer is full, meaning it has data ready to be read.
fn ec_wait_for_obf() -> bool {
    let mut timeout = 1_000_000; // Prevent infinite loop
    while unsafe { io::inb(EC_COMMAND_PORT) } & EC_STATUS_OBF == 0 {
        unsafe { io::wait(); }
        timeout -= 1;
        if timeout == 0 {
            crate::serial_println!("[EC] Timeout waiting for OBF set.");
            return false;
        }
    }
    true
}

/// Reads a byte from a specific EC register address.
pub fn ec_read_byte(address: u8) -> Option<u8> {
    ec_wait_for_ibf();
    unsafe { io::outb(EC_COMMAND_PORT, EC_CMD_READ); }
    ec_wait_for_ibf();
    unsafe { io::outb(EC_DATA_PORT, address); }

    if ec_wait_for_obf() {
        Some(unsafe { io::inb(EC_DATA_PORT) })
    } else {
        None
    }
}

/// Writes a byte to a specific EC register address.
/// This involves sending a WRITE command, the address, and the data to the EC.
pub fn ec_write_byte(address: u8, value: u8) {
    ec_wait_for_ibf();
    unsafe { io::outb(EC_COMMAND_PORT, EC_CMD_WRITE); }
    ec_wait_for_ibf();
    unsafe { io::outb(EC_DATA_PORT, address); }
    ec_wait_for_ibf();
    unsafe { io::outb(EC_DATA_PORT, value); }
}

/// Queries the EC for an event.
pub fn ec_query_event() -> Option<u8> {
    ec_wait_for_ibf();
    unsafe { io::outb(EC_COMMAND_PORT, EC_CMD_QUERY); }
    if ec_wait_for_obf() {
        Some(unsafe { io::inb(EC_DATA_PORT) })
    } else {
        None
    }
}

/// Initializes the EC driver.
pub fn init() {
    crate::serial_println!("[EC] Initialized EC driver (assuming standard ports).");
}