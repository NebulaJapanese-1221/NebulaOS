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
// and no password to proceed.

use crate::drivers::{framebuffer, keyboard};
use alloc::string::String;

const BACKGROUND: u32 = 0x0B1220;
const PANEL: u32 = 0x111827;
const ACCENT: u32 = 0x66D9EF;
const TEXT: u32 = 0xF1F5F9;
const MUTED: u32 = 0xA8B5C9;
const WARN: u32 = 0xE74C3C;
const MARGIN_X: u32 = 48;
const MARGIN_Y: u32 = 60;

const USERNAME: &[u8] = b"user";
const PASSWORD: &[u8] = b"";

pub struct LoginScreen {
    username: [u8; 32],
    username_len: usize,
    password: [u8; 32],
    password_len: usize,
    active_field: u8, // 0 = username, 1 = password
    error: bool,
}

impl LoginScreen {
    pub fn new() -> Self {
        Self {
            username: [0; 32],
            username_len: 0,
            password: [0; 32],
            password_len: 0,
            active_field: 0,
            error: false,
        }
    }

    pub fn run(mut self) -> bool {
        self.render();

        loop {
            if let Some(key) = keyboard::poll_key() {
                if self.handle_key(key) {
                    return true;
                }
                self.render();
            }
        }
    }

    fn handle_key(&mut self, key: u8) -> bool {
        match key {
            8 => {
                // Backspace
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

    fn authenticate(&self) -> bool {
        let user_ok = self.username_len == USERNAME.len()
            && self.username[..self.username_len] == USERNAME[..];
        let pass_ok = self.password_len == PASSWORD.len()
            && self.password[..self.password_len] == PASSWORD[..];
        user_ok && pass_ok
    }

    fn render(&self) {
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        if width == 0 || height == 0 {
            return;
        }

        framebuffer::clear_screen(BACKGROUND);

        // Title
        framebuffer::draw_text(MARGIN_X, MARGIN_Y, "NEBULAOS LOGIN", TEXT, 2);
        framebuffer::draw_text(
            MARGIN_X,
            MARGIN_Y + 40,
            "Please log in to start the desktop.",
            MUTED,
            1,
        );

        // Username field
        let user_y = MARGIN_Y + 90;
        framebuffer::draw_text(MARGIN_X, user_y, "Username:", MUTED, 1);
        let user_input = core::str::from_utf8(&self.username[..self.username_len]).unwrap_or("");
        framebuffer::fill_rect(MARGIN_X, user_y + 18, 240, 24, PANEL);
        framebuffer::draw_rect(
            MARGIN_X,
            user_y + 18,
            240,
            24,
            if self.active_field == 0 { ACCENT } else { 0x516786 },
        );
        framebuffer::draw_text(MARGIN_X + 8, user_y + 24, user_input, TEXT, 1);

        // Password field
        let pass_y = user_y + 60;
        framebuffer::draw_text(MARGIN_X, pass_y, "Password:", MUTED, 1);
        let pass_display: String = core::iter::repeat('*').take(self.password_len).collect();
        framebuffer::fill_rect(MARGIN_X, pass_y + 18, 240, 24, PANEL);
        framebuffer::draw_rect(
            MARGIN_X,
            pass_y + 18,
            240,
            24,
            if self.active_field == 1 { ACCENT } else { 0x516786 },
        );
        framebuffer::draw_text(MARGIN_X + 8, pass_y + 24, &pass_display, TEXT, 1);

        // Error message
        if self.error {
            framebuffer::draw_text(MARGIN_X, pass_y + 52, "Invalid username or password.", WARN, 1);
        }

        // Hint
        framebuffer::draw_text(
            MARGIN_X,
            height - 40,
            "Username: user  |  Password: (none)  |  Tab to switch, Enter to log in",
            MUTED,
            1,
        );

        framebuffer::present();
    }
}
