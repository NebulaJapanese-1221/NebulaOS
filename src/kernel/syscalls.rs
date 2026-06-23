use crate::serial_println;
use crate::process::ProcessState;
use crate::allocator::ALLOCATOR; // Import the allocator
use alloc::string::ToString; // For .to_string()
use alloc::vec::Vec; // If Vec is used for syscall arguments etc.
use core::arch::asm;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct SyscallRegisters {
    // Saved by assembly stub: push ds, es, fs, gs
    pub gs: u32, pub fs: u32, pub es: u32, pub ds: u32,
    // Saved by assembly stub: pushal (edi, esi, ebp, esp, ebx, edx, ecx, eax)
    pub edi: u32, pub esi: u32, pub ebp: u32, pub kernel_esp: u32, // kernel_esp is top of kernel stack
    pub ebx: u32, pub edx: u32, pub ecx: u32, pub eax: u32,
    // Saved by hardware (always present for interrupts/exceptions)
    pub eip: u32, pub cs: u32, pub eflags: u32,
    // Saved by hardware for inter-privilege level transfer (interrupts from user to kernel)
    pub esp: u32, // User-mode stack pointer
    pub ss: u32,  // User-mode segment selector
}

impl SyscallRegisters {
    pub fn is_user(&self) -> bool {
        (self.cs & 0x3) == 3 // Check the DPL bit of the code segment selector
    }

    pub fn get_user_esp(&self) -> u32 {
        if self.is_user() {
            self.esp // ESP is pushed by the CPU on inter-privilege transfer
        } else {
            self.kernel_esp // Use the saved kernel stack pointer if from kernel mode
        }
    }
}

pub fn syscall_handler_rust(regs_ptr: &mut SyscallRegisters) -> u32 {
    // Use a local copy to avoid unaligned access issues with packed struct
    // and to modify registers before writing back.
    let mut regs = *regs_ptr; 
    let eax = regs.eax;

    // Debugging unknown syscalls
    if eax != 0 && eax != 1 && eax != 2 && eax != 3 && eax != 4 && eax != 5 && eax != 6 {
        serial_println!("DEBUG SYSCALL: ID={} (User={})", eax, regs.is_user());
    }

    let mut return_val = regs_ptr as *mut _ as u32; // Default return is current regs pointer

    match eax {
        0 => { // Syscall 0: Yield
            return_val = crate::scheduler::schedule(regs_ptr as *mut _ as u32);
        },
        1 => { // Syscall 1: Print to Serial (Kernel only for now)
            serial_println!("Syscall: Kernel received request to print!");
        },
        2 => { // Syscall 2: Get System Time
            let time = crate::rtc::get_time();
            regs.ebx = time.hour as u32;
            regs.ecx = time.minute as u32;
            regs.edx = time.second as u32;
        },
        3 => { // Syscall 3: Draw Pixel
            // ebx: x, ecx: y, edx: color
            let x = regs.ebx as usize;
            let y = regs.ecx as usize;
            let color = regs.edx;
            // TODO: Add bounds checking and potentially user memory validation.
            crate::framebuffer::FRAMEBUFFER.lock().draw_pixel(x, y, color);
        },
        4 => { // Syscall 4: Sleep
            // ebx: milliseconds
            let ms = regs.ebx as usize;
            let ticks_to_sleep = ms / 10; // Assuming 100Hz PIT = 10ms per tick
            
            let mut sched = crate::scheduler::SCHEDULER.lock();
            if regs.is_user() { // Only allow user processes to sleep
                if let Some(pid) = sched.current_pid {
                    let wakeup_tick = sched.ticks + ticks_to_sleep;
                    if pid < sched.processes.len() {
                        sched.processes.as_mut_slice()[pid].state = ProcessState::Sleeping(wakeup_tick);
                    }
                }
            }
            drop(sched); // Release lock before yielding
            return_val = crate::scheduler::schedule(regs_ptr as *mut _ as u32);
        },
        5 => { // Syscall 5: Exit Process
            if regs.is_user() { // Only allow user processes to exit
                return_val = crate::scheduler::exit_current_process(regs_ptr as *mut _ as u32);
            } else {
                serial_println!("Kernel tried to exit via syscall!");
            }
        },
        6 => { // Syscall 6: Spawn (Exec) New Process
            // ebx: entry point address (virtual address)
            if regs.is_user() { // Only allow user mode to spawn for now
                 let entry_point = regs.ebx;
                 let user_kernel_stack_size = 4096; // Default sizes
                 let user_stack_size = 4096 * 4; // 16KB user stack

                 // NOTE: This currently uses the kernel's page directory.
                 // A proper exec would load a new program and create a new page directory.
                 let new_pid = {
                    let mut sched = crate::scheduler::SCHEDULER.lock();
                    sched.spawn_user_process(entry_point, user_stack_size, user_kernel_stack_size)
                 };
                 serial_println!("Spawned new user process with PID: {}", new_pid);
            }
        },
        _ => {
            serial_println!("Unknown syscall: {}", eax);
        }
    }
    // Write the (potentially modified) registers back to the original pointer
    *regs_ptr = regs;
    return_val
}

#[allow(dead_code)]
pub fn test_syscall() {
    unsafe {
        core::arch::asm!(
            "mov eax, 1", // Example: Syscall 1 (Print)
            "int 0x80",
            out("eax") _,
            options(nostack, preserves_flags)
        );
    }
}

#[allow(dead_code)]
pub fn syscall_exec(entry_point: u32) {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 6,
            in("ebx") entry_point,
        );
    }
}

#[allow(dead_code)]
pub fn syscall_sleep(ms: u32) {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 4,
            in("ebx") ms,
        );
    }
}

#[allow(dead_code)]
pub fn syscall_exit() -> ! {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 5,
            options(noreturn)
        );
    }
}

#[allow(dead_code)]
pub fn syscall_yield() {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 0,
            options(nostack, preserves_flags)
        );
    }
}

#[allow(dead_code)]
pub fn syscall_get_time() -> (u32, u32, u32) {
    let h: u32; let m: u32; let s: u32;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            inout("eax") 2 => _,
            out("ebx") h,
            out("ecx") m,
            out("edx") s,
            options(nostack, preserves_flags)
        );
    }
    (h, m, s)
}

#[allow(dead_code)]
pub fn syscall_draw_pixel(x: u32, y: u32, color: u32) {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 3,
            in("ebx") x,
            in("ecx") y,
            in("edx") color,
            options(nostack, preserves_flags)
        );
    }
}