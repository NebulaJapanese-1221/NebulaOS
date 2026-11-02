use crate::drivers::{framebuffer, pit};

const BACKGROUND_TOP: u32 = 0x0B1220;
const BACKGROUND_BOTTOM: u32 = 0x182C4D;
const ACCENT: u32 = 0xD8F7FF;
const SHADOW: u32 = 0x06101F;
const TEXT: u32 = 0xF1F5F9;
const MUTED: u32 = 0xA8B5C9;
const MIN_DISPLAY_TICKS: u32 = 500;
const VERSION: &str = env!("CARGO_PKG_VERSION");

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
            Self::GraphicsReady => 40,
            Self::StorageReady => 60,
            Self::InputReady => 80,
            Self::DesktopReady => 100,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::GraphicsReady => "KERNEL AND GRAPHICS READY",
            Self::StorageReady => "STORAGE PROBE COMPLETE",
            Self::InputReady => "INPUT SETUP COMPLETE",
            Self::DesktopReady => "DESKTOP READY",
        }
    }
}

pub struct BootScreen {
    started_at: u32,
    stage: BootStage,
}

impl BootScreen {
    pub fn new() -> Self {
        let screen = Self {
            started_at: pit::ticks(),
            stage: BootStage::GraphicsReady,
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
        if elapsed < MIN_DISPLAY_TICKS {
            pit::sleep_ticks(MIN_DISPLAY_TICKS - elapsed);
        }
    }

    fn render(&self) {
        let width = framebuffer::get_width();
        let height = framebuffer::get_height();
        if width == 0 || height == 0 {
            return;
        }

        framebuffer::clear_screen(BACKGROUND_TOP);
        for y in 0..height {
            framebuffer::fill_rect(
                0,
                y,
                width,
                1,
                blend_rgb(BACKGROUND_TOP, BACKGROUND_BOTTOM, y * 255 / height),
            );
        }

        let center_x = width / 2;
        let center_y = height / 2;
        framebuffer::draw_text(
            center_x.saturating_sub(72),
            center_y.saturating_sub(106),
            "NebulaOS",
            TEXT,
            3,
        );
        draw_cursive_alpha(
            center_x.saturating_sub(132),
            center_y.saturating_sub(10),
            4,
            ACCENT,
        );

        let label = self.stage.label();
        let label_x = center_x.saturating_sub((label.len() as u32 * 6) / 2);
        framebuffer::draw_text(label_x, center_y + 102, label, MUTED, 1);

        let bar_width = width.saturating_sub(80).min(360);
        let bar_x = center_x.saturating_sub(bar_width / 2);
        let bar_y = center_y + 124;
        framebuffer::fill_rect(bar_x, bar_y, bar_width, 12, 0x35506E);
        framebuffer::fill_rect(
            bar_x,
            bar_y,
            bar_width * self.stage.progress() / 100,
            12,
            ACCENT,
        );
        framebuffer::draw_rect(bar_x, bar_y, bar_width, 12, 0x8297B3);
        framebuffer::draw_text(
            center_x.saturating_sub(24),
            center_y + 148,
            VERSION,
            MUTED,
            1,
        );
        framebuffer::present();
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

fn draw_cursive_alpha(x: u32, y: u32, scale: u32, color: u32) {
    let path: [(i32, i32); 40] = [
        (0, 19),
        (5, 9),
        (10, 2),
        (14, 4),
        (18, 19),
        (15, 13),
        (11, 12),
        (8, 14),
        (9, 17),
        (13, 18),
        (18, 16),
        (20, 7),
        (23, 2),
        (25, 4),
        (22, 17),
        (25, 18),
        (29, 16),
        (31, 10),
        (35, 8),
        (39, 10),
        (38, 14),
        (34, 15),
        (31, 13),
        (30, 18),
        (28, 25),
        (30, 18),
        (35, 17),
        (39, 14),
        (42, 8),
        (46, 7),
        (48, 10),
        (47, 17),
        (50, 18),
        (53, 14),
        (56, 9),
        (59, 8),
        (61, 11),
        (59, 16),
        (62, 18),
        (66, 15),
    ];
    for pair in path.windows(2) {
        draw_script_line(
            x as i32 + pair[0].0 * scale as i32 + 2,
            y as i32 + pair[0].1 * scale as i32 + 2,
            x as i32 + pair[1].0 * scale as i32 + 2,
            y as i32 + pair[1].1 * scale as i32 + 2,
            scale.max(2) + 2,
            SHADOW,
        );
        draw_script_line(
            x as i32 + pair[0].0 * scale as i32,
            y as i32 + pair[0].1 * scale as i32,
            x as i32 + pair[1].0 * scale as i32,
            y as i32 + pair[1].1 * scale as i32,
            scale.max(2) + 1,
            color,
        );
    }
}

fn draw_script_line(mut x0: i32, mut y0: i32, x1: i32, y1: i32, thickness: u32, color: u32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        framebuffer::fill_rect(
            x0.max(0) as u32,
            y0.max(0) as u32,
            thickness,
            thickness,
            color,
        );
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x0 += sx;
        }
        if twice <= dx {
            error += dx;
            y0 += sy;
        }
    }
}
