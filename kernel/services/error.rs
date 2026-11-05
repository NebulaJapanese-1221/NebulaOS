use core::panic::PanicInfo;

pub fn handle_exception(vector: u32, error_code: u32, registers: *mut u32) {
    let code_segment = unsafe { *registers.add(15) };
    let instruction_pointer = unsafe { *registers.add(14) };
    let user_mode = code_segment & 3 == 3;

    crate::drivers::serial::write_string(if user_mode {
        "User process exception: "
    } else {
        "Fatal kernel exception: "
    });
    crate::drivers::serial::write_string(exception_name(vector));
    crate::drivers::serial::write_string(" vector=");
    crate::drivers::serial::write_dec(vector);
    crate::drivers::serial::write_string(" error=");
    crate::drivers::serial::write_hex(error_code);
    crate::drivers::serial::write_string(" eip=");
    crate::drivers::serial::write_hex(instruction_pointer);
    if user_mode {
        crate::drivers::serial::write_string(" pid=");
        crate::drivers::serial::write_dec(crate::services::process::current_pid());
    }
    crate::drivers::serial::write_string("\r\n");

    if user_mode {
        crate::graphics::wm::request_desktop_redraw();
        crate::services::process::terminate_current_user_process();
        return;
    }

    crate::graphics::system_screen::show(
        "SYSTEM ERROR",
        "Fatal kernel exception. See serial diagnostics.",
    );
    halt()
}

pub fn report_user_exit(code: u32) {
    if code == 0 {
        return;
    }
    crate::drivers::serial::write_string("User process ");
    crate::drivers::serial::write_dec(crate::services::process::current_pid());
    if code == u32::MAX {
        crate::drivers::serial::write_string(" panicked or terminated after an application error");
    } else {
        crate::drivers::serial::write_string(" exited with code ");
        crate::drivers::serial::write_dec(code);
    }
    crate::drivers::serial::write_string(".\r\n");
}

pub fn handle_panic(info: &PanicInfo<'_>) -> ! {
    crate::drivers::serial::write_string("KERNEL PANIC: ");
    crate::drivers::serial::write_fmt(format_args!("{info}"));
    crate::drivers::serial::write_string("\r\n");
    let mut output = [0; 128];
    let length = crate::services::user_process::copy_last_output(&mut output);
    if length != 0 {
        crate::drivers::serial::write_string("Last user output: ");
        for byte in &output[..length] {
            crate::drivers::serial::write_byte(*byte);
        }
        crate::drivers::serial::write_string("\r\n");
    }
    crate::graphics::system_screen::show("SYSTEM ERROR", "Kernel panic. See serial diagnostics.");
    halt()
}

fn exception_name(vector: u32) -> &'static str {
    match vector {
        0 => "divide error",
        1 => "debug",
        2 => "non-maskable interrupt",
        3 => "breakpoint",
        4 => "overflow",
        5 => "bound range",
        6 => "invalid opcode",
        7 => "device unavailable",
        8 => "double fault",
        10 => "invalid TSS",
        11 => "segment not present",
        12 => "stack fault",
        13 => "general protection fault",
        14 => "page fault",
        16 => "x87 floating point",
        17 => "alignment check",
        18 => "machine check",
        19 => "SIMD floating point",
        _ => "processor exception",
    }
}

fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}
