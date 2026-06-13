use crate::syscalls::SyscallRegisters;
use alloc::boxed::Box;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ProcessState {
    Ready,
    Running,
    Sleeping(usize),
    #[allow(dead_code)]
    Dead,
}

#[allow(dead_code)]
pub struct Process {
    pub id: usize,
    pub kernel_stack_ptr: u32,
    pub state: ProcessState,
    pub stack: [u8; 4096],
}

impl Process {
    #[allow(dead_code)]
    pub fn new(id: usize, entry_point: u32) -> Box<Self> {
        let mut p = Box::new(Self {
            id,
            kernel_stack_ptr: 0,
            state: ProcessState::Ready,
            stack: [0; 4096],
        });

        // Calculate the address of the top of the stack and place initial registers there
        let stack_top = p.stack.as_ptr() as usize + 4096;
        let regs_ptr = (stack_top - core::mem::size_of::<SyscallRegisters>()) as *mut SyscallRegisters;

        unsafe {
            let regs = &mut *regs_ptr;
            core::ptr::write(regs, core::mem::zeroed());
            regs.eip = entry_point;
            regs.cs = 0x08; // Kernel code segment
            regs.ds = 0x10; regs.es = 0x10; regs.fs = 0x10; regs.gs = 0x10;
            regs.eflags = 0x202; // IF set
            regs.kernel_esp = regs_ptr as u32;
        }

        p.kernel_stack_ptr = regs_ptr as u32;
        p
    }
}