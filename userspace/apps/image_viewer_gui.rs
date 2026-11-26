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

use super::gui::{Application, Button, Canvas, Event, PointerEvent, Rect, TextField};
use crate::libc;
use alloc::vec::Vec;
use alloc::string::{String, ToString};

const SUPPORTED_EXTENSIONS: [&str; 4] = ["BMP", "PPM", "PBM", "PGM"];

pub struct ImageViewer {
    current_file: Option<String>,
    image_data: Option<ImageData>,
    zoom: u32,
    pan_x: i32,
    pan_y: i32,
    drag_start: Option<(u32, u32)>,
    is_dragging: bool,
    file_list: Vec<String>,
    selected_index: Option<usize>,
    list_scroll: usize,
    open_button: Button,
    close_button: Button,
    zoom_in_button: Button,
    zoom_out_button: Button,
    fit_button: Button,
    prev_button: Button,
    next_button: Button,
    last_error: Option<&'static str>,
}

#[derive(Clone)]
struct ImageData {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

impl ImageViewer {
    pub fn new() -> Self {
        Self {
            current_file: None,
            image_data: None,
            zoom: 1,
            pan_x: 0,
            pan_y: 0,
            drag_start: None,
            is_dragging: false,
            file_list: Vec::new(),
            selected_index: None,
            list_scroll: 0,
            open_button: Button::new("OPEN"),
            close_button: Button::new("CLOSE"),
            zoom_in_button: Button::new("ZOOM +"),
            zoom_out_button: Button::new("ZOOM -"),
            fit_button: Button::new("FIT"),
            prev_button: Button::new("PREV"),
            next_button: Button::new("NEXT"),
            last_error: None,
        }
    }

    fn refresh_file_list(&mut self) {
        self.file_list.clear();
        let mut buf = [libc::VfsDirEntry::default(); 64];
        if let Ok(count) = unsafe { libc::vfs_readdir(3, 0, &mut buf) } {
            for i in 0..count {
                let entry = &buf[i];
                let name = core::str::from_utf8(&entry.name).unwrap_or("");
                let name = name.trim_end_matches('\0');
                if !name.is_empty() && name != "." && name != ".." {
                    let upper = name.to_uppercase();
                    if SUPPORTED_EXTENSIONS.iter().any(|ext| upper.ends_with(ext)) {
                        self.file_list.push(name.to_string());
                    }
                }
            }
        }
    }

    fn load_image(&mut self, filename: &str) -> Result<(), &'static str> {
        let mut file_buf = [0u8; 64 * 1024];
        let fd = unsafe { libc::vfs_open(filename, 0, 0) };
        if fd == u32::MAX {
            return Err("Failed to open file");
        }
        
        let mut total_read = 0;
        loop {
            let mut chunk = [0u8; 4096];
            match unsafe { libc::vfs_read(fd, &mut chunk) } {
                Ok(0) => break,
                Ok(n) => {
                    if total_read + n > file_buf.len() {
                        break;
                    }
                    file_buf[total_read..total_read + n].copy_from_slice(&chunk[..n]);
                    total_read += n;
                }
                Err(_) => {
                    unsafe { libc::vfs_close(fd); }
                    return Err("Read error");
                }
            }
        }
        unsafe { libc::vfs_close(fd); }

        let image = self.parse_image(&file_buf[..total_read], filename)?;
        self.image_data = Some(image);
        self.current_file = Some(filename.to_string());
        self.zoom = 1;
        self.pan_x = 0;
        self.pan_y = 0;
        Ok(())
    }

    fn parse_image(&self, data: &[u8], filename: &str) -> Result<ImageData, &'static str> {
        let upper = filename.to_uppercase();
        
