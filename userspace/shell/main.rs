// NebulaOS Shell - Userspace shell application

fn main() {
    println!("NebulaOS Shell v0.1.0");
    println!("Type 'help' for commands.");

    loop {
        print!("> ");
        let cmd = read_line();

        if cmd == "help" {
            println!("Available commands:");
            print("  help    - Show this help\r\n");
            print("  clear   - Clear screen\r\n");
            print("  echo    - Echo text\r\n");
            print("  reboot  - Reboot system\r\n");
            print("  gui     - Switch to GUI mode\r\n");
        } else if cmd == "clear" {
            println!("Screen cleared.");
        } else if cmd.starts_with("echo ") {
            let text = &cmd[5..];
            println!("{}", text);
        } else if cmd == "reboot" {
            println!("Rebooting...");
        } else if cmd == "gui" {
            println!("Switching to GUI mode...");
        } else if cmd.is_empty() {
            // Do nothing
        } else {
            print("Unknown command: ");
            println!("{}", cmd);
        }
    }
}

fn print(s: &str) {
    // Syscall to print to screen
    let _ = s;
}

fn println(s: &str) {
    print(s);
    print("\r\n");
}

fn read_line() -> String {
    // Syscall to read from keyboard via userspace runtime
    String::new()
}
