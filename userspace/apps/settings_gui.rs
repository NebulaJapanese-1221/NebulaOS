use super::gui::{Application, Canvas, Checkbox, Event, FocusGroup, Menu, PointerEvent, Rect};

const SPEEDS: [&str; 3] = ["SLOW", "NORMAL", "FAST"];

pub struct Settings {
    dark_theme: Checkbox,
    mouse_speed: Menu,
    focus: FocusGroup,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            dark_theme: Checkbox::new(true),
            mouse_speed: Menu::new(1),
            focus: FocusGroup::new(0),
        }
    }
}

impl Application for Settings {
    fn title(&self) -> &'static str {
        "SETTINGS"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, _pointer: PointerEvent) {
        canvas.draw_text(area.x + 24, area.y + 20, "APPEARANCE", 0x1596B5, 1);
        self.dark_theme.render(
            canvas,
            Rect {
                x: area.x + 24,
                y: area.y + 48,
                width: 16,
                height: 16,
            },
            "DARK WALLPAPER",
        );
        canvas.draw_text(area.x + 24, area.y + 92, "DISPLAY MODE", 0x1596B5, 1);
        canvas.draw_text(
            area.x + 24,
            area.y + 112,
            "1024 X 768 (SET BY GRUB AT BOOT)",
            0x111827,
            1,
        );
        canvas.draw_text(area.x + 24, area.y + 150, "MOUSE SPEED", 0x1596B5, 1);
        self.mouse_speed.render(
            canvas,
            Rect {
                x: area.x + 24,
                y: area.y + 174,
                width: 150,
                height: 3 * 28,
            },
            &SPEEDS,
        );
        canvas.draw_text(
            area.x + 24,
            area.y + 270,
            "KEYBOARD: US QWERTY",
            0x111827,
            1,
        );
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match event {
            Event::Pointer(pointer) => {
                let toggled = self.dark_theme.handle_pointer(
                    Rect {
                        x: area.x + 24,
                        y: area.y + 48,
                        width: 16,
                        height: 16,
                    },
                    pointer,
                );
                if toggled {
                    crate::graphics::wm::set_dark_theme(self.dark_theme.checked);
                }
                let selected = self.mouse_speed.handle_pointer(
                    Rect {
                        x: area.x + 24,
                        y: area.y + 174,
                        width: 150,
                        height: 3 * 28,
                    },
                    &SPEEDS,
                    pointer,
                );
                if selected {
                    crate::drivers::mouse::set_sensitivity([1, 2, 4][self.mouse_speed.selected]);
                }
                toggled || selected
            }
            Event::Key(b'\t') => {
                self.focus.next(2);
                true
            }
            Event::Key(b' ') if self.focus.selected() == 0 => {
                self.dark_theme.checked = !self.dark_theme.checked;
                crate::graphics::wm::set_dark_theme(self.dark_theme.checked);
                true
            }
            Event::Key(b'-' | b'[') if self.focus.selected() == 1 => {
                self.mouse_speed.selected = self.mouse_speed.selected.saturating_sub(1);
                crate::drivers::mouse::set_sensitivity([1, 2, 4][self.mouse_speed.selected]);
                true
            }
            Event::Key(b'+' | b']') if self.focus.selected() == 1 => {
                self.mouse_speed.selected = (self.mouse_speed.selected + 1).min(2);
                crate::drivers::mouse::set_sensitivity([1, 2, 4][self.mouse_speed.selected]);
                true
            }
            Event::Key(_) => false,
        }
    }
}
