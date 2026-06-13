use crate::serial_println;
use crate::process::ProcessState;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct SyscallRegisters {
    // Saved by assembly stub: push ds, es, fs, gs
    pub gs: u32, pub fs: u32, pub es: u32, pub ds: u32,
    // Saved by assembly stub: pushal (edi, esi, ebp, esp, ebx, edx, ecx, eax)
    pub edi: u32, pub esi: u32, pub ebp: u32, pub kernel_esp: u32,
    pub ebx: u32, pub edx: u32, pub ecx: u32, pub eax: u32,
    // Saved by hardware (always present)
    pub eip: u32, pub cs: u32, pub eflags: u32,
}

impl SyscallRegisters {
    /// Returns true if the interrupt came from user mode (Ring 3)
    pub fn is_user(&self) -> bool {
        (self.cs & 0x3) == 3
    }

    /// Safely gets the user-mode stack pointer if applicable
    pub fn get_user_esp(&self) -> u32 {
        if self.is_user() {
            unsafe { *((self as *const _ as *const u32).add(15)) }
        } else {
            self.kernel_esp
        }
    }
}

pub fn syscall_handler_rust(regs_ptr: &mut SyscallRegisters) -> u32 {
    // Create a local aligned copy to avoid unaligned access on the packed struct
    let mut regs = *regs_ptr;
    let eax = regs.eax;

    if eax != 1 && eax != 0 && eax != 2 {
        serial_println!("DEBUG SYSCALL: ID={} (User={})", eax, regs.is_user());
    }

    match eax {
        0 => {
            // Syscall 0: Yield
            return crate::scheduler::schedule(regs_ptr as *mut _ as u32);
        },
        1 => {
            // Example Syscall 1: Print to Serial
            // ebx could be a pointer to a string (in a real OS with paging)
            serial_println!("Syscall: Kernel received request to print!");
        },
        2 => {
            // Syscall 2: Get System Time
            let time = crate::rtc::get_time();
            regs.ebx = time.hour as u32;
            regs.ecx = time.minute as u32;
            regs.edx = time.second as u32;
        },
        3 => {
            // Syscall 3: Draw Pixel
            // ebx: x, ecx: y, edx: color
            let x = regs.ebx as usize;
            let y = regs.ecx as usize;
            let color = regs.edx;
            crate::framebuffer::FRAMEBUFFER.lock().draw_pixel(x, y, color);
        },
        4 => {
            // Syscall 4: Sleep
            // ebx: milliseconds
            let ms = regs.ebx as usize;
            let ticks_to_sleep = ms / 10; // 100Hz = 10ms per tick
            
            let mut sched = crate::scheduler::SCHEDULER.lock();
            if let Some(pid) = sched.current_pid {
                let wakeup_tick = sched.ticks + ticks_to_sleep;
                sched.processes.as_mut_slice()[pid].state = ProcessState::Sleeping(wakeup_tick);
            }
            drop(sched); // Release lock before yielding
            return crate::scheduler::schedule(regs_ptr as *mut _ as u32);
        },
        5 => {
            // Syscall 5: Exit
            return crate::scheduler::exit_current_process(regs_ptr as *mut _ as u32);
        },
        6 => {
            // Syscall 6: Exec/Spawn
            // ebx: entry point address
            crate::scheduler::SCHEDULER.lock().spawn(regs.ebx);
        },
        _ => {
            serial_println!("Unknown syscall: {}", eax);
        }
    }
    // Write the (potentially modified) registers back to the original pointer
    *regs_ptr = regs;
    regs_ptr as *mut _ as u32
}

/// Helper function to trigger a syscall from kernel-land (for testing)
pub fn test_syscall() {
    unsafe {
        core::arch::asm!(
            "mov eax, 1",
            "int 0x80",
            out("eax") _,
            options(nostack, preserves_flags)
        );
    }
}

/// Userspace-style wrapper to spawn a new process at the given entry point
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

/// Userspace-style wrapper to sleep for a duration
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

/// Userspace-style wrapper to exit the current process
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

/// Userspace-style wrapper to yield the current time slice
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

/// Userspace-style wrapper to get time via syscall
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

/// Userspace-style wrapper to draw a pixel via syscall
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