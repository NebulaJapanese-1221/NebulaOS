use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
use crate::kernel::io;
use crate::drivers::rtc;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::arch::asm;
use crate::kernel::paging::VirtualAddressSpace;

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

    pub fn try_lock(&self) -> Option<IrqSafeGuard<'_, T>> {
        let flags: u32;
        unsafe { asm!("pushfd; pop {0}", out(reg) flags, options(nomem, nostack)); }
        let interrupts_enabled = (flags & 0x200) != 0;

        self.inner.try_lock().map(|guard| {
            unsafe { asm!("cli", options(nomem, nostack)); }
            IrqSafeGuard { guard, interrupts_enabled }
        })
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
    // Safety: Verify ESP is within a reasonable kernel memory range to prevent recursive faults
    if esp < 0x1000 || esp > 0xFFFFFFC0 { return; }

    // Offset 52 is EIP in our common interrupt/syscall/task stack frame
    unsafe {
        let eip = *((esp + 52) as *const usize);
        KERNEL_TRACE.lock().record(task_id, eip);
    }
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
    pub age: usize,
    pub guard_page: usize,
    pub address_space: Option<VirtualAddressSpace>,
    pub heap_start: usize,
    pub heap_limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerMode {
    Booting,
    Running,
}

const MAX_TASKS: usize = 32;

