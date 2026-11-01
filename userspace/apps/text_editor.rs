// Text Editor application for NebulaOS

fn main() {
    println!("NebulaOS Text Editor");
    println!("Type text. Type 'quit' to save and exit.");
    println!("----------------------------------------");
    
    let mut content = String::new();
    
    loop {
        print!("> ");
        let input = read_line();
        
        if input == "quit" || input == "exit" {
            println!("Saving and exiting...");
            break;
        } else if cmd == "help" {
            println!("Commands:");
            println!("  help    - Show this help");
            println("  quit    - Save and exit");
            println("  clear   - Clear text");
            println("  show    - Show current text");
        } else if cmd == "clear" {
            content.clear();
            println!("Text cleared.");
        } else if cmd == "show" {
            if content.is_empty() {
                println!("(empty)");
            } else {
                println!("{}", content);
            }
        } else if cmd.is_empty() {
            // Do nothing
        } else {
            content.push_str(&cmd);
            content.push('\n');
        }
    }
    
    println!("Editor closed.");
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
    // Syscall to read from keyboard
    String::new()
}
