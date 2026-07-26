use alloc::collections::VecDeque;
use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::sync::Spinlock;
use crate::process::{Process, ProcessState};

/// Scheduler structure
pub struct Scheduler {
    pub processes: VecDeque<Box<Process>>,
    current_process_index: usize,
    pub tick_count: usize,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Scheduler {
            processes: VecDeque::new(),
            current_process_index: 0,
            tick_count: 0,
        }
    }

    /// Spawn a new kernel task
    pub fn spawn_kernel_task(&mut self, entry_point: usize) -> usize {
        let pid = self.processes.len() + 1;
        let mut process = Process::new_kernel_task(pid as usize, entry_point);

        // Create a main thread
        let _thread = process.create_thread(entry_point, 4096);

        self.processes.push_back(Box::new(process));
        pid
    }

    /// Spawn a new user process
    pub fn spawn_user_process(
        &mut self,
        entry_point: usize,
        user_stack_size: usize,
        kernel_stack_size: usize,
    ) -> usize {
        let pid = self.processes.len() + 1;
        let mut process = Process::new_user_process(
            pid as usize,
            entry_point,
            user_stack_size,
            kernel_stack_size,
        );

        // Create a main thread
        let _thread = process.create_thread(entry_point, user_stack_size);
        self.processes.push_back(Box::new(process));
        pid
    }

    /// Create a new thread in an existing process
    pub fn create_thread(
        &mut self,
        pid: usize,
        entry: usize,
        stack_size: usize,
    ) -> Option<usize> {
        if let Some(process) = self.processes.iter_mut().find(|p| p.pid == pid) {
            let thread = process.create_thread(entry, stack_size);
            Some(thread.tid)
        } else {
            None
        }
    }

    /// Get the next process to run
    pub fn next_process(&mut self) -> Option<&mut Process> {
        self.tick_count += 1;

        // Round-robin scheduling - collect indices first to avoid borrow conflicts
        let len = self.processes.len();
        if len == 0 {
            return None;
        }

        let mut attempts = 0;
        while attempts < len {
            self.current_process_index = (self.current_process_index + 1) % len;

            let has_runnable = self.processes.get(self.current_process_index)
                .map(|p| p.threads.iter().any(|t| t.state == ProcessState::Ready))
                .unwrap_or(false);

            if has_runnable {
                // Return a raw pointer workaround - safe because we hold the SpinlockGuard
                let process: &mut Process = self.processes.get_mut(self.current_process_index)?;
                return Some(process);
            }

            attempts += 1;
        }

        None
    }

    /// Get the current process
    pub fn current_process(&self) -> Option<&Process> {
        self.processes.get(self.current_process_index).map(|v| &**v)
    }

    /// Get the current process (mutable)
    pub fn current_process_mut(&mut self) -> Option<&mut Process> {
        self.processes.get_mut(self.current_process_index).map(|v| &mut **v)
    }

    /// Block the current process
    pub fn block_current(&mut self) {
        if let Some(process) = self.current_process_mut() {
            process.set_state(ProcessState::Blocked);
        }
    }

    /// Wake up a process
    pub fn wake_up(&mut self, pid: usize) {
        if let Some(process) = self.processes.iter_mut().find(|p| p.pid == pid) {
            process.set_state(ProcessState::Ready);
        }
    }

    /// Exit the current process
    pub fn exit_current(&mut self, _exit_code: i32) {
        if self.current_process_index < self.processes.len() {
            // Collect children PIDs before removing
            let children: Vec<usize> = self.processes.get(self.current_process_index)
                .map(|p| p.children.clone())
                .unwrap_or_default();

            // Remove the process
            self.processes.remove(self.current_process_index);

            // Reparent children to init process
            for &child_pid in &children {
                if let Some(child) = self.processes.iter_mut().find(|p| p.pid == child_pid) {
                    child.parent_pid = 1; // Reparent to init
                }
            }

            // If we removed the current process, select a new one
            if self.current_process_index >= self.processes.len() {
                self.current_process_index = 0;
            }
        }
    }

    /// Send signal to a process
    pub fn send_signal(&mut self, pid: usize, signal: crate::process::Signal) {
        if let Some(process) = self.processes.iter_mut().find(|p| p.pid == pid) {
            process.send_signal(signal);
        }
    }

    /// Get process by ID
    pub fn get_process(&self, pid: usize) -> Option<&Process> {
        self.processes.iter().find(|p| p.pid == pid).map(|v| &**v)
    }

    /// Get process by ID (mutable)
    pub fn get_process_mut(&mut self, pid: usize) -> Option<&mut Process> {
        // Find the index first to avoid borrow conflict
        let idx = self.processes.iter().position(|p| p.pid == pid);
        idx.and_then(move |i| self.processes.get_mut(i).map(|v| &mut **v))
    }
}

/// Global scheduler instance - initialized via init()
use core::sync::atomic::{AtomicBool, Ordering};
static SCHEDULER_INITIALIZED: AtomicBool = AtomicBool::new(false);
static mut SCHEDULER_DATA: Option<Spinlock<Scheduler>> = None;

pub fn get_scheduler() -> &'static Spinlock<Scheduler> {
    if !SCHEDULER_INITIALIZED.load(Ordering::SeqCst) {
        unsafe {
            SCHEDULER_DATA = Some(Spinlock::new(Scheduler::new()));
            SCHEDULER_INITIALIZED.store(true, Ordering::SeqCst);
        }
    }
    unsafe { SCHEDULER_DATA.as_ref().unwrap() }
}

/// Initialize the scheduler
pub fn init() {
    let scheduler = get_scheduler().lock();
    // Create init process (PID 1)
    // spawn_kernel_task takes &mut self, but we need to drop the lock first
    drop(scheduler);
    let mut sched = get_scheduler().lock();
    sched.spawn_kernel_task(0); // Placeholder entry point
}

/// Timer tick - called on each timer interrupt
pub fn timer_tick() {
    let mut scheduler = get_scheduler().lock();
    scheduler.tick_count += 1;
}

/// Schedule - switch to next process
pub fn schedule(_regs: u32) -> u32 {
    0
}

