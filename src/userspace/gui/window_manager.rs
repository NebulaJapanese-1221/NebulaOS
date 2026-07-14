use crate::framebuffer::{Framebuffer, Rect};
use super::{draw_string, TITLE_BAR_HEIGHT, TASKBAR_HEIGHT};
use alloc::vec::Vec;

pub const CURSOR_BITMAP: [u16; 19] = [
    0b110000000000, 0b111000000000, 0b111100000000, 0b111110000000,
    0b111111000000, 0b111111100000, 0b111111110000, 0b111111111000,
    0b111111111100, 0b111111111110, 0b111111111111, 0b111111110000,
    0b111011110000, 0b110001111000, 0b100000111000, 0b000000111000,
    0b000000011100, 0b000000011100, 0b000000000000,
];

#[derive(Copy, Clone, PartialEq)]
pub enum AppType {
    None,
    Calculator,
    Terminal,
    TextEditor,
    FileManager,
    WebBrowser,
    ImageViewer,
    SystemMonitor,
}

pub enum AppData {
    None,
    Calculator(crate::apps::calculator::CalculatorState),
    Terminal(crate::apps::terminal::TerminalState),
    TextEditor(crate::apps::text_editor::TextEditorState),
    FileManager(crate::apps::file_manager::FileManagerState),
    WebBrowser(crate::apps::web_browser::WebBrowserState),
    ImageViewer(crate::apps::image_viewer::ImageViewerState),
    SystemMonitor(crate::apps::system_monitor::SystemMonitorState),
}

impl AppData {
    pub fn draw(&self, fb: &mut Framebuffer, bounds: Rect, is_focused: bool) {
        match self {
            AppData::Calculator(state) => crate::apps::calculator::CalculatorApp::draw(fb, bounds, state),
            AppData::Terminal(state) => crate::apps::terminal::TerminalApp::draw(fb, bounds, state),
            AppData::TextEditor(state) => crate::apps::text_editor::TextEditorApp::draw(fb, bounds, state, is_focused),
            AppData::FileManager(state) => crate::apps::file_manager::FileManagerApp::draw(fb, bounds, state),
            AppData::WebBrowser(state) => crate::apps::web_browser::WebBrowserApp::draw(fb, bounds, state),
            AppData::ImageViewer(state) => crate::apps::image_viewer::ImageViewerApp::draw(fb, bounds, state),
            AppData::SystemMonitor(state) => crate::apps::system_monitor::SystemMonitorApp::draw(fb, bounds, state),
            AppData::None => {}
        }
    }

    pub fn handle_keyboard_input(&mut self, c: char) {
        match self {
            AppData::TextEditor(state) => crate::apps::text_editor::TextEditorApp::handle_keyboard_input(state, c),
            AppData::Calculator(state) => crate::apps::calculator::CalculatorApp::handle_keyboard_input(state, c),
            AppData::Terminal(state) => crate::apps::terminal::TerminalApp::handle_keypress(state, c),
            AppData::FileManager(state) => crate::apps::file_manager::FileManagerApp::handle_keyboard_input(state, c),
            AppData::WebBrowser(state) => crate::apps::web_browser::WebBrowserApp::handle_keyboard_input(state, c),
            AppData::ImageViewer(state) => crate::apps::image_viewer::ImageViewerApp::handle_keyboard_input(state, c),
            AppData::SystemMonitor(state) => crate::apps::system_monitor::SystemMonitorApp::handle_keyboard_input(state, c),
            AppData::None => {}
        }
    }

    pub fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        match self {
            AppData::Calculator(state) => crate::apps::calculator::CalculatorApp::handle_click(state, bounds, mx, my),
            AppData::Terminal(state) => crate::apps::terminal::TerminalApp::handle_click(state, bounds, mx, my),
            AppData::TextEditor(state) => crate::apps::text_editor::TextEditorApp::handle_click(state, bounds, mx, my),
            AppData::FileManager(state) => crate::apps::file_manager::FileManagerApp::handle_click(state, bounds, mx, my),
            AppData::WebBrowser(state) => crate::apps::web_browser::WebBrowserApp::handle_click(state, bounds, mx, my),
            AppData::ImageViewer(state) => crate::apps::image_viewer::ImageViewerApp::handle_click(state, bounds, mx, my),
            AppData::SystemMonitor(state) => crate::apps::system_monitor::SystemMonitorApp::handle_click(state, bounds, mx, my),
            AppData::None => {}
        }
    }
}

pub struct Window {
    pub title: &'static str,
    pub bounds: Rect,
    pub app_type: AppType,
    pub data: AppData,
    pub is_maximized: bool,
    pub old_bounds: Rect,
    pub is_minimized: bool,
}

