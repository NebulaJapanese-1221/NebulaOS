use crate::drivers::framebuffer;
use crate::userspace::gui::{Window, rect::Rect};
use alloc::boxed::Box;

pub enum AppEvent {
    MouseClick { x: isize, y: isize, width: usize, height: usize },
    MouseDoubleClick { x: isize, y: isize, width: usize, height: usize },
    MouseRightClick { x: isize, y: isize, width: usize, height: usize },
    MouseMove { x: isize, y: isize, width: usize, height: usize },
    KeyPress { key: char },
    Scroll { delta: isize, width: usize, height: usize },
    Tick { tick_count: usize },
}

pub trait App: Send {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect);
    fn handle_event(&mut self, event: &AppEvent, win: &Window) -> Option<Rect>;
    fn box_clone(&self) -> Box<dyn App>;

    /// Returns the current dynamic title of the application, if any.
    fn get_title(&self) -> Option<alloc::string::String> { None }
}

impl Clone for Box<dyn App> {
    fn clone(&self) -> Box<dyn App> {
        self.box_clone()
    }
}