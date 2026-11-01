// Process management and round-robin scheduler

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Pcb {
    pub stack_ptr: u32,
    pub state: ProcessState,
    pub pid: u32,
    pub ticks_remaining: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ProcessState {
    Unused,
    Ready,
    Running,
    Blocked,
}

const MAX_PROCESSES: usize = 16;
const PROCESS_STACK_SIZE: usize = 4096;
const PROCESS_STACK_BASE: usize = 0x00600000;

static mut PROCESSES: [Pcb; MAX_PROCESSES] = [
    Pcb {
        stack_ptr: 0,
        state: ProcessState::Unused,
        pid: 0,
        ticks_remaining: 0,
    }; MAX_PROCESSES
];

static mut CURRENT_PID: u32 = 0;
static mut PROCESS_COUNT: usize = 0;
static mut SCHEDULER_ENABLED: bool = false;

pub fn init() {
    unsafe {
        SCHEDULER_ENABLED = false;
    }
}

pub fn add_process(entry_point: u32) -> u32 {
    unsafe {
        if PROCESS_COUNT >= MAX_PROCESSES {
            return 0xFFFFFFFF;
        }

        let mut slot = 0;
        while slot < MAX_PROCESSES && PROCESSES[slot].state != ProcessState::Unused {
            slot += 1;
        }
        if slot >= MAX_PROCESSES {
            return 0xFFFFFFFF;
        }

        let pid = (slot + 1) as u32;
        let stack_top = (PROCESS_STACK_BASE + slot * PROCESS_STACK_SIZE + PROCESS_STACK_SIZE) as u32;

        let stack_ptr = stack_top - 68;
        let stack_base = stack_ptr as *mut u32;

        // gs, fs, es, ds
        *stack_base.add(0) = 0x10;
        *stack_base.add(1) = 0x10;
        *stack_base.add(2) = 0x10;
        *stack_base.add(3) = 0x10;

        // pushad registers: edi, esi, ebp, esp_dummy, ebx, edx, ecx, eax
        *stack_base.add(4) = 0;
        *stack_base.add(5) = 0;
        *stack_base.add(6) = 0;
        *stack_base.add(7) = 0;
        *stack_base.add(8) = 0;
        *stack_base.add(9) = 0;
        *stack_base.add(10) = 0;
        *stack_base.add(11) = 0;

        // interrupt_num (dummy)
        *stack_base.add(12) = 0;
        // error_code (dummy)
        *stack_base.add(13) = 0;

        // iret frame: EIP, CS, EFLAGS
        *stack_base.add(14) = entry_point;
        *stack_base.add(15) = 0x08;
        *stack_base.add(16) = 0x00000202;

        PROCESSES[slot].stack_ptr = stack_ptr;
        PROCESSES[slot].state = ProcessState::Ready;
        PROCESSES[slot].pid = pid;
        PROCESSES[slot].ticks_remaining = 5;
        PROCESS_COUNT += 1;

        pid
    }
}

pub fn remove_process(pid: u32) {
    unsafe {
        if pid == 0 || pid > MAX_PROCESSES as u32 {
            return;
        }
        let slot = (pid - 1) as usize;
        PROCESSES[slot].state = ProcessState::Unused;
        PROCESSES[slot].stack_ptr = 0;
        if PROCESS_COUNT > 0 {
            PROCESS_COUNT -= 1;
        }
    }
}

pub fn current_pid() -> u32 {
    unsafe { CURRENT_PID }
}

pub fn is_scheduler_enabled() -> bool {
    unsafe { SCHEDULER_ENABLED }
}

pub fn enable_scheduler() {
    unsafe {
        SCHEDULER_ENABLED = true;
    }
}

// Called from isr_common (assembly) on timer tick
// current_esp: the saved stack pointer of the interrupted process
// Returns: the next process's stack pointer
#[no_mangle]
pub extern "C" fn schedule_rust(current_esp: u32) -> u32 {
    unsafe {
        if !SCHEDULER_ENABLED || PROCESS_COUNT == 0 {
            return current_esp;
        }

        // Save current esp to current PCB
        if CURRENT_PID != 0 {
            let slot = (CURRENT_PID - 1) as usize;
            PROCESSES[slot].stack_ptr = current_esp;
            PROCESSES[slot].state = ProcessState::Ready;
        }

        // Round-robin: find next ready process
        let mut next_pid: u32 = 0;
        let start = if CURRENT_PID != 0 { CURRENT_PID as usize } else { 0 };

        for i in 1..=MAX_PROCESSES {
            let idx = (start + i - 1) % MAX_PROCESSES;
            if PROCESSES[idx].state == ProcessState::Ready {
                PROCESSES[idx].state = ProcessState::Running;
                PROCESSES[idx].ticks_remaining = 5;
                next_pid = (idx + 1) as u32;
                break;
            }
        }

        if next_pid == 0 {
            // No other process, return to current
            if CURRENT_PID != 0 {
                let slot = (CURRENT_PID - 1) as usize;
                PROCESSES[slot].ticks_remaining = 5;
                return PROCESSES[slot].stack_ptr;
            }
            return current_esp;
        }

        CURRENT_PID = next_pid;
        PROCESSES[(next_pid - 1) as usize].stack_ptr
    }
}
