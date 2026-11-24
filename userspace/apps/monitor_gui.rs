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
use crate::libc::{
    system_stats, uptime_ticks, pci_count, pci_device,
    SystemStats, PciDeviceInfo,
};

pub struct SystemMonitor;

impl SystemMonitor {
    pub const fn new() -> Self {
        Self
    }
}

impl Application for SystemMonitor {
    fn title(&self) -> &'static str {
        "SYSTEM MONITOR"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, _pointer: PointerEvent) {
        let mut stats = SystemStats::default();
        let available = system_stats(&mut stats).is_ok();
        let uptime = uptime_ticks();
        canvas.draw_text(area.x + 24, area.y + 24, "SYSTEM HEALTH", 0x1596B5, 1);
        canvas.draw_text(area.x + 24, area.y + 56, "UPTIME TICKS", 0x111827, 1);
        if let Ok(ticks) = uptime {
            draw_number(canvas, area.x + 180, area.y + 56, ticks);
        } else {
            canvas.draw_text(area.x + 180, area.y + 56, "UNAVAILABLE", 0xA23B45, 1);
        }
        canvas.draw_text(area.x + 24, area.y + 84, "FREE MEMORY FRAMES", 0x111827, 1);
        if available {
            draw_number(canvas, area.x + 180, area.y + 84, stats.free_frames);
        }
        canvas.draw_text(
            area.x + 24,
            area.y + 112,
            "TOTAL MEMORY FRAMES",
            0x111827,
            1,
        );
        if available {
            draw_number(canvas, area.x + 180, area.y + 112, stats.total_frames);
        }
        canvas.draw_text(area.x + 24, area.y + 156, "CPU: IA-32", 0x111827, 1);
        canvas.draw_text(
            area.x + 24,
            area.y + 180,
            "PROCESSES: USER MODE",
            0x111827,
            1,
        );

        // PCI device info
        let pci_count = pci_count().unwrap_or(0);
        canvas.draw_text(area.x + 24, area.y + 216, "PCI DEVICES", 0x1596B5, 1);
        draw_number(canvas, area.x + 180, area.y + 216, pci_count);
        for i in 0..pci_count.min(4) {
            let mut info = PciDeviceInfo::default();
            if pci_device(i, &mut info).is_ok() {
                let y = area.y + 236 + i as u32 * 20;
                canvas.draw_text(area.x + 24, y, "PCI", 0x111827, 1);
                draw_hex(canvas, area.x + 56, y, info.bus as u32);
                draw_hex(canvas, area.x + 80, y, info.device as u32);
                draw_hex(canvas, area.x + 104, y, info.function as u32);
                draw_hex(canvas, area.x + 132, y, info.vendor_id as u32);
                draw_hex(canvas, area.x + 172, y, info.device_id as u32);
            }
        }

        if !available {
            canvas.draw_text(
                area.x + 24,
                area.y + 220,
                "SYSTEM STATS UNAVAILABLE",
                0xA23B45,
                1,
            );
        }
    }

    fn handle_event(&mut self, _event: Event, _area: Rect) -> bool {
        false
    }
}

fn draw_number(canvas: &mut dyn Canvas, x: u32, y: u32, mut value: u32) {
    let mut buffer = [0u8; 10];
    let mut index = buffer.len();
    loop {
        index -= 1;
        buffer[index] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    if let Ok(text) = core::str::from_utf8(&buffer[index..]) {
        canvas.draw_text(x, y, text, 0x111827, 1);
    }
}

fn draw_hex(canvas: &mut dyn Canvas, x: u32, y: u32, value: u32) {
    let digits = [
        (value >> 12) & 0xF,
        (value >> 8) & 0xF,
        (value >> 4) & 0xF,
        value & 0xF,
    ];
    let mut buf = [0u8; 5];
    buf[0] = b'0';
    buf[1] = b'x';
    for (i, &d) in digits.iter().enumerate() {
        buf[i + 2] = if d < 10 { b'0' + d as u8 } else { b'A' + d as u8 - 10 };
    }
    if let Ok(text) = core::str::from_utf8(&buf) {
        canvas.draw_text(x, y, text, 0x111827, 1);
    }
}
