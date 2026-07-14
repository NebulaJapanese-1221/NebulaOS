// Scheduler for NebulaOS
// Enhanced with thread support

use alloc::collections::VecDeque;
use alloc::boxed::Box;
use spin::Mutex;
use x86_64::structures::paging::{PageTable, Mapper, Size4KiB, FrameAllocator};
use x86_64::{VirtAddr, PhysAddr};
use crate::process::{Process, ProcessState, Thread};
use core::sync::atomic::{AtomicUsize, Ordering};

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
    pub fn spawn_kernel_task(&mut self, entry_point: u32) -> usize {
        let pid = self.processes.len() + 1;
        let mut process = Process::new_kernel_task(pid as usize, entry_point);

        // Create a main thread
        let _thread = process.create_thread(entry_point, 4096);

        self.processes.push_back(process);
        pid
    }

    /// Spawn a new user process
    pub fn spawn_user_process(
        &mut self,
        entry_point: u32,
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
        self.processes.push_back(process);
        pid
    }

    /// Create a new thread in an existing process
    pub fn create_thread(
        &mut self,
        pid: usize,
        entry: u32,
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

        // Round-robin scheduling
        let mut attempts = 0;
        while attempts < self.processes.len() {
            self.current_process_index = (self.current_process_index + 1) % self.processes.len();

            if let Some(process) = self.processes.get_mut(self.current_process_index) {
                // Check if process has any runnable threads
                if process.threads.iter().any(|t| t.state == ProcessState::Ready) {
                    return Some(process);
                }
            }

            attempts += 1;
        }

        None
    }

    /// Get the current process
    pub fn current_process(&self) -> Option<&Process> {
        self.processes.get(self.current_process_index)
    }

    /// Get the current process (mutable)
    pub fn current_process_mut(&mut self) -> Option<&mut Process> {
        self.processes.get_mut(self.current_process_index)
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
    pub fn exit_current(&mut self, exit_code: i32) {
        if let Some(process) = self.processes.remove(self.current_process_index) {
            // Clean up resources
            // Reparent children to init process
            for &child_pid in &process.children {
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
        self.processes.iter().find(|p| p.pid == pid)
    }

    /// Get process by ID (mutable)
    pub fn get_process_mut(&mut self, pid: usize) -> Option<&mut Process> {
        self.processes.iter_mut().find(|p| p.pid == pid)
    }
}

/// Global scheduler instance
pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

/// Initialize the scheduler
pub fn init() {
    // Create init process (PID 1)
    let mut scheduler = SCHEDULER.lock();
    scheduler.spawn_kernel_task(0); // Placeholder entry point
}
