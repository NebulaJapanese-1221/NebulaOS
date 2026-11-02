// Syscall interface (int 0x80) and interrupt handler

const SYSCALL_PRINT: u32 = 0;
const SYSCALL_EXIT: u32 = 2;
const SYSCALL_GET_TICKS: u32 = 3;
const MAX_SYSCALL_WRITE: usize = 256;

// Register offsets from regs_ptr (ESP after isr_common pushes)
const REG_EAX: usize = 44 / 4;
const REG_EBX: usize = 32 / 4;
const REG_ECX: usize = 40 / 4;
const REG_CS: usize = 60 / 4;

// This is called from isr_common in assembly
// interrupt_num: the interrupt vector
// error_code: the error code (0 if none)
// regs_ptr: pointer to saved registers on the stack
#[no_mangle]
pub extern "C" fn isr_handler_rust(
    interrupt_num: u32,
    _error_code: u32,
    regs_ptr: *mut u32,
) -> u32 {
    if interrupt_num == 128 {
        return handle_syscall(regs_ptr);
    }

    if interrupt_num == 32 {
        crate::drivers::pit::pit_tick();
    } else if interrupt_num < 32 && !regs_ptr.is_null() && unsafe { *regs_ptr.add(REG_CS) & 3 == 3 }
    {
        crate::drivers::user_process::exit_from_syscall(regs_ptr, 0x8000_0000 | interrupt_num);
    }

    0
}

fn handle_syscall(regs_ptr: *mut u32) -> u32 {
    if regs_ptr.is_null() || unsafe { *regs_ptr.add(REG_CS) & 3 != 3 } {
        return u32::MAX;
    }
    unsafe {
        let syscall_num = *regs_ptr.add(REG_EAX);

        let result = match syscall_num {
            SYSCALL_PRINT => {
                let address = *regs_ptr.add(REG_EBX);
                let len = *regs_ptr.add(REG_ECX) as usize;
                if len > MAX_SYSCALL_WRITE
                    || !crate::drivers::paging::user_range_is_mapped(address, len)
                {
                    u32::MAX
                } else {
                    let mut buffer = [0u8; MAX_SYSCALL_WRITE];
                    if len > 0 {
                        core::ptr::copy_nonoverlapping(
                            address as *const u8,
                            buffer.as_mut_ptr(),
                            len,
                        );
                    }
                    crate::drivers::wm::console_write(&buffer[..len]);
                    crate::drivers::user_process::record_syscall_output(&buffer[..len]);
                    len as u32
                }
            }
            SYSCALL_EXIT => {
                if crate::drivers::user_process::exit_from_syscall(regs_ptr, *regs_ptr.add(REG_EBX))
                {
                    0
                } else {
                    u32::MAX
                }
            }
            SYSCALL_GET_TICKS => crate::drivers::pit::ticks(),
            _ => u32::MAX,
        };
        *regs_ptr.add(REG_EAX) = result;
        result
    }
}
