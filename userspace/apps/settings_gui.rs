// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

use super::gui::{Application, Button, Canvas, Checkbox, Event, Menu, PointerEvent, Rect};

const CATEGORIES: [&str; 3] = ["DISPLAY", "MOUSE", "SYSTEM"];
const SENSITIVITIES: [&str; 4] = ["1 - LOW", "2 - MED-LOW", "3 - MED-HIGH", "4 - HIGH"];

pub struct Settings {
    dark_theme: Checkbox,
    mouse_sensitivity: Menu,
    category_buttons: [Button; 3],
    category: usize,
    status: &'static str,
}

impl Settings {
    pub fn new_with_state(dark_theme: bool, mouse_sensitivity: u32) -> Self {
        Self {
            dark_theme: Checkbox::new(dark_theme),
            mouse_sensitivity: Menu::new(mouse_sensitivity.saturating_sub(1).min(3) as usize),
            category_buttons: [
                Button::new(CATEGORIES[0]),
                Button::new(CATEGORIES[1]),
                Button::new(CATEGORIES[2]),
            ],
            category: 0,
            status: "",
        }
    }

    fn category_rect(area: Rect, index: usize) -> Rect {
        Rect {
            x: area.x + 16 + index as u32 * 128,
            y: area.y + 12,
            width: 116,
            height: 28,
        }
    }

    fn display_rect(area: Rect) -> Rect {
        Rect {
            x: area.x + 24,
            y: area.y + 72,
            width: 16,
            height: 16,
        }
    }

    fn mouse_rect(area: Rect) -> Rect {
        Rect {
            x: area.x + 24,
            y: area.y + 76,
            width: 180,
            height: 4 * 28,
        }
    }

    fn change_sensitivity(&mut self, selected: usize) {
        let selected = selected.min(SENSITIVITIES.len() - 1);
        let old = self.mouse_sensitivity.selected;
        self.mouse_sensitivity.selected = selected;
        if crate::syscall::set_mouse_sensitivity((selected + 1) as u32).is_err() {
            self.mouse_sensitivity.selected = old;
            self.status = "SYSTEM SERVICE UNAVAILABLE";
        } else {
            self.status = "";
        }
    }

    fn apply_theme(&mut self) {
        if crate::syscall::set_dark_theme(self.dark_theme.checked).is_err() {
            self.dark_theme.checked = !self.dark_theme.checked;
            self.status = "SYSTEM SERVICE UNAVAILABLE";
        } else {
            self.status = "";
        }
    }
}

impl Application for Settings {
    fn title(&self) -> &'static str {
        "SETTINGS"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        for (index, button) in self.category_buttons.iter().enumerate() {
            let rect = Self::category_rect(area, index);
            button.draw_chrome(canvas, rect, pointer);
            let color = if self.category == index {
                0x66D9EF
            } else {
                0xF1F5F9
            };
            canvas.draw_text(
                rect.x
                    + rect
                        .width
                        .saturating_sub(CATEGORIES[index].len() as u32 * 6)
                        / 2,
                rect.y + 10,
                CATEGORIES[index],
                color,
                1,
            );
        }

        match self.category {
            0 => {
                canvas.draw_text(area.x + 24, area.y + 56, "WALLPAPER", 0x1596B5, 1);
                self.dark_theme
                    .render(canvas, Self::display_rect(area), "DARK WALLPAPER");
                canvas.draw_text(area.x + 24, area.y + 116, "DISPLAY MODE", 0x1596B5, 1);
                canvas.draw_text(
                    area.x + 24,
                    area.y + 138,
                    "1024 X 768 (SET BY GRUB AT BOOT)",
                    0x111827,
                    1,
                );
            }
            1 => {
                canvas.draw_text(area.x + 24, area.y + 56, "POINTER SENSITIVITY", 0x1596B5, 1);
                self.mouse_sensitivity
                    .render(canvas, Self::mouse_rect(area), &SENSITIVITIES);
                canvas.draw_text(
                    area.x + 24,
                    area.y + 202,
                    "Select a level from 1 (slow) to 4 (fast).",
                    0x111827,
                    1,
                );
            }
            _ => {
                canvas.draw_text(area.x + 24, area.y + 56, "SYSTEM", 0x1596B5, 1);
                canvas.draw_text(area.x + 24, area.y + 84, "KEYBOARD: US QWERTY", 0x111827, 1);
                canvas.draw_text(area.x + 24, area.y + 108, "APPS RUN IN RING 3", 0x111827, 1);
                canvas.draw_text(
                    area.x + 24,
                    area.y + 132,
                    "USE TAB TO SWITCH CATEGORIES",
                    0x111827,
                    1,
                );
            }
        }
        canvas.draw_text(
            area.x + 24,
            area.y + area.height.saturating_sub(18),
            self.status,
            0xA23B45,
            1,
        );
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match event {
            Event::Pointer(pointer) => {
                let mut changed = false;
                if self.category == 0 {
                    if self
                        .dark_theme
                        .handle_pointer(Self::display_rect(area), pointer)
                    {
                        self.apply_theme();
                        changed = true;
                    }
                }
                if self.category == 1
                    && self.mouse_sensitivity.handle_pointer(
                        Self::mouse_rect(area),
                        &SENSITIVITIES,
                        pointer,
                    )
                {
                    self.change_sensitivity(self.mouse_sensitivity.selected);
                    changed = true;
                }

                for (index, button) in self.category_buttons.iter_mut().enumerate() {
                    if button.handle_pointer(Self::category_rect(area, index), pointer) {
                        changed |= self.category != index;
                        self.category = index;
                    }
                }
                changed
            }
            Event::Key(b'\t') => {
                self.category = (self.category + 1) % CATEGORIES.len();
                self.status = "";
                true
            }
            Event::Key(b' ') if self.category == 0 => {
                self.dark_theme.checked = !self.dark_theme.checked;
                self.apply_theme();
                true
            }
            Event::Key(b'-' | b'[') if self.category == 1 => {
                self.change_sensitivity(self.mouse_sensitivity.selected.saturating_sub(1));
                true
            }
            Event::Key(b'+' | b']') if self.category == 1 => {
                self.change_sensitivity((self.mouse_sensitivity.selected + 1).min(3));
                true
            }
            Event::Key(_) => false,
        }
    }

    fn pointer_target(&self, x: u32, y: u32, area: Rect) -> Option<u32> {
        (0..self.category_buttons.len())
            .find(|index| Self::category_rect(area, *index).contains(x, y))
            .map(|index| index as u32)
    }
}
