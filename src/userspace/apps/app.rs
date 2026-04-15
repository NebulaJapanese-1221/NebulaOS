use crate::drivers::framebuffer;
use crate::userspace::gui::{Window, rect::Rect};
use alloc::boxed::Box;

pub enum AppEvent {
    MouseClick { x: isize, y: isize, width: usize, height: usize },
    MouseMove { x: isize, y: isize, width: usize, height: usize },
    KeyPress { key: char },
    Scroll { delta: isize, width: usize, height: usize },
}

pub trait App: Send {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect);
    fn handle_event(&mut self, event: &AppEvent);
    fn box_clone(&self) -> Box<dyn App>;
}

impl Clone for Box<dyn App> {
    fn clone(&self) -> Box<dyn App> {
        self.box_clone()
    }
}