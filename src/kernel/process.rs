use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
use crate::kernel::io;
use crate::drivers::rtc;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::arch::asm;

/// A Mutex wrapper that disables interrupts while the lock is held.
/// This prevents deadlocks when a lock is shared between a kernel task and an interrupt handler.
pub struct IrqSafeMutex<T> {
    inner: Mutex<T>,
}

impl<T> IrqSafeMutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            inner: Mutex::new(data),
        }
    }

    pub fn lock(&self) -> IrqSafeGuard<'_, T> {
        let flags: u32;
        unsafe {
            asm!("pushfd; pop {0}", out(reg) flags, options(nomem, nostack));
        }
        let interrupts_enabled = (flags & 0x200) != 0;

        unsafe { asm!("cli", options(nomem, nostack)); }

        IrqSafeGuard {
            guard: self.inner.lock(),
            interrupts_enabled,
        }
    }
}

pub struct IrqSafeGuard<'a, T> {
    guard: MutexGuard<'a, T>,
    interrupts_enabled: bool,
}

impl<'a, T> core::ops::Deref for IrqSafeGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &*self.guard }
}

impl<'a, T> core::ops::DerefMut for IrqSafeGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.guard }
}

impl<'a, T> Drop for IrqSafeGuard<'a, T> {
    fn drop(&mut self) {
        if self.interrupts_enabled {
            unsafe { asm!("sti", options(nomem, nostack)); }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Sleeping,
    Blocked,
}

pub struct Task {
    pub id: usize,
    pub kernel_stack: Vec<u8>,
    pub kernel_esp: usize,
    pub priority: usize,
    pub sleep_until: Option<usize>,
    pub state: TaskState,
}

pub struct Scheduler {
    pub tasks: Vec<Task>,
    pub current_index: usize,
    kernel_esp: usize,
    last_tsc: u64,
    initialized: bool,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current_index: 0, // This will be updated on first schedule
            kernel_esp: 0,
            last_tsc: 0,
            initialized: false,
        }
    }

    /// Creates a new task jumping to the given entry point.
    pub fn add_task(&mut self, entry_point: usize, priority: usize) {
        let id = self.tasks.len();
        
        // Allocate a kernel stack for this task
        let stack_size = 8192;
        let mut stack = Vec::with_capacity(stack_size);
        stack.resize(stack_size, 0);
        
        // Calculate the top of the stack (high address)
        let stack_top = stack.as_ptr() as usize + stack_size;
        let mut sp = stack_top;

        unsafe {
            // Helper to push a value onto the stack
            let mut push = |val: usize| {
                sp -= 4;
                *(sp as *mut usize) = val;
            };

            // Setup stack frame to match `timer_handler` expectations (iret context)
            // 1. IRET Frame
            push(0x202);      // EFLAGS (Interrupts Enabled)
            push(0x08);       // CS (Kernel Code Segment)
            push(entry_point);// EIP

            // 2. Error Code / Dummy
            push(0);

            // 3. Segment Registers
            push(0x10); // GS
            push(0x10); // FS
            push(0x10); // ES
            push(0x10); // DS

            // 4. General Purpose Registers (pusha)
            // EDI, ESI, EBP, ESP, EBX, EDX, ECX, EAX
            for _ in 0..8 { push(0); }
        }

        self.tasks.push(Task {
            id,
            kernel_stack: stack,
            kernel_esp: sp,
            priority,
            state: TaskState::Ready,
            sleep_until: None,
        });
    }

    /// Sets the priority of a given task.
    /// Returns true if the task was found and priority updated, false otherwise.
    pub fn set_task_priority(&mut self, task_id: usize, new_priority: usize) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.priority = new_priority;
            true
        } else {
            false
        }
    }

    /// Returns the priority of a given task.
    pub fn get_task_priority(&self, task_id: usize) -> Option<usize> {
        self.tasks.iter().find(|t| t.id == task_id).map(|t| t.priority)
    }

    /// Returns the ID of the currently running task.
    pub fn get_current_task_id(&self) -> usize {
        if self.current_index < self.tasks.len() {
            self.tasks[self.current_index].id
        } else {
            usize::MAX // Represents the main kernel loop
        }
    }

    /// Puts the current task to sleep for the specified milliseconds.
    pub fn sleep_current_task(&mut self, ms: usize) {
        if self.current_index < self.tasks.len() {
            let until = TICKS.load(Ordering::Relaxed) + ms;
            let task = &mut self.tasks[self.current_index];
            task.sleep_until = Some(until);
            task.state = TaskState::Sleeping;
        }
    }

    /// Marks the current task as blocked (waiting for I/O).
    pub fn block_current_task(&mut self) {
        if self.current_index < self.tasks.len() {
            self.tasks[self.current_index].state = TaskState::Blocked;
        }
    }

    /// Unblocks a specific task, making it eligible for scheduling again.
    pub fn unblock_task(&mut self, task_id: usize) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.state = TaskState::Ready;
        }
    }

    /// Picks the next task to run based on priority and state.
    fn pick_next(&mut self) {
        let total_user_tasks = self.tasks.len();
        const KERNEL_PRIORITY: usize = 10;

        // Find the maximum priority level among all ready tasks
        let mut max_priority = KERNEL_PRIORITY;
        for task in &self.tasks {
            if task.state == TaskState::Ready && task.priority > max_priority {
                max_priority = task.priority;
            }
        }

        // Round-robin selection among tasks with max_priority
        let start_search = (self.current_index + 1) % (total_user_tasks + 1);
        for i in 0..=(total_user_tasks) {
            let idx = (start_search + i) % (total_user_tasks + 1);
            let (task_prio, is_ready) = if idx < total_user_tasks {
                (self.tasks[idx].priority, self.tasks[idx].state == TaskState::Ready)
            } else {
                (KERNEL_PRIORITY, true)
            };

            if is_ready && task_prio == max_priority {
                self.current_index = idx;
                break;
            }
        }
    }
}

