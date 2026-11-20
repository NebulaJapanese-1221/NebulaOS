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

// Login screen: shown before the desktop loads. Requires username "user"
// and no password to proceed. Supports mouse-driven field focus and
// blinking caret textboxes.

use crate::drivers::{framebuffer, keyboard, mouse};
use alloc::string::String;

const BACKGROUND: u32 = 0x0B1220;
const PANEL: u32 = 0x111827;
const ACCENT: u32 = 0x66D9EF;
const TEXT: u32 = 0xF1F5F9;
const MUTED: u32 = 0xA8B5C9;
const WARN: u32 = 0xE74C3C;
const MARGIN_Y: u32 = 60;

const USERNAME: &[u8] = b"user";
const PASSWORD: &[u8] = b"";

#[derive(Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Rect {
    fn contains(self, x: u32, y: u32) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.width)
            && y < self.y.saturating_add(self.height)
    }
}

pub struct LoginScreen {
    username: [u8; 32],
    username_len: usize,
    password: [u8; 32],
    password_len: usize,
    active_field: u8, // 0 = username, 1 = password
    error: bool,
    frame: u32,
    user_rect: Rect,
    pass_rect: Rect,
}

impl LoginScreen {
    pub fn new() -> Self {
        let width = framebuffer::get_width().max(640);
        let _height = framebuffer::get_height().max(480);
        let panel_w = 320u32;
        let panel_x = width.saturating_sub(panel_w) / 2;
        let user_rect = Rect {
            x: panel_x + 24,
            y: 0, // set in render
            width: panel_w.saturating_sub(48),
            height: 24,
        };
        let pass_rect = Rect {
            x: panel_x + 24,
            y: 0, // set in render
            width: panel_w.saturating_sub(48),
            height: 24,
        };
        Self {
            username: [0; 32],
            username_len: 0,
            password: [0; 32],
            password_len: 0,
            active_field: 0,
            error: false,
            frame: 0,
            user_rect,
            pass_rect,
        }
    }

    pub fn run(mut self) -> bool {
        self.render();

        loop {
            self.frame = self.frame.wrapping_add(1);
            if let Some(key) = keyboard::poll_key() {
                if self.handle_key(key) {
                    return true;
                }
                self.render();
            }
            if let Some(m) = mouse::poll() {
                if self.handle_mouse(m) {
                    return true;
                }
                self.render();
            }
        }
    }

    fn handle_key(&mut self, key: u8) -> bool {
        match key {
            8 => {
                if self.active_field == 0 && self.username_len > 0 {
                    self.username_len -= 1;
                    self.username[self.username_len] = 0;
                } else if self.active_field == 1 && self.password_len > 0 {
                    self.password_len -= 1;
                    self.password[self.password_len] = 0;
                }
                self.error = false;
                true
            }
            b'\t' => {
                self.active_field = 1 - self.active_field;
                true
            }
            b'\n' => {
                if self.authenticate() {
                    return true;
                }
                self.error = true;
                true
            }
            key @ (32..=126) => {
                if self.active_field == 0 && self.username_len < self.username.len() {
                    self.username[self.username_len] = key;
                    self.username_len += 1;
                } else if self.active_field == 1 && self.password_len < self.password.len() {
                    self.password[self.password_len] = key;
                    self.password_len += 1;
                }
                self.error = false;
                true
            }
            _ => false,
        }
    }

    fn handle_mouse(&mut self, m: mouse::MouseState) -> bool {
        let x = m.x.max(0) as u32;
        let y = m.y.max(0) as u32;

        if m.left {
            if self.user_rect.contains(x, y) && self.active_field != 0 {
                self.active_field = 0;
                return false;
            }
            if self.pass_rect.contains(x, y) && self.active_field != 1 {
                self.active_field = 1;
                return false;
            }
        }
        false
    }

    fn authenticate(&self) -> bool {
        let user_ok = self.username_len == USERNAME.len()
            && self.username[..self.username_len] == USERNAME[..];
        let pass_ok = self.password_len == PASSWORD.len()
            && self.password[..self.password_len] == PASSWORD[..];
        user_ok && pass_ok
    }

