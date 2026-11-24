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

use super::gui::{Application, Button, Canvas, Event, PointerEvent, Rect, TextField};
use crate::libc::{process_info, terminate_process, ProcessInfo, ProcessEntry};

const TEXT: u32 = 0x111827;
const MUTED: u32 = 0xA8B5C9;
const ACCENT: u32 = 0x66D9EF;
const WARN: u32 = 0xE74C3C;
const ROW_HEIGHT: u32 = 20;
const MAX_ROWS: usize = 8;

const STATE_LABELS: [&str; 4] = ["UNUSED", "READY", "RUNNING", "BLOCKED"];

pub struct TaskManager {
    info: ProcessInfo,
    kill_field: TextField,
    kill_input: [u8; 16],
    kill_len: usize,
    status: &'static str,
    selected_pid: u32,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            info: ProcessInfo::default(),
            kill_field: TextField::new(Rect {
                x: 0,
                y: 0,
                width: 60,
                height: 24,
            }),
            kill_input: [0; 16],
            kill_len: 0,
            status: "",
            selected_pid: 0,
        }
    }

    fn refresh(&mut self) {
        self.info = ProcessInfo::default();
        let _ = process_info(&mut self.info);
    }

    fn draw_row(canvas: &mut dyn Canvas, x: u32, y: u32, pid: u32, app_id: u32, state: u32, events: u32, color: u32) {
        canvas.draw_text(x + 12, y, "PID ", MUTED, 1);
        let mut buf = [0u8; 4];
        let mut len = 0;
        if pid < 10 {
            buf[0] = b'0' + pid as u8;
            len = 1;
        } else {
            buf[0] = b'0' + (pid / 10) as u8;
            buf[1] = b'0' + (pid % 10) as u8;
            len = 2;
        }
        if let Ok(t) = core::str::from_utf8(&buf[..len]) {
            canvas.draw_text(x + 42, y, t, color, 1);
        }
        canvas.draw_text(x + 80, y, "APP ", MUTED, 1);
        let mut buf2 = [0u8; 4];
        let mut len2 = 0;
        if app_id < 10 {
            buf2[0] = b'0' + app_id as u8;
            len2 = 1;
        } else {
            buf2[0] = b'0' + (app_id / 10) as u8;
            buf2[1] = b'0' + (app_id % 10) as u8;
            len2 = 2;
        }
        if let Ok(t) = core::str::from_utf8(&buf2[..len2]) {
            canvas.draw_text(x + 110, y, t, color, 1);
        }
        let state_label = if state as usize < STATE_LABELS.len() {
            STATE_LABELS[state as usize]
        } else {
            "UNKNOWN"
        };
        canvas.draw_text(x + 150, y, state_label, color, 1);
        canvas.draw_text(x + 240, y, "EV ", MUTED, 1);
        let mut buf3 = [0u8; 4];
        let mut len3 = 0;
        if events < 10 {
            buf3[0] = b'0' + events as u8;
            len3 = 1;
        } else {
            buf3[0] = b'0' + (events / 10) as u8;
            buf3[1] = b'0' + (events % 10) as u8;
            len3 = 2;
        }
        if let Ok(t) = core::str::from_utf8(&buf3[..len3]) {
            canvas.draw_text(x + 266, y, t, color, 1);
        }
    }
}

impl Application for TaskManager {
    fn title(&self) -> &'static str {
        "TASK MANAGER"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        canvas.fill_rect(area, 0xF2F5FA);

        canvas.draw_text(area.x + 16, area.y + 20, "RUNNING USER PROCESSES", TEXT, 1);

        // Column headers
        let hx = area.x + 16;
        canvas.draw_text(hx + 42, area.y + 44, "PID", MUTED, 1);
        canvas.draw_text(hx + 110, area.y + 44, "APP", MUTED, 1);
        canvas.draw_text(hx + 150, area.y + 44, "STATE", MUTED, 1);
        canvas.draw_text(hx + 266, area.y + 44, "EVENTS", MUTED, 1);
        canvas.draw_rect(hx, area.y + 62, area.width.saturating_sub(32), 1, 0x516786);

