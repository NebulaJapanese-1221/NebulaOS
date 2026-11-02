use super::gui::{Application, Canvas, Event, PointerEvent, Rect};

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
        let ticks = crate::drivers::pit::ticks();
        let free = crate::memory::frame_allocator::free_frame_count();
        let total = crate::memory::frame_allocator::total_frame_count();
        canvas.draw_text(area.x + 24, area.y + 24, "SYSTEM HEALTH", 0x1596B5, 1);
        canvas.draw_text(area.x + 24, area.y + 56, "UPTIME TICKS", 0x111827, 1);
        draw_number(canvas, area.x + 180, area.y + 56, ticks);
        canvas.draw_text(area.x + 24, area.y + 84, "FREE MEMORY FRAMES", 0x111827, 1);
        draw_number(canvas, area.x + 180, area.y + 84, free as u32);
        canvas.draw_text(
            area.x + 24,
            area.y + 112,
            "TOTAL MEMORY FRAMES",
            0x111827,
            1,
        );
        draw_number(canvas, area.x + 180, area.y + 112, total as u32);
        canvas.draw_text(area.x + 24, area.y + 156, "CPU: IA-32", 0x111827, 1);
        canvas.draw_text(
            area.x + 24,
            area.y + 180,
            "PROCESSES: KERNEL GUI",
            0x111827,
            1,
        );
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
