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

use super::gui::{Application, Canvas, Event, PointerEvent, Rect};
use crate::syscall;

const TEXT: u32 = 0x111827;
const ACCENT: u32 = 0x66D9EF;
const MUTED: u32 = 0xA8B5C9;

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

pub struct Calendar {
    month: u8,
    year: u32,
}

impl Calendar {
    pub fn new() -> Self {
        let ticks = syscall::uptime_ticks().unwrap_or(0);
        let month = ((ticks / 3600) % 12) as u8 + 1;
        let year = 2026 + (ticks / 43200) as u32;
        Self { month, year }
    }

    fn refresh(&mut self) {
        let ticks = syscall::uptime_ticks().unwrap_or(0);
        self.month = ((ticks / 3600) % 12) as u8 + 1;
        self.year = 2026 + (ticks / 43200) as u32;
    }

    fn days_in_month(month: u8, year: u32) -> u8 {
        match month {
            2 => {
                let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
                if leap { 29 } else { 28 }
            }
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    fn starting_weekday(month: u8, year: u32) -> u8 {
        let mut days = 0u32;
        for y in 2026..year {
            days += if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        }
        for m in 1..month {
            days += Self::days_in_month(m, year) as u32;
        }
        (4 + days) % 7
    }

    fn draw_u32(canvas: &mut dyn Canvas, x: u32, y: u32, value: u32, color: u32) {
        let mut buf = [0u8; 10];
        let mut len = 0;
        if value == 0 {
            buf[0] = b'0';
            len = 1;
        } else {
            let mut digits = [0u8; 10];
            let mut count = 0;
            let mut v = value;
            while v > 0 {
                digits[count] = b'0' + (v % 10) as u8;
                count += 1;
                v /= 10;
            }
            for digit in digits[..count].iter().rev() {
                buf[len] = *digit;
                len += 1;
            }
        }
        if let Ok(text) = core::str::from_utf8(&buf[..len]) {
            canvas.draw_text(x, y, text, color, 1);
        }
    }

    fn draw_u8(canvas: &mut dyn Canvas, x: u32, y: u32, value: u8, color: u32) {
        let mut buf = [0u8; 3];
        let mut len = 0;
        if value < 10 {
            buf[0] = b'0' + value;
            len = 1;
        } else {
            buf[0] = b'0' + (value / 10);
            buf[1] = b'0' + (value % 10);
            len = 2;
        }
        if let Ok(text) = core::str::from_utf8(&buf[..len]) {
            canvas.draw_text(x, y, text, color, 1);
        }
    }
}

impl Application for Calendar {
    fn title(&self) -> &'static str {
        "CALENDAR"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, _pointer: PointerEvent) {
        canvas.fill_rect(area, 0xF2F5FA);

        // Header: month/year
        let month_name = MONTHS[(self.month - 1) as usize];
        let month_name_bytes = month_name.as_bytes();
        let year_len = if self.year < 10 { 1 } else { 2 };
        let header_len = month_name_bytes.len() + 1 + year_len as usize;
        let mut header = [0u8; 32];
        let mut hi = 0;
        for b in month_name_bytes {
            header[hi] = *b;
            hi += 1;
        }
        header[hi] = b' ';
        hi += 1;
        if self.year < 10 {
            header[hi] = b'0' + (self.year as u8);
            hi += 1;
        } else {
            header[hi] = b'0' + ((self.year / 10) as u8);
            header[hi + 1] = b'0' + ((self.year % 10) as u8);
            hi += 2;
        }
        let x = area.x + (area.width.saturating_sub(header_len as u32 * 7) / 2);
        if let Ok(text) = core::str::from_utf8(&header[..hi]) {
            canvas.draw_text(x, area.y + 24, text, TEXT, 1);
        }

        // Weekday labels
        let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let grid_x = area.x + 40;
        let grid_y = area.y + 56;
        let cell_w = 34u32;
        for (i, day) in days.iter().enumerate() {
            canvas.draw_text(grid_x + i as u32 * cell_w, grid_y, day, MUTED, 1);
        }

        // Day numbers
        let start = Self::starting_weekday(self.month, self.year);
        let total = Self::days_in_month(self.month, self.year);
        let mut day = 1u8;
        for row in 0..6 {
            for col in 0..7 {
                if (row == 0 && col < start as u32) || day > total {
                    continue;
                }
                let dx = grid_x + col as u32 * cell_w;
                let dy = grid_y + 20 + row as u32 * 20;
                Self::draw_u8(canvas, dx + 12, dy, day, TEXT);
                day += 1;
            }
        }

        canvas.draw_text(
            area.x + 12,
            area.y + area.height.saturating_sub(20),
            "Calendar derived from uptime ticks",
            ACCENT,
            1,
        );
    }

    fn handle_event(&mut self, event: Event, _area: Rect) -> bool {
        match event {
            Event::Key(_) => {
                self.refresh();
                true
            }
            Event::Pointer(_) => false,
        }
    }

    fn pointer_target(&self, _x: u32, _y: u32, _area: Rect) -> Option<u32> {
        None
    }
}
