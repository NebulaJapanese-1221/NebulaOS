use crate::stdint::*;

pub const MAX_PROCESSES: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpuContext {
    pub r15: u64, pub r14: u64, pub r13: u64, pub r12: u64,
    pub r11: u64, pub r10: u64, pub r9: u64, pub r8: u64,
    pub rbp: u64, pub rdi: u64, pub rsi: u64, pub rdx: u64,
    pub rcx: u64, pub rbx: u64, pub rax: u64,
    pub rip: u64, pub cs: u64, pub rflags: u64, pub rsp: u64, pub ss: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Process {
    pub pid: u32,
    pub state: u32,
    pub context: CpuContext,
    pub stack_base: usize,
    pub stack_size: u32,
    pub entry_point: *mut u8,
    pub fd_table: [i32; 8],
}

pub const PROC_READY: u32 = 0;
pub const PROC_RUNNING: u32 = 1;
pub const PROC_BLOCKED: u32 = 2;
pub const PROC_ZOMBIE: u32 = 3;

pub static mut PROCESS_TABLE: [Process; MAX_PROCESSES as usize] = [Process { pid: 0, state: 0, context: unsafe { core::mem::zeroed() }, stack_base: 0, stack_size: 0, entry_point: core::ptr::null_mut(), fd_table: [-1; 8] }; MAX_PROCESSES as usize];
static mut NEXT_PID: u32 = 1;
static mut CURRENT_PROCESS: *mut Process = core::ptr::null_mut();

pub unsafe fn process_init() {
    for i in 0..MAX_PROCESSES {
        (*PROCESS_TABLE.as_mut_ptr().add(i as usize)).pid = 0;
        (*PROCESS_TABLE.as_mut_ptr().add(i as usize)).state = PROC_ZOMBIE;
    }
    CURRENT_PROCESS = core::ptr::null_mut();
}

pub unsafe fn process_create(entry: *mut u8, stack_size: u32) -> i32 {
    for i in 0..MAX_PROCESSES {
        let proc = &mut *PROCESS_TABLE.as_mut_ptr().add(i as usize);
        if proc.state == PROC_ZOMBIE {
            proc.pid = NEXT_PID;
            NEXT_PID += 1;
            proc.state = PROC_READY;
            proc.entry_point = entry;
            proc.stack_size = (stack_size + 15) & !15;
            proc.stack_base = 0x10000000 + (i as usize) * 0x10000;
            proc.context = CpuContext {
                rip: entry as u64,
                rsp: proc.stack_base as u64,
                cs: 0x08,
                ss: 0x10,
                rflags: 0x202,
                ..unsafe { core::mem::zeroed() }
            };
            for f in 0..8 {
                proc.fd_table[f] = -1;
            }
            if CURRENT_PROCESS.is_null() {
                CURRENT_PROCESS = proc as *mut Process;
            }
            return proc.pid as i32;
        }
    }
    -1
}

pub unsafe fn process_get_current() -> *mut Process {
    CURRENT_PROCESS
}

pub unsafe fn process_set_current(pid: u32) {
    for i in 0..MAX_PROCESSES {
        if (*PROCESS_TABLE.as_ptr().add(i as usize)).pid == pid {
            CURRENT_PROCESS = PROCESS_TABLE.as_mut_ptr().add(i as usize);
            return;
        }
    }
}

pub unsafe fn process_destroy(pid: u32) {
    for i in 0..MAX_PROCESSES {
        if (*PROCESS_TABLE.as_ptr().add(i as usize)).pid == pid {
            (*PROCESS_TABLE.as_mut_ptr().add(i as usize)).state = PROC_ZOMBIE;
            if CURRENT_PROCESS == PROCESS_TABLE.as_ptr().add(i as usize) as *mut Process {
                CURRENT_PROCESS = core::ptr::null_mut();
            }
            return;
        }
    }
}