pub struct Scheduler {
    pub tasks: [Option<Task>; MAX_TASKS],
    pub current_index: usize,
    pub mode: SchedulerMode,
    last_tsc: u64,
    initialized: bool,
    pub current_task_start_tick: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        const EMPTY_TASK: Option<Task> = None;
        Self {
            tasks: [EMPTY_TASK; MAX_TASKS],
            current_index: 0, // This will be updated on first schedule
            mode: SchedulerMode::Booting,
            last_tsc: 0,
            initialized: false,
            current_task_start_tick: 0,
        }
    }

    /// Initializes the current execution context as the "Boot" task (Task 0).
    /// This must be called after the heap is ready but before interrupts are enabled.
    pub fn init_boot_task(&mut self, current_esp: usize) {
        if self.initialized { return; }

        self.tasks[0] = Some(Task {
            id: 0,
            kernel_stack: Vec::new(),
            kernel_esp: current_esp,
            priority: 20, // Equal to system tasks to allow co-operative boot
            state: TaskState::Ready,
            sleep_until: None,
            age: 0,
            guard_page: 0,
            address_space: None,
            heap_start: 0,
            heap_limit: 0,
        });
        self.current_index = 0;
        self.initialized = true;
    }

    /// Creates a new task jumping to the given entry point.
    /// The entry_point function will be called by a wrapper that handles task exit.
    pub fn add_task(
        &mut self,
        entry_point: usize,
        priority: usize,
        address_space: Option<VirtualAddressSpace>,
        heap_start: usize,
        heap_limit: usize,
    ) {
        // Find an available slot in the fixed-size array
        let mut slot_idx = None;
        for i in 0..MAX_TASKS {
            if self.tasks[i].is_none() {
                slot_idx = Some(i);
                break;
            }
        }
        
        let idx = slot_idx.expect("Scheduler: Maximum task limit reached!");
        let id = idx; // Use slot index as ID for stable mapping
        
        // Allocate a kernel stack for this task
        let stack_size = 65536;
        // Allocate extra space (3 pages) for: 1 page for Canary, 1 page for Guard, 1 page for Alignment Slop
        let mut stack = Vec::with_capacity(stack_size + 12288);
        stack.resize(stack_size + 12288, 0);
        
        let raw_ptr = stack.as_ptr() as usize;

        // 1. Initialize Stack Canary at the very base of the allocation
        unsafe { core::ptr::write_unaligned(raw_ptr as *mut u32, 0xDEADBEEF); }

        // 2. Align guard page. We ensure it's at least 4KB away from raw_ptr 
        // so that the canary at raw_ptr isn't unmapped if raw_ptr is page-aligned.
        let guard_page = (raw_ptr + 8191) & !4095;
        let usable_stack_bottom = guard_page + 4096;
        
        // Hardware Guard: Unmap the page immediately before the usable stack
        VirtualAddressSpace::kernel().unmap_page(guard_page);

        // Calculate the top of the stack (high address)
        let stack_top = (usable_stack_bottom + stack_size - 64) & !0xF;
        let mut sp = stack_top;

        unsafe {
            // Helper to push a value onto the stack
            let push = |val: usize, sp_ptr: &mut usize| {
                *sp_ptr -= 4;
                *(*sp_ptr as *mut usize) = val;
            };

            if let Some(ref vas) = address_space {
                // User task: Set up for user mode execution
                // 1. Initialize user stack. We map only the top page eagerly to handle 
                // the initial ring transition; the rest is demand-paged.
                let user_stack_top = 0xC0000000;
                let frame = crate::kernel::paging::allocate_frame().expect("User stack top alloc failed");
                vas.map_page(user_stack_top - 4096, frame as usize, crate::kernel::paging::FLAG_PRESENT | crate::kernel::paging::FLAG_WRITABLE | crate::kernel::paging::FLAG_USER);

                // 2. Push User-Mode hardware frame (SS, ESP, EFLAGS, CS, EIP)
                push(0x23, &mut sp);        // SS (User Data Segment + RPL 3)
                push(user_stack_top - 16, &mut sp); // ESP (aligned)
                push(0x3202, &mut sp);      // EFLAGS
                push(0x1B, &mut sp);       // CS (User Code Segment + RPL 3)
                push(entry_point, &mut sp); // EIP (User task entry point)
            } else {
                // Kernel task: Set up for kernel mode execution
                push(0x202, &mut sp);      // EFLAGS (Interrupts Enabled)
                push(0x08, &mut sp);       // CS (Kernel Code Segment)
                push(task_entry_wrapper as *const () as usize, &mut sp); 
            }

            // 2. Error Code / Dummy
            push(0, &mut sp);

            // 3. Segment Registers
            let data_segment = if address_space.is_some() { 0x23 } else { 0x10 }; // User Data Segment + RPL 3 or Kernel Data Segment
            push(data_segment, &mut sp); // DS
            push(data_segment, &mut sp); // ES
            push(data_segment, &mut sp); // FS
            push(data_segment, &mut sp); // GS (Lowest address)

            // 4. General Purpose Registers (pusha)
            if address_space.is_some() {
                push(0, &mut sp); // EAX (User task entry point is now EIP)
            } else {
                push(entry_point, &mut sp); // EAX (Entry point for wrapper)
            }
            for _ in 0..7 { push(0, &mut sp); } // ECX, EDX, EBX, ESP, EBP, ESI, EDI
        }

        self.tasks[idx] = Some(Task {
            id,
            kernel_stack: stack,
            kernel_esp: sp,
            priority,
            state: TaskState::Ready,
            sleep_until: None,
            age: 0,
            guard_page,
            address_space,
            heap_start,
            heap_limit,
        });
    }

    /// Sets the priority of a given task.
    /// Returns true if the task was found and priority updated, false otherwise.
    pub fn set_task_priority(&mut self, task_id: usize, new_priority: usize) -> bool {
        if let Some(task) = self.tasks.iter_mut().flatten().find(|t| t.id == task_id) {
            task.priority = new_priority;
            true
        } else {
            false
        }
    }

    /// Returns the priority of a given task.
    pub fn get_task_priority(&self, task_id: usize) -> Option<usize> {
        self.tasks.iter().flatten().find(|t| t.id == task_id).map(|t| t.priority)
    }

    /// Returns the ID of the currently running task.
    pub fn get_current_task_id(&self) -> usize {
        match &self.tasks[self.current_index] {
            Some(task) => task.id,
            None => usize::MAX,
        }
    }

    /// Puts the current task to sleep for the specified milliseconds.
    pub fn sleep_current_task(&mut self, ms: usize) {
        let idx = self.current_index;
        if let Some(task) = &mut self.tasks[idx] {
            let until = TICKS.load(Ordering::Relaxed) + ms;
            task.sleep_until = Some(until);
            task.state = TaskState::Sleeping;
        }
    }

    /// Marks the current task as blocked (waiting for I/O).
    #[allow(dead_code)]
    pub fn block_current_task(&mut self) {
        let idx = self.current_index;
        if let Some(task) = &mut self.tasks[idx] {
            task.state = TaskState::Blocked;
        }
    }

    /// Unblocks a specific task, making it eligible for scheduling again.
    pub fn unblock_task(&mut self, task_id: usize) {
        if let Some(task) = self.tasks.iter_mut().flatten().find(|t| t.id == task_id) {
            task.state = TaskState::Ready;
        }
    }

    /// Picks the next task to run based on priority and state.
    fn pick_next(&mut self) {
        // 1. Aging: Increment the age of all tasks that are ready to run but waiting.
        for task in self.tasks.iter_mut().flatten() {
            if task.state == TaskState::Ready {
                task.age = task.age.saturating_add(1);
            }
        }

        // 2. Calculate the maximum effective priority level (base priority + age) 
        // among all ready tasks.
        let mut max_effective_priority = 0;
        for slot in &self.tasks {
            if let Some(task) = slot {
                if task.state == TaskState::Ready {
                    // During boot, only allow system tasks (>=20) and the Idle task (0)
                    if self.mode == SchedulerMode::Booting && task.priority > 0 && task.priority < 20 {
                        continue;
                    }

                    let effective = task.priority.saturating_add(task.age);
                    if effective > max_effective_priority {
                        max_effective_priority = effective;
                    }
                }
            }
        }

        if max_effective_priority == 0 { return; }

        // 3. Round-robin selection among tasks that match the highest effective priority.
        let start_search = (self.current_index + 1) % MAX_TASKS;
        for i in 0..MAX_TASKS {
            let idx = (start_search + i) % MAX_TASKS;
            if let Some(task) = &mut self.tasks[idx] {
                if self.mode == SchedulerMode::Booting && task.priority > 0 && task.priority < 20 {
                    continue;
                }

                let effective = task.priority.saturating_add(task.age);
                if task.state == TaskState::Ready && effective == max_effective_priority {
                    task.age = 0; // Reset age for the selected task
                    self.current_index = idx;
                    return;
                }
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
    let idx = scheduler.current_index;
    if let Some(task) = &mut scheduler.tasks[idx] {
        task.state = TaskState::Exited;
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
    let current_idx = scheduler.current_index;

    if let Some(task) = &mut scheduler.tasks[current_idx] {
        // Save result 0 for the yielding task (EAX slot in pusha frame)
        unsafe { *((current_esp + 28) as *mut usize) = 0; }
        task.kernel_esp = current_esp;
    }

    let now = crate::kernel::cpu::read_tsc();
    if scheduler.last_tsc > 0 && now > scheduler.last_tsc {
        let delta = now - scheduler.last_tsc;
        let was_idle = crate::kernel::cpu::IS_IDLE.load(Ordering::Relaxed);
        crate::kernel::cpu::accumulate_usage(delta, was_idle);
    }
    scheduler.last_tsc = now;

    scheduler.pick_next();

    // Switch Address Space
    if let Some(task) = &scheduler.tasks[scheduler.current_index] {
        match &task.address_space {
            Some(vas) => unsafe { vas.switch(); },
            None => unsafe { asm!("mov cr3, {}", in(reg) crate::kernel::paging::get_kernel_pd_ptr()); }
        }
    }

    // Restore Next Task (Copy values out to avoid references into a potentially shifting Vec)
    let (next_esp, next_id, stack_ptr, stack_len, stack_raw_ptr) = {
        let t = scheduler.tasks[scheduler.current_index].as_ref().expect("Scheduler error: Picked None task");
        (t.kernel_esp, t.id, t.kernel_stack.as_ptr() as usize, t.kernel_stack.len(), t.kernel_stack.as_ptr())
    };

    // 5.5 Verify Stack Canary
    if stack_len > 0 {
        let canary = unsafe { core::ptr::read_unaligned(stack_raw_ptr as *const u32) };
        if canary != 0xDEADBEEF {
            drop(scheduler);
            panic!("STACK_CORRUPTION_DETECTED: Task {} canary is {:#x} (expected 0xDEADBEEF)", next_id, canary);
        }
    };

    scheduler.current_task_start_tick = TICKS.load(Ordering::Relaxed);
    record_context_switch(next_id, next_esp);

    if stack_len > 0 {
        let kstack_top = stack_ptr + stack_len;
        crate::kernel::gdt::set_interrupt_stack(kstack_top as u32);
    }
    next_esp
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
    let current_ticks = TICKS.load(Ordering::Relaxed);

    // 0.5 Task Watchdog: Detect CPU Hogs
    if scheduler.initialized {
        let current_idx = scheduler.current_index;
        if let Some(task) = &scheduler.tasks[current_idx] {
            if task.id > 1 && current_ticks.wrapping_sub(scheduler.current_task_start_tick) > 500 {
                let task_id = task.id;
                let frame_ptr = (current_esp + 52) as *const crate::kernel::interrupts::InterruptStackFrame;
                let frame = unsafe { &*frame_ptr };
                drop(scheduler);
                crate::kernel::exceptions::show_exception_screen("TASK_CPU_HOG_WATCHDOG", frame, Some(task_id as u32), None);
            }
        }
    }

    // 2. Save ESP of the task we are switching FROM immediately
    let current_idx = scheduler.current_index;
    if let Some(task) = &mut scheduler.tasks[current_idx] {
        task.kernel_esp = current_esp;
    }

    // 3. Reap inactive exited tasks
    for i in 0..MAX_TASKS {
        if let Some(task) = &scheduler.tasks[i] {
            let is_current = i == scheduler.current_index;
            if task.state == TaskState::Exited && task.id != 0 && !is_current {
                // Restore the guard page so the allocator doesn't fault when freeing the Vec
                if task.guard_page != 0 {
                    VirtualAddressSpace::kernel().map_page(task.guard_page, task.guard_page, crate::kernel::paging::FLAG_PRESENT | crate::kernel::paging::FLAG_WRITABLE);
                }
                scheduler.tasks[i] = None;
            }
        }
    }

    // 4. Wake up sleepers (Flatten skips None variants)
    for task in scheduler.tasks.iter_mut().flatten() {
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
    let old_idx = scheduler.current_index;
    scheduler.pick_next();

    // Reset the watchdog timer if we actually switched to a different task
    if old_idx != scheduler.current_index {
        scheduler.current_task_start_tick = current_ticks;
    }

    // Switch Address Space
    if let Some(task) = &scheduler.tasks[scheduler.current_index] {
        match &task.address_space {
            Some(vas) => unsafe { vas.switch(); },
            None => unsafe { asm!("mov cr3, {}", in(reg) crate::kernel::paging::get_kernel_pd_ptr()); }
        }
    }

    // 6. Restore Next Task (Copy values out to avoid references into a potentially shifting Vec)
    let (next_esp, next_id, stack_ptr, stack_len, stack_raw_ptr) = {
        let next_idx = scheduler.current_index;
        let t = scheduler.tasks[next_idx].as_ref().expect("Scheduler error: Restoring None task");
        (t.kernel_esp, t.id, t.kernel_stack.as_ptr() as usize, t.kernel_stack.len(), t.kernel_stack.as_ptr())
    };

    // 6.5 Verify Stack Canary
    if stack_len > 0 {
        let canary = unsafe { core::ptr::read_unaligned(stack_raw_ptr as *const u32) };
        if canary != 0xDEADBEEF {
            // Use try_lock if available or force release to avoid deadlock during panic print
            panic!("STACK_CORRUPTION_DETECTED: Task {} canary is {:#x} (expected 0xDEADBEEF)", next_id, canary);
        }
    };

    record_context_switch(next_id, next_esp);

    if stack_len > 0 {
        let kstack_top = stack_ptr + stack_len;
        crate::kernel::gdt::set_interrupt_stack(kstack_top as u32);
    }

    next_esp
}