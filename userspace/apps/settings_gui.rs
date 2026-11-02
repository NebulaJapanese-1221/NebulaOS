use super::gui::{Application, Canvas, Checkbox, Event, FocusGroup, Menu, PointerEvent, Rect};

const SPEEDS: [&str; 3] = ["SLOW", "NORMAL", "FAST"];

pub struct Settings {
    dark_theme: Checkbox,
    mouse_speed: Menu,
    focus: FocusGroup,
    status: &'static str,
}

impl Settings {
    pub fn new() -> Self {
        Self::new_with_state(true, 2)
    }

    pub fn new_with_state(dark_theme: bool, mouse_sensitivity: u32) -> Self {
        let selected_speed = match mouse_sensitivity {
            1 => 0,
            4 => 2,
            _ => 1,
        };
        Self {
            dark_theme: Checkbox::new(dark_theme),
            mouse_speed: Menu::new(selected_speed),
            focus: FocusGroup::new(0),
            status: "",
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
            area.y + 220,
            "KEYBOARD: US QWERTY",
            0x111827,
            1,
        );
        canvas.draw_text(area.x + 24, area.y + 238, self.status, 0xA23B45, 1);
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
                    if crate::syscall::set_dark_theme(self.dark_theme.checked).is_err() {
                        self.dark_theme.checked = !self.dark_theme.checked;
                        self.status = "SYSTEM SERVICE UNAVAILABLE";
                        return true;
                    }
                }
                let old_speed = self.mouse_speed.selected;
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
                    if crate::syscall::set_mouse_sensitivity([1, 2, 4][self.mouse_speed.selected])
                        .is_err()
                    {
                        self.mouse_speed.selected = old_speed;
                        self.status = "SYSTEM SERVICE UNAVAILABLE";
                        return true;
                    }
                }
                self.status = "";
                toggled || selected
            }
            Event::Key(b'\t') => {
                self.focus.next(2);
                true
            }
            Event::Key(b' ') if self.focus.selected() == 0 => {
                self.dark_theme.checked = !self.dark_theme.checked;
                if crate::syscall::set_dark_theme(self.dark_theme.checked).is_err() {
                    self.dark_theme.checked = !self.dark_theme.checked;
                    self.status = "SYSTEM SERVICE UNAVAILABLE";
                } else {
                    self.status = "";
                }
                true
            }
            Event::Key(b'-' | b'[') if self.focus.selected() == 1 => {
                let old_speed = self.mouse_speed.selected;
                self.mouse_speed.selected = self.mouse_speed.selected.saturating_sub(1);
                if crate::syscall::set_mouse_sensitivity([1, 2, 4][self.mouse_speed.selected])
                    .is_err()
                {
                    self.mouse_speed.selected = old_speed;
                    self.status = "SYSTEM SERVICE UNAVAILABLE";
                } else {
                    self.status = "";
                }
                true
            }
            Event::Key(b'+' | b']') if self.focus.selected() == 1 => {
                let old_speed = self.mouse_speed.selected;
                self.mouse_speed.selected = (self.mouse_speed.selected + 1).min(2);
                if crate::syscall::set_mouse_sensitivity([1, 2, 4][self.mouse_speed.selected])
                    .is_err()
                {
                    self.mouse_speed.selected = old_speed;
                    self.status = "SYSTEM SERVICE UNAVAILABLE";
                } else {
                    self.status = "";
                }
                true
            }
            Event::Key(_) => false,
        }
    }
}
