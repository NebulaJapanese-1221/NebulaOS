// Widget System for NebulaOS GUI
// Provides reusable UI components

use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::String;

/// Widget trait
pub trait Widget {
    /// Draw the widget
    fn draw(&self, fb: &mut Framebuffer, bounds: Rect, is_focused: bool);
    
    /// Handle mouse click
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) -> bool;
    
    /// Handle keyboard input
    fn handle_key(&mut self, key: char) -> bool;
    
    /// Get the preferred size of the widget
    fn preferred_size(&self) -> (u32, u32);
}

/// Button widget
pub struct Button {
    pub text: String,
    pub on_click: Option<fn() -> ()>,
    pub enabled: bool,
    pub pressed: bool,
}

impl Button {
    pub fn new(text: &str) -> Self {
        Button {
            text: text.to_string(),
            on_click: None,
            enabled: true,
            pressed: false,
        }
    }
    
    pub fn with_handler(text: &str, handler: fn() -> ()) -> Self {
        let mut button = Self::new(text);
        button.on_click = Some(handler);
        button
    }
}

impl Widget for Button {
    fn draw(&self, fb: &mut Framebuffer, bounds: Rect, is_focused: bool) {
        let x = bounds.x as usize;
        let y = bounds.y as usize;
        let w = bounds.width as usize;
        let h = bounds.height as usize;
        
        // Draw button background
        let bg_color = if self.pressed {
            0x00888888 // Pressed color
        } else if is_focused {
            0x00AAAAAA // Focused color
        } else {
            0x00CCCCCC // Normal color
        };
        
        fb.draw_rect(x, y, w, h, bg_color);
        
        // Draw button border
        fb.draw_rect(x, y, w, 1, 0x00000000); // Top border
        fb.draw_rect(x, y + h - 1, w, 1, 0x00000000); // Bottom border
        fb.draw_rect(x, y, 1, h, 0x00000000); // Left border
        fb.draw_rect(x + w - 1, y, 1, h, 0x00000000); // Right border
        
        // Draw button text
        let text_color = if self.enabled { 0x00000000 } else { 0x00888888 };
        let text_x = x + (w - self.text.len() * 8) / 2;
        let text_y = y + (h - 8) / 2;
        draw_string(fb, text_x, text_y, &self.text, text_color);
    }
    
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) -> bool {
        let x = bounds.x as i32;
        let y = bounds.y as i32;
        let w = bounds.width as i32;
        let h = bounds.height as i32;
        
        if mx >= x && mx < x + w && my >= y && my < y + h && self.enabled {
            self.pressed = true;
            if let Some(handler) = self.on_click {
                handler();
            }
            true
        } else {
            self.pressed = false;
            false
        }
    }
    
    fn handle_key(&mut self, key: char) -> bool {
        if key == '\n' || key == ' ' {
            if let Some(handler) = self.on_click {
                handler();
            }
            true
        } else {
            false
        }
    }
    
    fn preferred_size(&self) -> (u32, u32) {
        let width = (self.text.len() * 8 + 20) as u32;
        let height = 24;
        (width, height)
    }
}

/// Text box widget
pub struct TextBox {
    pub text: String,
    pub cursor_pos: usize,
    pub focused: bool,
    pub max_length: usize,
}

impl TextBox {
    pub fn new() -> Self {
        TextBox {
            text: String::new(),
            cursor_pos: 0,
            focused: false,
            max_length: 256,
        }
    }
    
    pub fn with_text(text: &str) -> Self {
        let mut textbox = Self::new();
        textbox.text = text.to_string();
        textbox.cursor_pos = text.len();
        textbox
    }
}

impl Widget for TextBox {
    fn draw(&self, fb: &mut Framebuffer, bounds: Rect, is_focused: bool) {
        let x = bounds.x as usize;
        let y = bounds.y as usize;
        let w = bounds.width as usize;
        let h = bounds.height as usize;
        
        // Draw background
        fb.draw_rect(x, y, w, h, 0x00FFFFFF);
        
        // Draw border
        let border_color = if is_focused { 0x000000FF } else { 0x00000000 };
        fb.draw_rect(x, y, w, 1, border_color); // Top border
        fb.draw_rect(x, y + h - 1, w, 1, border_color); // Bottom border
        fb.draw_rect(x, y, 1, h, border_color); // Left border
        fb.draw_rect(x + w - 1, y, 1, h, border_color); // Right border
        
        // Draw text
        draw_string(fb, x + 5, y + 5, &self.text, 0x00000000);
        
        // Draw cursor if focused
        if is_focused && self.focused {
            let cursor_x = x + 5 + self.cursor_pos * 8;
            fb.draw_rect(cursor_x, y + 5, 1, 12, 0x00000000);
        }
    }
    
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) -> bool {
        let x = bounds.x as i32;
        let y = bounds.y as i32;
        let w = bounds.width as i32;
        let h = bounds.height as i32;
        
        if mx >= x && mx < x + w && my >= y && my < y + h {
            // Calculate cursor position based on click
            let click_pos = ((mx - x) as usize - 5) / 8;
            self.cursor_pos = click_pos.min(self.text.len());
            self.focused = true;
            true
        } else {
            self.focused = false;
            false
        }
    }
    
    fn handle_key(&mut self, key: char) -> bool {
        if !self.focused {
            return false;
        }
        
        match key {
            '\n' => false, // Enter key
            '\x08' => { // Backspace
                if self.cursor_pos > 0 {
                    self.text.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
                true
            }
            _ => { // Regular character
                if self.text.len() < self.max_length {
                    self.text.insert(self.cursor_pos, key);
                    self.cursor_pos += 1;
                }
                true
            }
        }
    }
    
    fn preferred_size(&self) -> (u32, u32) {
        (200, 24)
    }
}

