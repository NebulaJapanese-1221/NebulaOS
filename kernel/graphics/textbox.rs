// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

// Reusable textbox widget for kernel-space UI.

use crate::drivers::framebuffer;

const TEXTBOX_HEIGHT: u32 = 24;
const TEXTBOX_PADDING: u32 = 8;
const CARET_BLINK_TICKS: u32 = 30;

#[derive(Clone, Copy)]
pub struct Textbox {
    pub rect: Rect,
    pub buffer: [u8; 32],
    pub len: usize,
    pub focused: bool,
    pub password: bool,
    pub caret_blink: bool,
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn contains(self, x: u32, y: u32) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.width)
            && y < self.y.saturating_add(self.height)
    }
}

impl Textbox {
    pub const fn new(rect: Rect) -> Self {
        Self {
            rect,
            buffer: [0; 32],
            len: 0,
            focused: false,
            password: false,
            caret_blink: false,
        }
    }

    pub fn set_password(&mut self, password: bool) {
        self.password = password;
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        self.rect.contains(x, y)
    }

    pub fn handle_key(&mut self, key: u8) -> bool {
        if !self.focused {
            return false;
        }
        match key {
            8 => {
                if self.len > 0 {
                    self.len -= 1;
                    self.buffer[self.len] = 0;
                }
                true
            }
            key @ (32..=126) => {
                if self.len < self.buffer.len() {
                    self.buffer[self.len] = key;
                    self.len += 1;
                }
                true
            }
            _ => false,
        }
    }

    pub fn handle_click(&mut self, x: u32, y: u32) -> bool {
        if self.rect.contains(x, y) {
            self.focused = true;
            true
        } else {
            self.focused = false;
            false
        }
    }

    pub fn blur(&mut self) {
        self.focused = false;
    }

    pub fn set_caret_blink(&mut self, blink: bool) {
        self.caret_blink = blink;
    }

    pub fn render(&self, canvas: &mut dyn Canvas, label: &str, label_y: u32) {
        // Draw label
        framebuffer::draw_text(self.rect.x, label_y, label, MUTED, 1);

        // Draw textbox background
        let bg = if self.focused { PANEL } else { 0x172236 };
        framebuffer::fill_rect(self.rect.x, self.rect.y, self.rect.width, self.rect.height, bg);

        // Draw border
        let border = if self.focused { ACCENT } else { 0x516786 };
        framebuffer::draw_rect(self.rect.x, self.rect.y, self.rect.width, self.rect.height, border);

        // Draw text
        let display: alloc::string::String = if self.password {
            core::iter::repeat('*').take(self.len).collect()
        } else {
            core::str::from_utf8(&self.buffer[..self.len]).unwrap_or("").to_string()
        };
        framebuffer::draw_text(
            self.rect.x + TEXTBOX_PADDING,
            self.rect.y + TEXTBOX_PADDING / 2,
            &display,
            TEXT,
            1,
        );

        // Draw caret
        if self.focused && self.caret_blink {
            let caret_x = self.rect.x + TEXTBOX_PADDING + self.len as u32 * 6;
            framebuffer::fill_rect(
                caret_x,
                self.rect.y + 4,
                1,
                self.rect.height.saturating_sub(8),
                TEXT,
            );
        }
    }

    pub fn text(&self) -> &[u8] {
        &self.buffer[..self.len]
    }

    pub fn clear(&mut self) {
        self.buffer = [0; 32];
        self.len = 0;
    }
}

pub trait Canvas {
    fn draw_text(&mut self, x: u32, y: u32, text: &str, color: u32, scale: u32);
    fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: u32);
    fn draw_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: u32);
}

impl Canvas for () {
    fn draw_text(&mut self, _x: u32, _y: u32, _text: &str, _color: u32, _scale: u32) {}
    fn fill_rect(&mut self, _x: u32, _y: u32, _w: u32, _h: u32, _color: u32) {}
    fn draw_rect(&mut self, _x: u32, _y: u32, _w: u32, _h: u32, _color: u32) {}
}
