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

use crate::drivers::framebuffer;

pub fn show(title: &str, message: &str) {
    let width = framebuffer::get_width();
    let height = framebuffer::get_height();
    framebuffer::clear_screen(0x101820);
    let panel_y = (height / 2).saturating_sub(54);
    framebuffer::fill_rect(
        0,
        panel_y,
        width,
        108.min(height.saturating_sub(panel_y)),
        0x17263A,
    );
    framebuffer::fill_rect(
        0,
        panel_y,
        width,
        3.min(height.saturating_sub(panel_y)),
        0x66D9EF,
    );
    let title_x = width.saturating_sub(title.len() as u32 * 12) / 2;
    let message_x = width.saturating_sub(message.len() as u32 * 6) / 2;
    framebuffer::draw_text(title_x, (height / 2).saturating_sub(22), title, 0xF1F5F9, 2);
    framebuffer::draw_text(
        message_x,
        (height / 2).saturating_add(16),
        message,
        0xA8B5C9,
        1,
    );
    framebuffer::present();
}
