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

/// Walks the stack and prints a stack trace to the serial port.
/// This function is unsafe because it reads from arbitrary memory locations by following the base pointer.
pub unsafe fn print_stack_trace() {
    let mut rbp: u32;
    asm!("mov {}, ebp", out(reg) rbp, options(nomem, nostack));

    crate::serial_println!("\nStack Trace (current RBP: 0x{:08x}):", rbp);

    if rbp == 0 {
        return;
    }

    // Basic alignment check to avoid immediate faults on garbage
    if rbp & 3 != 0 {
        crate::serial_println!("  <RBP unaligned, stopping trace>");
        return;
    }

    let mut i = 0;
    // Limit to 20 frames to prevent infinite loops in case of stack corruption.
    while rbp != 0 && i < 20 {
        // The return address is stored on the stack just above the saved RBP.
        let return_address = *(rbp as *const u32).add(1);
        
        let symbol_info = crate::kernel::symbols::resolve(return_address as usize);
        if let Some((name, offset)) = symbol_info {
            crate::serial_println!("  Frame {:02}: 0x{:08x} <{}+{:#x}>", i, return_address, name, offset);
        } else {
            crate::serial_println!("  Frame {:02}: 0x{:08x} <unknown>", i, return_address);
        }

        crate::serial_println!("  Frame {:02}: Return to 0x{:08x}", i, return_address);
        // The next RBP is the value pointed to by the current RBP.
        let next_rbp = *(rbp as *const u32);
        
        // Basic sanity check: Stack usually grows down, so previous frame RBP should be > current RBP
        if next_rbp <= rbp && next_rbp != 0 {
             crate::serial_println!("  <Stack Corruption or End detected>");
             break;
        }
        rbp = next_rbp;
        i += 1;
    }
}

/// Walks the stack and prints a stack trace to the provided writer.
/// This function is unsafe because it reads from arbitrary memory locations by following the base pointer.
pub unsafe fn print_stack_trace_to<W: fmt::Write>(writer: &mut W) {
    let mut rbp: u32;
    asm!("mov {}, ebp", out(reg) rbp, options(nomem, nostack));

    let _ = writeln!(writer, "\nStack Trace (RBP: {:#x}):", rbp);
    let mut i = 0;
    while rbp != 0 && i < 20 {
        let return_address = *(rbp as *const u32).add(1);

        let symbol_info = crate::kernel::symbols::resolve(return_address as usize);
        if let Some((name, offset)) = symbol_info {
            let _ = writeln!(writer, "  Frame {:02}: 0x{:08x} <{}+{:#x}>", i, return_address, name, offset);
        } else {
            let _ = writeln!(writer, "  Frame {:02}: 0x{:08x} <unknown>", i, return_address);
        }
        let _ = writeln!(writer, "  Frame {:02}: Return to 0x{:08x}", i, return_address);
        
        let next_rbp = *(rbp as *const u32);
        if next_rbp <= rbp && next_rbp != 0 {
             let _ = writeln!(writer, "  <Stack End or Corruption>");
             break;
        }
        rbp = next_rbp;
        i += 1;
    }
}

/// Prints a hexadecimal dump of the memory around the stack pointer.
pub unsafe fn dump_stack_memory<W: fmt::Write>(writer: &mut W) {
    let esp: u32;
    let ebp: u32;
    asm!("mov {}, esp", out(reg) esp, options(nomem, nostack));
    asm!("mov {}, ebp", out(reg) ebp, options(nomem, nostack));
    
    // In standard x86 stack frames, EBP points to the saved EBP,
    // and EBP + 4 points to the return address.
    let return_addr_ptr = ebp + 4;
    
    let _ = writeln!(writer, "\nMEMORY_DUMP (ESP: {:#x}):", esp);
    let start_addr = (esp & !0xF).saturating_sub(32);
    
    for i in 0..10 {
        let addr = start_addr + (i * 16);
        let _ = write!(writer, "{:08x}: ", addr);
        
        // Hex values (grouped as u32)
        for j in 0..4 {
            let current_ptr = addr + (j * 4);
            let val = *(current_ptr as *const u32);
            if current_ptr == return_addr_ptr {
                let _ = write!(writer, "[{:08x}]", val);
            } else {
                let _ = write!(writer, " {:08x} ", val);
            }
        }

        // ASCII representation
        let _ = write!(writer, " |");
        for j in 0..16 {
            let byte = *((addr + j) as *const u8);
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

    // Capture CPU state immediately
    let (eax, ebx, ecx, edx, esi, edi): (u32, u32, u32, u32, u32, u32);
    let (cr0, cr2, cr3, cr4): (u32, u32, u32, u32);
    unsafe {
        asm!("mov {0}, eax", out(reg) eax);
        asm!("mov {0}, ebx", out(reg) ebx);
        asm!("mov {0}, ecx", out(reg) ecx);
        asm!("mov {0}, edx", out(reg) edx);
        asm!("mov {0}, esi", out(reg) esi);
        asm!("mov {0}, edi", out(reg) edi);
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
    crate::serial_println!("CONTEXT:\nIP: {:#010x}  CS: {:#06x}  FLAGS: {:#010x}", frame.instruction_pointer, frame.code_segment, frame.cpu_flags);
    crate::serial_println!("GPR: EAX={:#x} EBX={:#x} ECX={:#x} EDX={:#x}", eax, ebx, ecx, edx);
    crate::serial_println!("GPR: ESI={:#x} EDI={:#x}", esi, edi);
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
    let _ = writeln!(writer, "\nREGISTERS:\n----------\nIP: {:#010x} CS: {:#06x} FLAGS: {:#010x}", frame.instruction_pointer, frame.code_segment, frame.cpu_flags);
    let _ = writeln!(writer, "EAX: {:#010x} EBX: {:#010x} ECX: {:#010x} EDX: {:#010x}", eax, ebx, ecx, edx);
    let _ = writeln!(writer, "ESI: {:#010x} EDI: {:#010x}", esi, edi);
    let _ = writeln!(writer, "\nCONTROL REGISTERS:\n------------------\nCR0: {:#010x} CR2: {:#010x}\nCR3: {:#010x} CR4: {:#010x}", cr0, cr2, cr3, cr4);
    
    let _ = writeln!(writer, "\nTechnical Information:\n----------------------\nIP: {:#010x}  CS: {:#06x}  FLAGS: {:#010x}", 
        frame.instruction_pointer, frame.code_segment, frame.cpu_flags);
    unsafe { print_stack_trace_to(&mut writer); }
    unsafe { dump_stack_memory(&mut writer); }
    
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