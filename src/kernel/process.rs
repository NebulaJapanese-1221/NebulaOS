use alloc::vec::Vec;
use spin::Mutex;
use crate::kernel::io;
use crate::drivers::rtc;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct Task {
    pub id: usize,
    pub kernel_stack: Vec<u8>,
    pub kernel_esp: usize,
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
    pub fn add_task(&mut self, entry_point: usize) {
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
        });
    }
}

pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
pub static TICKS: AtomicUsize = AtomicUsize::new(0);

/// Called by the assembly timer handler. 
/// Updates the scheduler and returns the ESP of the next task.
#[no_mangle]
pub extern "C" fn schedule(current_esp: usize) -> usize {
    // 1. Handle Timer Logic
    rtc::handle_timer_tick();
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe { io::outb(0x20, 0x20); } // Send EOI

    let mut scheduler = SCHEDULER.lock();
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

    // 3. Round Robin to pick the next task
    scheduler.current_index = (scheduler.current_index + 1) % (total_user_tasks + 1);

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

// Naked assembly handler for the Timer Interrupt (IRQ 0)
// We use this instead of x86-interrupt ABI to manually control the stack switching.
core::arch::global_asm!(
    ".global timer_handler",
    "timer_handler:",
    // 1. Save Context
    "push 0",           // Dummy error code for stack alignment consistency
    
    // Save Segment Registers
    "push ds",
    "push es",
    "push fs",
    "push gs",
    
    "pusha",            // Save General Registers (EDI, ESI, EBP, ESP, EBX, EDX, ECX, EAX)

    // 2. Load Kernel Data Segment
    "mov ax, 0x10",
    "mov ds, ax",
    "mov es, ax",
    "mov fs, ax",
    "mov gs, ax",

    // 3. Call Schedule(current_esp)
    "mov eax, esp",     // Pass current ESP as argument
    "push eax",
    "call schedule",    // Returns new ESP in EAX
    "add esp, 4",       // Pop argument

    // 4. Switch Stack
    "mov esp, eax",     // Switch to new task's stack

    // 5. Restore Context
    "popa",             // Restore General Registers
    "pop gs",
    "pop fs",
    "pop es",
    "pop ds",
    "add esp, 4",       // Pop error code
    "iretd"             // Return from interrupt (pops CS, EIP, EFLAGS, [ESP, SS])
);

extern "C" {
    pub fn timer_handler();
}