    fn render(&mut self) {
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        if width == 0 || height == 0 {
            return;
        }

        let panel_w = 400u32;
        let panel_x = width.saturating_sub(panel_w) / 2;
        let title_y = MARGIN_Y;
        let user_label_y = title_y + 70;
        let user_y = user_label_y + 20;
        let pass_label_y = user_y + 50;
        let pass_y = pass_label_y + 20;

        self.user_rect = Rect {
            x: panel_x + 40,
            y: user_y,
            width: panel_w.saturating_sub(80),
            height: 24,
        };
        self.pass_rect = Rect {
            x: panel_x + 40,
            y: pass_y,
            width: panel_w.saturating_sub(80),
            height: 24,
        };

        framebuffer::clear_screen(BACKGROUND);

        // Title
        framebuffer::draw_text(
            panel_x + (panel_w.saturating_sub(14 * 12)) / 2,
            title_y,
            "NEBULAOS LOGIN",
            TEXT,
            2,
        );
        framebuffer::draw_text(
            panel_x + (panel_w.saturating_sub(38 * 6)) / 2,
            title_y + 40,
            "Please log in to start the desktop.",
            MUTED,
            1,
        );

        // Username field
        framebuffer::draw_text(panel_x + 40, user_label_y, "Username:", MUTED, 1);
        framebuffer::fill_rect(
            self.user_rect.x,
            self.user_rect.y,
            self.user_rect.width,
            self.user_rect.height,
            PANEL,
        );
        framebuffer::draw_rect(
            self.user_rect.x,
            self.user_rect.y,
            self.user_rect.width,
            self.user_rect.height,
            if self.active_field == 0 { ACCENT } else { 0x516786 },
        );
        let user_input = core::str::from_utf8(&self.username[..self.username_len]).unwrap_or("");
        framebuffer::draw_text(
            self.user_rect.x + 8,
            self.user_rect.y + 8,
            user_input,
            TEXT,
            1,
        );
        if self.active_field == 0 && self.frame / CARET_BLINK_TICKS % 2 == 0 {
            let caret_x = self.user_rect.x + 8 + self.username_len as u32 * 6;
            framebuffer::fill_rect(
                caret_x,
                self.user_rect.y + 4,
                1,
                self.user_rect.height.saturating_sub(8),
                TEXT,
            );
        }

        // Password field
        framebuffer::draw_text(panel_x + 40, pass_label_y, "Password:", MUTED, 1);
        framebuffer::fill_rect(
            self.pass_rect.x,
            self.pass_rect.y,
            self.pass_rect.width,
            self.pass_rect.height,
            PANEL,
        );
        framebuffer::draw_rect(
            self.pass_rect.x,
            self.pass_rect.y,
            self.pass_rect.width,
            self.pass_rect.height,
            if self.active_field == 1 { ACCENT } else { 0x516786 },
        );
        let pass_display: String =
            core::iter::repeat('*').take(self.password_len).collect();
        framebuffer::draw_text(
            self.pass_rect.x + 8,
            self.pass_rect.y + 8,
            &pass_display,
            TEXT,
            1,
        );
        if self.active_field == 1 && self.frame / CARET_BLINK_TICKS % 2 == 0 {
            let caret_x = self.pass_rect.x + 8 + self.password_len as u32 * 6;
            framebuffer::fill_rect(
                caret_x,
                self.pass_rect.y + 4,
                1,
                self.pass_rect.height.saturating_sub(8),
                TEXT,
            );
        }

        // Error message
        if self.error {
            framebuffer::draw_text(
                panel_x + 40,
                pass_y + 40,
                "Invalid username or password.",
                WARN,
                1,
            );
        }

        // Hints
        framebuffer::draw_text(
            panel_x + 40,
            height - 60,
            "Username: user  |  Password: (none)",
            MUTED,
            1,
        );
        framebuffer::draw_text(
            panel_x + 40,
            height - 40,
            "Click fields | Tab to switch | Enter to log in | F12: diagnostic mode",
            MUTED,
            1,
        );

        framebuffer::present();
    }
}

const CARET_BLINK_TICKS: u32 = 30;
