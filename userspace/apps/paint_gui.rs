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

use super::gui::{Application, Button, Canvas, Event, Menu, PointerEvent, Rect};

const COLORS: [u32; 5] = [0x111827, 0xE45B67, 0xF7B955, 0x1596B5, 0xF1F5F9];
const COLOR_NAMES: [&str; 5] = ["INK", "RED", "GOLD", "BLUE", "WHITE"];

pub struct Paint {
    marks: [(u16, u16, u32); 256],
    mark_count: usize,
    previous: Option<(u32, u32)>,
    color_menu: Menu,
    clear_button: Button,
}

impl Paint {
    pub fn new() -> Self {
        Self {
            marks: [(0, 0, COLORS[0]); 256],
            mark_count: 0,
            previous: None,
            color_menu: Menu::new(0),
            clear_button: Button::new("CLEAR"),
        }
    }

    fn canvas_rect(area: Rect) -> Rect {
        Rect {
            x: area.x + 120,
            y: area.y + 16,
            width: area.width.saturating_sub(140),
            height: area.height.saturating_sub(32),
        }
    }

    fn clear_rect(area: Rect) -> Rect {
        Rect {
            x: area.x + 16,
            y: area.y + 160,
            width: 82,
            height: 26,
        }
    }

    fn draw_line(&mut self, from: (u32, u32), to: (u32, u32), paper: Rect) {
        let mut x0 = from.0 as i32;
        let mut y0 = from.1 as i32;
        let x1 = to.0 as i32;
        let y1 = to.1 as i32;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        let color = COLORS[self.color_menu.selected];
        loop {
            if self.mark_count == self.marks.len() {
                self.marks.copy_within(1.., 0);
                self.mark_count -= 1;
            }
            self.marks[self.mark_count] = (
                x0.saturating_sub(paper.x as i32) as u16,
                y0.saturating_sub(paper.y as i32) as u16,
                color,
            );
            self.mark_count += 1;
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice = 2 * error;
            if twice >= dy {
                error += dy;
                x0 += sx;
            }
            if twice <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }
}

impl Application for Paint {
    fn title(&self) -> &'static str {
        "PAINT"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        let paper = Self::canvas_rect(area);
        canvas.fill_rect(paper, 0xF1F5F9);
        canvas.draw_rect(paper, 0x516786);
        for &(x, y, color) in &self.marks[..self.mark_count] {
            let x = paper.x + x as u32;
            let y = paper.y + y as u32;
            if paper.contains(x, y) {
                canvas.fill_rect(
                    Rect {
                        x,
                        y,
                        width: 3,
                        height: 3,
                    },
                    color,
                );
            }
        }
        canvas.draw_text(area.x + 16, area.y + 18, "COLOR", 0xF1F5F9, 1);
        self.color_menu.render(
            canvas,
            Rect {
                x: area.x + 16,
                y: area.y + 38,
                width: 82,
                height: 5 * 28,
            },
            &COLOR_NAMES,
        );
        self.clear_button
            .render(canvas, Self::clear_rect(area), pointer);
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match event {
            Event::Pointer(pointer) => {
                let mut changed = self.color_menu.handle_pointer(
                    Rect {
                        x: area.x + 16,
                        y: area.y + 38,
                        width: 82,
                        height: 5 * 28,
                    },
                    &COLOR_NAMES,
                    pointer,
                );
                if self
                    .clear_button
                    .handle_pointer(Self::clear_rect(area), pointer)
                {
                    self.mark_count = 0;
                    self.previous = None;
                    changed = true;
                }
                let paper = Self::canvas_rect(area);
                if pointer.left_down && paper.contains(pointer.x, pointer.y) {
                    if let Some(previous) = self.previous {
                        self.draw_line(previous, (pointer.x, pointer.y), paper);
                    } else {
                        self.draw_line((pointer.x, pointer.y), (pointer.x, pointer.y), paper);
                    }
                    self.previous = Some((pointer.x, pointer.y));
                    changed = true;
                } else {
                    self.previous = None;
                }
                changed
            }
            Event::Key(b'c' | b'C') => {
                self.mark_count = 0;
                true
            }
            Event::Key(_) => false,
        }
    }

    fn pointer_target(&self, x: u32, y: u32, area: Rect) -> Option<u32> {
        Self::clear_rect(area).contains(x, y).then_some(0)
    }
}
