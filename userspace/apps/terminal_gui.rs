use super::gui::{Application, Canvas, Event, PointerEvent, Rect};
use crate::{storage, syscall};

const LINE_COUNT: usize = 24;
const LINE_WIDTH: usize = 64;
const COMMAND_WIDTH: usize = 64;
const TEXT: u32 = 0xD6E4F0;
const PROMPT: u32 = 0x66D9EF;

pub struct Terminal {
    lines: [[u8; LINE_WIDTH]; LINE_COUNT],
    lengths: [u8; LINE_COUNT],
    line_count: usize,
    command: [u8; COMMAND_WIDTH],
    command_len: usize,
}

impl Terminal {
    pub fn new() -> Self {
        let mut terminal = Self {
            lines: [[0; LINE_WIDTH]; LINE_COUNT],
            lengths: [0; LINE_COUNT],
            line_count: 0,
            command: [0; COMMAND_WIDTH],
            command_len: 0,
        };
        terminal.append_line(b"NebulaOS terminal. Type help for commands.");
        terminal
    }

    fn append_line(&mut self, text: &[u8]) {
        if self.line_count == LINE_COUNT {
            self.lines.copy_within(1.., 0);
            self.lengths.copy_within(1.., 0);
            self.line_count -= 1;
        }
        let row = self.line_count;
        let length = text.len().min(LINE_WIDTH);
        for (destination, source) in self.lines[row][..length]
            .iter_mut()
            .zip(text[..length].iter())
        {
            *destination = if source.is_ascii_graphic() || *source == b' ' {
                *source
            } else {
                b'.'
            };
        }
        self.lengths[row] = length as u8;
        self.line_count += 1;
    }

    fn append_number(&mut self, prefix: &[u8], mut number: u32, suffix: &[u8]) {
        let mut line = [0u8; LINE_WIDTH];
        let mut length = prefix.len().min(LINE_WIDTH);
        line[..length].copy_from_slice(&prefix[..length]);
        let mut digits = [0u8; 10];
        let mut digit_count = 0;
        loop {
            digits[digit_count] = b'0' + (number % 10) as u8;
            digit_count += 1;
            number /= 10;
            if number == 0 {
                break;
            }
        }
        for digit in digits[..digit_count].iter().rev() {
            if length == LINE_WIDTH {
                break;
            }
            line[length] = *digit;
            length += 1;
        }
        for byte in suffix {
            if length == LINE_WIDTH {
                break;
            }
            line[length] = *byte;
            length += 1;
        }
        self.append_line(&line[..length]);
    }

    fn run_command(&mut self, command: &[u8]) {
        let trimmed = trim(command);
        if trimmed.is_empty() {
            return;
        }
        let mut entered = [0u8; COMMAND_WIDTH + 2];
        entered[..2].copy_from_slice(b"> ");
        let entered_len = trimmed.len().min(COMMAND_WIDTH);
        entered[2..2 + entered_len].copy_from_slice(&trimmed[..entered_len]);
        self.append_line(&entered[..2 + entered_len]);
        let (name, arguments) = split_command(trimmed);
        match name {
            b"help" => {
                self.append_line(b"help echo clear uptime mem ps devices");
                self.append_line(b"ls cat theme speed sleep kill reboot shutdown exit");
            }
            b"echo" => self.append_line(arguments),
            b"clear" => {
                self.line_count = 0;
                self.append_line(b"Terminal cleared.");
            }
            b"uptime" => match syscall::uptime_ticks() {
                Ok(ticks) => self.append_number(b"Ticks: ", ticks, b""),
                Err(_) => self.append_line(b"Unable to read uptime."),
            },
            b"mem" => self.show_memory(),
            b"ps" => self.show_processes(),
            b"devices" => self.show_devices(),
            b"ls" => self.list_files(),
            b"cat" => self.show_file(arguments),
            b"theme" => self.set_theme(arguments),
            b"speed" => self.set_speed(arguments),
            b"sleep" => self.sleep(arguments),
            b"kill" => self.kill(arguments),
            b"reboot" => {
                self.append_line(b"Rebooting NebulaOS...");
                let _ = syscall::reboot();
            }
            b"shutdown" => {
                self.append_line(b"Shutting down NebulaOS...");
                let _ = syscall::shutdown();
            }
            b"exit" => {
                let _ = syscall::exit(0);
            }
            _ => self.append_line(b"Unknown command. Type help."),
        }
    }

