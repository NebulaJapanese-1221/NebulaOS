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
    Exited,
    #[allow(dead_code)]
    Blocked,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TraceEntry {
    pub task_id: usize,
    pub eip: usize,
}

pub struct TraceBuffer {
    pub entries: [TraceEntry; 10],
    pub index: usize,
}

impl TraceBuffer {
    pub const fn new() -> Self {
        Self {
            entries: [TraceEntry { task_id: 0, eip: 0 }; 10],
            index: 0,
        }
    }

    pub fn record(&mut self, task_id: usize, eip: usize) {
        self.entries[self.index] = TraceEntry { task_id, eip };
        self.index = (self.index + 1) % 10;
    }
}

pub static KERNEL_TRACE: IrqSafeMutex<TraceBuffer> = IrqSafeMutex::new(TraceBuffer::new());

pub fn record_context_switch(task_id: usize, esp: usize) {
    // Offset 52 is EIP in our common interrupt/syscall/task stack frame
    let eip = unsafe { *((esp + 52) as *const usize) };
    KERNEL_TRACE.lock().record(task_id, eip);
}

pub fn print_kernel_trace() {
    let trace = KERNEL_TRACE.lock();
    crate::serial_println!("\n[TRACE] Last 10 Context Switches (Oldest to Newest):");
    for i in 0..10 {
        let idx = (trace.index + i) % 10;
        let entry = &trace.entries[idx];
        if entry.eip != 0 {
            crate::serial_println!("  #{:02} -> Task {}: EIP 0x{:08x}", i, entry.task_id, entry.eip);
        }
    }
}

#[repr(C)]
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
    last_tsc: u64,
    initialized: bool,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current_index: 0, // This will be updated on first schedule
            last_tsc: 0,
            initialized: false,
        }
    }

    /// Creates a new task jumping to the given entry point.
    /// The entry_point function will be called by a wrapper that handles task exit.
    pub fn add_task(&mut self, entry_point: usize, priority: usize) {
        let id = self.tasks.len();
        
        // Allocate a kernel stack for this task
        let stack_size = 16384;
        let mut stack = Vec::with_capacity(stack_size);
        stack.resize(stack_size, 0);
        
        // Calculate the top of the stack (high address)
        // Ensure 16-byte alignment for real hardware (SSE/Alignment safety)
        let stack_top = (stack.as_ptr() as usize + stack_size) & !0xF;
        let mut sp = stack_top;

        unsafe {
            // Helper to push a value onto the stack
            let push = |val: usize, sp_ptr: &mut usize| {
                *sp_ptr -= 4;
                *(*sp_ptr as *mut usize) = val;
            };

            // 1. IRET Frame
            push(0x202, &mut sp);      // EFLAGS (Interrupts Enabled)
            push(0x08, &mut sp);       // CS (Kernel Code Segment)
            push(task_entry_wrapper as *const () as usize, &mut sp); 

            // 2. Error Code / Dummy
            push(0, &mut sp);

            // 3. Segment Registers
            push(0x10, &mut sp); // DS
            push(0x10, &mut sp); // ES
            push(0x10, &mut sp); // FS
            push(0x10, &mut sp); // GS (Lowest address)

            // 4. General Purpose Registers (pusha)
            push(entry_point, &mut sp); // EAX (Entry point for wrapper)
            for _ in 0..7 { push(0, &mut sp); } // ECX, EDX, EBX, ESP, EBP, ESI, EDI
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
            usize::MAX 
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
    #[allow(dead_code)]
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
        let total_tasks = self.tasks.len();
        if total_tasks == 0 { return; }

        // Find the maximum priority level among all ready tasks
        let mut max_priority = 0;
        for task in &self.tasks {
            if task.state == TaskState::Ready && task.priority > max_priority {
                max_priority = task.priority;
            }
        }

        // Round-robin selection among tasks with max_priority
        let start_search = (self.current_index + 1) % total_tasks;
        for i in 0..total_tasks {
            let idx = (start_search + i) % total_tasks;
            let task = &self.tasks[idx];

            if task.state == TaskState::Ready && task.priority == max_priority {
                self.current_index = idx;
                break;
            }
        }
    }
}

pub static SCHEDULER: IrqSafeMutex<Scheduler> = IrqSafeMutex::new(Scheduler::new());
pub static TICKS: AtomicUsize = AtomicUsize::new(0);

/// Wrapper that executes the task and handles cleanup on return.
#[no_mangle]
pub extern "C" fn task_entry_wrapper() {
    let entry_point: usize;
    unsafe {
        asm!("mov {}, eax", out(reg) entry_point);
        let func: extern "C" fn() = core::mem::transmute(entry_point);
        func();
    }
    task_exit_handler();
}

