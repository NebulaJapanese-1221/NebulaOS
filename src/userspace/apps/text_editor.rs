use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::String;

pub struct TextEditorState {
    pub content: String,
    pub cursor_pos: usize,
    pub blink_state: bool, // For cursor blinking
}

impl TextEditorState {
    pub fn new() -> Self {
        Self {
            content: String::from(""),
            cursor_pos: 0,
            blink_state: true,
        }
    }
}

pub struct TextEditorApp;

impl TextEditorApp {
    pub fn draw(fb: &mut Framebuffer, bounds: Rect, state: &TextEditorState, is_focused: bool) {
        let x = bounds.x as usize;
        let y = bounds.y as usize + TITLE_BAR_HEIGHT as usize;
        let w = bounds.width as usize;
        let h = (bounds.height as usize).saturating_sub(TITLE_BAR_HEIGHT as usize);

        // Editor background (white)
        fb.draw_rect(x + 2, y + 2, w - 4, h - 4, 0x00FFFFFF);

        // Render text
        let text_start_x = x + 10;
        let text_start_y = y + 10;
        draw_string(fb, text_start_x, text_start_y, state.content.as_str(), 0x00000000);

        // Draw blinking cursor if focused
        if is_focused && state.blink_state {
            let cursor_x = text_start_x + state.cursor_pos * 8; // Assuming 8px wide font
            let cursor_y = text_start_y;
            fb.draw_rect(cursor_x, cursor_y, 8, 8, 0x00000000); // Black cursor
        }
    }

    pub fn handle_click(_state: &mut TextEditorState, _bounds: Rect, _mx: i32, _my: i32) {
        // Text editor interaction logic will be implemented here
    }

    pub fn handle_keyboard_input(state: &mut TextEditorState, c: char) {
        match c {
            '\x08' => { // Backspace
                if state.cursor_pos > 0 {
                    state.content.remove(state.cursor_pos - 1);
                    state.cursor_pos -= 1;
                }
            }
            '\n' => { // Enter key
                state.content.insert(state.cursor_pos, '\n');
                state.cursor_pos += 1;
            }
            _ => {
                // Only allow printable ASCII characters for now
                if c.is_ascii_graphic() || c == ' ' {
                    state.content.insert(state.cursor_pos, c);
                    state.cursor_pos += 1;
                }
            }
        }
        // Reset blink state on input
        state.blink_state = true;
    }
}