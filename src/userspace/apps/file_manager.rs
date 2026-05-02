use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window, rect::Rect};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use core::sync::atomic::Ordering;

#[derive(Clone, Debug)]
pub struct FileManager {
    pub selected_index: Option<usize>,
    pub context_menu_open: bool,
    pub context_menu_pos: (isize, isize),
}

impl FileManager {
    pub fn new() -> Self {
        Self { selected_index: None, context_menu_open: false, context_menu_pos: (0, 0) }
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
            gui::draw_icon(fb, &gui::icons::USB_DRIVE, drive_rect.x + 5, drive_rect.y + 6, 0x00_FFFFFF, clip);
            font::draw_string(fb, drive_rect.x + 20, drive_rect.y + 4, "USB Disk", 0x00_FFFFFF, Some(clip));
            
            // Details
            let detail = alloc::format!("Device: {} ({} MB)", drive.name, drive.size_mb);
            font::draw_string(fb, win.x + 130, win.y + title_height as isize + 10, &detail, 0x00_00FF00, Some(clip));
            font::draw_string(fb, win.x + 130, win.y + title_height as isize + 30, "Contents:", 0x00_FFFFFF, Some(clip));

            // Highlight selected directory
            if self.selected_index == Some(0) {
                gui::draw_rect(fb, win.x + 130, win.y + title_height as isize + 53, win.width - 140, 20, 0x00_3E_3E_42, Some(clip));
            }

            // Render Directory entry
            gui::draw_icon(fb, &gui::icons::FOLDER, win.x + 135, win.y + title_height as isize + 55, 0x00_AAAAFF, clip);
            font::draw_string(fb, win.x + 152, win.y + title_height as isize + 53, "root/", 0x00_AAAAFF, Some(clip));

            // Highlight selected file
            if self.selected_index == Some(1) {
                gui::draw_rect(fb, win.x + 130, win.y + title_height as isize + 78, win.width - 140, 20, 0x00_3E_3E_42, Some(clip));
            }

            // Render File entry (placeholder)
            gui::draw_icon(fb, &gui::icons::FILE, win.x + 135, win.y + title_height as isize + 80, 0x00_FFFFFF, clip);
            font::draw_string(fb, win.x + 152, win.y + title_height as isize + 78, "README.txt", 0x00_FFFFFF, Some(clip));
            
            y_offset += 30;
        }
        
        if drives.is_empty() {
            font::draw_string(fb, win.x + 130, win.y + title_height as isize + 10, "Connect a USB drive to begin.", 0x00_888888, Some(clip));
        }

        // Draw local context menu
        if self.context_menu_open {
            let items = alloc::vec![
                gui::ContextMenuItem { label: "Open".into(), enabled: true },
                gui::ContextMenuItem { label: "Properties".into(), enabled: true },
            ];
            gui::draw_context_menu_box(
                fb, win.x + self.context_menu_pos.0, 
                win.y + title_height as isize + self.context_menu_pos.1, 
                &items, -1, -1, clip // Tooltip hover not needed for app context
            );
        }
    }

    fn handle_event(&mut self, event: &AppEvent, win: &Window) -> Option<Rect> {
        match event {
            AppEvent::MouseClick { x, y, .. } => {
                if self.context_menu_open {
                    self.context_menu_open = false;
                    // Check menu item hits (relative to win content)
                    let (mx, my) = self.context_menu_pos;
                    if *x >= mx && *x < mx + 100 && *y >= my && *y < my + 50 {
                        if *y < my + 25 {
                            crate::serial_println!("[FileManager] Context Action: Open");
                        } else {
                            crate::serial_println!("[FileManager] Context Action: Properties");
                        }
                    }
                    return Some(win.rect());
                }
                // Hit test for directory "root/" (y range approx 53-73)
                if *y >= 53 && *y < 73 && *x >= 130 {
                    self.selected_index = Some(0);
                    return Some(win.rect());
                }
                // Hit test for file "README.txt" (y range approx 78-98)
                if *y >= 78 && *y < 98 && *x >= 130 {
                    self.selected_index = Some(1);
                    return Some(win.rect());
                }
                self.selected_index = None;
                return Some(win.rect());
            }
            AppEvent::MouseRightClick { x, y, .. } => {
                self.context_menu_open = true;
                self.context_menu_pos = (*x, *y);
                return Some(win.rect());
            }
            AppEvent::MouseDoubleClick { x, y, .. } => {
                if *y >= 53 && *y < 73 && *x >= 130 {
                    // Placeholder: Actual navigation logic would update current path
                    crate::serial_println!("[FileManager] Opening folder: root/");
                    return Some(win.rect());
                }
            }
            _ => {}
        }
        None
    }

    fn box_clone(&self) -> Box<dyn App> { Box::new(self.clone()) }
}