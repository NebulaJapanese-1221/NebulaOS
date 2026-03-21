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
    eax: usize,
    ebx: usize,
    ecx: usize,
    _edx: usize
) -> usize {
    match eax {
        0 => {
            // Syscall 0: Yield / Sleep
            unsafe { asm!("hlt") };
            0
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
        _ => {
            crate::serial_println!("Unknown Syscall: {}", eax);
            usize::MAX
        }
    }
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

    // 5. Call the dispatcher
    "call syscall_dispatcher",

    // 6. Cleanup arguments (4 * 4 bytes)
    "add esp, 16",

    // 7. Save the return value into the EAX slot on the stack so `popa` restores it.
    "mov [ebp + 28], eax",

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