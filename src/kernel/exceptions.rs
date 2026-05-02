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
                font::draw_char(self.fb, self.x, self.y, c, 0xFFFFFFFF, None);
                self.x += 8;
            }
        }
        Ok(())
    }
}

/// Walks the stack and prints a stack trace to the serial port.
/// This function is unsafe because it reads from arbitrary memory locations by following the base pointer.
pub unsafe fn print_stack_trace() {
    let mut rbp: usize;
    unsafe {
        #[cfg(target_arch = "x86")]
        asm!("mov {}, ebp", out(reg) rbp, options(nomem, nostack));
        #[cfg(target_arch = "x86_64")]
        asm!("mov {}, rbp", out(reg) rbp, options(nomem, nostack));
    }

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
        // Alignment check: x86 stack frames must be 4-byte aligned.
        if rbp & 3 != 0 {
            crate::serial_println!("  <Frame {:02}: RBP misaligned ({:#010x}), stopping trace>", i, rbp);
            break;
        }

        // The return address is stored on the stack just above the saved RBP.
        let return_address = unsafe { core::ptr::read_unaligned((rbp as *const usize).add(1)) };
        crate::serial_println!("  Frame {:02}: Return to 0x{:08x}", i, return_address);

        // The next RBP is the value pointed to by the current RBP.
        let next_rbp = unsafe { core::ptr::read_unaligned(rbp as *const usize) };
        
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
    let mut rbp: usize;
    unsafe {
        #[cfg(target_arch = "x86")]
        asm!("mov {}, ebp", out(reg) rbp, options(nomem, nostack));
        #[cfg(target_arch = "x86_64")]
        asm!("mov {}, rbp", out(reg) rbp, options(nomem, nostack));
    }

    let _ = writeln!(writer, "\nStack Trace (RBP: {:#x}):", rbp);
    let mut i = 0;
    while rbp != 0 && i < 20 {
        if rbp & 3 != 0 {
            let _ = writeln!(writer, "  <Frame {:02}: RBP misaligned ({:#x}), stopping trace>", i, rbp);
            break;
        }

        let return_address = unsafe { core::ptr::read_unaligned((rbp as *const usize).add(1)) };
        let _ = writeln!(writer, "  Frame {:02}: Return to {:#x}", i, return_address);
        
        let next_rbp = unsafe { core::ptr::read_unaligned(rbp as *const usize) };
        if next_rbp <= rbp && next_rbp != 0 {
             let _ = writeln!(writer, "  <Stack End or Corruption>");
             break;
        }
        rbp = next_rbp;
        i += 1;
    }
}

/// Draws a simple representation of a QR code linking to the GitHub page.
pub fn draw_qr_code(fb: &mut framebuffer::Framebuffer, x: isize, y: isize) {
    // Pre-calculated QR Matrix (Version 3, 29x29) for the URL:
    // https://github.com/NebulaJapanese-1221/NebulaOS
    const QR_MATRIX: [u32; 29] = [
        0x1FC0007F, 0x10400041, 0x1540005D, 0x1540005D, 0x1540005D, 0x10400041, 0x1FC0007F, 0x00000000,
        0x0B4B6EAB, 0x1001CE01, 0x0D8A9967, 0x04620A6B, 0x1F86B93F, 0x00000000, 0x17B090BD, 0x180E078F,
        0x1A22A70B, 0x0C3EE33F, 0x1841C06D, 0x00000000, 0x1FC00000, 0x10400000, 0x15400000, 0x15400000,
        0x15410000, 0x10411000, 0x1FC10000, 0x00000000, 0x00000000
    ];

    let module_size: isize = 4;
    let qr_pixels = 29 * module_size;
    
    // Draw a clean white background with a quiet zone for the QR code and text
    crate::userspace::gui::draw_rect(fb, x - 10, y - 10, 400, (qr_pixels + 60) as usize, 0xFFFFFFFF, None);
    
    for row in 0..29 {
        let row_data = QR_MATRIX[row];
        for col in 0..29 {
            // Draw black module if the bit is set (bit 28 is column 0)
            if (row_data >> (28 - col)) & 1 == 1 {
                crate::userspace::gui::draw_rect(fb, x + (col as isize * module_size), y + (row as isize * module_size), module_size as usize, module_size as usize, 0x00_000000, None);
            }
        }
    }

    let text_y = y + qr_pixels + 10;
    font::draw_string(fb, x, text_y, "Scan to create an issue on GitHub:", 0x00_000000, None);
    font::draw_string(fb, x, text_y + 14, "https://github.com/NebulaJapanese-1221/NebulaOS", 0x00_333333, None);
}

pub fn show_exception_screen(name: &str, frame: &InterruptStackFrame, error_code: Option<u32>) -> ! {
    unsafe { asm!("cli", options(nomem, nostack)); }

    // Print details to serial port immediately, as drawing to screen might fail or deadlock
    crate::serial_println!("\nCRITICAL SYSTEM ERROR: {}", name);
    if let Some(code) = error_code {
        crate::serial_println!("Error Code: {:#x}", code);
    }
    crate::serial_println!("CONTEXT:\nIP: {:#010x}  CS: {:#06x}  FLAGS: {:#010x}", frame.instruction_pointer, frame.code_segment, frame.cpu_flags);
    
    crate::serial_println!("Stack Frame:\n{:#?}", frame);
    unsafe { print_stack_trace(); }

    let mut fb = FRAMEBUFFER.lock();
    fb.clear(0x00_CC0000); // Red background (RSOD)

    font::draw_string(&mut fb, 30, 30, ":(", 0xFFFFFFFF, None);
    font::draw_string(&mut fb, 30, 60, "NebulaOS ran into a problem and needs to restart.", 0xFFFFFFFF, None);

    let mut writer = PanicWriter::new(&mut fb, 30, 90);
    
    use core::fmt::Write;
    let _ = writeln!(writer, "Stop Code: {}", name);
    if let Some(code) = error_code {
        let _ = writeln!(writer, "Error Code: {:#x}", code);
    }
    let _ = writeln!(writer, "\nTechnical Information:\n----------------------\nIP: {:#010x}  CS: {:#06x}  FLAGS: {:#010x}", frame.instruction_pointer, frame.code_segment, frame.cpu_flags);
    unsafe { print_stack_trace_to(&mut writer); }
    
    // Draw support QR Code
    draw_qr_code(&mut fb, 30, 430);

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

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "C" fn double_fault_task_handler() -> ! {
    unsafe { naked_asm!(
        // The error code is already on the stack, so we can just call the handler.
        "call double_fault_panic"
    ); }
}

// This function is safe to call as it's on a new stack.
#[unsafe(no_mangle)]
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