        if upper.ends_with(".BMP") {
            self.parse_bmp(data)
        } else if upper.ends_with(".PPM") {
            self.parse_ppm(data)
        } else if upper.ends_with(".PBM") {
            self.parse_pbm(data)
        } else if upper.ends_with(".PGM") {
            self.parse_pgm(data)
        } else {
            Err("Unsupported format")
        }
    }

    fn parse_bmp(&self, data: &[u8]) -> Result<ImageData, &'static str> {
        if data.len() < 54 {
            return Err("Invalid BMP: too small");
        }
        if data[0] != b'B' || data[1] != b'M' {
            return Err("Invalid BMP: not a BMP file");
        }

        let file_size = u32::from_le_bytes([data[2], data[3], data[4], data[5]]);
        let data_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]);
        let header_size = u32::from_le_bytes([data[14], data[15], data[16], data[17]]);
        
        if header_size < 40 {
            return Err("Unsupported BMP header");
        }

        let width = u32::from_le_bytes([data[18], data[19], data[20], data[21]]);
        let height = u32::from_le_bytes([data[22], data[23], data[24], data[25]]);
        let planes = u16::from_le_bytes([data[26], data[27]]);
        let bits_per_pixel = u16::from_le_bytes([data[28], data[29]]);
        let compression = u32::from_le_bytes([data[30], data[31], data[32], data[33]]);

        if planes != 1 || bits_per_pixel != 24 || compression != 0 {
            return Err("Unsupported BMP format (only 24-bit uncompressed)");
        }

        let row_size = ((width * 3 + 3) / 4) * 4;
        let image_data_offset = data_offset as usize;
        let image_data_end = image_data_offset + (row_size * height) as usize;
        
        if data.len() < image_data_end {
            return Err("BMP data truncated");
        }

        let mut pixels = Vec::with_capacity((width * height) as usize);
        let abs_height = height;
        let top_down = (data[23] & 0x80) != 0;

        for y in 0..abs_height {
            let src_y = if top_down { y } else { abs_height - 1 - y };
            let row_start = image_data_offset + (src_y * row_size) as usize;
            
            for x in 0..width {
                let pixel_offset = row_start + (x * 3) as usize;
                if pixel_offset + 2 >= data.len() {
                    return Err("BMP data truncated");
                }
                let b = data[pixel_offset];
                let g = data[pixel_offset + 1];
                let r = data[pixel_offset + 2];
                let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                pixels.push(color);
            }
        }

        Ok(ImageData { width, height: abs_height, pixels })
    }

    fn parse_ppm(&self, data: &[u8]) -> Result<ImageData, &'static str> {
        let header_end = data.windows(3).position(|w| w == b"\n255\n" || w == b"\r\n255\r\n")
            .or_else(|| data.windows(2).position(|w| w == b"\n255"))
            .ok_or("Invalid PPM header")?;
        
        let header = core::str::from_utf8(&data[..header_end]).map_err(|_| "Invalid PPM header")?;
        let lines: Vec<&str> = header.lines().filter(|l| !l.starts_with('#')).collect();
        
        if lines.len() < 3 || lines[0] != "P6" {
            return Err("Only binary PPM (P6) supported");
        }
        
        let dims: Vec<&str> = lines[1].split_whitespace().collect();
        if dims.len() != 2 {
            return Err("Invalid PPM dimensions");
        }
        let width = dims[0].parse().map_err(|_| "Invalid width")?;
        let height = dims[1].parse().map_err(|_| "Invalid height")?;

        let pixel_data_start = header_end + 4;
        let expected_size = (width * height * 3) as usize;
        if data.len() < pixel_data_start + expected_size {
            return Err("PPM data truncated");
        }

        let mut pixels = Vec::with_capacity((width * height) as usize);
        for i in 0..(width * height) as usize {
            let base = pixel_data_start + i * 3;
            let r = data[base];
            let g = data[base + 1];
            let b = data[base + 2];
            let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
            pixels.push(color);
        }

        Ok(ImageData { width, height, pixels })
    }

    fn parse_pgm(&self, data: &[u8]) -> Result<ImageData, &'static str> {
        let header_end = data.windows(3).position(|w| w == b"\n255\n" || w == b"\r\n255\r\n")
            .or_else(|| data.windows(2).position(|w| w == b"\n255"))
            .ok_or("Invalid PGM header")?;
        
        let header = core::str::from_utf8(&data[..header_end]).map_err(|_| "Invalid PGM header")?;
        let lines: Vec<&str> = header.lines().filter(|l| !l.starts_with('#')).collect();
        
        if lines.len() < 3 || lines[0] != "P5" {
            return Err("Only binary PGM (P5) supported");
        }
        
        let dims: Vec<&str> = lines[1].split_whitespace().collect();
        if dims.len() != 2 {
            return Err("Invalid PGM dimensions");
        }
        let width = dims[0].parse().map_err(|_| "Invalid width")?;
        let height = dims[1].parse().map_err(|_| "Invalid height")?;

        let pixel_data_start = header_end + 4;
        let expected_size = (width * height) as usize;
        if data.len() < pixel_data_start + expected_size {
            return Err("PGM data truncated");
        }

        let mut pixels = Vec::with_capacity((width * height) as usize);
        for i in 0..(width * height) as usize {
            let gray = data[pixel_data_start + i];
            let color = ((gray as u32) << 16) | ((gray as u32) << 8) | (gray as u32);
            pixels.push(color);
        }

        Ok(ImageData { width, height, pixels })
    }

    fn parse_pbm(&self, data: &[u8]) -> Result<ImageData, &'static str> {
        let header_end = data.windows(2).position(|w| w == b"\n\n" || w == b"\r\n\r\n")
            .ok_or("Invalid PBM header")?;
        
        let header = core::str::from_utf8(&data[..header_end]).map_err(|_| "Invalid PBM header")?;
        let lines: Vec<&str> = header.lines().filter(|l| !l.starts_with('#')).collect();
        
        if lines.len() < 2 || lines[0] != "P4" {
            return Err("Only binary PBM (P4) supported");
        }
        
        let dims: Vec<&str> = lines[1].split_whitespace().collect();
        if dims.len() != 2 {
            return Err("Invalid PBM dimensions");
        }
        let width = dims[0].parse().map_err(|_| "Invalid width")?;
        let height = dims[1].parse().map_err(|_| "Invalid height")?;

        let pixel_data_start = header_end + 2;
        let row_bytes = (width + 7) / 8;
        let expected_size = (row_bytes * height) as usize;
        if data.len() < pixel_data_start + expected_size {
            return Err("PBM data truncated");
        }

        let mut pixels = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            let row_start = pixel_data_start + (y * row_bytes) as usize;
            for x in 0..width {
                let byte_idx = x / 8;
                let bit_idx = 7 - (x % 8);
                let byte = data[row_start + byte_idx];
                let pixel = if (byte >> bit_idx) & 1 == 1 { 0xFFFFFF } else { 0x000000 };
                pixels.push(pixel);
            }
        }

        Ok(ImageData { width, height, pixels })
    }

    fn fit_to_window(&mut self, area: Rect) {
        if let Some(img) = &self.image_data {
            let content_w = area.width;
            let content_h = area.height.saturating_sub(80);
            let scale_x = content_w / img.width.max(1);
            let scale_y = content_h / img.height.max(1);
            self.zoom = scale_x.min(scale_y).max(1);
            self.pan_x = ((content_w as i32 - (img.width as i32 * self.zoom as i32)) / 2) as i32;
            self.pan_y = ((content_h as i32 - (img.height as i32 * self.zoom as i32)) / 2) as i32;
        }
    }
}

