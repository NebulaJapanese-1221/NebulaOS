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

use super::gui::{Application, Canvas, Event, PointerEvent, Rect, TextField};
use crate::syscall;

const TEXT: u32 = 0x111827;
const MUTED: u32 = 0xA8B5C9;
const ACCENT: u32 = 0x66D9EF;
const WARN: u32 = 0xE74C3C;
const ROW_HEIGHT: u32 = 16;
const MAX_VISIBLE: usize = 22;

struct DocEntry {
    name: &'static str,
    desc: &'static str,
}

const DOCS: [DocEntry; 22] = [
    DocEntry {
        name: "print",
        desc: "Write a string to the system console (SYSCALL_PRINT).",
    },
    DocEntry {
        name: "exit",
        desc: "Terminate the current user process (SYSCALL_EXIT).",
    },
    DocEntry {
        name: "get_ticks",
        desc: "Return timer ticks since boot (SYSCALL_GET_TICKS).",
    },
    DocEntry {
        name: "draw_fill_rect",
        desc: "Draw a filled rectangle in the app window (SYSCALL_DRAW_FILL_RECT).",
    },
    DocEntry {
        name: "draw_rect",
        desc: "Draw a rectangle outline (SYSCALL_DRAW_RECT).",
    },
    DocEntry {
        name: "draw_text",
        desc: "Render text at a position (SYSCALL_DRAW_TEXT).",
    },
    DocEntry {
        name: "poll_event",
        desc: "Wait for and retrieve the next input event (SYSCALL_POLL_EVENT).",
    },
    DocEntry {
        name: "get_app_context",
        desc: "Get app ID, window rect, theme, and sensitivity (SYSCALL_GET_APP_CONTEXT).",
    },
    DocEntry {
        name: "storage_list",
        desc: "List files in storage (SYSCALL_STORAGE_LIST).",
    },
    DocEntry {
        name: "storage_read",
        desc: "Read file contents from storage (SYSCALL_STORAGE_READ).",
    },
    DocEntry {
        name: "storage_write",
        desc: "Write data to a file in storage (SYSCALL_STORAGE_WRITE).",
    },
    DocEntry {
        name: "storage_delete",
        desc: "Delete a file from storage (SYSCALL_STORAGE_DELETE).",
    },
    DocEntry {
        name: "storage_rename",
        desc: "Rename a file in storage (SYSCALL_STORAGE_RENAME).",
    },
    DocEntry {
        name: "storage_format",
        desc: "Format the storage disk if blank (SYSCALL_STORAGE_FORMAT).",
    },
    DocEntry {
        name: "mkdir",
        desc: "Create a directory (syscall op 6).",
    },
    DocEntry {
        name: "system_stats",
        desc: "Retrieve memory frame statistics (SYSCALL_SYSTEM_STATS).",
    },
    DocEntry {
        name: "set_theme",
        desc: "Set dark/light wallpaper theme (SYSCALL_SET_WALLPAPER_THEME).",
    },
    DocEntry {
        name: "set_mouse_sensitivity",
        desc: "Set mouse speed 1-4 (SYSCALL_SET_MOUSE_SENSITIVITY).",
    },
    DocEntry {
        name: "process_info",
        desc: "List all user processes and their states (SYSCALL_PROCESS_INFO).",
    },
    DocEntry {
        name: "memory_info",
        desc: "Retrieve total/free/allocated frame info (SYSCALL_MEMORY_INFO).",
    },
    DocEntry {
        name: "device_info",
        desc: "Retrieve framebuffer, mouse, and storage status (SYSCALL_DEVICE_INFO).",
    },
    DocEntry {
        name: "yield",
        desc: "Yield the CPU to another ready process (SYSCALL_YIELD).",
    },
    DocEntry {
        name: "sleep",
        desc: "Sleep the current process for N ticks (SYSCALL_SLEEP).",
    },
    DocEntry {
        name: "terminate_process",
        desc: "Terminate a user process by PID (SYSCALL_TERMINATE_PROCESS).",
    },
    DocEntry {
        name: "reboot",
        desc: "Reboot the system (SYSCALL_REBOOT).",
    },
    DocEntry {
        name: "shutdown",
        desc: "Power off the system (SYSCALL_SHUTDOWN).",
    },
    DocEntry {
        name: "clipboard_set",
        desc: "Copy text to the shared clipboard (SYSCALL_CLIPBOARD_SET).",
    },
    DocEntry {
        name: "clipboard_get",
        desc: "Paste text from the shared clipboard (SYSCALL_CLIPBOARD_GET).",
    },
];

pub struct HelpViewer {
    entries: &'static [DocEntry],
    filtered: Vec<usize>,
    filter_field: TextField,
    filter_input: [u8; 32],
    filter_len: usize,
    scroll: usize,
    status: &'static str,
}

