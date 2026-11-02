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
            status: "GUI apps are isolated Ring-3 processes.",
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
            "Rust ELF; private memory and timer scheduling.",
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
        canvas.draw_text(area.x + 16, area.y + 152, self.status, TEXT, 1);
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
        self.status = "This application itself is running in Ring 3.";
        true
    }
}
