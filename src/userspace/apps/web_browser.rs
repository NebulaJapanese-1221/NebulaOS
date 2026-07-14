// Web Browser Application for NebulaOS
// Simple HTML renderer

use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT, Widget, Button, TextBox, WidgetContainer};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct WebBrowserState {
    pub current_url: String,
    pub html_content: String,
    pub scroll_offset: usize,
    pub address_bar: TextBox,
    pub back_button: Button,
    pub forward_button: Button,
    pub refresh_button: Button,
    pub widget_container: WidgetContainer,
}

impl WebBrowserState {
    pub fn new() -> Self {
        let mut address_bar = TextBox::new();
        address_bar.text = String::from("https://example.com");
        
        let back_button = Button::new("< Back");
        let forward_button = Button::new("Forward >");
        let refresh_button = Button::new("Refresh");
        
        let mut widget_container = WidgetContainer::new();
        widget_container.add_widget(Box::new(address_bar.clone()));
        widget_container.add_widget(Box::new(back_button.clone()));
        widget_container.add_widget(Box::new(forward_button.clone()));
        widget_container.add_widget(Box::new(refresh_button.clone()));
        
        WebBrowserState {
            current_url: String::from("https://example.com"),
            html_content: String::from("<html><body><h1>Example Domain</h1><p>This domain is for use in illustrative examples in documents.</p></body></html>"),
            scroll_offset: 0,
            address_bar,
            back_button,
            forward_button,
            refresh_button,
            widget_container,
        }
    }
    
    pub fn navigate(&mut self, url: &str) {
        self.current_url = url.to_string();
        // In a real implementation, we would fetch the URL
        // For now, we'll use placeholder content
        self.html_content = format!("<html><body><h1>{}</h1><p>This is a placeholder page for {}.</p></body></html>", url, url);
        self.scroll_offset = 0;
    }
    
    pub fn go_back(&mut self) {
        // In a real implementation, we would use a history stack
        self.navigate("https://example.com");
    }
    
    pub fn go_forward(&mut self) {
        // In a real implementation, we would use a history stack
        self.navigate("https://example.com");
    }
    
    pub fn refresh(&mut self) {
        self.navigate(&self.current_url);
    }
    
    pub fn handle_keypress(&mut self, c: char) {
        self.widget_container.handle_key(c);
        
        // If Enter is pressed in the address bar, navigate to the URL
        if c == '\n' && self.widget_container.focused_widget == Some(0) {
            self.navigate(&self.address_bar.text);
        }
    }
}

pub struct WebBrowserApp;

impl WebBrowserApp {
    pub fn draw(fb: &mut Framebuffer, bounds: Rect, state: &mut WebBrowserState) {
        let x = bounds.x as usize;
        let y = bounds.y as usize + TITLE_BAR_HEIGHT as usize;
        let w = bounds.width as usize;
        let h = (bounds.height as usize).saturating_sub(TITLE_BAR_HEIGHT as usize);

        // Draw background
        fb.draw_rect(x, y, w, h, 0x00FFFFFF);

        // Draw toolbar
        fb.draw_rect(x, y, w, 40, 0x00EEEEEE);

        // Draw navigation buttons and address bar
        let mut widget_y = y + 5;
        let mut widget_x = x + 5;

        // Draw back button
        let (btn_w, btn_h) = state.back_button.preferred_size();
        let back_btn_bounds = Rect {
            x: widget_x as u32,
            y: widget_y as u32,
            width: btn_w,
            height: btn_h,
        };
        state.back_button.draw(fb, back_btn_bounds, false);
        widget_x += btn_w as usize + 5;

        // Draw forward button
        let (btn_w, btn_h) = state.forward_button.preferred_size();
        let forward_btn_bounds = Rect {
            x: widget_x as u32,
            y: widget_y as u32,
            width: btn_w,
            height: btn_h,
        };
        state.forward_button.draw(fb, forward_btn_bounds, false);
        widget_x += btn_w as usize + 5;

        // Draw refresh button
        let (btn_w, btn_h) = state.refresh_button.preferred_size();
        let refresh_btn_bounds = Rect {
            x: widget_x as u32,
            y: widget_y as u32,
            width: btn_w,
            height: btn_h,
        };
        state.refresh_button.draw(fb, refresh_btn_bounds, false);
        widget_x += btn_w as usize + 10;

        // Draw address bar
        let (txt_w, txt_h) = state.address_bar.preferred_size();
        let address_bar_bounds = Rect {
            x: widget_x as u32,
            y: widget_y as u32,
            width: w as u32 - (widget_x as u32 - x as u32) - 10,
            height: txt_h,
        };
        state.address_bar.draw(fb, address_bar_bounds, true);

        // Draw web content area
        let content_y = y + 45;
        let content_h = h - 45;
        fb.draw_rect(x, content_y, w, content_h, 0x00FFFFFF);

        // Render HTML content (simplified)
        let mut current_y = content_y + 10;
        
        // Parse and render HTML (very simplified)
        let lines = self.parse_html(&state.html_content);
        for line in lines {
            draw_string(fb, x + 10, current_y, &line, 0x00000000);
            current_y += 15;
        }
    }

    fn parse_html(&self, html: &str) -> Vec<String> {
        let mut lines = Vec::new();
        let mut in_tag = false;
        let mut current_line = String::new();
        
        for c in html.chars() {
            if c == '<' {
                in_tag = true;
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                    current_line.clear();
                }
            } else if c == '>' {
                in_tag = false;
            } else if !in_tag {
                current_line.push(c);
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }

    pub fn handle_click(state: &mut WebBrowserState, bounds: Rect, mx: i32, my: i32) {
        let x = bounds.x as i32;
        let y = bounds.y as i32 + TITLE_BAR_HEIGHT as i32;
        let w = bounds.width as i32;
        let h = (bounds.height as i32) - TITLE_BAR_HEIGHT as i32;

        // Check if click is in the toolbar area
        if my >= y && my <= y + 40 {
            // Check navigation buttons
            let mut widget_x = x + 5;
            
            // Back button
            let (btn_w, btn_h) = state.back_button.preferred_size();
            if mx >= widget_x && mx <= widget_x + btn_w as i32 &&
               my >= y + 5 && my <= y + 5 + btn_h as i32 {
                state.go_back();
                return;
            }
            widget_x += btn_w as i32 + 5;

            // Forward button
            let (btn_w, btn_h) = state.forward_button.preferred_size();
            if mx >= widget_x && mx <= widget_x + btn_w as i32 &&
               my >= y + 5 && my <= y + 5 + btn_h as i32 {
                state.go_forward();
                return;
            }
            widget_x += btn_w as i32 + 5;

            // Refresh button
            let (btn_w, btn_h) = state.refresh_button.preferred_size();
            if mx >= widget_x && mx <= widget_x + btn_w as i32 &&
               my >= y + 5 && my <= y + 5 + btn_h as i32 {
                state.refresh();
                return;
            }
        }
        
        // Handle address bar click
        state.widget_container.handle_click(bounds, mx, my);
    }

    pub fn handle_keyboard_input(state: &mut WebBrowserState, c: char) {
        state.handle_keypress(c);
    }
}