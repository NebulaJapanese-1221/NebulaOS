use crate::common::stdint::*;
use crate::common::vga;
use crate::drivers::keyboard;

const SHELL_BUFFER_SIZE: usize = 256;
const SHELL_MAX_ARGS: usize = 8;

static mut COMMAND_BUFFER: [u8; SHELL_BUFFER_SIZE] = [0; SHELL_BUFFER_SIZE];
static mut COMMAND_POS: usize = 0;

type CommandHandler = unsafe extern "C" fn(i32, *mut *mut u8);

struct Command {
    name: &'static [u8],
    description: &'static [u8],
    handler: CommandHandler,
}

static COMMANDS: [Command; 16] = [
    Command { name: b"help", description: b"Show this help message", handler: cmd_help },
    Command { name: b"clear", description: b"Clear the screen", handler: cmd_clear },
    Command { name: b"reboot", description: b"Reboot the system", handler: cmd_reboot },
    Command { name: b"meminfo", description: b"Show memory information", handler: cmd_meminfo },
    Command { name: b"echo", description: b"Echo arguments", handler: cmd_echo },
    Command { name: b"color", description: b"Change text color", handler: cmd_color },
    Command { name: b"version", description: b"Show kernel version", handler: cmd_version },
    Command { name: b"ls", description: b"List directory", handler: cmd_ls },
    Command { name: b"dir", description: b"List directory", handler: cmd_ls },
    Command { name: b"cat", description: b"Display file", handler: cmd_cat },
    Command { name: b"ps", description: b"List processes", handler: cmd_ps },
    Command { name: b"exec", description: b"Execute ELF", handler: cmd_exec },
    Command { name: b"run", description: b"Execute ELF", handler: cmd_exec },
    Command { name: b"kill", description: b"Terminate process", handler: cmd_kill },
    Command { name: b"touch", description: b"Stub", handler: cmd_touch },
    Command { name: b"mkdir", description: b"Stub", handler: cmd_mkdir },
];

pub unsafe fn shell_init() {
    COMMAND_POS = 0;
    COMMAND_BUFFER[0] = 0;
    keyboard::keyboard_set_handler(shell_keyboard_callback);
    crate::common::fs::fs_mount();
    vga::puts(b"NebulaOS Shell\n\0" as *const u8 as *const u8);
    vga::puts(b"Type 'help' for available commands\n\0" as *const u8 as *const u8);
    vga::puts(b"> \0" as *const u8 as *const u8);
}

pub unsafe fn shell_run() {
    shell_init();
    unsafe { asm!("sti"); }
    loop {
        unsafe { asm!("hlt"); }
    }
}

pub unsafe extern "C" fn shell_keyboard_callback(scancode: u8, _modifiers: u8, _pressed: u8) {
    process_scancode(scancode);
}

