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
use crate::{storage, syscall};

const TEXT: u32 = 0x111827;
const MUTED: u32 = 0xA8B5C9;
const ACCENT: u32 = 0x66D9EF;
const WARN: u32 = 0xE74C3C;
const FILE_BUFFER_SIZE: usize = 512;
const MAX_FILES: usize = 16;
const ROW_HEIGHT: u32 = 18;

pub struct FileViewer {
    files: storage::FileInfoArray,
    file_count: usize,
    selected: Option<usize>,
    content: [u8; FILE_BUFFER_SIZE],
    content_len: usize,
    name_field: TextField,
    name_input: [u8; 16],
    name_len: usize,
    status: &'static str,
    scroll: usize,
}

impl FileViewer {
    pub fn new() -> Self {
        Self {
            files: storage::FileInfoArray::default(),
            file_count: 0,
            selected: None,
            content: [0; FILE_BUFFER_SIZE],
            content_len: 0,
            name_field: TextField::new(Rect { x: 0, y: 0, width: 60, height: 24 }),
            name_input: [0; 16],
            name_len: 0,
            status: "",
            scroll: 0,
        }
    }

    fn refresh_files(&mut self) {
        match storage::list_files() {
            Ok(arr) => {
                self.files = arr;
                self.file_count = arr.count();
                self.selected = None;
                self.content_len = 0;
            }
            Err(_) => {
                self.file_count = 0;
                self.status = "Storage unavailable.";
            }
        }
    }

    fn load_selected(&mut self) {
        let Some(index) = self.selected else {
            return;
        };
        if index >= self.file_count {
            return;
        }
        let file = &self.files.entries[index];
        let name = file.name_str();
        let mut filename = [0u8; 15];
        let name_len = name.len().min(filename.len());
        for (d, s) in filename[..name_len].iter_mut().zip(name.as_bytes()[..name_len].iter()) {
            *d = *s;
        }
        match storage::read_file(&filename[..name_len], &mut self.content) {
            Ok(len) => {
                self.content_len = len;
                self.status = "File loaded.";
            }
            Err(_) => {
                self.content_len = 0;
                self.status = "Could not read file.";
            }
        }
    }
}

impl Application for FileViewer {
    fn title(&self) -> &'static str {
        "FILE VIEWER"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        canvas.fill_rect(area, 0xF2F5FA);

        canvas.draw_text(area.x + 16, area.y + 20, "FILE VIEWER", TEXT, 1);

        // File list panel (left)
        let list_x = area.x + 16;
        let list_y = area.y + 48;
        let list_w = 220u32;
        canvas.draw_rect(list_x, list_y, list_w, area.height.saturating_sub(96), 0xE2E8F0);
        canvas.draw_rect(list_x, list_y, list_w, area.height.saturating_sub(96), 0x516786);
        canvas.draw_text(list_x + 8, list_y + 8, "FILES", MUTED, 1);

        let mut y = list_y + 32;
        for i in 0..self.file_count {
            if y + ROW_HEIGHT > area.height.saturating_sub(40) {
                break;
            }
            let file = &self.files.entries[i];
            let is_selected = self.selected == Some(i);
            let row_color = if is_selected { ACCENT } else { TEXT };
            let name = file.name_str();
            let name_len = name.len().min(24);
            if let Ok(text) = core::str::from_utf8(&name.as_bytes()[..name_len]) {
                canvas.draw_text(list_x + 12, y, text, row_color, 1);
            }
            let size_str = file.length.to_string();
            canvas.draw_text(list_x + 180, y, &size_str, MUTED, 1);
            y += ROW_HEIGHT;
        }

        if self.file_count == 0 {
            canvas.draw_text(list_x + 12, list_y + 32, "No files.", MUTED, 1);
        }

        // Content panel (right) - hex dump
        let content_x = area.x + 252;
        let content_y = area.y + 48;
        let content_w = area.width.saturating_sub(268);
        canvas.draw_rect(content_x, content_y, content_w, area.height.saturating_sub(96), 0xE2E8F0);
        canvas.draw_rect(content_x, content_y, content_w, area.height.saturating_sub(96), 0x516786);
        canvas.draw_text(content_x + 8, content_y + 8, "CONTENT (HEX)", MUTED, 1);

