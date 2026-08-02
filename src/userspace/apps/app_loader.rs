// Application Loader GUI for NebulaOS
// Browse, register, and launch external applications

use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use crate::services::loader::{self, AppManifest};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;

/// App Loader State
pub struct AppLoaderState {
    pub apps: Vec<AppManifest>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub info_text: String,
}

impl AppLoaderState {
    pub fn new() -> Self {
        Self {
            apps: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            info_text: String::new(),
        }
    }

    /// Refresh the app list from the loader service
    pub fn refresh_apps(&mut self) {
        let loader = loader::get_loader_service().lock();
        self.apps = loader.list_apps();
        if self.selected_index >= self.apps.len() {
            self.selected_index = 0;
        }
    }

    /// Launch the selected app
    pub fn launch_selected(&mut self) {
        if self.apps.is_empty() {
            self.info_text = "No apps registered.".to_string();
            return;
        }

        if self.selected_index >= self.apps.len() {
            return;
        }

        let app = &self.apps[self.selected_index];
        let name = app.name.clone();
        
        let mut loader = loader::get_loader_service().lock();
        match loader.launch_app(app.app_id) {
            Ok(pid) => {
                self.info_text = format!("Launched '{}' (PID: {})", name, pid);
            }
            Err(e) => {
                self.info_text = format!("Failed to launch '{}': {}", name, e);
            }
        }
    }
}

/// App Loader GUI App
pub struct AppLoaderApp;

impl AppLoaderApp {
    pub fn draw(fb: &mut Framebuffer, bounds: Rect, state: &AppLoaderState) {
        let x = bounds.x as usize;
        let y = bounds.y as usize + TITLE_BAR_HEIGHT as usize;
        let w = bounds.width as usize;
        let h = (bounds.height as usize).saturating_sub(TITLE_BAR_HEIGHT as usize);

        // Background
        fb.draw_rect(x, y, w, h, 0x00202020);

        // Title
        fb.draw_rect(x, y, w, 30, 0x00333333);
        draw_string(fb, x + 10, y + 8, "Application Loader", 0x00FFFFFF);

        // Info bar
        if !state.info_text.is_empty() {
            fb.draw_rect(x, y + 30, w, 20, 0x00111144);
            draw_string(fb, x + 10, y + 35, state.info_text.as_str(), 0x00AAAAFF);
        }

        let list_y = y + 55;
        let list_h = h - 60;

        // Draw app list
        let mut current_y = list_y;
        let visible_count = list_h / 25;
        let display_apps = state.apps.iter()
            .skip(state.scroll_offset)
            .take(visible_count)
            .enumerate();

        for (i, app) in display_apps {
            let actual_idx = state.scroll_offset + i;
            let bg_color = if actual_idx == state.selected_index {
                0x000078D7 // Highlighted
            } else if i % 2 == 0 {
                0x00282828
            } else {
                0x00303030
            };

            fb.draw_rect(x + 5, current_y, w - 10, 22, bg_color);

            let icon = if app.is_builtin { "[B] " } else { "[E] " };
            let label = format!("{}{} v{}", icon, app.name, app.version);
            draw_string(fb, x + 10, current_y + 6, &label, 0x00FFFFFF);

            current_y += 25;
        }

        // Draw scrollbar if needed
        if state.apps.len() > visible_count {
            let scrollbar_x = x + w - 12;
            let scrollbar_h = list_h;
            fb.draw_rect(scrollbar_x, list_y, 8, scrollbar_h, 0x00444444);

            let thumb_h = (scrollbar_h as f32 * (visible_count as f32 / state.apps.len() as f32)) as usize;
            let thumb_y = list_y + ((state.scroll_offset as f32 / (state.apps.len() - visible_count) as f32) * (scrollbar_h - thumb_h) as f32) as usize;
            fb.draw_rect(scrollbar_x, thumb_y, 8, thumb_h, 0x00AAAAAA);
        }

        // Bottom helper bar
        fb.draw_rect(x, y + h - 20, w, 20, 0x00333333);
        draw_string(fb, x + 10, y + h - 15, "Up/Down: Navigate | Enter: Launch | R: Refresh", 0x00AAAAAA);
    }

    pub fn handle_click(state: &mut AppLoaderState, bounds: Rect, mx: i32, my: i32) {
        let x = bounds.x as i32;
        let y = (bounds.y as i32) + TITLE_BAR_HEIGHT as i32;
        let w = bounds.width as i32;
        let h = (bounds.height as i32) - TITLE_BAR_HEIGHT as i32;

        // Check if click is in the app list area
        let list_y = y + 55;
        let list_h = h - 60;

        if mx >= x + 5 && mx <= x + w - 10 && my >= list_y && my <= list_y + list_h {
            let relative_y = (my - list_y) / 25;
            let clicked_index = state.scroll_offset + relative_y as usize;
            
            if clicked_index < state.apps.len() {
                state.selected_index = clicked_index;
            }
        }
    }

    pub fn handle_keyboard_input(state: &mut AppLoaderState, c: char) {
        const VISIBLE_COUNT: usize = 10;

        match c {
            'j' | '\x1b' if c == 'j' => { // Down
                if !state.apps.is_empty() && state.selected_index + 1 < state.apps.len() {
                    state.selected_index += 1;
                    if state.selected_index >= state.scroll_offset + VISIBLE_COUNT {
                        state.scroll_offset += 1;
                    }
                }
            }
            'k' | '\x1b' if c == 'k' => { // Up
                if state.selected_index > 0 {
                    state.selected_index -= 1;
                    if state.selected_index < state.scroll_offset {
                        state.scroll_offset = state.scroll_offset.saturating_sub(1);
                    }
                }
            }
            '\n' => { // Enter - launch
                state.launch_selected();
            }
            'r' | 'R' => { // Refresh
                state.refresh_apps();
                state.info_text = format!("Refreshed: {} apps found", state.apps.len());
            }
            _ => {}
        }
    }
}