        // Process rows
        let mut y = area.y + 72;
        for i in 0..self.info.count as usize {
            if i >= MAX_ROWS {
                break;
            }
            let entry = self.info.entries[i];
            let is_selected = self.selected_pid == entry.pid;
            let row_color = if is_selected { ACCENT } else { TEXT };
            Self::draw_row(canvas, hx, y, entry.pid, entry.app_id, entry.state, entry.pending_events, row_color);
            y += ROW_HEIGHT;
        }

        if self.info.count == 0 {
            canvas.draw_text(hx, area.y + 72, "No user processes running.", MUTED, 1);
        }

        // Kill panel
        let panel_y = area.y + area.height.saturating_sub(76);
        canvas.draw_rect(area.x + 16, panel_y, area.width.saturating_sub(32), 58, 0xE2E8F0);
        canvas.draw_rect(area.x + 16, panel_y, area.width.saturating_sub(32), 58, 0x516786);
        canvas.draw_text(area.x + 28, panel_y + 10, "Kill PID:", MUTED, 1);

        self.kill_field.rect = Rect {
            x: area.x + 80,
            y: panel_y + 6,
            width: 60,
            height: 24,
        };
        let kill_text = core::str::from_utf8(&self.kill_input[..self.kill_len]).unwrap_or("");
        self.kill_field.render(canvas, kill_text);

        let kill_btn = Rect {
            x: area.x + 154,
            y: panel_y + 6,
            width: 56,
            height: 24,
        };
        let button = super::gui::Button::new("KILL");
        button.render(canvas, kill_btn, pointer);

        canvas.draw_text(area.x + 28, panel_y + 42, self.status, WARN, 1);
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match event {
            Event::Key(8) if self.kill_len > 0 => {
                self.kill_len -= 1;
                self.kill_input[self.kill_len] = 0;
                true
            }
            Event::Key(key @ 32..=126) if self.kill_len < self.kill_input.len() => {
                self.kill_input[self.kill_len] = key;
                self.kill_len += 1;
                true
            }
            Event::Key(b'\n') => {
                self.do_kill();
                true
            }
            Event::Key(_) => false,
            Event::Pointer(pointer) => {
                if self.kill_field.handle_pointer(pointer) {
                    return true;
                }
                let kill_btn = Rect {
                    x: area.x + 154,
                    y: area.y + area.height.saturating_sub(70),
                    width: 56,
                    height: 24,
                };
                if super::gui::Button::new("KILL").handle_pointer(kill_btn, pointer) {
                    self.do_kill();
                    return true;
                }
                false
            }
        }
    }

    fn pointer_target(&self, x: u32, y: u32, area: Rect) -> Option<u32> {
        let kill_btn = Rect {
            x: area.x + 154,
            y: area.y + area.height.saturating_sub(70),
            width: 56,
            height: 24,
        };
        if kill_btn.contains(x, y) {
            return Some(0);
        }
        self.kill_field.rect.contains(x, y).then_some(1)
    }

    fn do_kill(&mut self) {
        if self.kill_len == 0 {
            self.status = "Enter a PID.";
            return;
        }
        let pid: u32 = self.kill_input[..self.kill_len]
            .iter()
            .try_fold(0u32, |acc, &b| {
                if b.is_ascii_digit() {
                    Some(acc * 10 + (b as u32 - b'0' as u32))
                } else {
                    None
                }
            })
            .unwrap_or(0);
        if pid == 0 {
            self.status = "Invalid PID.";
            return;
        }
        let _ = ProcessInfo::default();
        match terminate_process(pid) {
            Ok(()) => {
                self.status = "Process terminated.";
                self.refresh();
            }
            Err(_) => self.status = "Cannot terminate that process.",
        }
    }
}