        let mut cy = content_y + 32;
        let bytes_per_row = 16u32;
        let visible_rows = ((area.height.saturating_sub(120)) / 14) as usize;
        let end = (self.scroll + visible_rows).min(self.content_len.div_ceil(bytes_per_row as usize));
        let mut row_index = self.scroll;
        while row_index < end {
            let offset = row_index * bytes_per_row as usize;
            let remaining = self.content_len.saturating_sub(offset);
            let row_bytes = remaining.min(bytes_per_row as usize);

            // Offset address
            let mut addr_buf = [0u8; 8];
            let mut addr_len = 0;
            let mut addr_val = offset as u32;
            for _ in 0..8 {
                addr_buf[7 - addr_len] = b'0' + (addr_val % 16) as u8;
                addr_val /= 16;
                addr_len += 1;
                if addr_val == 0 {
                    break;
                }
            }
            canvas.draw_text(content_x + 8, cy, &format_hex_prefix(&addr_buf[8 - addr_len..]), MUTED, 1);

            // Hex bytes
            let mut bx = content_x + 52;
            for b in 0..bytes_per_row {
                if b < row_bytes as u32 {
                    let byte = self.content[offset + b as usize];
                    canvas.draw_text(bx, cy, &byte_to_hex(byte), TEXT, 1);
                }
                bx += 24;
            }

            // ASCII
            let mut ax = content_x + 52 + bytes_per_row * 24 + 8;
            for b in 0..row_bytes as u32 {
                let byte = self.content[offset + b as usize];
                let ch = if byte.is_ascii_graphic() || byte == b' ' { byte } else { b'.' };
                let mut ch_buf = [0u8; 1];
                ch_buf[0] = ch;
                if let Ok(t) = core::str::from_utf8(&ch_buf) {
                    canvas.draw_text(ax, cy, t, TEXT, 1);
                }
                ax += 8;
            }

            cy += 14;
            row_index += 1;
        }

        // Bottom panel: open file by name
        let panel_y = area.y + area.height.saturating_sub(44);
        canvas.draw_rect(area.x + 16, panel_y, area.width.saturating_sub(32), 34, 0xE2E8F0);
        canvas.draw_rect(area.x + 16, panel_y, area.width.saturating_sub(32), 34, 0x516786);
        canvas.draw_text(area.x + 28, panel_y + 10, "Open:", MUTED, 1);

        self.name_field.rect = Rect {
            x: area.x + 70,
            y: panel_y + 5,
            width: 60,
            height: 24,
        };
        let name_text = core::str::from_utf8(&self.name_input[..self.name_len]).unwrap_or("");
        self.name_field.render(canvas, name_text);

        let open_btn = Rect {
            x: area.x + 144,
            y: panel_y + 5,
            width: 56,
            height: 24,
        };
        super::gui::Button::new("OPEN").render(canvas, open_btn, pointer);

        canvas.draw_text(area.x + 28, panel_y + 32, self.status, WARN, 1);
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match event {
            Event::Key(8) if self.name_len > 0 => {
                self.name_len -= 1;
                self.name_input[self.name_len] = 0;
                true
            }
            Event::Key(key @ 32..=126) if self.name_len < self.name_input.len() => {
                self.name_input[self.name_len] = key;
                self.name_len += 1;
                true
            }
            Event::Key(b'\n') => {
                self.open_by_name();
                true
            }
            Event::Key(_) => false,
            Event::Pointer(pointer) => {
                if self.name_field.handle_pointer(pointer) {
                    return true;
                }
                // File list clicks
                let list_x = area.x + 16;
                let mut y = area.y + 48 + 32;
                for i in 0..self.file_count {
                    if y + ROW_HEIGHT > area.height.saturating_sub(40) {
                        break;
                    }
                    let row_rect = Rect {
                        x: list_x,
                        y,
                        width: 220,
                        height: ROW_HEIGHT,
                    };
                    if pointer.left_down && row_rect.contains(pointer.x, pointer.y) {
                        self.selected = Some(i);
                        self.load_selected();
                        return true;
                    }
                    y += ROW_HEIGHT;
                }
                // Open button
                let open_btn = Rect {
                    x: area.x + 144,
                    y: area.y + area.height.saturating_sub(39),
                    width: 56,
                    height: 24,
                };
                if pointer.left_down && open_btn.contains(pointer.x, pointer.y) {
                    self.open_by_name();
                    return true;
                }
                false
            }
        }
    }

    fn pointer_target(&self, x: u32, y: u32, area: Rect) -> Option<u32> {
        let open_btn = Rect {
            x: area.x + 144,
            y: area.y + area.height.saturating_sub(39),
            width: 56,
            height: 24,
        };
        if open_btn.contains(x, y) {
            return Some(0);
        }
        self.name_field.rect.contains(x, y).then_some(1)
    }

    fn open_by_name(&mut self) {
        if self.name_len == 0 {
            self.status = "Enter a filename.";
            return;
        }
        let name = &self.name_input[..self.name_len];
        let mut filename = [0u8; 15];
        let name_len = name.len().min(filename.len());
        for (d, s) in filename[..name_len].iter_mut().zip(name.iter()) {
            *d = *s;
        }
        match storage::read_file(&filename[..name_len], &mut self.content) {
            Ok(len) => {
                self.content_len = len;
                self.selected = None;
                self.status = "File loaded.";
            }
            Err(_) => {
                self.content_len = 0;
                self.status = "Could not read file.";
            }
        }
    }
}

fn byte_to_hex(byte: u8) -> [u8; 2] {
    let hi = byte >> 4;
    let lo = byte & 0x0F;
    [
        if hi < 10 { b'0' + hi } else { b'A' + (hi - 10) },
        if lo < 10 { b'0' + lo } else { b'A' + (lo - 10) },
    ]
}

fn format_hex_prefix(bytes: &[u8]) -> [u8; 9] {
    let mut buf = [b'0'; 9];
    buf[0] = b'0';
    buf[1] = b'x';
    let mut i = 2;
    for b in bytes {
        buf[i] = *b;
        i += 1;
    }
    buf
}
