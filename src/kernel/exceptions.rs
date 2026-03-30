use core::arch::{asm, naked_asm};
use core::fmt;
use crate::drivers::framebuffer::{self, FRAMEBUFFER};
use crate::userspace::fonts::font;
use super::interrupts::InterruptStackFrame;

// --- Panic/Exception Screen Implementation ---

pub struct PanicWriter<'a> {
    fb: &'a mut framebuffer::Framebuffer,
    x: isize,
    y: isize,
    start_x: isize,
}

impl<'a> PanicWriter<'a> {
    pub fn new(fb: &'a mut framebuffer::Framebuffer, x: isize, y: isize) -> Self {
        Self { fb, x, y, start_x: x }
    }
}

impl<'a> fmt::Write for PanicWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if c == '\n' {
                self.y += 16;
                self.x = self.start_x;
            } else {
                font::draw_char(self.fb, self.x, self.y, c, 0xFF_FFFFFF, None);
                self.x += 8;
            }
        }
        Ok(())
    }
}

/// A simple wrapper to allow writing formatted strings to the serial port.
struct SerialWriter;
impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Access the hardware directly to bypass LogLevel filtering and prefixes
        let mut port = crate::drivers::serial::SERIAL1.lock();
        for byte in s.bytes() {
            port.write_byte(byte);
        }
        Ok(())
    }
}

/// Walks the stack and prints a stack trace to the serial port.
/// This function is unsafe because it reads from arbitrary memory locations by following the base pointer.
pub unsafe fn print_stack_trace() {
    let mut writer = SerialWriter;
    unsafe { print_stack_trace_to(&mut writer); }
}

/// Walks the stack and prints a stack trace to the provided writer.
/// This function is unsafe because it reads from arbitrary memory locations by following the base pointer.
pub unsafe fn print_stack_trace_to<W: fmt::Write>(writer: &mut W) {
    let rbp: u32;
    unsafe {
        asm!("mov {}, ebp", out(reg) rbp, options(nomem, nostack));
    }

    let _ = writeln!(writer, "\nStack Trace (current RBP: 0x{:08x}):", rbp);

    if rbp == 0 { return; }

    // Sanity check: Ensure RBP is not in null-page or clearly out of physical bounds (assuming max 128MB)
    if rbp < 0x1000 || rbp > 0x08000000 { 
        let _ = writeln!(writer, "  <RBP invalid, stopping trace>");
        return;
    }

    let mut current_rbp = rbp;
    let mut i = 0;

    // Limit to 20 frames to prevent infinite loops in case of stack corruption.
    while current_rbp != 0 && i < 20 {
        // Safety: Verify pointer before dereferencing to prevent recursive panics
        if current_rbp < 0x1000 || current_rbp > 0x07FFFFFC { break; }

        let return_address = unsafe { *(current_rbp as *const u32).add(1) };

        if let Some((name, offset)) = crate::kernel::symbols::resolve(return_address as usize) {
            let _ = writeln!(writer, "  Frame {:02}: 0x{:08x} <{}+{:#x}>", i, return_address, name, offset);
        } else {
            let _ = writeln!(writer, "  Frame {:02}: 0x{:08x} <unknown>", i, return_address);
        }
        
        // The next RBP is the value pointed to by the current RBP.
        let next_rbp = unsafe { *(current_rbp as *const u32) };
        
        // Basic sanity check: Stack usually grows down, so previous frame RBP should be > current RBP
        if next_rbp <= current_rbp && next_rbp != 0 {
             let _ = writeln!(writer, "  <Stack End or Corruption>");
             break;
        }
        current_rbp = next_rbp;
        i += 1;
    }
}

/// Prints a hexadecimal dump of the memory around the stack pointer.
pub fn dump_stack_memory<W: fmt::Write>(writer: &mut W, esp: u32, ebp: u32) {
    // In standard x86 stack frames, EBP points to the saved EBP,
    // and EBP + 4 points to the return address.
    let return_addr_ptr = ebp + 4;
    
    let _ = writeln!(writer, "\nMEMORY_DUMP (ESP: {:#x}):", esp);
    
    // Sanity check: If ESP is null or near-zero, we cannot dump memory safely.
    if esp < 0x1000 {
        let _ = writeln!(writer, "  <Invalid ESP: Memory dump aborted to prevent recursive fault>");
        return;
    }

    let start_addr = (esp & !0xF).saturating_sub(64);
    
    for i in 0..10 {
        let addr = start_addr + (i * 16);
        if addr > 0x07FFFFF0 { break; } // Bound check for 128MB RAM systems

        let _ = write!(writer, "{:08x}: ", addr);
        
        // Hex values (grouped as u32)
        for j in 0..4 {
            let current_ptr = addr + (j * 4);
            
            // Safe dereference check
            let val = if current_ptr >= 0x1000 {
                unsafe { *(current_ptr as *const u32) }
            } else {
                0x00000000
            };

            if current_ptr == return_addr_ptr {
                let _ = write!(writer, "[{:08x}]", val);
            } else {
                let _ = write!(writer, " {:08x} ", val);
            }
        }

        // ASCII representation
        let _ = write!(writer, " |");
        for j in 0..16 {
            let current_ptr = addr + j;
            let byte = if current_ptr >= 0x1000 {
                unsafe { *(current_ptr as *const u8) }
            } else {
                0
            };

            if byte >= 32 && byte <= 126 {
                let _ = write!(writer, "{}", byte as char);
            } else {
                let _ = write!(writer, ".");
            }
        }
        let _ = write!(writer, "|");
        let _ = writeln!(writer, "");
    }
}