impl Application for ImageViewer {
    fn title(&self) -> &'static str {
        "IMAGE VIEWER"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        let bg_color = 0xE2E8F0;
        let panel_color = 0xFFFFFF;
        let accent = 0x1596B5;
        let text_color = 0x111827;
        let border = 0x516786;
        let list_bg = 0xF1F5F9;

        canvas.fill_rect(area, bg_color);

        let title_y = area.y + 15;
        canvas.draw_text(area.x + 20, title_y, "IMAGE VIEWER", text_color, 2);

        let toolbar_y = area.y + 50;
        let buttons = [
            (&self.open_button, "OPEN", 20),
            (&self.close_button, "CLOSE", 100),
            (&self.zoom_in_button, "ZOOM +", 180),
            (&self.zoom_out_button, "ZOOM -", 260),
            (&self.fit_button, "FIT", 340),
            (&self.prev_button, "PREV", 420),
            (&self.next_button, "NEXT", 500),
        ];
        for (btn, label, x) in buttons {
            let rect = Rect { x: area.x + x, y: toolbar_y, width: 70, height: 30 };
            let color = if btn.armed { accent } else { panel_color };
            canvas.fill_rect(rect, color);
            canvas.draw_rect(rect, border);
            canvas.draw_text(rect.x + 8, rect.y + 10, label, text_color, 1);
        }