    fn show_memory(&mut self) {
        let mut info = syscall::MemoryInfo::default();
        if syscall::memory_info(&mut info).is_err() {
            self.append_line(b"Memory diagnostics unavailable.");
            return;
        }
        self.append_number(b"Free frames: ", info.free_frames, b"");
        self.append_number(b"Allocated: ", info.allocated_frames, b"");
        self.append_number(b"Frame size: ", info.frame_size, b" bytes");
    }

    fn show_processes(&mut self) {
        let mut info = syscall::ProcessInfo::default();
        if syscall::process_info(&mut info).is_err() {
            self.append_line(b"Process information unavailable.");
            return;
        }
        self.append_number(b"Running user processes: ", info.count, b"");
        for process in info.entries.iter().take(info.count as usize) {
            let state = match process.state {
                1 => b"READY".as_slice(),
                2 => b"RUNNING".as_slice(),
                3 => b"BLOCKED".as_slice(),
                _ => b"UNKNOWN".as_slice(),
            };
            let mut line = [0u8; LINE_WIDTH];
            let mut length = 0;
            append_bytes(&mut line, &mut length, b"PID ");
            append_u32(&mut line, &mut length, process.pid);
            append_bytes(&mut line, &mut length, b" APP ");
            append_u32(&mut line, &mut length, process.app_id);
            append_bytes(&mut line, &mut length, b" ");
            append_bytes(&mut line, &mut length, state);
            self.append_line(&line[..length]);
        }
    }

    fn show_devices(&mut self) {
        let mut info = syscall::DeviceInfo::default();
        if syscall::device_info(&mut info).is_err() {
            self.append_line(b"Device information unavailable.");
            return;
        }
        let mut line = [0u8; LINE_WIDTH];
        let mut length = 0;
        append_bytes(&mut line, &mut length, b"Display ");
        append_u32(&mut line, &mut length, info.framebuffer_width);
        append_bytes(&mut line, &mut length, b"x");
        append_u32(&mut line, &mut length, info.framebuffer_height);
        self.append_line(&line[..length]);
        self.append_line(if info.mouse_available != 0 {
            b"Mouse: ready"
        } else {
            b"Mouse: unavailable"
        });
        self.append_line(if info.storage_available != 0 {
            b"Storage: ready"
        } else {
            b"Storage: unavailable"
        });
        self.append_number(b"Mouse sensitivity: ", info.mouse_sensitivity, b"");
    }

    fn list_files(&mut self) {
        match storage::list_files() {
            Ok(files) => {
                let mut found = false;
                for file in files.iter().filter(|file| file.name[0] != 0) {
                    found = true;
                    let name_len = file.name.iter().position(|byte| *byte == 0).unwrap_or(16);
                    let mut line = [0u8; LINE_WIDTH];
                    let name_len = name_len.min(LINE_WIDTH);
                    line[..name_len].copy_from_slice(&file.name[..name_len]);
                    let mut length = name_len;
                    append_bytes(&mut line, &mut length, b" ");
                    append_u32(&mut line, &mut length, file.length as u32);
                    append_bytes(&mut line, &mut length, b" B");
                    self.append_line(&line[..length]);
                }
                if !found {
                    self.append_line(b"No files.");
                }
            }
            Err(_) => self.append_line(b"Storage unavailable or not formatted."),
        }
    }

    fn show_file(&mut self, name: &[u8]) {
        let name = trim(name);
        if name.is_empty() {
            self.append_line(b"Usage: cat FILE");
            return;
        }
        let mut filename = [0u8; 15];
        let name_len = name.len().min(filename.len());
        for (destination, source) in filename[..name_len].iter_mut().zip(name[..name_len].iter()) {
            *destination = source.to_ascii_uppercase();
        }
        let mut content = [0u8; 512];
        match storage::read_file(&filename[..name_len], &mut content) {
            Ok(0) => self.append_line(b"(empty file)"),
            Ok(length) => {
                for line in content[..length].split(|byte| *byte == b'\n') {
                    self.append_line(trim_end(line));
                }
            }
            Err(_) => self.append_line(b"Could not read file."),
        }
    }

    fn set_theme(&mut self, value: &[u8]) {
        let enabled = match trim(value) {
            b"dark" => true,
            b"light" => false,
            _ => {
                self.append_line(b"Usage: theme dark|light");
                return;
            }
        };
        match syscall::set_dark_theme(enabled) {
            Ok(()) => self.append_line(b"Wallpaper theme updated."),
            Err(_) => self.append_line(b"Unable to change wallpaper theme."),
        }
    }

