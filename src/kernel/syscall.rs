use core::arch::asm;

// --- Dispatcher ---

/// The main syscall dispatcher.
/// This is called by the assembly handler after registers have been pushed.
/// We use the C calling convention (cdecl) where arguments are passed on the stack.
///
/// Arguments correspond to registers:
/// eax: Syscall Number
/// ebx, ecx, edx: Arguments 1, 2, 3
#[no_mangle]
pub extern "C" fn syscall_dispatcher(
    esp: usize,
    eax: usize,
    ebx: usize,
    ecx: usize,
    edx: usize
) -> usize {
    let result = match eax {
        0 => {
            // Syscall 0: Yield
            return crate::kernel::process::yield_now(esp);
        }
        1 => {
            // Syscall 1: Print (ebx = ptr, ecx = len)
            let ptr = ebx as *const u8;
            let len = ecx;
            
            // Safety: We blindly trust the user pointer/len for now. 
            // In a real kernel, we must validate this memory range!
            let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
            
            if let Ok(s) = core::str::from_utf8(slice) {
                crate::serial_print!("{}", s);
            }
            len // Return bytes written
        }
        2 => {
            // Syscall 2: Exec (ebx = ptr, ecx = len)
            // Loads and executes an ELF binary from memory.
            let ptr = ebx as *const u8;
            let len = ecx;
            
            let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
            if crate::kernel::elf::load_and_run(slice) {
                1 // Success
            } else {
                0 // Fail
            }
        }
        3 => {
            // Syscall 3: Set Task Priority (ebx = task_id, ecx = new_priority)
            let task_id = ebx;
            let new_priority = ecx;
            if crate::kernel::process::SCHEDULER.lock().set_task_priority(task_id, new_priority) {
                1 // Success
            } else {
                0 // Fail
            }
        }
        4 => {
            // Syscall 4: Get Current Task ID
            crate::kernel::process::SCHEDULER.lock().get_current_task_id()
        }
        5 => {
            // Syscall 5: Sleep (ebx = ms)
            crate::kernel::process::SCHEDULER.lock().sleep_current_task(ebx);
            return crate::kernel::process::yield_now(esp);
        }
        6 => {
            // Syscall 6: Get Task Priority (ebx = task_id)
            let task_id = ebx;
            if let Some(priority) = crate::kernel::process::SCHEDULER.lock().get_task_priority(task_id) {
                priority
            } else {
                usize::MAX
            }
        }
        7 => {
            // Syscall 7: Get CPU Usage
            // Returns the current CPU usage percentage (0-100)
            crate::kernel::cpu::CPU_USAGE.load(core::sync::atomic::Ordering::Relaxed)
        }
        _ => {
            crate::serial_println!("Unknown Syscall: {}", eax);
            usize::MAX
        }
    };

    // For non-yielding syscalls, write the result to the EAX slot on the stack
    unsafe {
        *((esp + 28) as *mut usize) = result;
    }
    esp
}

// --- Assembly Handler ---

// This assembly stub handles the interrupt 0x80.
// It saves the processor state, sets up arguments for the Rust dispatcher,
// calls it, and then restores the state (placing the return value in EAX).
core::arch::global_asm!(
    ".global syscall_handler",
    "syscall_handler:",
    // Save Segment Registers
    "push ds",
    "push es",
    "push fs",
    "push gs",
    // 2. Save all general purpose registers
    // Pushes: EAX, ECX, EDX, EBX, ESP, EBP, ESI, EDI
    "pusha",

    // 3. Load Kernel Segments
    "mov ax, 0x10",
    "mov ds, ax",
    "mov es, ax",

    // 4. Prepare arguments for syscall_dispatcher(eax, ebx, ecx, edx)
    // After `pusha`, ESP points to the saved registers. We use it as a base.
    "mov ebp, esp",

    // The order for `pusha` is eax, ecx, edx, ebx, esp, ebp, esi, edi.
    // We push arguments right-to-left for the C calling convention.
    "push [ebp + 20]",  // Arg 3: edx
    "push [ebp + 24]",  // Arg 2: ecx
    "push [ebp + 16]",  // Arg 1: ebx
    "push [ebp + 28]",  // Arg 0: eax
    "push ebp",         // Arg: current_esp

    // 5. Call the dispatcher
    "call syscall_dispatcher",

    // 6. Cleanup arguments (5 * 4 bytes)
    "add esp, 20",

    // 7. Switch stack to the returned ESP (allows yielding to new tasks)
    "mov esp, eax",

    // 8. Restore all registers and return from interrupt
    "popa",
    "pop gs",
    "pop fs",
    "pop es",
    "pop ds",
    "iretd"
);

extern "C" {
    pub fn syscall_handler();
}