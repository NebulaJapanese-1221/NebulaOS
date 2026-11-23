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

// Userspace framebuffer wrapper

use crate::desktop::syscalls::{DrawCommand, draw_fill_rect, draw_rect, draw_text, get_width, get_height, present, present_rect, get_cursor, set_cursor, CursorInfo};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CursorType {
    Arrow = 0,
    Text = 1,
    Hand = 2,
    Crosshair = 3,
    Move = 4,
}

pub fn get_width() -> u32 {
    syscalls::get_width()
}

pub fn get_height() -> u32 {
    syscalls::get_height()
}

pub fn fill_rect(x: u32, y: u32, width: u32, height: u32, color: u32) -> Result<(), u32> {
    draw_fill_rect(&DrawCommand::fill_rect(x, y, width, height, color))
}

pub fn draw_rect(x: u32, y: u32, width: u32, height: u32, color: u32) -> Result<(), u32> {
    draw_rect(&DrawCommand::rect(x, y, width, height, color))
}

pub fn draw_text(x: u32, y: u32, text: &str, color: u32, scale: u32) -> Result<(), u32> {
    let width = text.len() as u32 * 6 * scale;
    let height = 8 * scale;
    draw_text(&DrawCommand::text(x, y, width, height, color, text, scale))
}

pub fn present() -> Result<(), u32> {
    syscalls::present()
}

pub fn present_rect(x: u32, y: u32, width: u32, height: u32) -> Result<(), u32> {
    syscalls::present_rect(x, y, width, height)
}

pub fn set_cursor(x: u32, y: u32, cursor_type: CursorType) -> Result<(), u32> {
    set_cursor(&CursorInfo { x, y, cursor_type: cursor_type as u32 })
}

pub fn mark_dirty(x: u32, y: u32, width: u32, height: u32) {
    // In userspace, we don't track dirty rects the same way
    // The kernel handles this when we draw
    let _ = present_rect(x, y, width, height);
}