pub fn show_exception_screen(name: &str, frame: &InterruptStackFrame, error_code: Option<u32>) -> ! {
    unsafe { asm!("cli", options(nomem, nostack)); }

    // Calculate the ESP at the time of the fault. 
    // In Ring 0, the CPU pushes EFLAGS, CS, and EIP (12 bytes).
    // If an error code exists, it pushes that first (total 16 bytes).
    let faulting_esp = match error_code {
        Some(_) => (frame as *const _ as u32) + 16,
        None => (frame as *const _ as u32) + 12,
    };

    // Capture CPU state immediately
    let (eax, ebx, ecx, edx, esi, edi, ebp): (u32, u32, u32, u32, u32, u32, u32);
    let (cr0, cr2, cr3, cr4): (u32, u32, u32, u32);
    unsafe {
        asm!("mov {0}, eax", out(reg) eax);
        asm!("mov {0}, ebx", out(reg) ebx);
        asm!("mov {0}, ecx", out(reg) ecx);
        asm!("mov {0}, edx", out(reg) edx);
        asm!("mov {0}, esi", out(reg) esi);
        asm!("mov {0}, edi", out(reg) edi);
        asm!("mov {0}, ebp", out(reg) ebp);
        asm!("mov {0}, cr0", out(reg) cr0);
        asm!("mov {0}, cr2", out(reg) cr2);
        asm!("mov {0}, cr3", out(reg) cr3);
        asm!("mov {0}, cr4", out(reg) cr4);
    }

    // Print details to serial port immediately, as drawing to screen might fail or deadlock
    crate::serial_println!("\nCRITICAL SYSTEM ERROR: {}", name);
    if let Some(code) = error_code {
        crate::serial_println!("Error Code: {:#x}", code);
    }
    crate::serial_println!("CONTEXT: IP={:#010x} CS={:#06x} FLAGS={:#010x}", frame.instruction_pointer, frame.code_segment, frame.cpu_flags);
    crate::serial_println!("GPR: EAX={:#x} EBX={:#x} ECX={:#x} EDX={:#x}", eax, ebx, ecx, edx);
    crate::serial_println!("GPR: ESI={:#x} EDI={:#x} EBP={:#x}", esi, edi, ebp);
    crate::serial_println!("CRs: CR0={:#x} CR2={:#x} CR3={:#x} CR4={:#x}", cr0, cr2, cr3, cr4);

    crate::serial_println!("Stack Frame:\n{:#?}", frame);
    unsafe { print_stack_trace(); }
    crate::kernel::process::print_kernel_trace();

    // Force unlock the framebuffer to prevent deadlock if the exception happened while drawing
    unsafe { FRAMEBUFFER.force_unlock(); }
    let mut fb = FRAMEBUFFER.lock();
    fb.clear(0x00_AA0000); // Clean Crimson

    font::draw_string(&mut fb, 30, 30, "!! CRITICAL SYSTEM ERROR !!", 0xFFFFFFFF, None);
    font::draw_string(&mut fb, 30, 60, "NebulaOS has encountered an unrecoverable logic fault.", 0xFFDDDDDD, None);
    font::draw_string(&mut fb, 30, 75, "The system has been halted to preserve data integrity.", 0xFFDDDDDD, None);

    let mut writer = PanicWriter::new(&mut fb, 30, 90);
    
    use core::fmt::Write;
    let _ = writeln!(writer, "Stop Code: {}", name);
    if let Some(code) = error_code {
        let _ = writeln!(writer, "Error Code: {:#x}", code);
    }
    let _ = writeln!(writer, "\nTechnical Information:\n----------------------\nIP: {:#010x}  CS: {:#06x}  FLAGS: {:#010x}", 
        frame.instruction_pointer, frame.code_segment, frame.cpu_flags);
    unsafe { print_stack_trace_to(&mut writer); }
    unsafe { dump_stack_memory(&mut writer, faulting_esp, ebp); }
    
    fb.present();

    loop { unsafe { asm!("hlt") } }
}


