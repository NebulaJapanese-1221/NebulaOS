use alloc::vec::Vec;
use spin::Mutex;
use crate::kernel::io;
use crate::drivers::rtc;

pub struct Task {
    pub id: usize,
    pub kernel_stack: Vec<u8>,
    pub kernel_esp: usize,
}

pub struct Scheduler {
    pub tasks: Vec<Task>,
    pub current_index: usize,
    kernel_esp: usize,
    initialized: bool,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current_index: 0, // This will be updated on first schedule
            kernel_esp: 0,
            initialized: false,
        }
    }

}

pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

/// Called by the assembly timer handler. 
/// Updates the scheduler and returns the ESP of the next task.
#[no_mangle]
pub extern "C" fn schedule(current_esp: usize) -> usize {
    // 1. Handle Timer Logic
    rtc::handle_timer_tick();
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
    }

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