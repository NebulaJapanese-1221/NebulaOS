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

// Diagnostic mode: full-screen hardware and system information display,
// entered by pressing F12 during boot.

use crate::drivers::{framebuffer, keyboard, pci, pit, rtc};
use crate::memory::frame_allocator;
use alloc::format;
use alloc::string::ToString;

const BACKGROUND: u32 = 0x0B1220;
const PANEL: u32 = 0x111827;
const ACCENT: u32 = 0x66D9EF;
const TEXT: u32 = 0xF1F5F9;
const MUTED: u32 = 0xA8B5C9;
const WARN: u32 = 0xF1C40F;
const ERROR: u32 = 0xE74C3C;
const OK: u32 = 0x2ECC71;
const LINE_HEIGHT: u32 = 14;
const MARGIN_X: u32 = 24;
const MARGIN_Y: u32 = 32;
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Diagnostic {
    frame: u32,
}

impl Diagnostic {
    pub fn new() -> Self {
        Self { frame: 0 }
    }

    pub fn run(mut self) -> ! {
        self.render_header();
        self.render_sections();
        framebuffer::present();

        loop {
            self.frame = self.frame.wrapping_add(1);
            pit::sleep_ticks(5);
            if keyboard::f12_pressed() {
                break;
            }
        }
        crate::arch_halt();
    }

    fn render_header(&self) {
        let width = framebuffer::get_width();
        framebuffer::clear_screen(BACKGROUND);
        framebuffer::fill_rect(0, 0, width, 48, PANEL);
        framebuffer::draw_text(MARGIN_X, 14, "NEBULAOS DIAGNOSTIC MODE", TEXT, 2);
        framebuffer::draw_text(
            MARGIN_X,
            36,
            &format!("Version {}  |  Press F12 to exit", VERSION),
            MUTED,
            1,
        );
        framebuffer::present();
    }

    fn render_sections(&self) {
        let mut y = MARGIN_Y + 16;
        self.section("SYSTEM", y);
        y += LINE_HEIGHT;
        self.kv("Kernel version", VERSION, y);
        y += LINE_HEIGHT;
        let ticks = pit::ticks();
        self.kv("Uptime (ticks)", &ticks.to_string(), y);
        y += LINE_HEIGHT;
        let cpu_count = crate::arch::smp::cpu_count();
        self.kv("CPU count", &cpu_count.to_string(), y);
        y += LINE_HEIGHT;

        self.section("MEMORY", y);
        y += LINE_HEIGHT;
        let total = frame_allocator::total_frame_count();
        let free = frame_allocator::free_frame_count();
        let allocated = total - free;
        self.kv("Total frames", &total.to_string(), y);
        y += LINE_HEIGHT;
        self.kv("Free frames", &free.to_string(), y);
        y += LINE_HEIGHT;
        self.kv("Allocated frames", &allocated.to_string(), y);
        y += LINE_HEIGHT;
        self.kv("Frame size (bytes)", "4096", y);
        y += LINE_HEIGHT + 4;

        self.section("REAL-TIME CLOCK", y);
        y += LINE_HEIGHT;
        if let Some(t) = rtc::read_time() {
            self.kv("Available", "yes", y);
            y += LINE_HEIGHT;
            self.kv(
                "Date/time",
                &format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", t.year, t.month, t.day, t.hour, t.minute, t.second),
                y,
            );
        } else {
            self.kv_colored("Available", "no", WARN, y);
        }
        y += LINE_HEIGHT + 4;

        self.section("PCI DEVICES", y);
        y += LINE_HEIGHT;
        let count = pci::device_count();
        self.kv("Device count", &count.to_string(), y);
        y += LINE_HEIGHT;
        for index in 0..count.min(12) {
            if let Some(dev) = pci::device_at(index) {
                self.kv_pci(index, &dev, y);
                y += LINE_HEIGHT;
            }
        }
        if count > 12 {
            self.kv_colored("...", &format!("and {} more", count - 12), MUTED, y);
            y += LINE_HEIGHT;
        }
        y += 4;

        self.section("STORAGE", y);
        y += LINE_HEIGHT;
        match crate::drivers::disk::is_available() {
            true => self.kv_colored("Available", "yes", OK, y),
            false => self.kv_colored("Available", "no", WARN, y),
        }
        y += LINE_HEIGHT;
        match crate::drivers::disk::is_formatted() {
            true => self.kv_colored("Formatted", "yes", OK, y),
            false => self.kv_colored("Formatted", "no", WARN, y),
        }
        y += LINE_HEIGHT + 4;

        self.section("INPUT", y);
        y += LINE_HEIGHT;
        self.kv("Keyboard", "PS/2", y);
        y += LINE_HEIGHT;
        self.kv("Mouse", if pci::find_by_class(0x03).is_some() { "detected" } else { "none" }, y);

        framebuffer::present();
    }

    fn section(&self, label: &str, y: u32) {
        framebuffer::draw_text(MARGIN_X, y, label, ACCENT, 1);
        framebuffer::draw_rect(MARGIN_X, y + LINE_HEIGHT, 200, 2, 0x35506E);
    }

    fn kv(&self, key: &str, value: &str, y: u32) {
        framebuffer::draw_text(MARGIN_X + 8, y, key, MUTED, 1);
        framebuffer::draw_text(MARGIN_X + 160, y, value, TEXT, 1);
    }

    fn kv_colored(&self, key: &str, value: &str, color: u32, y: u32) {
        framebuffer::draw_text(MARGIN_X + 8, y, key, MUTED, 1);
        framebuffer::draw_text(MARGIN_X + 160, y, value, color, 1);
    }

    fn kv_pci(&self, _index: usize, dev: &pci::PciDevice, y: u32) {
        let label = format!("{:02}:{:02}.{}", dev.bus, dev.device, dev.function);
        framebuffer::draw_text(MARGIN_X + 8, y, &label, MUTED, 1);
        framebuffer::draw_text(
            MARGIN_X + 80,
            y,
            &format!("ven=0x{:04X} dev=0x{:04X}", dev.vendor_id, dev.device_id),
            TEXT,
            1,
        );
        framebuffer::draw_text(
            MARGIN_X + 240,
            y,
            pci::class_name(dev.class, dev.subclass),
            TEXT,
            1,
        );
    }
}
