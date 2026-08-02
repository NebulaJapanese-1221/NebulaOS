use crate::framebuffer::{Framebuffer, Rect};
use super::{draw_string, TITLE_BAR_HEIGHT, TASKBAR_HEIGHT};
use crate::services::loader;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::format;

pub const CURSOR_BITMAP: [u16; 19] = [
    0b110000000000, 0b111000000000, 0b111100000000, 0b111110000000,
    0b111111000000, 0b111111100000, 0b111111110000, 0b111111111000,
    0b111111111100, 0b111111111110, 0b111111111111, 0b111111110000,
    0b111011110000, 0b110001111000, 0b100000111000, 0b000000111000,
    0b000000011100, 0b000000011100, 0b000000000000,
];

/// Dynamic app types - apps are now loaded from the AppLoaderService
#[derive(Debug, Clone, PartialEq)]
pub enum AppType {
    None,
    BuiltIn {
        app_id: u32,
        name: String,
    },
    AppLoader,
}

/// AppData now uses dynamic dispatch through the loader service
pub enum AppData {
    None,
    BuiltIn {
        app_id: u32,
        name: String,
        state: Box<dyn AppStateHandler>,
    },
    AppLoader(crate::apps::app_loader::AppLoaderState),
}

/// Trait that built-in apps must implement for dynamic dispatch
pub trait AppStateHandler {
    fn draw(&mut self, fb: &mut Framebuffer, bounds: Rect, is_focused: bool);
    fn handle_keyboard_input(&mut self, c: char);
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32);
    fn should_close(&self) -> bool { false }
    fn get_title(&self) -> &str { "" }
}

// Implement AppStateHandler for each built-in app

impl AppStateHandler for crate::apps::calculator::CalculatorState {
    fn draw(&mut self, fb: &mut Framebuffer, bounds: Rect, _is_focused: bool) {
        crate::apps::calculator::CalculatorApp::draw(fb, bounds, self);
    }
    fn handle_keyboard_input(&mut self, c: char) {
        crate::apps::calculator::CalculatorApp::handle_keyboard_input(self, c);
    }
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        crate::apps::calculator::CalculatorApp::handle_click(self, bounds, mx, my);
    }
}

impl AppStateHandler for crate::apps::terminal::TerminalState {
    fn draw(&mut self, fb: &mut Framebuffer, bounds: Rect, _is_focused: bool) {
        crate::apps::terminal::TerminalApp::draw(fb, bounds, self);
    }
    fn handle_keyboard_input(&mut self, c: char) {
        crate::apps::terminal::TerminalApp::handle_keypress(self, c);
    }
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        crate::apps::terminal::TerminalApp::handle_click(self, bounds, mx, my);
    }
    fn should_close(&self) -> bool { self.should_close }
}

impl AppStateHandler for crate::apps::text_editor::TextEditorState {
    fn draw(&mut self, fb: &mut Framebuffer, bounds: Rect, is_focused: bool) {
        crate::apps::text_editor::TextEditorApp::draw(fb, bounds, self, is_focused);
    }
    fn handle_keyboard_input(&mut self, c: char) {
        crate::apps::text_editor::TextEditorApp::handle_keyboard_input(self, c);
    }
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        crate::apps::text_editor::TextEditorApp::handle_click(self, bounds, mx, my);
    }
}

impl AppStateHandler for crate::apps::file_manager::FileManagerState {
    fn draw(&mut self, fb: &mut Framebuffer, bounds: Rect, _is_focused: bool) {
        crate::apps::file_manager::FileManagerApp::draw(fb, bounds, self);
    }
    fn handle_keyboard_input(&mut self, c: char) {
        crate::apps::file_manager::FileManagerApp::handle_keyboard_input(self, c);
    }
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        crate::apps::file_manager::FileManagerApp::handle_click(self, bounds, mx, my);
    }
}

