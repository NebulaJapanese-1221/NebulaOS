use crate::syscalls::SyscallRegisters;
use crate::memory::paging::{PageEntry, PAGE_WRITABLE, PAGE_USER, PAGE_PRESENT, PAGE_SIZE}; // Import paging structures
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::arch::asm;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ProcessState {
    Ready,
    Running,
    Sleeping(usize),
    #[allow(dead_code)]
    Dead,
}

// Adjusted to include page directory and user stack info
pub struct Process {
    pub id: usize,
    pub state: ProcessState,
    pub kernel_stack_ptr: u32, // Pointer to the current top of the kernel stack for this process
    pub stack: [u8; 4096],   // Kernel stack for this process

    pub page_directory_phys_addr: u32, // Physical address of this process's page directory
    pub user_stack_base: u32,          // Base virtual address of the user stack
    pub user_eip: u32,                 // Entry point for user code
}

impl Process {
    // This `new` function will need to be more sophisticated to set up a user process context.
    // For now, it's still largely kernel-focused but includes paging fields.
    #[allow(dead_code)]
    pub fn new_kernel_task(id: usize, entry_point: u32) -> Box<Self> {
        let mut p = Box::new(Self {
            id,
            state: ProcessState::Ready,
            kernel_stack_ptr: 0,
            stack: [0; 4096],

            // For kernel tasks, use the kernel's page directory.
            page_directory_phys_addr: crate::memory::paging::get_kernel_page_directory_phys_addr(),
            user_stack_base: 0, // Not applicable for kernel tasks
            user_eip: 0,        // Not applicable for kernel tasks
        });

        // Calculate the address of the top of the stack for initial registers
        let stack_top = p.stack.as_ptr() as usize + 4096;
        let regs_ptr = (stack_top - core::mem::size_of::<SyscallRegisters>()) as *mut SyscallRegisters;

        unsafe {
            let regs = &mut *regs_ptr;
            core::ptr::write(regs, core::mem::zeroed());
            regs.eip = entry_point;
            regs.cs = 0x08; // Kernel code segment
            regs.ds = 0x10; regs.es = 0x10; regs.fs = 0x10; regs.gs = 0x10;
            regs.eflags = 0x202; // IF set
            regs.kernel_esp = regs_ptr as u32; // Store the kernel stack pointer for this process
        }

        p.kernel_stack_ptr = regs_ptr as u32;
        p
    }

    /// Creates a new process for user-space execution.
    /// Requires: entry_point, user_stack_size, kernel_stack_size
    #[allow(dead_code)]
    pub fn new_user_process(
        id: usize,
        entry_point: u32, // Virtual address of the user program's entry point
        user_stack_size: usize,
        kernel_stack_size: usize,
        // In a real scenario, you'd also pass program data/sections here.
    ) -> Box<Self> {
        // 1. Allocate physical memory for the page directory.
        //    This requires a physical memory allocator. For now, placeholder.
        let page_directory_phys_addr = crate::memory::paging::create_user_page_directory();
        // TODO: Allocate actual physical memory for this page directory.

        // 2. Allocate kernel stack for this process.
        let kernel_stack_base = unsafe {
            let layout = alloc::alloc::Layout::from_size_align(kernel_stack_size, 16).unwrap();
            let ptr = crate::allocator::ALLOCATOR.alloc(layout) as u32; // Use kernel heap
            ptr + kernel_stack_size as u32 // Stack grows downwards
        };

        // 3. Allocate user stack.
        let user_stack_base = unsafe {
            let layout = alloc::alloc::Layout::from_size_align(user_stack_size, 16).unwrap();
            let ptr = crate::allocator::ALLOCATOR.alloc(layout) as u32; // Use kernel heap
            ptr + user_stack_size as u32 // Stack grows downwards
        };
        // TODO: Map this user stack region in the process's page directory.

        let mut p = Box::new(Self {
            id,
            state: ProcessState::Ready,
            kernel_stack_ptr: kernel_stack_base, // Initial kernel stack top
            stack: [0; 4096], // Kernel stack (can be optimized later)

            page_directory_phys_addr,
            user_stack_base,
            user_eip: entry_point,
        });

        // Setup initial registers on the kernel stack for the first context switch to user mode.
        // This frame will be used by IRETD.
        unsafe {
            let regs_ptr = (p.kernel_stack_ptr as usize - core::mem::size_of::<SyscallRegisters>()) as *mut SyscallRegisters;
            let regs = &mut *regs_ptr;
            core::ptr::write(regs, core::mem::zeroed());

            // User mode context:
            regs.eip = entry_point;
            regs.cs = 0x1B; // User code segment (DPL 3)
            regs.ss = 0x23; // User data segment (DPL 3)
            regs.eflags = 0x202; // IF set, so interrupts are enabled in user mode
            regs.esp = p.user_stack_base; // Set user stack pointer

            // Kernel mode context for when returning from interrupt/syscall:
            regs.gs = 0; regs.fs = 0; regs.es = 0; regs.ds = 0; // Initial kernel segments
            regs.kernel_esp = regs_ptr as u32; // This is the current kernel stack pointer
            
            // Map user stack into the process's page directory (placeholder)
            // TODO: This requires actually setting up page tables.
            // For now, the page directory is just a copy of kernel's.
            // let user_stack_phys = ... // Allocate physical frames for user stack
            // map_page(p.page_directory_phys_addr, p.user_stack_base, user_stack_phys, PAGE_PRESENT | PAGE_WRITABLE | PAGE_USER);

        }

        p.kernel_stack_ptr = regs_ptr as u32; // Update the saved kernel stack pointer
        p
    }
}

// Dummy function to create a user process. Replace with ELF loading.
#[allow(dead_code)]
pub fn create_user_process(
    id: usize,
    entry_point: u32,
    user_stack_size: usize,
    kernel_stack_size: usize,
) -> Box<Process> {
    // This is where you would parse an ELF file, map its sections,
    // and then call `new_user_process` with the correct parameters.
    // For now, we'll just simulate it.
    Process::new_user_process(id, entry_point, user_stack_size, kernel_stack_size)
}
