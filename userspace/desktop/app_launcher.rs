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

// App launcher for userspace desktop

use crate::desktop::syscalls::{SpawnAppRequest, RedrawRequest, spawn_app, request_redraw, terminate_process};
use crate::desktop::framebuffer::AppRect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserProcessError {
    OutOfMemory,
    InvalidElf,
    NoLoadableSegments,
    InvalidEntryPoint,
    LoadFailed,
}

pub fn spawn_app(app_id: u32, content: AppRect) -> Result<u32, UserProcessError> {
    let req = SpawnAppRequest {
        app_id,
        x: content.x,
        y: content.y,
        width: content.width,
        height: content.height,
    };
    spawn_app(&req).map_err(|_| UserProcessError::LoadFailed)
}

pub fn request_redraw(pid: u32, content: AppRect, mouse_x: u32, mouse_y: u32, left_down: bool) -> Result<(), UserProcessError> {
    let req = RedrawRequest {
        pid,
        x: content.x,
        y: content.y,
        width: content.width,
        height: content.height,
        mouse_x,
        mouse_y,
        left_down: left_down as u32,
    };
    request_redraw(&req).map_err(|_| UserProcessError::LoadFailed)
}

pub fn terminate(pid: u32) -> Result<(), UserProcessError> {
    terminate_process(pid).map_err(|_| UserProcessError::LoadFailed)
}

pub fn process_exists(pid: u32) -> bool {
    // In userspace, we can't directly check - we'll track this in the window manager
    true
}