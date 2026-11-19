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

// Revamped boot screen: modern, polished boot sequence display.

use crate::drivers::{framebuffer, pit};
use alloc::format;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// Palette: deep space / nebula theme
const BG_TOP: u32 = 0x050A14;
const BG_BOTTOM: u32 = 0x0F1B33;
const PANEL: u32 = 0x0B1626;
const ACCENT: u32 = 0x66D9EF;
const ACCENT_DIM: u32 = 0x2E6B8A;
const TEXT: u32 = 0xF1F5F9;
const MUTED: u32 = 0x7D94A8;
const OK: u32 = 0x2ECC71;
const WARN: u32 = 0xF1C40F;
const LINE_SPACING: u32 = 20;
const MARGIN_X: u32 = 48;
const MARGIN_Y: u32 = 60;

#[derive(Clone, Copy)]
pub enum BootStage {
    GraphicsReady,
    StorageReady,
    InputReady,
    DesktopReady,
}

impl BootStage {
    fn progress(self) -> u32 {
        match self {
            Self::GraphicsReady => 25,
            Self::StorageReady => 50,
            Self::InputReady => 75,
            Self::DesktopReady => 100,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::GraphicsReady => "GRAPHICS INITIALIZED",
            Self::StorageReady => "STORAGE PROBE COMPLETE",
            Self::InputReady => "INPUT DEVICES READY",
            Self::DesktopReady => "DESKTOP ENVIRONMENT",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::GraphicsReady => "Framebuffer and display pipeline active",
            Self::StorageReady => "IDE storage and filesystem checked",
            Self::InputReady => "Keyboard and mouse initialized",
            Self::DesktopReady => "Window manager and applications loaded",
        }
    }
}

pub struct BootScreen {
    started_at: u32,
    stage: BootStage,
    dots: u32,
}

impl BootScreen {
    pub fn new() -> Self {
        let screen = Self {
            started_at: pit::ticks(),
            stage: BootStage::GraphicsReady,
            dots: 0,
        };
        screen.render();
        screen
    }

    pub fn update(&mut self, stage: BootStage) {
        self.stage = stage;
        self.render();
    }

    pub fn wait_until_minimum_duration(&self) {
        let elapsed = pit::ticks().wrapping_sub(self.started_at);
        if elapsed < 600 {
            pit::sleep_ticks(600 - elapsed);
        }
    }

    fn render(&self) {
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        if width == 0 || height == 0 {
            return;
        }

        // Deep space gradient background
        for y in 0..height {
            let blend = y * 255 / height.max(1);
            framebuffer::fill_rect(0, y, width, 1, blend_rgb(BG_TOP, BG_BOTTOM, blend));
        }

        // Subtle vignette
        self.draw_vignette(width, height);

        // Header: NebulaOS title
        let title_x = MARGIN_X;
        let title_y = MARGIN_Y;
        framebuffer::draw_text(title_x, title_y, "NEBULAOS", TEXT, 3);
        framebuffer::draw_text(
            title_x,
            title_y + 44,
            "A modern, from-scratch operating system",
            MUTED,
            1,
        );
        framebuffer::draw_text(
            title_x,
            title_y + 64,
            &format!("Version {}", VERSION),
            ACCENT,
            1,
        );

        // Horizontal divider
        framebuffer::draw_rect(MARGIN_X, title_y + 92, 320, 2, PANEL);

        // Progress section
        let progress_y = title_y + 120;
        framebuffer::draw_text(MARGIN_X, progress_y, "BOOT PROGRESS", MUTED, 1);
        let bar_width = 400u32;
        let bar_x = MARGIN_X;
        let bar_y = progress_y + 24;
        let bar_height = 10u32;

        framebuffer::fill_rect(bar_x, bar_y, bar_width, bar_height, PANEL);
        let filled = bar_width * self.stage.progress() / 100;
        if filled > 0 {
            framebuffer::fill_rect(bar_x, bar_y, filled, bar_height, ACCENT);
        }
        framebuffer::draw_rect(bar_x, bar_y, bar_width, bar_height, ACCENT_DIM);

        let pct = self.stage.progress();
        framebuffer::draw_text(
            bar_x + bar_width + 12,
            bar_y.saturating_sub(2),
            &format!("{}%", pct),
            TEXT,
            1,
        );

        // Stage checklist
        let list_y = bar_y + bar_height + 40;
        let stages = [
            (BootStage::GraphicsReady, "Graphics subsystem", "Display pipeline online"),
            (BootStage::StorageReady, "Storage subsystem", "IDE disk and filesystem"),
            (BootStage::InputReady, "Input subsystem", "Keyboard and mouse"),
            (BootStage::DesktopReady, "Desktop environment", "Window manager and apps"),
        ];

        let mut y = list_y;
        for (stage, title, desc) in stages {
            let done = self.stage.progress() >= stage.progress();
            let dot_color = if done { OK } else { ACCENT_DIM };
            let text_color = if done { TEXT } else { MUTED };

            // Status dot
            let dot_x = MARGIN_X;
            let dot_y = y + 2;
            framebuffer::fill_rect(dot_x, dot_y, 8, 8, dot_color);
            framebuffer::draw_rect(dot_x, dot_y, 8, 8, if done { OK } else { PANEL });

            // Title
            framebuffer::draw_text(dot_x + 20, y, title, text_color, 1);
            // Description
            framebuffer::draw_text(dot_x + 20, y + 16, desc, if done { MUTED } else { PANEL }, 1);

            y += 52;
        }

        // Footer
        let footer_y = height - 40;
        framebuffer::draw_rect(MARGIN_X, footer_y - 10, 320, 1, PANEL);
        framebuffer::draw_text(
            MARGIN_X,
            footer_y,
            "Press F12 during boot for diagnostic mode",
            MUTED,
            1,
        );

        framebuffer::present();
    }

    fn draw_vignette(&self, _width: u32, _height: u32) {
        // Vignette handled by gradient background; kept for API compatibility
    }
}

fn blend_rgb(from: u32, to: u32, amount: u32) -> u32 {
    let mix = |shift: u32| {
        let first = (from >> shift) & 0xFF;
        let second = (to >> shift) & 0xFF;
        (first * (255 - amount) + second * amount) / 255
    };
    mix(16) << 16 | mix(8) << 8 | mix(0)
}
