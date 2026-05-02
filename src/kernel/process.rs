use alloc::vec::Vec;
use spin::Mutex;
use crate::kernel::io;
use crate::drivers::rtc;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Represents the CPU state saved on the stack during a context switch.
/// The layout must match the assembly entry/exit code exactly.
#[cfg(target_arch = "x86")]
#[repr(C, packed)]
struct StackFrame {
    // Pushed by pusha
    edi: u32,
    esi: u32,
    ebp: u32,
    unused_esp: u32,
    ebx: u32,
    edx: u32,
    ecx: u32,
    eax: u32,
    // Pushed manually in timer_handler
    gs: u32,
    fs: u32,
    es: u32,
    ds: u32,
    error_code: u32,
    // Pushed by hardware on interrupt
    eip: u32,
    cs: u32,
    eflags: u32,
}

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

        #[cfg(target_arch = "x86")]
        unsafe {
            // Position the struct at the top of the stack
            let frame_ptr = (stack_top - core::mem::size_of::<StackFrame>()) as *mut StackFrame;
            let frame = &mut *frame_ptr;

            frame.eflags = 0x202; // Interrupts enabled
            frame.cs = crate::kernel::gdt::KERNEL_CODE_SELECTOR as u32;
            frame.eip = entry_point as u32;
            frame.ds = crate::kernel::gdt::KERNEL_DATA_SELECTOR as u32;
            frame.es = frame.ds; frame.fs = frame.ds; frame.gs = frame.ds;
            
            self.tasks.push(Task {
                id,
                kernel_stack: stack,
                kernel_esp: frame_ptr as usize,
            });
        }
    }
}

pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
pub static TICKS: AtomicUsize = AtomicUsize::new(0);

pub fn get_task_count() -> usize {
    SCHEDULER.lock().tasks.len() + 1 // +1 for the kernel idle task
}

/// Called by the assembly timer handler. 
/// Updates the scheduler and returns the ESP of the next task.
#[unsafe(no_mangle)]
pub extern "C" fn schedule(current_esp: usize) -> usize {
    // 1. Handle Timer Logic
    rtc::handle_timer_tick();
    
    let inc = crate::kernel::cpu::get_tick_increment();
    TICKS.fetch_add(inc, Ordering::Relaxed);

    if TICKS.load(Ordering::Relaxed) % 100 == 0 {
        crate::kernel::cpu::update_usage_stats();
    }

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
#[cfg(target_arch = "x86")]
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

unsafe extern "C" { // Added unsafe to extern block
    pub fn timer_handler();
}