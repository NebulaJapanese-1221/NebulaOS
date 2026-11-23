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

// Userspace input wrapper

use crate::desktop::syscalls::{get_key, get_mouse};

#[derive(Clone, Copy, Debug, Default)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

pub fn poll_key() -> Option<u8> {
    get_key()
}

pub fn poll() -> Option<MouseState> {
    get_mouse().map(|(x, y, left, right, middle)| MouseState {
        x: x as i32,
        y: y as i32,
        left,
        right,
        middle,
    })
}

pub fn init() {
    // Input is handled by kernel, no init needed in userspace
}