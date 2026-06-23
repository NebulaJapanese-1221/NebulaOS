use crate::process::{Process, ProcessState};
use crate::sync::Spinlock;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::arch::asm;

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

    // Renamed add_process to be more specific to kernel tasks
    #[allow(dead_code)]
    pub fn add_kernel_task(&mut self, entry_point: u32) {
        let id = self.processes.len();
        self.processes.push(Process::new_kernel_task(id, entry_point));
    }

    // New method to spawn a user process
    #[allow(dead_code)]
    pub fn spawn_user_process(
        &mut self,
        entry_point: u32,
        user_stack_size: usize,
        kernel_stack_size: usize,
    ) -> usize {
        let id = self.processes.len();
        let process = Process::new_user_process(
            id,
            entry_point,
            user_stack_size,
            kernel_stack_size,
        );
        self.processes.push(process);
        id // Return the new process ID
    }

    pub fn spawn(&mut self, entry_point: u32) {
        // For backward compatibility, assume kernel task if only entry point is given
        self.add_kernel_task(entry_point);
    }
}

pub static SCHEDULER: Spinlock<Scheduler> = Spinlock::new(Scheduler::new());

/// Removes the current process and switches to the next one.
pub fn exit_current_process(regs_ptr: u32) -> u32 {
    let mut sched = SCHEDULER.lock();
    if let Some(pid) = sched.current_pid.take() {
        if pid < sched.processes.len() {
            // Remove process from the list
            sched.processes.remove(pid);
            // Adjust current_pid if necessary (e.g., if it was the last process)
            if pid == sched.processes.len() {
                 sched.current_pid = Some(0); // Wrap around or handle empty list
            } else if !sched.processes.is_empty() {
                 sched.current_pid = Some(pid); // New process at this index
            } else {
                 sched.current_pid = None; // No processes left
            }
        }
    }
    drop(sched); // Release lock before potentially long schedule operation
    schedule(regs_ptr) // Schedule the next process
}

/// Increments system ticks. Called by the timer interrupt.
pub fn timer_tick() {
    SCHEDULER.lock().ticks += 1;
}

#[no_mangle]
pub extern "C" fn schedule(regs_ptr: u32) -> u32 {
    let mut sched = SCHEDULER.lock();

    // Save current process state if there was one running
    if let Some(pid) = sched.current_pid.take() {
        if pid < sched.processes.len() {
            let proc = &mut sched.processes.as_mut_slice()[pid];
            // Store the current state of the registers as the process's context
            proc.kernel_stack_ptr = regs_ptr;
            // Only revert to Ready if it was explicitly Running.
            // Sleeping processes should not be reset to Ready here.
            if proc.state == ProcessState::Running {
                proc.state = ProcessState::Ready;
            }
        }
    }

    // If no processes, return current registers (should ideally halt or panic)
    if sched.processes.is_empty() {
        // This should not happen in a running system.
        // Perhaps halt the CPU or trigger a kernel panic.
        return regs_ptr;
    }

    // Find the next process to run (Round Robin)
    let start_idx = sched.current_pid.map_or(0, |pid| (pid + 1) % sched.processes.len());
    let mut next_pid = start_idx;
    let current_ticks = sched.ticks;

    loop {
        let proc = &mut sched.processes.as_mut_slice()[next_pid];
        match proc.state {
            ProcessState::Ready => break, // Found a ready process
            ProcessState::Sleeping(wake_tick) if current_ticks >= wake_tick => {
                proc.state = ProcessState::Ready; // Wake up
                break;
            }
            _ => { // Continue to next process
                next_pid = (next_pid + 1) % sched.processes.len();
                if next_pid == start_idx {
                    // Went through all processes and found no one ready.
                    // This could happen if all are sleeping or dead.
                    // Return current registers, effectively idling.
                    return regs_ptr;
                }
            }
        }
    }

    // Set the new current process and update its state to Running
    sched.current_pid = Some(next_pid);
    let next_proc = &mut sched.processes.as_mut_slice()[next_pid];
    next_proc.state = ProcessState::Running;

    // --- CRITICAL for Paging ---
    // Load the page directory of the new process into CR3.
    unsafe {
        asm!("mov cr3, {}", in(reg) next_proc.page_directory_phys_addr);
    }

    // Update the TSS with the kernel stack for the NEW process.
    // This ensures that if an interrupt or syscall occurs while in user mode,
    // the CPU switches to the correct kernel stack for this process.
    crate::gdt::set_kernel_stack(next_proc.kernel_stack_ptr);

    // Return the context (kernel stack pointer) of the process to be restored.
    next_proc.kernel_stack_ptr
}