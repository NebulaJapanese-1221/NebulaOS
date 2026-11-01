// Userspace runtime - provides runtime support for NebulaOS applications

pub fn print(s: &str) {
    // Syscall to print to screen
    let _ = s;
}

pub fn println(s: &str) {
    print(s);
    print("\r\n");
}

pub fn read_line() -> String {
    // Syscall to read from keyboard
    String::new()
}

pub fn exit(code: u8) -> ! {
    // Syscall to exit application
    loop {}
}