    fn set_speed(&mut self, value: &[u8]) {
        let speed = match trim(value) {
            b"slow" => 1,
            b"normal" => 2,
            b"fast" => 4,
            _ => {
                self.append_line(b"Usage: speed slow|normal|fast");
                return;
            }
        };
        match syscall::set_mouse_sensitivity(speed) {
            Ok(()) => self.append_line(b"Mouse speed updated."),
            Err(_) => self.append_line(b"Unable to change mouse speed."),
        }
    }

    fn sleep(&mut self, value: &[u8]) {
        let Some(ticks) = parse_u32(trim(value)) else {
            self.append_line(b"Usage: sleep TICKS");
            return;
        };
        if syscall::sleep_ticks(ticks).is_ok() {
            self.append_line(b"Sleep complete.");
        } else {
            self.append_line(b"Sleep syscall failed.");
        }
    }

    fn kill(&mut self, value: &[u8]) {
        let Some(pid) = parse_u32(trim(value)) else {
            self.append_line(b"Usage: kill PID");
            return;
        };
        match syscall::terminate_process(pid) {
            Ok(()) => self.append_line(b"Process terminated."),
            Err(_) => self.append_line(b"Cannot terminate that process."),
        }
    }
}

impl Application for Terminal {
    fn title(&self) -> &'static str {
        "TERMINAL"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, _pointer: PointerEvent) {
        canvas.fill_rect(area, 0x101820);
        let visible_lines = ((area.height.saturating_sub(30) / 11) as usize).min(LINE_COUNT);
        let first = self.line_count.saturating_sub(visible_lines);
        for line_index in first..self.line_count {
            let row = line_index - first;
            let length = self.lengths[line_index] as usize;
            if let Ok(text) = core::str::from_utf8(&self.lines[line_index][..length]) {
                canvas.draw_text(area.x + 8, area.y + 8 + row as u32 * 11, text, TEXT, 1);
            }
        }
        let mut prompt = [0u8; COMMAND_WIDTH + 2];
        prompt[..2].copy_from_slice(b"> ");
        prompt[2..2 + self.command_len].copy_from_slice(&self.command[..self.command_len]);
        if let Ok(text) = core::str::from_utf8(&prompt[..2 + self.command_len]) {
            canvas.draw_text(
                area.x + 8,
                area.y + area.height.saturating_sub(18),
                text,
                PROMPT,
                1,
            );
        }
    }

    fn handle_event(&mut self, event: Event, _area: Rect) -> bool {
        let Event::Key(key) = event else {
            return false;
        };
        match key {
            b'\n' => {
                let command = self.command;
                let length = self.command_len;
                self.command_len = 0;
                self.command = [0; COMMAND_WIDTH];
                self.run_command(&command[..length]);
            }
            8 | 127 => {
                if self.command_len > 0 {
                    self.command_len -= 1;
                    self.command[self.command_len] = 0;
                }
            }
            key @ (32..=126) if self.command_len < COMMAND_WIDTH => {
                self.command[self.command_len] = key;
                self.command_len += 1;
            }
            _ => return false,
        }
        true
    }
}

fn split_command(input: &[u8]) -> (&[u8], &[u8]) {
    let split = input
        .iter()
        .position(u8::is_ascii_whitespace)
        .unwrap_or(input.len());
    (&input[..split], trim(&input[split..]))
}

fn trim(input: &[u8]) -> &[u8] {
    let start = input
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(input.len());
    let end = input
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(start, |index| index + 1);
    &input[start..end]
}

fn trim_end(input: &[u8]) -> &[u8] {
    let end = input
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(0, |index| index + 1);
    &input[..end]
}

fn parse_u32(input: &[u8]) -> Option<u32> {
    if input.is_empty() {
        return None;
    }
    input.iter().try_fold(0u32, |value, byte| {
        byte.is_ascii_digit()
            .then(|| value.checked_mul(10)?.checked_add((byte - b'0') as u32))
            .flatten()
    })
}

fn append_bytes(output: &mut [u8], length: &mut usize, bytes: &[u8]) {
    let count = bytes.len().min(output.len().saturating_sub(*length));
    output[*length..*length + count].copy_from_slice(&bytes[..count]);
    *length += count;
}

fn append_u32(output: &mut [u8], length: &mut usize, mut value: u32) {
    let mut digits = [0u8; 10];
    let mut count = 0;
    loop {
        digits[count] = b'0' + (value % 10) as u8;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for digit in digits[..count].iter().rev() {
        append_bytes(output, length, core::slice::from_ref(digit));
    }
}