/// Dropdown widget
pub struct Dropdown {
    pub options: Vec<String>,
    pub selected: usize,
    pub expanded: bool,
    pub on_select: Option<fn(usize) -> ()>,
}

impl Dropdown {
    pub fn new(options: Vec<&str>) -> Self {
        Dropdown {
            options: options.iter().map(|s| s.to_string()).collect(),
            selected: 0,
            expanded: false,
            on_select: None,
        }
    }
    
    pub fn with_handler(options: Vec<&str>, handler: fn(usize) -> ()) -> Self {
        let mut dropdown = Self::new(options);
        dropdown.on_select = Some(handler);
        dropdown
    }
}

impl Widget for Dropdown {
    fn draw(&self, fb: &mut Framebuffer, bounds: Rect, is_focused: bool) {
        let x = bounds.x as usize;
        let y = bounds.y as usize;
        let w = bounds.width as usize;
        let h = bounds.height as usize;
        
        // Draw main button
        fb.draw_rect(x, y, w, h, 0x00CCCCCC);
        fb.draw_rect(x, y, w, 1, 0x00000000);
        fb.draw_rect(x, y + h - 1, w, 1, 0x00000000);
        fb.draw_rect(x, y, 1, h, 0x00000000);
        fb.draw_rect(x + w - 1, y, 1, h, 0x00000000);
        
        // Draw selected option
        if self.selected < self.options.len() {
            draw_string(fb, x + 5, y + 5, &self.options[self.selected], 0x00000000);
        }
        
        // Draw dropdown arrow
        let arrow_x = x + w - 15;
        let arrow_y = y + h / 2 - 2;
        fb.draw_rect(arrow_x, arrow_y, 3, 1, 0x00000000);
        fb.draw_rect(arrow_x + 1, arrow_y + 1, 3, 1, 0x00000000);
        fb.draw_rect(arrow_x + 2, arrow_y + 2, 3, 1, 0x00000000);
        
        // Draw options if expanded
        if self.expanded {
            let option_height = 20;
            for (i, option) in self.options.iter().enumerate() {
                let option_y = y + h + i * option_height;
                let is_selected = i == self.selected;
                
                let bg_color = if is_selected {
                    0x008888FF
                } else if is_focused && i == self.selected {
                    0x00AAAAFF
                } else {
                    0x00DDDDDD
                };
                
                fb.draw_rect(x, option_y, w, option_height, bg_color);
                fb.draw_rect(x, option_y, w, 1, 0x00000000);
                draw_string(fb, x + 5, option_y + 3, option, 0x00000000);
            }
        }
    }
    
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) -> bool {
        let x = bounds.x as i32;
        let y = bounds.y as i32;
        let w = bounds.width as i32;
        let h = bounds.height as i32;
        
        // Check if click is on the main button
        if mx >= x && mx < x + w && my >= y && my < y + h {
            self.expanded = !self.expanded;
            return true;
        }
        
        // Check if click is on an option
        if self.expanded {
            let option_height = 20;
            for (i, _) in self.options.iter().enumerate() {
                let option_y = y + h + i as i32 * option_height;
                if mx >= x && mx < x + w && my >= option_y && my < option_y + option_height {
                    self.selected = i;
                    self.expanded = false;
                    if let Some(handler) = self.on_select {
                        handler(i);
                    }
                    return true;
                }
            }
            
            // Click outside options - collapse
            self.expanded = false;
        }
        
        false
    }
    
    fn handle_key(&mut self, _key: char) -> bool {
        false
    }
    
    fn preferred_size(&self) -> (u32, u32) {
        let width = 150;
        let height = 24;
        (width, height)
    }
}

/// Widget container
pub struct WidgetContainer {
    pub widgets: Vec<Box<dyn Widget>>,
    pub focused_widget: Option<usize>,
}

impl WidgetContainer {
    pub fn new() -> Self {
        WidgetContainer {
            widgets: Vec::new(),
            focused_widget: None,
        }
    }
    
    pub fn add_widget(&mut self, widget: Box<dyn Widget>) {
        self.widgets.push(widget);
    }
    
    pub fn draw(&self, fb: &mut Framebuffer, bounds: Rect) {
        let mut y = bounds.y as usize;
        
        for (i, widget) in self.widgets.iter().enumerate() {
            let (w, h) = widget.preferred_size();
            let widget_bounds = Rect {
                x: bounds.x,
                y: y as u32,
                width: w,
                height: h,
            };
            
            let is_focused = self.focused_widget == Some(i);
            widget.draw(fb, widget_bounds, is_focused);
            
            y += h as usize + 5;
        }
    }
    
    pub fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        let mut y = bounds.y as i32;
        
        for (i, widget) in self.widgets.iter_mut().enumerate() {
            let (w, h) = widget.preferred_size();
            let widget_bounds = Rect {
                x: bounds.x,
                y: y as u32,
                width: w,
                height: h,
            };
            
            if widget.handle_click(widget_bounds, mx, my) {
                self.focused_widget = Some(i);
                return;
            }
            
            y += h as i32 + 5;
        }
        
        self.focused_widget = None;
    }
    
    pub fn handle_key(&mut self, key: char) {
        if let Some(index) = self.focused_widget {
            if let Some(widget) = self.widgets.get_mut(index) {
                widget.handle_key(key);
            }
        }
    }
}