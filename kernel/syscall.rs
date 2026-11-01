// Syscall interface (int 0x80) and interrupt handler

const SYSCALL_PRINT: u32 = 0;
const SYSCALL_EXIT: u32 = 2;
const SYSCALL_GET_TICKS: u32 = 3;

// Register offsets from regs_ptr (ESP after isr_common pushes)
const REG_EAX: usize = 44 / 4;
const REG_EBX: usize = 32 / 4;
const REG_ECX: usize = 40 / 4;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
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

    if interrupt_num == 32 {
        crate::drivers::pit::pit_tick();
    }

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
                crate::drivers::wm::console_write(s);
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
