use crate::stdint::*;
use crate::process;
use crate::scheduler;

pub const SYS_WRITE: u64 = 1;
pub const SYS_READ: u64 = 2;
pub const SYS_OPEN: u64 = 3;
pub const SYS_CLOSE: u64 = 4;
pub const SYS_EXIT: u64 = 5;
pub const SYS_GETPID: u64 = 6;
pub const SYS_YIELD: u64 = 7;

static mut SYSCALL_INITIALIZED: bool = false;

pub unsafe fn syscall_init() {
    SYSCALL_INITIALIZED = true;
}

#[no_mangle]
pub unsafe extern "C" fn syscall_dispatch(num: u64, arg1: u64, arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64) -> u64 {
    match num {
        SYS_WRITE => {
            let str = arg1 as *const u8;
            let len = arg2 as usize;
            for i in 0..len {
                let c = *str.add(i as usize);
            }
            len as u64
        }
        SYS_READ => 0,
        SYS_OPEN => -1i64 as u64,
        SYS_CLOSE => 0,
        SYS_EXIT => {
            crate::nebula::kernel_panic("Process exited");
            0
        }
        SYS_GETPID => {
            let proc = process::process_get_current();
            if !proc.is_null() {
                (*proc).pid as u64
            } else {
                0
            }
        }
        SYS_YIELD => {
            scheduler::scheduler_tick();
            0
        }
        _ => -1i64 as u64,
    }
}