        if let Some(img) = &self.image_data {
            let display_x = area.x + 20 + self.pan_x as u32;
            let display_y = area.y + 100 + self.pan_y as u32;
            let display_w = img.width * self.zoom;
            let display_h = img.height * self.zoom;

            let clip_x = display_x.max(area.x + 20);
            let clip_y = display_y.max(area.y + 100);
            let clip_w = (display_x + display_w).min(area.x + area.width - 20).saturating_sub(clip_x);
            let clip_h = (display_y + display_h).min(area.y + area.height - 20).saturating_sub(clip_y);

            if clip_w > 0 && clip_h > 0 {
                let src_x = ((clip_x as i32 - display_x as i32) / self.zoom as i32).max(0) as u32;
                let src_y = ((clip_y as i32 - display_y as i32) / self.zoom as i32).max(0) as u32;
                let src_w = (clip_w / self.zoom).min(img.width - src_x);
                let src_h = (clip_h / self.zoom).min(img.height - src_y);

                for dy in 0..src_h {
                    for dx in 0..src_w {
                        let pixel = img.pixels[((src_y + dy) * img.width + src_x + dx) as usize];
                        let px = clip_x + dx * self.zoom;
                        let py = clip_y + dy * self.zoom;
                        let rect = Rect { x: px, y: py, width: self.zoom, height: self.zoom };
                        canvas.fill_rect(rect, pixel);
                    }
                }
            }

            let info_y = area.y + area.height - 30;
            let info = alloc::format!("{}x{}  Zoom: {}x  File: {}", img.width, img.height, self.zoom, self.current_file.as_deref().unwrap_or(""));
            canvas.draw_text(area.x + 20, info_y, &info, text_color, 1);
        } else {
            let msg_y = area.y + 150;
            canvas.draw_text(area.x + 20, msg_y, "No image loaded. Click OPEN to select a file.", 0x516786, 1);
            canvas.draw_text(area.x + 20, msg_y + 20, "Supported: BMP, PPM, PGM, PBM", 0x516786, 1);
        }

        let list_x = area.x + area.width.saturating_sub(220);
        let list_y = area.y + 100;
        let list_w = 200;
        let list_h = area.height.saturating_sub(120);
        let list_rect = Rect { x: list_x, y: list_y, width: list_w, height: list_h };
        canvas.fill_rect(list_rect, list_bg);
        canvas.draw_rect(list_rect, border);

        let visible_count = (list_h / 20).min(self.file_list.len() as u32 - self.list_scroll) as usize;
        for i in 0..visible_count {
            let idx = self.list_scroll + i;
            if idx >= self.file_list.len() { break; }
            let item_y = list_y + 5 + (i as u32 * 20);
            let item_rect = Rect { x: list_x + 5, y: item_y, width: list_w - 10, height: 18 };
            let is_selected = self.selected_index == Some(idx);
            if is_selected {
                canvas.fill_rect(item_rect, accent);
            }
            let name = &self.file_list[idx];
            canvas.draw_text(list_x + 10, item_y + 6, name, if is_selected { 0xFFFFFF } else { text_color }, 1);
        }