// --- Exception Handlers ---

pub extern "x86-interrupt" fn divide_by_zero_handler(frame: &mut InterruptStackFrame) {
    show_exception_screen("DIVIDE BY ZERO", frame, None);
}

pub extern "x86-interrupt" fn debug_handler(frame: &mut InterruptStackFrame) {
    crate::serial_println!("EXCEPTION: DEBUG");
    // Debug exceptions (like single-step) generally resume, but for now we dump info
    crate::serial_println!("{:#?}", frame);
}

pub extern "x86-interrupt" fn non_maskable_interrupt_handler(frame: &mut InterruptStackFrame) {
    show_exception_screen("NON MASKABLE INTERRUPT", frame, None);
}

pub extern "x86-interrupt" fn breakpoint_handler(frame: &mut InterruptStackFrame) {
    crate::serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", frame);
}

pub extern "x86-interrupt" fn overflow_handler(frame: &mut InterruptStackFrame) {
    show_exception_screen("OVERFLOW", frame, None);
}

pub extern "x86-interrupt" fn bound_range_exceeded_handler(frame: &mut InterruptStackFrame) {
    show_exception_screen("BOUND RANGE EXCEEDED", frame, None);
}

pub extern "x86-interrupt" fn invalid_opcode_handler(frame: &mut InterruptStackFrame) {
    show_exception_screen("INVALID OPCODE", frame, None);
}

pub extern "x86-interrupt" fn device_not_available_handler(frame: &mut InterruptStackFrame) {
    show_exception_screen("DEVICE NOT AVAILABLE", frame, None);
}

pub extern "x86-interrupt" fn invalid_tss_handler(frame: &mut InterruptStackFrame, error_code: u32) {
    show_exception_screen("INVALID TSS", frame, Some(error_code));
}

pub extern "x86-interrupt" fn segment_not_present_handler(frame: &mut InterruptStackFrame, error_code: u32) {
    show_exception_screen("SEGMENT NOT PRESENT", frame, Some(error_code));
}

pub extern "x86-interrupt" fn stack_segment_fault_handler(frame: &mut InterruptStackFrame, error_code: u32) {
    show_exception_screen("STACK SEGMENT FAULT", frame, Some(error_code));
}

pub extern "x86-interrupt" fn gpf_handler(frame: &mut InterruptStackFrame, error_code: u32) {
    show_exception_screen("GENERAL PROTECTION FAULT", frame, Some(error_code));
}

pub extern "x86-interrupt" fn page_fault_handler(frame: &mut InterruptStackFrame, error_code: u32) {
    show_exception_screen("PAGE FAULT", frame, Some(error_code));
}

pub extern "x86-interrupt" fn x87_floating_point_handler(frame: &mut InterruptStackFrame) {
    show_exception_screen("x87 FLOATING POINT EXCEPTION", frame, None);
}

pub extern "x86-interrupt" fn alignment_check_handler(frame: &mut InterruptStackFrame, error_code: u32) {
    show_exception_screen("ALIGNMENT CHECK", frame, Some(error_code));
}

pub extern "x86-interrupt" fn machine_check_handler(frame: &mut InterruptStackFrame) {
    show_exception_screen("MACHINE CHECK", frame, None);
}

pub extern "x86-interrupt" fn simd_floating_point_handler(frame: &mut InterruptStackFrame) {
    show_exception_screen("SIMD FLOATING POINT EXCEPTION", frame, None);
}

pub extern "x86-interrupt" fn virtualization_handler(frame: &mut InterruptStackFrame) {
    show_exception_screen("VIRTUALIZATION EXCEPTION", frame, None);
}

pub extern "x86-interrupt" fn security_exception_handler(frame: &mut InterruptStackFrame, error_code: u32) {
    show_exception_screen("SECURITY EXCEPTION", frame, Some(error_code));
}

#[no_mangle]
#[unsafe(naked)]
pub extern "C" fn double_fault_task_handler() -> ! {
    naked_asm!(
        // The error code is already on the stack, so we can just call the handler.
        "call double_fault_panic"
    );
}

// This function is safe to call as it's on a new stack.
#[no_mangle]
extern "C" fn double_fault_panic(error_code: u32) -> ! {
    // Retrieve the state of the task that caused the double fault from the main TSS.
    // When the Task Gate triggers, the CPU saves the old state into the TSS pointed to
    // by the previous Task Register (TR). Since we only have one main TSS, that's where it is.
    let frame = unsafe {
        let tss = &raw const crate::kernel::gdt::TSS;
        InterruptStackFrame {
            instruction_pointer: (*tss).eip,
            code_segment: (*tss).cs,
            cpu_flags: (*tss).eflags,
        }
    };
    show_exception_screen("DOUBLE FAULT", &frame, Some(error_code));
}