pub static SCHEDULER: IrqSafeMutex<Scheduler> = IrqSafeMutex::new(Scheduler::new());
pub static TICKS: AtomicUsize = AtomicUsize::new(0);

/// Voluntary task yield. Saves current state and switches to the next task.
pub fn yield_now(current_esp: usize) -> usize {
    let mut scheduler = SCHEDULER.lock();
    let total_user_tasks = scheduler.tasks.len();
    if total_user_tasks == 0 { return current_esp; }

    // Save result 0 for the yielding task so the syscall returns 0 from its perspective upon resume
    unsafe { *((current_esp + 28) as *mut usize) = 0; }

    // Update CPU usage tracking
    let now = crate::kernel::cpu::read_tsc();
    if scheduler.last_tsc > 0 && now > scheduler.last_tsc {
        let delta = now - scheduler.last_tsc;
        let is_kernel = scheduler.current_index >= total_user_tasks;
        let was_idle = is_kernel && crate::kernel::cpu::IS_IDLE.load(Ordering::Relaxed);
        crate::kernel::cpu::accumulate_usage(delta, was_idle);
    }
    scheduler.last_tsc = now;

    // Save current ESP
    if scheduler.current_index < total_user_tasks {
        scheduler.tasks[scheduler.current_index].kernel_esp = current_esp;
    } else {
        scheduler.kernel_esp = current_esp;
    }

    scheduler.pick_next();

    // Switch to new stack
    if scheduler.current_index < total_user_tasks {
        let next_task = &scheduler.tasks[scheduler.current_index];
        let kstack_top = next_task.kernel_stack.as_ptr() as usize + next_task.kernel_stack.len();
        crate::kernel::gdt::set_interrupt_stack(kstack_top as u32);
        next_task.kernel_esp
    } else {
        scheduler.kernel_esp
    }
}

/// Called by the assembly timer handler. 
/// Updates the scheduler and returns the ESP of the next task.
#[no_mangle]
pub extern "C" fn schedule(current_esp: usize) -> usize {
    // 1. Handle Timer Logic
    const KERNEL_PRIORITY: usize = 10;

    rtc::handle_timer_tick();
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe { io::outb(0x20, 0x20); } // Send EOI

    let current_ticks = TICKS.load(Ordering::Relaxed);

    let mut scheduler = SCHEDULER.lock();

    // 1.5 Wake up tasks that have finished sleeping
    for task in &mut scheduler.tasks {
        if task.state == TaskState::Sleeping {
            if let Some(until) = task.sleep_until {
                if current_ticks >= until {
                    task.state = TaskState::Ready;
                    task.sleep_until = None;
                }
            }
        }
    }

    let total_user_tasks = scheduler.tasks.len();
    
    // If no tasks, just return current stack (continue running kernel loop)
    if total_user_tasks == 0 {
        return current_esp;
    }

    if !scheduler.initialized {
        // The first time schedule() is called, it's from the main kernel loop.
        // We set the current index to represent this kernel "task".
        scheduler.current_index = total_user_tasks;
        scheduler.initialized = true;
        scheduler.last_tsc = crate::kernel::cpu::read_tsc();
    }

    // --- CPU Usage Calculation ---
    let now = crate::kernel::cpu::read_tsc();
    if scheduler.last_tsc > 0 && now > scheduler.last_tsc {
        let delta = now - scheduler.last_tsc;
        let is_kernel = scheduler.current_index >= total_user_tasks;
        // If we were in the kernel task and the IS_IDLE flag was set, count as idle.
        // Otherwise (User task or Kernel doing GUI work), count as active.
        let was_idle = is_kernel && crate::kernel::cpu::IS_IDLE.load(Ordering::Relaxed);
        crate::kernel::cpu::accumulate_usage(delta, was_idle);
    }
    scheduler.last_tsc = now;

    // 2. Save ESP of the task we are switching FROM
    let current_task_index = scheduler.current_index;
    if current_task_index < total_user_tasks {
        // The current task is a user task
        scheduler.tasks[current_task_index].kernel_esp = current_esp;
    } else {
        // The current task is the main kernel loop
        scheduler.kernel_esp = current_esp;
    }

    // 3. Priority-based Selection
    scheduler.pick_next();

    // 4. Get ESP of the task we are switching TO
    if scheduler.current_index < total_user_tasks {
        // It's a user task. Update TSS and return its ESP.
        let next_task = &scheduler.tasks[scheduler.current_index];
        let kstack_top = next_task.kernel_stack.as_ptr() as usize + next_task.kernel_stack.len();
        crate::kernel::gdt::set_interrupt_stack(kstack_top as u32);
        next_task.kernel_esp
    } else {
        // It's the kernel main loop. Return its saved ESP.
        // No need to update TSS for Ring 0.
        scheduler.kernel_esp
    }
}