        if let Some(err) = self.last_error {
            let err_y = area.y + area.height - 40;
            canvas.draw_text(area.x + 20, err_y, err, 0xDC2626, 1);
        }
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match event {
            Event::Key(key) => {
                match key {
                    b'o' | b'O' => {
                        self.refresh_file_list();
                        true
                    }
                    b'c' | b'C' => {
                        self.image_data = None;
                        self.current_file = None;
                        true
                    }
                    b'=' | b'+' => {
                        if self.zoom < 8 { self.zoom += 1; }
                        true
                    }
                    b'-' | b'_' => {
                        if self.zoom > 1 { self.zoom -= 1; }
                        true
                    }
                    b'f' | b'F' => {
                        self.fit_to_window(area);
                        true
                    }
                    b'p' | b'P' => {
                        if let Some(idx) = self.selected_index {
                            if idx > 0 {
                                self.selected_index = Some(idx - 1);
                                if self.selected_index.unwrap() < self.list_scroll {
                                    self.list_scroll = self.selected_index.unwrap();
                                }
                                if let Some(file) = self.file_list.get(idx - 1) {
                                    let _ = self.load_image(file);
                                }
                            }
                        }
                        true
                    }
                    b'n' | b'N' => {
                        if let Some(idx) = self.selected_index {
                            if idx + 1 < self.file_list.len() {
                                self.selected_index = Some(idx + 1);
                                if self.selected_index.unwrap() >= self.list_scroll + 15 {
                                    self.list_scroll = self.selected_index.unwrap().saturating_sub(14);
                                }
                                if let Some(file) = self.file_list.get(idx + 1) {
                                    let _ = self.load_image(file);
                                }
                            }
                        }
                        true
                    }
                    _ => false,
                }
            }
            Event::Pointer(pointer) => {
                let mut handled = false;

                let toolbar_y = area.y + 50;
                let btns = [
                    (&mut self.open_button, 20, || { handled = true; }),
                    (&mut self.close_button, 100, || { 
                        self.image_data = None; 
                        self.current_file = None; 
                        handled = true; 
                    }),
                    (&mut self.zoom_in_button, 180, || { 
                        if self.zoom < 8 { self.zoom += 1; } 
                        handled = true; 
                    }),
                    (&mut self.zoom_out_button, 260, || { 
                        if self.zoom > 1 { self.zoom -= 1; } 
                        handled = true; 
                    }),
                    (&mut self.fit_button, 340, || { 
                        self.fit_to_window(area); 
                        handled = true; 
                    }),
                    (&mut self.prev_button, 420, || { 
                        if let Some(idx) = self.selected_index {
                            if idx > 0 {
                                self.selected_index = Some(idx - 1);
                                if let Some(file) = self.file_list.get(idx - 1) {
                                    let _ = self.load_image(file);
                                }
                            }
                        }
                        handled = true; 
                    }),
                    (&mut self.next_button, 500, || { 
                        if let Some(idx) = self.selected_index {
                            if idx + 1 < self.file_list.len() {
                                self.selected_index = Some(idx + 1);
                                if let Some(file) = self.file_list.get(idx + 1) {
                                    let _ = self.load_image(file);
                                }
                            }
                        }
                        handled = true; 
                    }),
                ];

                for (btn, x, action) in btns {
                    let rect = Rect { x: area.x + x, y: toolbar_y, width: 70, height: 30 };
                    let was_armed = btn.armed;
                    btn.armed = rect.contains(pointer.x, pointer.y);
                    if was_armed != btn.armed && !pointer.left_down && was_armed {
                        action();
                    }
                }

                let list_x = area.x + area.width.saturating_sub(220);
                let list_y = area.y + 100;
                let list_w = 200;
                let list_h = area.height.saturating_sub(120);
                let list_rect = Rect { x: list_x, y: list_y, width: list_w, height: list_h };

                if list_rect.contains(pointer.x, pointer.y) {
                    if pointer.left_down {
                        let rel_y = pointer.y.saturating_sub(list_y + 5);
                        let item_idx = (rel_y / 20) as usize + self.list_scroll;
                        if item_idx < self.file_list.len() {
                            self.selected_index = Some(item_idx);
                            if let Some(file) = self.file_list.get(item_idx) {
                                let _ = self.load_image(file);
                            }
                            handled = true;
                        }
                    }
                }

                if let Some(img) = &self.image_data {
                    let display_x = area.x + 20 + self.pan_x as u32;
                    let display_y = area.y + 100 + self.pan_y as u32;
                    let display_w = img.width * self.zoom;
                    let display_h = img.height * self.zoom;
                    let img_rect = Rect { x: display_x, y: display_y, width: display_w, height: display_h };

                    if img_rect.contains(pointer.x, pointer.y) {
                        if pointer.left_down {
                            if let Some((start_x, start_y)) = self.drag_start {
                                self.pan_x += pointer.x as i32 - start_x as i32;
                                self.pan_y += pointer.y as i32 - start_y as i32;
                                handled = true;
                            } else {
                                self.drag_start = Some((pointer.x, pointer.y));
                            }
                        } else {
                            self.drag_start = None;
                        }
                    }
                }

                handled
            }
        }
    }

    fn pointer_target(&self, x: u32, y: u32, area: Rect) -> Option<u32> {
        let toolbar_y = area.y + 50;
        for x_offset in [20, 100, 180, 260, 340, 420, 500] {
            let rect = Rect { x: area.x + x_offset, y: toolbar_y, width: 70, height: 30 };
            if rect.contains(x, y) { return Some(1); }
        }

        let list_x = area.x + area.width.saturating_sub(220);
        let list_y = area.y + 100;
        let list_w = 200;
        let list_h = area.height.saturating_sub(120);
        let list_rect = Rect { x: list_x, y: list_y, width: list_w, height: list_h };
        if list_rect.contains(x, y) { return Some(1); }

        if let Some(img) = &self.image_data {
            let display_x = area.x + 20 + self.pan_x as u32;
            let display_y = area.y + 100 + self.pan_y as u32;
            let display_w = img.width * self.zoom;
            let display_h = img.height * self.zoom;
            let img_rect = Rect { x: display_x, y: display_y, width: display_w, height: display_h };
            if img_rect.contains(x, y) { return Some(1); }
        }

        None
    }
}