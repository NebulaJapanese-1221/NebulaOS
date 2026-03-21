use crate::drivers::framebuffer;
use crate::userspace::gui::{font, Window};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use alloc::format;

#[derive(Clone, Copy, Debug)]
pub struct Settings {}

impl Settings {
    pub fn new() -> Self {
        Self {}
    }
}

impl App for Settings {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window) {
        let start_y = win.y + 25;
        font::draw_string(fb, win.x + 10, start_y, "NebulaOS Settings", 0x00_FFFFFF, None);
        let ver_str = format!("Version: {}", crate::kernel::VERSION);
        font::draw_string(fb, win.x + 10, start_y + 20, &ver_str, 0x00_CCCCCC, None);
        font::draw_string(fb, win.x + 10, start_y + 40, "Target: i686", 0x00_CCCCCC, None);
    }

    fn handle_event(&mut self, _event: &AppEvent) {}

    fn box_clone(&self) -> Box<dyn App> {
        Box::new((*self).clone())
    }
}