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
use crate::{drivers::rtc, syscall};

const TEXT: u32 = 0x111827;
const ACCENT: u32 = 0x66D9EF;

pub struct Clock {
    time: rtc::RtcTime,
    available: bool,
}

impl Clock {
    pub fn new() -> Self {
        let time = rtc::read_time().unwrap_or(rtc::RtcTime {
            year: 2026,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        });
        Self {
            available: rtc::is_available(),
            time,
        }
    }

    fn refresh(&mut self) {
        if let Some(t) = rtc::read_time() {
            self.time = t;
        }
    }

    fn format_time(buf: &mut [u8; 16], t: &rtc::RtcTime) -> usize {
        let mut len = 0;
        append_u32(buf, &mut len, t.year);
        buf[len] = b'-';
        len += 1;
        append_dec2(buf, &mut len, t.month);
        buf[len] = b'-';
        len += 1;
        append_dec2(buf, &mut len, t.day);
        buf[len] = b' ';
        len += 1;
        append_dec2(buf, &mut len, t.hour);
        buf[len] = b':';
        len += 1;
        append_dec2(buf, &mut len, t.minute);
        buf[len] = b':';
        len += 1;
        append_dec2(buf, &mut len, t.second);
        len
    }
}

impl Application for Clock {
    fn title(&self) -> &'static str {
        "CLOCK"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, _pointer: PointerEvent) {
        canvas.fill_rect(area, 0xF2F5FA);
        let mut buf = [0u8; 16];
        let len = Self::format_time(&mut buf, &self.time);
        if let Ok(text) = core::str::from_utf8(&buf[..len]) {
            let x = area.x + (area.width.saturating_sub(len as u32 * 6) / 2);
            let y = area.y + (area.height / 2).saturating_sub(4);
            canvas.draw_text(x, y, text, TEXT, 2);
        }
        let status = if self.available {
            "RTC ready"
        } else {
            "RTC unavailable"
        };
        canvas.draw_text(area.x + 12, area.y + area.height.saturating_sub(20), status, ACCENT, 1);
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

fn append_u32(buf: &mut [u8], len: &mut usize, value: u32) {
    if value == 0 {
        buf[*len] = b'0';
        *len += 1;
        return;
    }
    let mut digits = [0u8; 10];
    let mut count = 0;
    let mut v = value;
    while v > 0 {
        digits[count] = b'0' + (v % 10) as u8;
        count += 1;
        v /= 10;
    }
    for digit in digits[..count].iter().rev() {
        buf[*len] = *digit;
        *len += 1;
    }
}

fn append_dec2(buf: &mut [u8], len: &mut usize, value: u8) {
    buf[*len] = b'0' + (value / 10) as u8;
    *len += 1;
    buf[*len] = b'0' + (value % 10) as u8;
    *len += 1;
}
