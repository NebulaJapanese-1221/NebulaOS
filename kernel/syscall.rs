// Syscall interface (int 0x80) and interrupt handler

const SYSCALL_PRINT: u32 = 0;
const SYSCALL_READ_LINE: u32 = 1;
const SYSCALL_EXIT: u32 = 2;
const SYSCALL_GET_TICKS: u32 = 3;

// Register offsets from regs_ptr (ESP after isr_common pushes)
const REG_EAX: usize = 44 / 4;
const REG_EBX: usize = 32 / 4;
const REG_ECX: usize = 40 / 4;
const REG_EDX: usize = 36 / 4;
const REG_ESI: usize = 20 / 4;
const REG_EDI: usize = 16 / 4;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn serial_write_byte(byte: u8) {
    unsafe {
        core::arch::asm!("2: in al, dx", "test al, 0x20", "jz 2b", in("dx") 0x3FD, options(nostack));
    }
    port_byte_out(0x3F8, byte);
}

fn serial_write_string(s: &str) {
    for byte in s.bytes() {
        serial_write_byte(byte);
    }
}

pub fn init() {
    // Syscall handler (interrupt 128) is set up in IDT
}

// This is called from isr_common in assembly
// interrupt_num: the interrupt vector
// error_code: the error code (0 if none)
// regs_ptr: pointer to saved registers on the stack
#[no_mangle]
pub extern "C" fn isr_handler_rust(interrupt_num: u32, _error_code: u32, regs_ptr: *const u32) -> u32 {
    if interrupt_num == 128 {
        return handle_syscall(regs_ptr);
    }

    // Hardware interrupt or exception - log via serial
    drivers_serial_write_string("Interrupt: ");
    drivers_serial_write_dec(interrupt_num);
    drivers_serial_write_string("\r\n");

    0
}

fn handle_syscall(regs_ptr: *const u32) -> u32 {
    unsafe {
        let syscall_num = *regs_ptr.add(REG_EAX);

        match syscall_num {
            SYSCALL_PRINT => {
                let ptr = *regs_ptr.add(REG_EBX) as *const u8;
                let len = *regs_ptr.add(REG_ECX) as usize;
                let s = core::slice::from_raw_parts(ptr, len);
                for &b in s {
                    if b == b'\n' {
                        serial_write_byte(b'\r');
                    }
                    serial_write_byte(b);
                }
                0
            }
            SYSCALL_READ_LINE => {
                let _buf = *regs_ptr.add(REG_EBX) as *mut u8;
                let _len = *regs_ptr.add(REG_ECX) as usize;
                0
            }
            SYSCALL_EXIT => {
                let _code = *regs_ptr.add(REG_EBX) as u32;
                loop {}
            }
            SYSCALL_GET_TICKS => {
                crate::drivers::pit::ticks()
            }
            _ => 0xFFFFFFFF,
        }
    }
}

fn drivers_serial_write_string(s: &str) {
    for byte in s.bytes() {
        serial_write_byte(byte);
    }
}

fn drivers_serial_write_dec(mut val: u32) {
    if val == 0 {
        serial_write_byte(b'0');
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
        serial_write_byte(buf[i]);
    }
}
