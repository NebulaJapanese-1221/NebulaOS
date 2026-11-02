use crate::drivers::{disk, user_process};
use crate::userspace::apps::gui::{Application, Button, Canvas, Event, PointerEvent, Rect};

const TEXT: u32 = 0x111827;
const MUTED: u32 = 0x516786;
const ACCENT: u32 = 0x1596B5;

pub struct UserModeDemo {
    run_button: Button,
    status: &'static str,
}

impl UserModeDemo {
    pub const fn new() -> Self {
        Self {
            run_button: Button::new("START USER PROCESS"),
            status: "Loads DEMO.ELF from NebulaFS at ring 3.",
        }
    }
}

impl Application for UserModeDemo {
    fn title(&self) -> &'static str {
        "Ring-3 Demo"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        canvas.draw_text(area.x + 16, area.y + 20, "USER-MODE ELF TEST", TEXT, 1);
        canvas.draw_text(
            area.x + 16,
            area.y + 42,
            "Stored ELF; private memory and timer scheduling.",
            MUTED,
            1,
        );
        self.run_button.render(
            canvas,
            Rect {
                x: area.x + 16,
                y: area.y + 76,
                width: 176,
                height: 34,
            },
            pointer,
        );
        canvas.draw_text(area.x + 16, area.y + 128, self.status, ACCENT, 1);
        let mut output_buffer = [0u8; 64];
        let output_len = user_process::copy_last_output(&mut output_buffer);
        if let Ok(output) = core::str::from_utf8(&output_buffer[..output_len]) {
            canvas.draw_text(area.x + 16, area.y + 152, output, TEXT, 1);
        }
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        let run = match event {
            Event::Pointer(pointer) => self.run_button.handle_pointer(
                Rect {
                    x: area.x + 16,
                    y: area.y + 76,
                    width: 176,
                    height: 34,
                },
                pointer,
            ),
            Event::Key(b'\n' | b'\r') => true,
            Event::Key(_) => false,
        };
        if !run {
            return false;
        }
        self.status = match user_process::spawn_demo() {
            Ok(_) => "Storage ELF queued; timer preemption is active.",
            Err(user_process::UserProcessError::OutOfMemory) => {
                "Could not allocate process memory."
            }
            Err(user_process::UserProcessError::InvalidElf) => {
                "ELF header or program table rejected."
            }
            Err(user_process::UserProcessError::NoLoadableSegments) => {
                "ELF has no loadable segments."
            }
            Err(user_process::UserProcessError::InvalidEntryPoint) => {
                "ELF entry is not executable."
            }
            Err(user_process::UserProcessError::LoadFailed) => "Could not map the ELF image.",
            Err(user_process::UserProcessError::Storage(disk::StorageError::NoDisk)) => {
                "Attach an IDE data disk first."
            }
            Err(user_process::UserProcessError::Storage(disk::StorageError::NotFormatted)) => {
                "Format a blank disk in File Manager first."
            }
            Err(user_process::UserProcessError::Storage(disk::StorageError::NotFound)) => {
                "Could not find or install DEMO.ELF."
            }
            Err(user_process::UserProcessError::Storage(_)) => "NebulaFS could not load DEMO.ELF.",
        };
        true
    }
}