impl HelpViewer {
    pub fn new() -> Self {
        let mut viewer = Self {
            entries: &DOCS,
            filtered: (0..DOCS.len()).collect(),
            filter_field: TextField::new(Rect { x: 0, y: 0, width: 40, height: 24 }),
            filter_input: [0; 32],
            filter_len: 0,
            scroll: 0,
            status: "",
        };
        viewer.apply_filter();
        viewer
    }

    fn apply_filter(&mut self) {
        self.filtered.clear();
        if self.filter_len == 0 {
            self.filtered = (0..DOCS.len()).collect();
            return;
        }
        let input = core::str::from_utf8(&self.filter_input[..self.filter_len])
            .unwrap_or("")
            .to_lowercase();
        for (i, entry) in DOCS.iter().enumerate() {
            if entry.name.contains(&input) || entry.desc.contains(&input) {
                self.filtered.push(i);
            }
        }
    }
}

impl Application for HelpViewer {
    fn title(&self) -> &'static str {
        "HELP"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        canvas.fill_rect(area, 0xF2F5FA);

        canvas.draw_text(area.x + 16, area.y + 20, "NEBULAOS SYSCALL REFERENCE", TEXT, 1);

        // Filter bar
        let panel_y = area.y + 44;
        canvas.draw_rect(area.x + 16, panel_y, area.width.saturating_sub(32), 30, 0xE2E8F0);
        canvas.draw_rect(area.x + 16, panel_y, area.width.saturating_sub(32), 30, 0x516786);
        canvas.draw_text(area.x + 28, panel_y + 8, "Search:", MUTED, 1);

        self.filter_field.rect = Rect {
            x: area.x + 80,
            y: panel_y + 3,
            width: 40,
            height: 24,
        };
        let filter_text = core::str::from_utf8(&self.filter_input[..self.filter_len]).unwrap_or("");
        self.filter_field.render(canvas, filter_text);

        let count_text = build_count_text(self.filtered.len());
        canvas.draw_text(
            area.x + area.width.saturating_sub(100),
            panel_y + 8,
            &count_text,
            ACCENT,
            1,
        );

        // Results list
        let list_y = area.y + 90;
        let mut y = list_y;
        let visible_end = (self.scroll + MAX_VISIBLE).min(self.filtered.len());
        let mut idx = self.scroll;
        while idx < visible_end {
            let entry = &DOCS[self.filtered[idx]];
            let highlighted = idx % 2 == 0;
            canvas.draw_text(area.x + 20, y, entry.name, if highlighted { TEXT } else { MUTED }, 1);
            canvas.draw_text(area.x + 160, y, entry.desc, MUTED, 1);
            y += ROW_HEIGHT;
            idx += 1;
        }

        if self.filtered.is_empty() {
            canvas.draw_text(area.x + 20, list_y, "No matches.", WARN, 1);
        }

        // Scroll hint
        if self.filtered.len() > MAX_VISIBLE {
            canvas.draw_text(
                area.x + 20,
                area.y + area.height.saturating_sub(20),
                "Use arrow keys / scroll to navigate.",
                MUTED,
                1,
            );
        }

        canvas.draw_text(
            area.x + 20,
            area.y + area.height.saturating_sub(4),
            "Type to filter. Press Enter to focus search.",
            MUTED,
            1,
        );
    }

    fn handle_event(&mut self, event: Event, _area: Rect) -> bool {
        match event {
            Event::Key(8) if self.filter_len > 0 => {
                self.filter_len -= 1;
                self.filter_input[self.filter_len] = 0;
                self.apply_filter();
                self.scroll = 0;
                true
            }
            Event::Key(key @ 32..=126) if self.filter_len < self.filter_input.len() => {
                self.filter_input[self.filter_len] = key;
                self.filter_len += 1;
                self.apply_filter();
                self.scroll = 0;
                true
            }
            Event::Key(b'\n') => {
                self.status = "Search field focused.";
                true
            }
            Event::Key(_) => false,
            Event::Pointer(_) => false,
        }
    }

    fn pointer_target(&self, x: u32, y: u32, area: Rect) -> Option<u32> {
        self.filter_field.rect.contains(x, y).then_some(0)
    }
}

fn build_count_text(count: usize) -> [u8; 16] {
    let mut buf = [b' '; 16];
    let mut text = [0u8; 10];
    let mut len = 0;
    if count == 0 {
        text[0] = b'0';
        len = 1;
    } else {
        let mut digits = [0u8; 8];
        let mut count_digits = 0;
        let mut v = count as u32;
        while v > 0 {
            digits[count_digits] = b'0' + (v % 10) as u8;
            count_digits += 1;
            v /= 10;
        }
        for digit in digits[..count_digits].iter().rev() {
            text[len] = *digit;
            len += 1;
        }
    }
    let matches = b" matches";
    let mut i = 0;
    for b in text[..len].iter() {
        buf[i] = *b;
        i += 1;
    }
    for b in matches {
        buf[i] = *b;
        i += 1;
    }
    buf
}
