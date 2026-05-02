use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window, rect::Rect, button::Button};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use core::sync::atomic::Ordering;

#[derive(Clone, Debug)]
pub struct FileManager {
    pub selected_index: Option<usize>,
}

impl FileManager {
    pub fn new() -> Self {
        Self { selected_index: None }
    }
}

impl App for FileManager {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, clip: Rect) {
        let font_height = if gui::LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 10;
        
        // Sidebar
        gui::draw_rect(fb, win.x, win.y + title_height as isize, 120, win.height - title_height, 0x00_1E_1E_1E, Some(clip));
        font::draw_string(fb, win.x + 10, win.y + title_height as isize + 10, "DRIVES", 0x00_AAAAAA, Some(clip));

        // Main Content Area
        gui::draw_rect(fb, win.x + 120, win.y + title_height as isize, win.width - 120, win.height - title_height, 0x00_000000, Some(clip));

        // List USB Drives detected by the kernel stack
        let drives = crate::kernel::usb::DETECTED_DRIVES.lock();
        let mut y_offset = 40;
        for drive in drives.iter() {
            let drive_rect = Rect { x: win.x + 10, y: win.y + title_height as isize + y_offset, width: 100, height: 24 };
            gui::draw_rect(fb, drive_rect.x, drive_rect.y, drive_rect.width, drive_rect.height, 0x00_333333, Some(clip));
            font::draw_char(fb, drive_rect.x + 5, drive_rect.y + 4, 'U', 0x00_FFFFFF, Some(clip));
            font::draw_string(fb, drive_rect.x + 20, drive_rect.y + 4, "USB Disk", 0x00_FFFFFF, Some(clip));
            
            // Details
            let detail = alloc::format!("Device: {} ({} MB)", drive.name, drive.size_mb);
            font::draw_string(fb, win.x + 130, win.y + title_height as isize + 10, &detail, 0x00_00FF00, Some(clip));
            font::draw_string(fb, win.x + 130, win.y + title_height as isize + 30, "Contents:", 0x00_FFFFFF, Some(clip));
            font::draw_string(fb, win.x + 150, win.y + title_height as isize + 55, "> root/", 0x00_AAAAFF, Some(clip));
            
            y_offset += 30;
        }
        
        if drives.is_empty() {
            font::draw_string(fb, win.x + 130, win.y + title_height as isize + 10, "Connect a USB drive to begin.", 0x00_888888, Some(clip));
        }
    }

    fn handle_event(&mut self, _event: &AppEvent, _win: &Window) -> Option<Rect> { None }

    fn box_clone(&self) -> Box<dyn App> { Box::new(self.clone()) }
}