use crate::drivers::framebuffer;
use crate::userspace::gui::Window;
use alloc::boxed::Box;

pub enum AppEvent {
    MouseClick { x: isize, y: isize },
    KeyPress { key: char },
}

pub trait App: Send {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window);
    fn handle_event(&mut self, event: &AppEvent);
    fn box_clone(&self) -> Box<dyn App>;
}

impl Clone for Box<dyn App> {
    fn clone(&self) -> Box<dyn App> {
        self.box_clone()
    }
}