// Image Viewer Application for NebulaOS
// Simple image viewing application

use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct ImageViewerState {
    pub current_image: Option<Image>,
    pub zoom_level: f32,
    pub scroll_x: i32,
    pub scroll_y: i32,
}

#[derive(Debug, Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u32>, // RGBA pixels
}

impl ImageViewerState {
    pub fn new() -> Self {
        // Load a default image
        let default_image = Self::create_default_image();
        
        ImageViewerState {
            current_image: Some(default_image),
            zoom_level: 1.0,
            scroll_x: 0,
            scroll_y: 0,
        }
    }
    
    fn create_default_image() -> Image {
        // Create a simple placeholder image
        let width = 100;
        let height = 100;
        let mut data = Vec::with_capacity(width * height);
        
        for y in 0..height {
            for x in 0..width {
                // Create a simple gradient pattern
                let r = (x * 255 / width) as u8;
                let g = (y * 255 / height) as u8;
                let b = 128;
                data.push((r as u32) << 16 | (g as u32) << 8 | b as u32);
            }
        }
        
        Image {
            width,
            height,
            data,
        }
    }
    
    pub fn load_image(&mut self, path: &str) {
        // In a real implementation, we would load an image file
        // For now, we'll just create a placeholder
        self.current_image = Some(Self::create_default_image());
    }
    
    pub fn zoom_in(&mut self) {
        self.zoom_level *= 1.2;
    }
    
    pub fn zoom_out(&mut self) {
        self.zoom_level /= 1.2;
        if self.zoom_level < 0.1 {
            self.zoom_level = 0.1;
        }
    }
    
    pub fn scroll(&mut self, dx: i32, dy: i32) {
        self.scroll_x += dx;
        self.scroll_y += dy;
    }
}

pub struct ImageViewerApp;

impl ImageViewerApp {
    pub fn draw(fb: &mut Framebuffer, bounds: Rect, state: &ImageViewerState) {
        let x = bounds.x as usize;
        let y = bounds.y as usize + TITLE_BAR_HEIGHT as usize;
        let w = bounds.width as usize;
        let h = (bounds.height as usize).saturating_sub(TITLE_BAR_HEIGHT as usize);

        // Draw background
        fb.draw_rect(x, y, w, h, 0x00F0F0F0);

        // Draw image if available
        if let Some(image) = &state.current_image {
            // Calculate scaled dimensions
            let scaled_w = (image.width as f32 * state.zoom_level) as usize;
            let scaled_h = (image.height as f32 * state.zoom_level) as usize;
            
            // Calculate position with scrolling
            let start_x = x + 50 + state.scroll_x as usize;
            let start_y = y + 50 + state.scroll_y as usize;
            
            // Draw the image
            for py in 0..scaled_h.min(h - 100) {
                for px in 0..scaled_w.min(w - 100) {
                    // Calculate source pixel with bilinear interpolation (simplified)
                    let src_x = (px as f32 / state.zoom_level) as usize;
                    let src_y = (py as f32 / state.zoom_level) as usize;
                    
                    if src_x < image.width && src_y < image.height {
                        let pixel = image.data[src_y * image.width + src_x];
                        fb.draw_pixel(
                            start_x + px,
                            start_y + py,
                            pixel,
                        );
                    }
                }
            }
            
            // Draw image info
            draw_string(
                fb,
                x + 10,
                y + 10,
                &format!("Image: {}x{} (Zoom: {:.1}x)", image.width, image.height, state.zoom_level),
                0x00000000,
            );
        } else {
            draw_string(fb, x + 10, y + 10, "No image loaded", 0x00000000);
        }
    }

    pub fn handle_click(state: &mut ImageViewerState, bounds: Rect, mx: i32, my: i32) {
        // Check for zoom controls (would be drawn in a real implementation)
        // For now, just handle basic interaction
    }

    pub fn handle_keyboard_input(state: &mut ImageViewerState, c: char) {
        match c {
            '+' => state.zoom_in(),
            '-' => state.zoom_out(),
            'w' => state.scroll(0, -20),
            's' => state.scroll(0, 20),
            'a' => state.scroll(-20, 0),
            'd' => state.scroll(20, 0),
            _ => {}
        }
    }
}