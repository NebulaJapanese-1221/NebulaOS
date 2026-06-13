use crate::process::{Process, ProcessState};
use crate::sync::Spinlock; // Assuming Spinlock is in crate::sync
use alloc::vec::Vec;
use alloc::boxed::Box;

pub struct Scheduler {
    pub processes: Vec<Box<Process>>,
    pub current_pid: Option<usize>,
    pub ticks: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            processes: Vec::new(),
            current_pid: None,
            ticks: 0,
        }
    }

    #[allow(dead_code)]
    pub fn add_process(&mut self, entry_point: u32) {
        let id = self.processes.len();
        self.processes.push(Process::new(id, entry_point));
    }

    pub fn spawn(&mut self, entry_point: u32) {
        self.add_process(entry_point);
    }
}

pub static SCHEDULER: Spinlock<Scheduler> = Spinlock::new(Scheduler::new());

/// Removes the current process and switches to the next one.
pub fn exit_current_process(regs_ptr: u32) -> u32 {
    let mut sched = SCHEDULER.lock();
    if let Some(pid) = sched.current_pid.take() {
        if pid < sched.processes.len() {
            sched.processes.remove(pid);
        }
    }
    drop(sched);
    schedule(regs_ptr)
}

/// Increments system ticks. Called by the timer interrupt.
pub fn timer_tick() {
    SCHEDULER.lock().ticks += 1;
}

#[no_mangle]
pub extern "C" fn schedule(regs_ptr: u32) -> u32 {
    let mut sched = SCHEDULER.lock();
    if sched.processes.is_empty() { return regs_ptr; }

    // Save current state: only revert to Ready if the process was actually Running
    // (prevents overwriting Sleeping state if schedule was called by a sleep syscall)
    if let Some(pid) = sched.current_pid {
        let proc = &mut sched.processes.as_mut_slice()[pid];
        proc.kernel_stack_ptr = regs_ptr;
        if let ProcessState::Running = proc.state {
            proc.state = ProcessState::Ready;
        }
    }

    // Round Robin
    let start_pid = sched.current_pid.map_or(0, |p| (p + 1) % (sched.processes.len().max(1)));
    let mut next_pid = start_pid;
    let current_ticks = sched.ticks;

    loop {
        let proc = &mut sched.processes.as_mut_slice()[next_pid];
        match proc.state {
            ProcessState::Ready => break,
            ProcessState::Sleeping(wake_tick) if current_ticks >= wake_tick => {
                proc.state = ProcessState::Ready;
                break;
            }
            _ => {
                next_pid = (next_pid + 1) % sched.processes.len();
                if next_pid == start_pid { return regs_ptr; } // No task to run
            }
        }
    }

    sched.current_pid = Some(next_pid);
    let next_proc = &mut sched.processes.as_mut_slice()[next_pid];
    next_proc.state = ProcessState::Running;

    // Important: Update the TSS so the next interrupt lands on the NEW process stack
    crate::gdt::set_kernel_stack(next_proc.stack.as_ptr() as u32 + 4096);

    next_proc.kernel_stack_ptr
}