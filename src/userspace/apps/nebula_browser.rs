use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window, rect::Rect, button::Button};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use alloc::string::String;

#[derive(Clone)]
pub struct NebulaBrowser {
    pub url_buffer: String,
    pub is_editing: bool,
}

impl NebulaBrowser {
    pub fn new() -> Self {
        Self {
            url_buffer: String::from("nebula://welcome"),
            is_editing: false,
        }
    }
}

impl App for NebulaBrowser {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) {
        let font_height = if gui::LARGE_TEXT.load(core::sync::atomic::Ordering::Relaxed) { 32 } else { 16 };
        let title_height = font_height + 10;
        
        // Content area background
        gui::draw_rect(fb, win.x, win.y + title_height as isize, win.width, win.height - title_height, 0x00_1E_1E_1E, Some(dirty_rect));

        // Browser UI Mockup - Toolbar
        let toolbar_h = 36;
        let toolbar_y = win.y + title_height as isize;
        gui::draw_rect(fb, win.x, toolbar_y, win.width, toolbar_h, 0x00_2D_2D_30, Some(dirty_rect));
        
        // Navigation Buttons
        let back_btn = Button::new(win.x + 5, toolbar_y + 5, 26, 26, "<");
        back_btn.draw(fb, 0, 0, Some(dirty_rect));

        let refresh_btn = Button::new(win.x + 35, toolbar_y + 5, 26, 26, "R");
        refresh_btn.draw(fb, 0, 0, Some(dirty_rect));

        let search_btn = Button::new(win.x + 65, toolbar_y + 5, 26, 26, "S");
        search_btn.draw(fb, 0, 0, Some(dirty_rect));

        let home_btn = Button::new(win.x + 95, toolbar_y + 5, 26, 26, "H");
        home_btn.draw(fb, 0, 0, Some(dirty_rect));

        // URL Bar
        let url_bar_x = win.x + 125;
        let url_bar_w = win.width.saturating_sub(175);
        gui::draw_rect(fb, url_bar_x, toolbar_y + 6, url_bar_w, 24, 0x00_12_12_12, Some(dirty_rect));
        
        let text_color = if self.is_editing { 0x00_FFFFFF } else { 0x00_A0_A0_A0 };
        font::draw_string(fb, url_bar_x + 5, toolbar_y + 10, self.url_buffer.as_str(), text_color, Some(dirty_rect));

        // Go Button
        let go_btn = Button::new(win.x + win.width as isize - 45, toolbar_y + 5, 40, 26, "Go");
        go_btn.draw(fb, 0, 0, Some(dirty_rect));

        // Draw typing cursor if focused
        if self.is_editing {
            let cursor_x = url_bar_x + 5 + font::string_width(self.url_buffer.as_str()) as isize;
            if cursor_x < (url_bar_x + url_bar_w as isize - 5) {
                gui::draw_rect(fb, cursor_x, toolbar_y + 10, 2, 16, 0x00_00_7A_CC, Some(dirty_rect));
            }
        }

        // Page Content Placeholder
        let text = "NebulaBrowser";
        let text2 = "Networking stack not yet implemented.";
        let text3 = "NebulaBrowser is currently non-functional.";
        let text4 = "Nonfunctional, currently a placeholder.";
        let content_y = toolbar_y + toolbar_h as isize + 40;
        
        font::draw_string(fb, win.x + 20, content_y, text, 0x00_FFFFFF, Some(dirty_rect));
        font::draw_string(fb, win.x + 20, content_y + font_height as isize + 10, text2, 0x00_CCCCCC, Some(dirty_rect));
        font::draw_string(fb, win.x + 20, content_y + (font_height as isize + 10) * 2, text3, 0x00_A0_A0_A0, Some(dirty_rect));
        font::draw_string(fb, win.x + 20, content_y + (font_height as isize + 10) * 3, text4, 0x00_80_80_80, Some(dirty_rect));
    }

    fn handle_event(&mut self, event: &AppEvent, _win: &Window) -> Option<Rect> {
        match event {
            AppEvent::MouseClick { x, y, width, .. } => {
                // Home button hit test
                if *x >= 95 && *x < 121 && *y >= 5 && *y < 31 {
                    self.url_buffer = String::from("nebula://welcome");
                    self.is_editing = false;
                    return Some(_win.rect());
                }

                // URL bar hit test (relative to content area)
                let url_bar_w = width.saturating_sub(175);
                if *x >= 125 && *x < 125 + url_bar_w as isize && *y >= 6 && *y < 30 {
                    self.is_editing = true;
                } else {
                    self.is_editing = false;
                }
                return Some(_win.rect());
            }
            AppEvent::KeyPress { key } if self.is_editing => {
                match *key {
                    '\x08' => { self.url_buffer.pop(); } // Backspace
                    '\n' => { self.is_editing = false; } // Enter to submit/blur
                    c if !c.is_control() => { self.url_buffer.push(c); }
                    _ => {}
                }
                return Some(_win.rect());
            }
            _ => {}
        }
        None
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }
}