// Calculator application for NebulaOS

fn main() {
    println!("NebulaOS Calculator");
    println!("Enter expressions or 'quit' to exit.");
    
    loop {
        print!("calc> ");
        let input = read_line();
        
        if input == "quit" || input == "exit" {
            break;
        } else if cmd == "help" {
            println!("Commands: add, sub, mul, div, quit");
        } else if cmd.starts_with("add ") {
            let parts: Vec<&str> = cmd[4..].trim().split_whitespace().collect();
            if parts.len() == 2 {
                if let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                    println!("{}", a + b);
                } else {
                    println!("Invalid numbers");
                }
            } else {
                println!("Usage: add <num1> <num2>");
            }
        } else if cmd.starts_with("sub ") {
            let parts: Vec<&str> = cmd[4..].trim().split_whitespace().collect();
            if parts.len() == 2 {
                if let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                    println!("{}", a - b);
                } else {
                    println!("Invalid numbers");
                }
            } else {
                println!("Usage: sub <num1> <num2>");
            }
        } else if cmd.starts_with("mul ") {
            let parts: Vec<&str> = cmd[4..].trim().split_whitespace().collect();
            if parts.len() == 2 {
                if let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                    println!("{}", a * b);
                } else {
                    println!("Invalid numbers");
                }
            } else {
                println!("Usage: mul <num1> <num2>");
            }
        } else if cmd.starts_with("div ") {
            let parts: Vec<&str> = cmd[4..].trim().split_whitespace().collect();
            if parts.len() == 2 {
                if let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                    if b != 0.0 {
                        println!("{}", a / b);
                    } else {
                        println!("Division by zero");
                    }
                } else {
                    println!("Invalid numbers");
                }
            } else {
                println!("Usage: div <num1> <num2>");
            }
        } else if cmd.is_empty() {
            // Do nothing
        } else {
            println!("Unknown command: {}", cmd);
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
    // Syscall to read from keyboard
    String::new()
}