impl AppStateHandler for crate::apps::web_browser::WebBrowserState {
    fn draw(&mut self, fb: &mut Framebuffer, bounds: Rect, _is_focused: bool) {
        crate::apps::web_browser::WebBrowserApp::draw(fb, bounds, self);
    }
    fn handle_keyboard_input(&mut self, c: char) {
        crate::apps::web_browser::WebBrowserApp::handle_keyboard_input(self, c);
    }
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        crate::apps::web_browser::WebBrowserApp::handle_click(self, bounds, mx, my);
    }
}

impl AppStateHandler for crate::apps::image_viewer::ImageViewerState {
    fn draw(&mut self, fb: &mut Framebuffer, bounds: Rect, _is_focused: bool) {
        crate::apps::image_viewer::ImageViewerApp::draw(fb, bounds, self);
    }
    fn handle_keyboard_input(&mut self, c: char) {
        crate::apps::image_viewer::ImageViewerApp::handle_keyboard_input(self, c);
    }
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        crate::apps::image_viewer::ImageViewerApp::handle_click(self, bounds, mx, my);
    }
}

impl AppStateHandler for crate::apps::system_monitor::SystemMonitorState {
    fn draw(&mut self, fb: &mut Framebuffer, bounds: Rect, _is_focused: bool) {
        crate::apps::system_monitor::SystemMonitorApp::draw(fb, bounds, self);
    }
    fn handle_keyboard_input(&mut self, c: char) {
        crate::apps::system_monitor::SystemMonitorApp::handle_keyboard_input(self, c);
    }
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        crate::apps::system_monitor::SystemMonitorApp::handle_click(self, bounds, mx, my);
    }
}

impl AppStateHandler for crate::apps::system_settings::SystemSettingsState {
    fn draw(&mut self, fb: &mut Framebuffer, bounds: Rect, _is_focused: bool) {
        crate::apps::system_settings::SystemSettingsApp::draw(fb, bounds, self);
    }
    fn handle_keyboard_input(&mut self, c: char) {
        crate::apps::system_settings::SystemSettingsApp::handle_keyboard_input(self, c);
    }
    fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        crate::apps::system_settings::SystemSettingsApp::handle_click(self, bounds, mx, my);
    }
}

impl AppData {
    pub fn draw(&mut self, fb: &mut Framebuffer, bounds: Rect, is_focused: bool) {
        match self {
            AppData::BuiltIn { state, .. } => state.draw(fb, bounds, is_focused),
            AppData::AppLoader(state) => crate::apps::app_loader::AppLoaderApp::draw(fb, bounds, state),
            AppData::None => {}
        }
    }

    pub fn handle_keyboard_input(&mut self, c: char) {
        match self {
            AppData::BuiltIn { state, .. } => state.handle_keyboard_input(c),
            AppData::AppLoader(state) => crate::apps::app_loader::AppLoaderApp::handle_keyboard_input(state, c),
            AppData::None => {}
        }
    }

    pub fn handle_click(&mut self, bounds: Rect, mx: i32, my: i32) {
        match self {
            AppData::BuiltIn { state, .. } => state.handle_click(bounds, mx, my),
            AppData::AppLoader(state) => crate::apps::app_loader::AppLoaderApp::handle_click(state, bounds, mx, my),
            AppData::None => {}
        }
    }

    pub fn should_close(&self) -> bool {
        match self {
            AppData::BuiltIn { state, .. } => state.should_close(),
            _ => false,
        }
    }
}

pub struct Window {
    pub title: String,
    pub bounds: Rect,
    pub app_type: AppType,
    pub data: AppData,
    pub is_maximized: bool,
    pub old_bounds: Rect,
    pub is_minimized: bool,
}

impl Window {
    pub fn new(title: &str, x: u32, y: u32, width: u32, height: u32, app_type: AppType) -> Self {
        let data = match &app_type {
            AppType::BuiltIn { app_id, name } => {
                Self::create_app_data(*app_id, name)
            }
            AppType::AppLoader => {
                AppData::AppLoader(crate::apps::app_loader::AppLoaderState::new())
            }
            AppType::None => AppData::None,
        };

        Self {
            title: title.to_string(),
            bounds: Rect { x, y, width, height },
            app_type,
            data,
            is_maximized: false,
            old_bounds: Rect { x, y, width, height },
            is_minimized: false,
        }
    }

