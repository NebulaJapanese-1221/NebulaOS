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

const TEXT: u32 = 0x111827;
const ACCENT: u32 = 0x66D9EF;

pub struct Clock {
    uptime: u32,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            uptime: syscall::uptime_ticks().unwrap_or(0),
        }
    }

    fn refresh(&mut self) {
        if let Ok(ticks) = syscall::uptime_ticks() {
            self.uptime = ticks;
        }
    }

    fn format_uptime(buf: &mut [u8; 24], ticks: u32) -> usize {
        let seconds = ticks / 100;
        let minutes = seconds / 60;
        let hours = minutes / 60;
        let days = hours / 24;

        let mut len = 0;
        if days > 0 {
            len += append_u32(buf, len, days);
            buf[len] = b'd';
            len += 1;
            buf[len] = b' ';
            len += 1;
        }
        len += append_dec2(buf, len, (hours % 24) as u8);
        buf[len] = b':';
        len += 1;
        len += append_dec2(buf, len, (minutes % 60) as u8);
        buf[len] = b':';
        len += 1;
        len += append_dec2(buf, len, (seconds % 60) as u8);
        len
    }
}

impl Application for Clock {
    fn title(&self) -> &'static str {
        "CLOCK"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, _pointer: PointerEvent) {
        canvas.fill_rect(area, 0xF2F5FA);
        let mut buf = [0u8; 24];
        let len = Self::format_uptime(&mut buf, self.uptime);
        if let Ok(text) = core::str::from_utf8(&buf[..len]) {
            let x = area.x + (area.width.saturating_sub(len as u32 * 6) / 2);
            let y = area.y + (area.height / 2).saturating_sub(4);
            canvas.draw_text(x, y, text, TEXT, 2);
        }
        canvas.draw_text(
            area.x + 12,
            area.y + area.height.saturating_sub(20),
            "UPTIME SINCE BOOT",
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

fn append_u32(buf: &mut [u8], len: usize, value: u32) -> usize {
    if value == 0 {
        buf[len] = b'0';
        return len + 1;
    }
    let mut digits = [0u8; 10];
    let mut count = 0;
    let mut v = value;
    while v > 0 {
        digits[count] = b'0' + (v % 10) as u8;
        count += 1;
        v /= 10;
    }
    let mut i = len;
    for digit in digits[..count].iter().rev() {
        buf[i] = *digit;
        i += 1;
    }
    i
}

fn append_dec2(buf: &mut [u8], len: usize, value: u8) -> usize {
    buf[len] = b'0' + (value / 10) as u8;
    buf[len + 1] = b'0' + (value % 10) as u8;
    len + 2
}