unsafe fn process_scancode(scancode: u8) {
    if scancode & 0x80 != 0 {
        return;
    }
    match scancode {
        0x1C => {
            vga::putchar('\n');
            if COMMAND_POS > 0 {
                let mut argv: [*mut u8; SHELL_MAX_ARGS + 1] = [core::ptr::null_mut(); SHELL_MAX_ARGS + 1];
                let argc = parse_command(&mut COMMAND_BUFFER[..COMMAND_POS], &mut argv);
                if argc > 0 {
                    let mut found = false;
                    for cmd in &COMMANDS {
                        if strcmp(argv[0], cmd.name) == 0 {
                            (cmd.handler)(argc, argv.as_mut_ptr());
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        vga::puts(b"Unknown command: \0" as *const u8 as *const u8);
                        vga::puts(argv[0]);
                        vga::puts(b"\n\0" as *const u8 as *const u8);
                    }
                }
            }
            COMMAND_POS = 0;
            COMMAND_BUFFER[0] = 0;
            vga::puts(b"> \0" as *const u8 as *const u8);
        }
        0x0E => {
            if COMMAND_POS > 0 {
                COMMAND_POS -= 1;
                COMMAND_BUFFER[COMMAND_POS] = 0;
                vga::putchar('\b');
            }
        }
        _ => {
            let modifiers = keyboard::keyboard_get_modifiers();
            let c = keyboard::keyboard_scancode_to_ascii(scancode, modifiers);
            if c >= 32 && c <= 126 && COMMAND_POS < SHELL_BUFFER_SIZE - 1 {
                COMMAND_BUFFER[COMMAND_POS] = c as u8;
                COMMAND_POS += 1;
                COMMAND_BUFFER[COMMAND_POS] = 0;
                vga::putchar(c);
            }
        }
    }
}

unsafe fn parse_command(input: &[u8], argv: &mut [*mut u8]) -> i32 {
    let mut argc = 0;
    let mut ptr = input.as_ptr();
    let mut start = ptr;
    while *ptr == b' ' || *ptr == b'\t' {
        ptr = ptr.add(1);
        start = ptr;
    }
    while *ptr != 0 && argc < SHELL_MAX_ARGS {
        if *ptr == b' ' || *ptr == b'\t' {
            if start != ptr {
                *ptr = 0;
                argv[argc] = start as *mut u8;
                argc += 1;
            }
            start = ptr.add(1);
        }
        ptr = ptr.add(1);
    }
    if *start != 0 && argc < SHELL_MAX_ARGS {
        argv[argc] = start as *mut u8;
        argc += 1;
    }
    argv[argc] = core::ptr::null_mut();
    argc
}

unsafe fn strcmp(a: *const u8, b: &[u8]) -> i32 {
    let mut i = 0;
    while i < b.len() && *a.add(i) != 0 && *a.add(i) == b[i] {
        i += 1;
    }
    if i == b.len() && *a.add(i) == 0 {
        0
    } else if *a.add(i) < b[i] {
        -1
    } else {
        1
    }
}

unsafe fn cmd_help(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"Available commands: help clear reboot meminfo echo color version ls cat ps exec kill touch mkdir\n\0" as *const u8 as *const u8);
}

unsafe fn cmd_clear(_argc: i32, _argv: *mut *mut u8) {
    vga::clear();
    vga::puts(b"NebulaOS Shell\n\0" as *const u8 as *const u8);
    vga::puts(b"> \0" as *const u8 as *const u8);
}

unsafe fn cmd_reboot(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"Rebooting...\n\0" as *const u8 as *const u8);
    core::ptr::null_mut();
}

unsafe fn cmd_meminfo(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"Memory Information:\n\0" as *const u8 as *const u8);
    vga::puts(b"  Total: 128MB\n\0" as *const u8 as *const u8);
    vga::puts(b"  Used: 0MB\n\0" as *const u8 as *const u8);
    vga::puts(b"  Free: 128MB\n\0" as *const u8 as *const u8);
}

unsafe fn cmd_echo(_argc: i32, argv: *mut *mut u8) {
    if _argc < 2 {
        vga::puts(b"Usage: echo <message>\n\0" as *const u8 as *const u8);
        return;
    }
    let mut i = 1;
    while i < _argc {
        vga::puts(*argv.add(i));
        if i < _argc - 1 {
            vga::puts(b" \0" as *const u8 as *const u8);
        }
        i += 1;
    }
    vga::puts(b"\n\0" as *const u8 as *const u8);
}

unsafe fn cmd_color(_argc: i32, _argv: *mut *mut u8) {
    if _argc < 2 {
        vga::puts(b"Usage: color <fg> [bg]\n\0" as *const u8 as *const u8);
        return;
    }
}

unsafe fn cmd_version(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"NebulaOS Version: 0.0.1\n\0" as *const u8 as *const u8);
}

unsafe fn cmd_ls(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"ls: stub\n\0" as *const u8 as *const u8);
}

unsafe fn cmd_cat(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"cat: stub\n\0" as *const u8 as *const u8);
}

unsafe fn cmd_ps(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"PID  STATE     ENTRY\n\0" as *const u8 as *const u8);
}

unsafe fn cmd_exec(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"exec: stub\n\0" as *const u8 as *const u8);
}

unsafe fn cmd_kill(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"kill: stub\n\0" as *const u8 as *const u8);
}

unsafe fn cmd_touch(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"touch: not implemented\n\0" as *const u8 as *const u8);
}

unsafe fn cmd_mkdir(_argc: i32, _argv: *mut *mut u8) {
    vga::puts(b"mkdir: not implemented\n\0" as *const u8 as *const u8);
}