    fn create_app_data(app_id: u32, name: &str) -> AppData {
        match name {
            "Calculator" => AppData::BuiltIn {
                app_id,
                name: name.to_string(),
                state: Box::new(crate::apps::calculator::CalculatorState::new()),
            },
            "Terminal" => {
                let mut state = crate::apps::terminal::TerminalState::new();
                state.set_filesystem(crate::fs::NebulaFS::new("nebula_pool", 4096, 1024 * 1024));
                AppData::BuiltIn {
                    app_id,
                    name: name.to_string(),
                    state: Box::new(state),
                }
            }
            "Text Editor" => AppData::BuiltIn {
                app_id,
                name: name.to_string(),
                state: Box::new(crate::apps::text_editor::TextEditorState::new()),
            },
            "File Manager" => {
                let mut state = crate::apps::file_manager::FileManagerState::new();
                state.set_filesystem(crate::fs::NebulaFS::new("nebula_pool", 4096, 1024 * 1024));
                state.refresh_files();
                AppData::BuiltIn {
                    app_id,
                    name: name.to_string(),
                    state: Box::new(state),
                }
            }
            "Web Browser" => AppData::BuiltIn {
                app_id,
                name: name.to_string(),
                state: Box::new(crate::apps::web_browser::WebBrowserState::new()),
            },
            "Image Viewer" => AppData::BuiltIn {
                app_id,
                name: name.to_string(),
                state: Box::new(crate::apps::image_viewer::ImageViewerState::new()),
            },
            "System Monitor" => AppData::BuiltIn {
                app_id,
                name: name.to_string(),
                state: Box::new(crate::apps::system_monitor::SystemMonitorState::new()),
            },
            "System Settings" => AppData::BuiltIn {
                app_id,
                name: name.to_string(),
                state: Box::new(crate::apps::system_settings::SystemSettingsState::new()),
            },
            "App Loader" => AppData::AppLoader(crate::apps::app_loader::AppLoaderState::new()),
            _ => AppData::None,
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

        if mr && !self.last_right_btn {
            self.context_menu = Some((mx, my));
            menu_toggle = true;
        }
        self.last_right_btn = mr;

        if ml && !self.last_mouse_btn {
            if let Some((cx, cy)) = self.context_menu {
                let rel_x = mx - cx;
                let rel_y = my - cy;

                if rel_x >= 0 && rel_x < 150 && rel_y >= 0 && rel_y < 100 {
                    let item = rel_y / 20;
                    
                    // Get apps from loader service for dynamic menu
                    let apps_list = {
                        let ls = loader::get_loader_service().lock();
                        ls.list_apps()
                    };

                    if (item as usize) < apps_list.len() {
                        let app = &apps_list[item as usize];
                        self.windows.push(Window::new(
                            app.name.as_str(),
                            mx as u32,
                            my as u32,
                            500,
                            400,
                            AppType::BuiltIn {
                                app_id: app.app_id,
                                name: app.name.clone(),
                            },
                        ));
                    } else {
                        match item {
                            9 => {
                                // App Loader
                                self.windows.push(Window::new(
                                    "App Loader",
                                    mx as u32,
                                    my as u32,
                                    600,
                                    450,
                                    AppType::AppLoader,
                                ));
                            }
                            _ => {
                                self.windows.clear();
                            }
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
            let mut min_clicked = false;
            let mut max_clicked = false;

            if !taskbar_handled {
                for (i, win) in self.windows.iter().enumerate().rev() {
                    if win.is_minimized { continue; }
                    
                    let x = win.bounds.x as i32;
                    let y = win.bounds.y as i32;
                    let w = win.bounds.width as i32;

                    if mx >= x && mx <= x + w &&
                       my >= y && my <= y + TITLE_BAR_HEIGHT as i32 {
                        clicked_idx = Some(i);

                        if mx >= x + w - 25 {
                            close_clicked = true;
                            break;
                        }
                        if mx >= x + w - 50 {
                            max_clicked = true;
                            break;
                        }
                        if mx >= x + w - 75 {
                            min_clicked = true;
                            break;
                        }

                        is_dragging = true;
                        self.drag_off_x = mx - x;
                        self.drag_off_y = my - y;
                        break;
                    }

                    if mx >= win.bounds.x as i32 && mx <= (win.bounds.x + win.bounds.width) as i32 &&
                       my > (win.bounds.y + TITLE_BAR_HEIGHT) as i32 && my <= (win.bounds.y + win.bounds.height) as i32 {
                        clicked_idx = Some(i);
                        break;
                    }
                }
            }

            if let Some(idx) = clicked_idx {
                if close_clicked {
                    self.windows.remove(idx);
                } else {
                    let win = self.windows.remove(idx);
                    if max_clicked {
                        let (ow, oh) = if win.is_maximized {
                            (win.old_bounds.width, win.old_bounds.height)
                        } else {
                            (self.screen_w, self.screen_h - TASKBAR_HEIGHT)
                        };
                        let mut win = win;
                        if win.is_maximized {
                            win.bounds = win.old_bounds;
                        } else {
                            win.old_bounds = win.bounds;
                            win.bounds = Rect { x: 0, y: 0, width: ow, height: oh };
                        }
                        win.is_maximized = !win.is_maximized;
                        if !min_clicked {
                            self.windows.push(win);
                        }
                    } else {
                        if !min_clicked {
                            self.windows.push(win);
                        }
                    }
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

            if win.data.should_close() {
                if let Some(idx) = self.windows.iter().rev().position(|w| !w.is_minimized) {
                    let idx = self.windows.len() - 1 - idx;
                    self.windows.remove(idx);
                }
            }
        }
    }

    pub fn draw(&mut self, fb: &mut Framebuffer) {
        let window_count = self.windows.len();
        for (i, window) in self.windows.iter_mut().enumerate() {
            if window.is_minimized { continue; }

            fb.draw_rect(window.bounds.x as usize, window.bounds.y as usize, window.bounds.width as usize, window.bounds.height as usize, 0x00C0C0C0);
            
            fb.draw_rect(window.bounds.x as usize, window.bounds.y as usize, window.bounds.width as usize, TITLE_BAR_HEIGHT as usize, 0x000078D7);
            
            draw_string(fb, window.bounds.x as usize + 5, window.bounds.y as usize + 8, window.title.as_str(), 0xFFFFFF);

            let is_focused = i == window_count - 1;
            window.data.draw(fb, window.bounds, is_focused);
        }

        if let Some((cx, cy)) = self.context_menu {
            let apps = {
                let loader_service = loader::get_loader_service().lock();
                let apps_list = loader_service.list_apps();
                let menu_h = (apps_list.len() + 2) * 20;
                drop(loader_service);

                fb.draw_rect(cx as usize, cy as usize, 150, menu_h, 0x00E0E0E0);
                fb.draw_rect(cx as usize, cy as usize, 150, 1, 0x00000000);
                fb.draw_rect(cx as usize, cy as usize + menu_h - 1, 150, 1, 0x00000000);

                apps_list
            };

            for (i, app) in apps.iter().enumerate() {
                let label = format!("New {}", app.name);
                draw_string(fb, cx as usize + 10, cy as usize + 5 + (i * 20), label.as_str(), 0x000000);
            }

            // Draw App Loader and Close All
            draw_string(fb, cx as usize + 10, cy as usize + 5 + (apps.len() * 20), "New App Loader", 0x000000);
            draw_string(fb, cx as usize + 10, cy as usize + 5 + ((apps.len() + 1) * 20), "Close All", 0x000000);
        }
    }
}
