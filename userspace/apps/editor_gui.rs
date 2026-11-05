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

use super::gui::{Application, Button, Canvas, Event, PointerEvent, Rect};

pub struct TextEditor {
    input: [u8; 128],
    input_len: usize,
    clear_button: Button,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            input: [0; 128],
            input_len: 0,
            clear_button: Button::new("CLEAR"),
        }
    }

    fn clear_rect(area: Rect) -> Rect {
        Rect {
            x: area.x + area.width.saturating_sub(104),
            y: area.y + area.height.saturating_sub(40),
            width: 82,
            height: 28,
        }
    }
}

impl Application for TextEditor {
    fn title(&self) -> &'static str {
        "TEXT EDITOR"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        let editor = Rect {
            x: area.x + 22,
            y: area.y + 18,
            width: area.width.saturating_sub(44),
            height: area.height.saturating_sub(76),
        };
        canvas.fill_rect(editor, 0xE2E8F0);

        let mut line_start = 0usize;
        let mut line_y = editor.y + 14;
        for index in 0..=self.input_len {
            if index == self.input_len || self.input[index] == b'\n' {
                if let Ok(line) = core::str::from_utf8(&self.input[line_start..index]) {
                    canvas.draw_text(editor.x + 12, line_y, line, 0x111827, 1);
                }
                line_start = index + 1;
                line_y += 13;
                if line_y >= editor.y + editor.height.saturating_sub(4) {
                    break;
                }
            }
        }
        if self.input_len == 0 {
            canvas.draw_text(editor.x + 12, editor.y + 14, "TYPE TO EDIT", 0xA8B5C9, 1);
        }
        self.clear_button
            .render(canvas, Self::clear_rect(area), pointer);
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match event {
            Event::Key(8) if self.input_len > 0 => {
                self.input_len -= 1;
                true
            }
            Event::Key(b'\n') if self.input_len < self.input.len() => {
                self.input[self.input_len] = b'\n';
                self.input_len += 1;
                true
            }
            Event::Key(key @ 32..=126) if self.input_len < self.input.len() => {
                self.input[self.input_len] = key;
                self.input_len += 1;
                true
            }
            Event::Key(_) => false,
            Event::Pointer(pointer) => {
                if self
                    .clear_button
                    .handle_pointer(Self::clear_rect(area), pointer)
                {
                    self.input_len = 0;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn pointer_target(&self, x: u32, y: u32, area: Rect) -> Option<u32> {
        Self::clear_rect(area).contains(x, y).then_some(0)
    }
}