impl Window {
    pub fn new(title: &'static str, x: u32, y: u32, width: u32, height: u32, app_type: AppType) -> Self {
        let data = match app_type {
            AppType::Calculator => AppData::Calculator(crate::apps::calculator::CalculatorState::new()),
            AppType::TextEditor => AppData::TextEditor(crate::apps::text_editor::TextEditorState::new()),
            AppType::Terminal => AppData::Terminal(crate::apps::terminal::TerminalState::new()),
            AppType::FileManager => AppData::FileManager(crate::apps::file_manager::FileManagerState::new()),
            AppType::WebBrowser => AppData::WebBrowser(crate::apps::web_browser::WebBrowserState::new()),
            AppType::ImageViewer => AppData::ImageViewer(crate::apps::image_viewer::ImageViewerState::new()),
            AppType::SystemMonitor => AppData::SystemMonitor(crate::apps::system_monitor::SystemMonitorState::new()),
            _ => AppData::None,
        };
        Self {
            title,
            bounds: Rect { x, y, width, height },
            app_type,
            data,
            is_maximized: false,
            old_bounds: Rect { x, y, width, height },
            is_minimized: false,
        }
    }
}

pub struct WindowManager {
    pub windows: Vec<Window>,
    dragging_idx: Option<usize>,
    drag_off_x: i32,
    drag_off_y: i32,
    last_mouse_btn: bool,
    last_right_btn: bool,
    context_menu: Option<(i32, i32)>,
    screen_w: u32,
    screen_h: u32,
    vfs: Option<crate::fs::vfs::VFS>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            dragging_idx: None,
            drag_off_x: 0,
            drag_off_y: 0,
            last_mouse_btn: false,
            last_right_btn: false,
            context_menu: None,
            screen_w: 1024,
            screen_h: 768,
            vfs: None,
        }
    }

    pub fn set_screen_size(&mut self, w: u32, h: u32) {
        self.screen_w = w;
        self.screen_h = h;
    }

    pub fn set_filesystem(&mut self, vfs: crate::fs::vfs::VFS) {
        self.vfs = Some(vfs);
    }

    pub fn handle_mouse(&mut self, mx: i32, my: i32, ml: bool, mr: bool) -> bool {
        let mut menu_toggle = false;

        // Handle Right Click (Open Context Menu)
        if mr && !self.last_right_btn {
            self.context_menu = Some((mx, my));
            menu_toggle = true;
        }
        self.last_right_btn = mr;

        if ml && !self.last_mouse_btn {
            // If context menu is open, handle clicks on it first
            if let Some((cx, cy)) = self.context_menu {
                let rel_x = mx - cx;
                let rel_y = my - cy;

                if rel_x >= 0 && rel_x < 150 && rel_y >= 0 && rel_y < 100 {
                    let item = rel_y / 20;
                    match item {
                        0 => { // New Calculator
                            self.windows.push(Window::new("Calculator", mx as u32, my as u32, 220, 300, AppType::Calculator));
                        }
                        1 => { // New Text Editor
                            self.windows.push(Window::new("Text Editor", mx as u32, my as u32, 400, 300, AppType::TextEditor));
                        }
                        2 => { // New Terminal
                            self.windows.push(Window::new("Terminal", mx as u32, my as u32, 400, 300, AppType::Terminal));
                        }
                        3 => { // New Web Browser
                            self.windows.push(Window::new("Web Browser", mx as u32, my as u32, 600, 400, AppType::WebBrowser));
                        }
                        4 => { // New Image Viewer
                            self.windows.push(Window::new("Image Viewer", mx as u32, my as u32, 500, 400, AppType::ImageViewer));
                        }
                        5 => { // New System Monitor
                            self.windows.push(Window::new("System Monitor", mx as u32, my as u32, 500, 400, AppType::SystemMonitor));
                        }
                        6 => { // New File Manager
                            let mut fm = Window::new("File Manager", mx as u32, my as u32, 500, 400, AppType::FileManager);
                            if let AppData::FileManager(ref mut state) = fm.data {
                                if let Some(vfs) = &self.vfs {
                                    // Create a NebulaFS handle for the file manager
                                    state.set_filesystem(fs::NebulaFS::new("nebula_pool", 4096, 1024 * 1024));
                                }
                                state.refresh_files();
                            }
                            self.windows.push(fm);
                        }
                        _ => { // Close All
                            self.windows.clear();
                        }
                    }
                    self.context_menu = None;
                    self.last_mouse_btn = ml;
                    return false;
                } else {
                    self.context_menu = None;
                }
            }

            menu_toggle = true;
            self.dragging_idx = None;
            let mut taskbar_handled = false;

            // Check taskbar items (minimized windows)
            let taskbar_y = (self.screen_h - TASKBAR_HEIGHT) as i32;
            if my >= taskbar_y {
                let mut item_x = 80;
                for win in self.windows.iter_mut() {
                    if win.is_minimized {
                        if mx >= item_x && mx <= item_x + 110 && my >= taskbar_y + 5 {
                            win.is_minimized = false;
                            menu_toggle = false;
                            taskbar_handled = true;
                            break;
                        }
                        item_x += 115;
                    }
                }
            }

            let mut clicked_idx = None;
            let mut is_dragging = false;
            let mut close_clicked = false;
            
            // Hit test windows (top to bottom)
            if !taskbar_handled {
                for (i, win) in self.windows.iter_mut().enumerate().rev() {
                    if win.is_minimized { continue; }
                    
                    let x = win.bounds.x as i32;
                    let y = win.bounds.y as i32;
                    let w = win.bounds.width as i32;

                    // Check title bar (for dragging)
                    if mx >= x && mx <= x + w &&
                       my >= y && my <= y + TITLE_BAR_HEIGHT as i32 {
                        clicked_idx = Some(i);

                        // Close button check (rightmost 25px)
                        if mx >= x + w - 25 {
                            close_clicked = true;
                            break;
                        }
                        // Maximize button check
                        if mx >= x + w - 50 {
                            if win.is_maximized {
                                win.bounds = win.old_bounds;
                                win.is_maximized = false;
                            } else {
                                win.old_bounds = win.bounds;
                                win.bounds = Rect { x: 0, y: 0, width: self.screen_w, height: self.screen_h - TASKBAR_HEIGHT };
                                win.is_maximized = true;
                            }
                            break;
                        }
                        // Minimize button check
                        if mx >= x + w - 75 {
                            win.is_minimized = true;
                            break;
                        }

                        is_dragging = true;
                        self.drag_off_x = mx - x;
                        self.drag_off_y = my - y;
                        break;
                    }

                    // Check window body (for app interaction)
                    if mx >= win.bounds.x as i32 && mx <= (win.bounds.x + win.bounds.width) as i32 &&
                       my > (win.bounds.y + TITLE_BAR_HEIGHT) as i32 && my <= (win.bounds.y + win.bounds.height) as i32 {
                        clicked_idx = Some(i);
                        
                        win.data.handle_click(win.bounds, mx, my);
                        break;
                    }
                }
            }

            if let Some(idx) = clicked_idx {
                if close_clicked {
                    self.windows.remove(idx);
                } else {
                    let win = self.windows.remove(idx);
                    self.windows.push(win);
                    if is_dragging {
                        self.dragging_idx = Some(self.windows.len() - 1);
                    }
                }
                menu_toggle = false;
            }
        }

        if ml {
            if let Some(idx) = self.dragging_idx {
                if let Some(win) = self.windows.get_mut(idx) {
                    win.bounds.x = (mx - self.drag_off_x).max(0) as u32;
                    win.bounds.y = (my - self.drag_off_y).max(0) as u32;
                }
            }
        } else {
            self.dragging_idx = None;
        }

        self.last_mouse_btn = ml;
        menu_toggle
    }

    pub fn handle_keyboard_input(&mut self, c: char) {
        if let Some(win) = self.windows.iter_mut().rev().find(|w| !w.is_minimized) {
            win.data.handle_keyboard_input(c);

            if let AppData::Terminal(state) = &win.data {
                if state.should_close {
                    if let Some(idx) = self.windows.iter().rev().position(|w| !w.is_minimized) {
                        let idx = self.windows.len() - 1 - idx;
                        self.windows.remove(idx);
                    }
                }
            }
        }
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        for (i, window) in self.windows.iter().enumerate() {
            if window.is_minimized { continue; }

            fb.draw_rect(window.bounds.x as usize, window.bounds.y as usize, window.bounds.width as usize, window.bounds.height as usize, 0x00C0C0C0);
            
            fb.draw_rect(window.bounds.x as usize, window.bounds.y as usize, window.bounds.width as usize, TITLE_BAR_HEIGHT as usize, 0x000078D7);
            
            draw_string(fb, window.bounds.x as usize + 5, window.bounds.y as usize + 8, window.title, 0xFFFFFF);

            let is_focused = i == self.windows.len() - 1;
            window.data.draw(fb, window.bounds, is_focused);
        }

        if let Some((cx, cy)) = self.context_menu {
            fb.draw_rect(cx as usize, cy as usize, 150, 100, 0x00E0E0E0);
            fb.draw_rect(cx as usize, cy as usize, 150, 1, 0x00000000);
            fb.draw_rect(cx as usize, cy as usize + 99, 150, 1, 0x00000000);
            
            draw_string(fb, cx as usize + 10, cy as usize + 5,  "New Calculator", 0x000000);
            draw_string(fb, cx as usize + 10, cy as usize + 25, "New Text Editor", 0x000000);
            draw_string(fb, cx as usize + 10, cy as usize + 45, "New Terminal", 0x000000);
            draw_string(fb, cx as usize + 10, cy as usize + 65, "New Web Browser", 0x000000);
            draw_string(fb, cx as usize + 10, cy as usize + 85, "New Image Viewer", 0x000000);
            draw_string(fb, cx as usize + 10, cy as usize + 105, "New System Monitor", 0x000000);
            draw_string(fb, cx as usize + 10, cy as usize + 125, "New File Manager", 0x000000);
            draw_string(fb, cx as usize + 10, cy as usize + 145, "Close All", 0x000000);
        }
    }
}