/// Handler called when a task's entry point returns.
/// Marks the task as exited and yields forever until reaped.
pub extern "C" fn task_exit_handler() {
    let mut scheduler = SCHEDULER.lock();
    let current = scheduler.current_index;
    if current < scheduler.tasks.len() {
        scheduler.tasks[current].state = TaskState::Exited;
    }
    drop(scheduler);
    loop { unsafe { asm!("int 0x80", in("eax") 0usize); } }
}

/// The Idle Task: Runs when no other tasks are ready.
pub extern "C" fn idle_task() {
    loop {
        crate::kernel::cpu::IS_IDLE.store(true, Ordering::Relaxed);
        unsafe { asm!("hlt", options(nomem, nostack)); }
        crate::kernel::cpu::IS_IDLE.store(false, Ordering::Relaxed);
    }
}

/// Voluntary task yield. Saves current state and switches to the next task.
pub fn yield_now(current_esp: usize) -> usize {
    let mut scheduler = SCHEDULER.lock();
    if scheduler.tasks.is_empty() { return current_esp; }

    // Save result 0 for the yielding task (EAX slot in pusha frame)
    unsafe { *((current_esp + 28) as *mut usize) = 0; }

    let now = crate::kernel::cpu::read_tsc();
    if scheduler.last_tsc > 0 && now > scheduler.last_tsc {
        let delta = now - scheduler.last_tsc;
        let was_idle = crate::kernel::cpu::IS_IDLE.load(Ordering::Relaxed);
        crate::kernel::cpu::accumulate_usage(delta, was_idle);
    }
    scheduler.last_tsc = now;

    let current_idx = scheduler.current_index;
    scheduler.tasks[current_idx].kernel_esp = current_esp;

    scheduler.pick_next();

    let next_task = &scheduler.tasks[scheduler.current_index];
    record_context_switch(next_task.id, next_task.kernel_esp);

    if !next_task.kernel_stack.is_empty() {
        let kstack_top = next_task.kernel_stack.as_ptr() as usize + next_task.kernel_stack.len();
        crate::kernel::gdt::set_interrupt_stack(kstack_top as u32);
    }
    next_task.kernel_esp
}

/// Called by the assembly timer handler. 
/// Updates the scheduler and returns the ESP of the next task.
#[no_mangle]
pub extern "C" fn schedule(current_esp: usize) -> usize {
    // 1. Handle Timer Logic
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe { io::outb(0x20, 0x20); } // Send EOI

    // Update RTC/System time after EOI to prevent hardware bus stalls
    rtc::handle_timer_tick();

    let mut scheduler = SCHEDULER.lock();

    // 1. Promote/Initialize Kernel Task if needed
    if !scheduler.initialized {
        let main_id = scheduler.tasks.len();
        scheduler.tasks.push(Task {
            id: main_id,
            kernel_stack: Vec::new(), // The bootloader/kernel stack is managed externally
            kernel_esp: current_esp,
            priority: 10,
            state: TaskState::Ready,
            sleep_until: None,
        });

        scheduler.current_index = main_id;
        scheduler.initialized = true;
        scheduler.last_tsc = crate::kernel::cpu::read_tsc();
    }

    // 2. Save ESP of the task we are switching FROM immediately
    // We do this before reaping to ensure current_index is still valid
    let task_count = scheduler.tasks.len();
    if scheduler.current_index < task_count {
        let current_idx = scheduler.current_index;
        scheduler.tasks[current_idx].kernel_esp = current_esp;
    }

    let current_ticks = TICKS.load(Ordering::Relaxed);

    // 3. Reap inactive exited tasks
    let mut i = 0;
    while i < scheduler.tasks.len() {
        // Protection: Never reap Task 0 or the task we just saved (the current one)
        let is_current = i == scheduler.current_index;
        if scheduler.tasks[i].state == TaskState::Exited && scheduler.tasks[i].id != 0 && !is_current {
            scheduler.tasks.remove(i);
            if i < scheduler.current_index {
                scheduler.current_index -= 1;
            }
        } else {
            i += 1;
        }
    }

    // 4. Wake up sleepers
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

    // --- CPU Usage Calculation ---
    let now = crate::kernel::cpu::read_tsc();
    if scheduler.last_tsc > 0 && now > scheduler.last_tsc {
        let delta = now - scheduler.last_tsc;
        let was_idle = crate::kernel::cpu::IS_IDLE.load(Ordering::Relaxed);
        crate::kernel::cpu::accumulate_usage(delta, was_idle);
    }
    scheduler.last_tsc = now;

    // 5. Select Next Task
    scheduler.pick_next();

    // 6. Restore Next Task
    let next_task = &scheduler.tasks[scheduler.current_index];
    
    record_context_switch(next_task.id, next_task.kernel_esp);

    // Update TSS only if the task has a managed stack (User/New Kernel tasks)
    if !next_task.kernel_stack.is_empty() {
        let kstack_top = next_task.kernel_stack.as_ptr() as usize + next_task.kernel_stack.len();
        crate::kernel::gdt::set_interrupt_stack(kstack_top as u32);
    }
    
    next_task.kernel_esp
}