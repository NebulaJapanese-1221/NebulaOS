use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window, rect::Rect, button::Button};
use super::app::{App, AppEvent};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::{format, format as f}; // Ensure format macro is available
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use core::sync::atomic::{Ordering, AtomicBool};
use crate::kernel::net::io::{SliceReader, SliceWriter};
use core::cell::RefCell;
use crate::kernel::nebula_js::DomCommand;

#[derive(Clone, PartialEq)]
enum BrowserState {
    Idle,
    ResolvingDns,
    Connecting,
    SendingRequest,
    ReceivingData,
    TlsHandshake,
}

#[derive(Clone, PartialEq)]
enum ParsedUrl {
    Nebula(String), // e.g., "welcome", "settings"
    Http { host: String, path: String },
    Https { host: String, path: String },
    Invalid,
}

pub struct NebulaBrowser {
    pub url_buffer: String,
    pub is_editing: bool,
    pub current_page: String,
    pub page_title: String,
    pub is_loading: bool,
    pub show_settings: bool,
    pub homepage: String,
    pub history_cleared: bool,
    state: BrowserState,
    remote_ip: Option<smoltcp::wire::IpAddress>,
    pub favorites: Vec<String>,
    pub show_favorites: bool,
    tag_styles: Vec<(String, u32)>,
    parsed_current_url: ParsedUrl,
    javascript_code: String, // Store JS code for future execution
    dns_query_handle: Option<smoltcp::socket::dns::QueryHandle>,
    request_start_tick: usize,
    redirect_count: usize,
    link_hitboxes: RefCell<Vec<(Rect, String)>>, // Tracks link positions for click handling
    event_hitboxes: RefCell<Vec<(Rect, String)>>, // Tracks JS event handlers (onclick)
    onmouseover_hitboxes: RefCell<Vec<(Rect, String)>>, // Tracks hover JS handlers
    onmouseout_hitboxes: RefCell<Vec<(Rect, String)>>,  // Tracks mouseout JS handlers
    tls_conn: Option<rustls::client::ClientConnection>,
    pub loading_progress: usize,
    id_styles: RefCell<Vec<(String, String, u32)>>, // Per-element style overrides (id, property, color)
    last_hover_id: RefCell<Option<String>>,
}

impl Clone for NebulaBrowser {
    fn clone(&self) -> Self {
        Self {
            url_buffer: self.url_buffer.clone(),
            is_editing: self.is_editing,
            current_page: self.current_page.clone(),
            page_title: self.page_title.clone(),
            is_loading: self.is_loading,
            show_settings: self.show_settings,
            homepage: self.homepage.clone(),
            history_cleared: self.history_cleared,
            state: self.state.clone(),
            remote_ip: self.remote_ip,
            favorites: self.favorites.clone(),
            show_favorites: self.show_favorites,
            tag_styles: self.tag_styles.clone(),
            parsed_current_url: self.parsed_current_url.clone(),
            javascript_code: self.javascript_code.clone(),
            dns_query_handle: self.dns_query_handle,
            request_start_tick: self.request_start_tick,
            redirect_count: self.redirect_count,
            link_hitboxes: RefCell::new((*self.link_hitboxes.borrow()).clone()),
            event_hitboxes: RefCell::new((*self.event_hitboxes.borrow()).clone()),
            onmouseover_hitboxes: RefCell::new((*self.onmouseover_hitboxes.borrow()).clone()),
            onmouseout_hitboxes: RefCell::new((*self.onmouseout_hitboxes.borrow()).clone()),
            loading_progress: self.loading_progress,
            id_styles: RefCell::new((*self.id_styles.borrow()).clone()),
            last_hover_id: RefCell::new((*self.last_hover_id.borrow()).clone()),
            tls_conn: None, // TLS connections cannot be cloned
        }
    }
}

impl NebulaBrowser {
    pub fn new() -> Self {
        let mut browser = Self {
            url_buffer: String::from("nebula://welcome"),
            is_editing: false,
            current_page: String::new(), // Will be set after parsing initial URL
            page_title: String::from("NebulaBrowser"),
            is_loading: false,
            show_settings: false,
            homepage: String::from("nebula://welcome"),
            history_cleared: false,
            state: BrowserState::Idle,
            remote_ip: None,
            favorites: Vec::new(),
            show_favorites: false,
            tag_styles: Vec::new(),
            parsed_current_url: ParsedUrl::Invalid, // Will be set after parsing initial URL
            javascript_code: String::new(),
            dns_query_handle: None,
            request_start_tick: 0,
            redirect_count: 0,
            link_hitboxes: RefCell::new(Vec::new()),
            event_hitboxes: RefCell::new(Vec::new()),
            onmouseover_hitboxes: RefCell::new(Vec::new()),
            onmouseout_hitboxes: RefCell::new(Vec::new()),
            loading_progress: 0,
            id_styles: RefCell::new(Vec::new()),
            last_hover_id: RefCell::new(None),
            tls_conn: None,
        };
        // Initialize with the welcome page
        browser.parsed_current_url = Self::parse_url(browser.url_buffer.as_str());
        browser.load_internal_page("welcome");
        
        browser
    }

    /// Basic BMP decoder and renderer for 24-bit uncompressed bitmaps.
    fn draw_bmp(&self, fb: &mut framebuffer::Framebuffer, x: isize, y: isize, data: &[u8], clip: Rect) -> (usize, usize) {
        if data.len() < 54 || &data[0..2] != b"BM" { return (0, 0); }

        // Parse headers
        let pixel_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
        let width = u32::from_le_bytes([data[18], data[19], data[20], data[21]]) as usize;
        let height = u32::from_le_bytes([data[22], data[23], data[24], data[25]]) as usize;
        let bpp = u16::from_le_bytes([data[28], data[29]]) as usize;

        if bpp != 24 { return (0, 0); } // Only support 24-bit BGR for basic implementation

        // BMP rows are padded to 4-byte boundaries
        let row_size = (width * 3 + 3) & !3;

        for py in 0..height {
            for px in 0..width {
                // BMPs are stored bottom-to-top
                let data_idx = pixel_offset + (height - 1 - py) * row_size + (px * 3);
                if data_idx + 2 >= data.len() { break; }

                let b = data[data_idx];
                let g = data[data_idx + 1];
                let r = data[data_idx + 2];
                let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);

                let (sx, sy) = (x + px as isize, y + py as isize);
                if clip.contains(sx, sy) {
                    fb.set_pixel(sx as usize, sy as usize, color);
                }
            }
        }
        (width, height)
    }

    /// Applies DOM modifications to the current page string.
    fn apply_dom_commands(&mut self, commands: Vec<DomCommand>) {
        for cmd in commands {
            match cmd {
                DomCommand::UpdateInnerHTML { id, content } => {
                    // Find the position of the ID attribute
                    let id_marker = format!("id=\"{}\"", id);
                    if let Some(id_pos) = self.current_page.find(&id_marker) {
                        // Move to the end of the opening tag
                        if let Some(tag_end_offset) = self.current_page[id_pos..].find('>') {
                            let content_start = id_pos + tag_end_offset + 1;
                            
                            // Find the beginning of the closing tag
                            if let Some(close_tag_offset) = self.current_page[content_start..].find("</") {
                                let content_end = content_start + close_tag_offset;
                                
                                // Splice the new content into the HTML string
                                let mut new_page = self.current_page[..content_start].to_string();
                                new_page.push_str(&content);
                                new_page.push_str(&self.current_page[content_end..]);
                                self.current_page = new_page;
                            }
                        }
                    }
                }
                DomCommand::UpdateStyle { id, property, value } => {
                    self.apply_style_command(id, property, value);
                }
                DomCommand::SetTitle { title } => {
                    self.page_title = title;
                }
            }
        }
    }

    fn apply_style_command(&self, id: String, property: String, value: String) {
        if property == "color" {
            let color_val = if value.starts_with('#') {
                u32::from_str_radix(&value[1..], 16).unwrap_or(0xCCCCCC)
            } else if value == "red" {
                0x00_FF0000
            } else if value == "blue" {
                0x00_0000FF
            } else {
                0x00_CCCCCC
            };

            let mut styles = self.id_styles.borrow_mut();
            if let Some(idx) = styles.iter().position(|(sid, prop, _)| sid == &id && prop == &property) {
                styles[idx].2 = color_val;
            } else {
                styles.push((id, property, color_val));
            }
        }
    }

    fn draw_toolbar_spinner(&self, fb: &mut framebuffer::Framebuffer, cx: isize, cy: isize, clip: Rect) {
        let tick = crate::kernel::process::TICKS.load(Ordering::Relaxed);
        let head = (tick / 80) % 8; // Advance the rotation every 80ms

        let spokes: [(isize, isize); 8] = [
            (0, -6), (4, -4), (6, 0), (4, 4),
            (0, 6), (-4, 4), (-6, 0), (-4, -4)
        ];

        for i in 0..8 {
            let diff = (i + 8 - head as usize) % 8;
            let color = match diff {
                0 => 0x00_00_CC_FF, // Bright Cyan head
                1 => 0x00_00_88_AA, // Tail segment 1
                2 => 0x00_00_44_66, // Tail segment 2
                _ => 0x00_2D_2D_30, // Background color (effectively invisible)
            };
            let (dx, dy) = spokes[i];
            gui::draw_rect(fb, cx + dx, cy + dy, 3, 3, color, Some(clip));
        }
    }

    fn parse_url(url_str: &str) -> ParsedUrl {
        if url_str.starts_with("nebula://") {
            let path = url_str.trim_start_matches("nebula://").to_string();
            ParsedUrl::Nebula(path)
        } else if url_str.starts_with("https://") {
            let remainder = url_str.trim_start_matches("https://");
            if let Some(slash_idx) = remainder.find('/') {
                let host = remainder[..slash_idx].to_string();
                let path = remainder[slash_idx..].to_string();
                ParsedUrl::Https { host, path }
            } else {
                ParsedUrl::Https { host: remainder.to_string(), path: "/".to_string() }
            }
        } else if url_str.starts_with("http://") {
            let remainder = url_str.trim_start_matches("http://");
            if let Some(slash_idx) = remainder.find('/') {
                let host = remainder[..slash_idx].to_string();
                let path = remainder[slash_idx..].to_string();
                ParsedUrl::Http { host, path }
            } else {
                ParsedUrl::Http { host: remainder.to_string(), path: "/".to_string() }
            }
        } else {
            ParsedUrl::Invalid
        }
    }

    fn load_internal_page(&mut self, page_name: &str) {
        match page_name {
            "welcome" => {
                self.current_page = String::from("<html><style>h1 { color: #00CCFF; } b { color: #FFFFFF; }</style><h1>Welcome to NebulaOS</h1><p>This is a structured HTML rendering test.</p><br><b>Network Status:</b> Online</html>");
            }
            "settings" => {
                self.show_settings = true;
                self.current_page = String::from("<html><h1>Browser Settings</h1><p>Configure your browser preferences here.</p></html>");
            }
            "os-settings" => {
                self.current_page = String::from("<html><h1>OS Settings</h1><p>System configuration can be managed here.</p><br><b id=\"status\">Status: Operational</b><br><br><button onclick=\"document.getElementById('status').innerHTML = 'Update Checked!';\">Check for Updates</button><br><br><a href=\"nebula://welcome\">Back to Welcome</a></html>");
            }
            _ => {
                self.current_page = format!("<html><h1>Error</h1><p>Nebula page '{}' not found.</p></html>", page_name);
            }
        }
        self.parse_styles_from_html();
        self.is_loading = false;
        self.state = BrowserState::Idle;
        self.loading_progress = 100;
    }

    // Helper to get a specific CSS property for a tag
    fn get_style_property(&self, tag: &str, property: &str) -> Option<String> {
        // For now, tag_styles stores (selector, color_u32)
        // We need to expand this to store (selector, Vec<(property_name, value)>)
        // This is a placeholder for a more robust CSS engine.
        None // For now, only color is directly supported via get_tag_color
    }

    fn get_element_color(&self, tag: &str, id: Option<&str>) -> u32 {
        // Check ID-specific styles first (JS overrides)
        if let Some(id_val) = id {
            for (sid, prop, color) in self.id_styles.borrow().iter() {
                if sid == id_val && prop == "color" { return *color; }
            }
        }
        // Fallback to CSS tag styles
        for (selector, color) in &self.tag_styles {
            if selector == tag { return *color; }
        }
        match tag {
            "h1" => 0x00_55_AA_FF,
            "b" => 0x00_FFFFFF,
            "a" => 0x00_00_AA_FF, // Blue for links
            _ => 0x00_CCCCCC,
        }
    }

    fn parse_styles_from_html(&mut self) {
        self.tag_styles.clear();
        self.javascript_code.clear(); // Clear JS code on new page load

        let html_content = self.current_page.to_owned(); // Use to_owned() to avoid borrowing issues with self.current_page
        let html = html_content.as_str();
        let mut start_idx = 0;
        while let Some(style_start) = html[start_idx..].find("<style>") {
            let content_start = start_idx + style_start + 7;
            if let Some(style_end) = html[content_start..].find("</style>") {
                let css = &html[content_start..content_start + style_end];
                for rule in css.split('}') {
                    if let Some(brace) = rule.find('{') {
                        let selector = rule[..brace].trim();
                        let body = &rule[brace + 1..];
                        if let Some(col_idx) = body.find("color:") {
                            let val = body[col_idx + 6..].split(';').next().unwrap_or("").trim();
                            if val.starts_with('#') && val.len() >= 7 { // Allow for shorter hex codes like #FFF
                                if let Ok(color) = u32::from_str_radix(&val[1..], 16) {
                                    self.tag_styles.push((selector.to_string(), color));
                                }
                            } else if val == "red" {
                                self.tag_styles.push((selector.to_string(), 0x00_FF0000));
                            } else if val == "blue" {
                                self.tag_styles.push((selector.to_string(), 0x00_0000FF));
                            } // Add more named colors as needed
                        } else if let Some(bg_col_idx) = body.find("background-color:") {
                            let val = body[bg_col_idx + 17..].split(';').next().unwrap_or("").trim();
                            if val.starts_with('#') && val.len() >= 7 {
                                // For now, we'll just store the background color, but not apply it directly in render_html
                                // A more complex renderer would use this.
                            }
                        }
                    }
                }
                start_idx = content_start + style_end + 8;
            } else { break; }
        }

        // Extract JavaScript code
        let mut found_js = false;
        start_idx = 0;
        while let Some(script_start) = html[start_idx..].find("<script>") {
            let content_start = start_idx + script_start + 8;
            if let Some(script_end) = html[content_start..].find("</script>") {
                self.javascript_code.push_str(&html[content_start..content_start + script_end]);
                self.javascript_code.push('\n'); // Add newline for multi-script blocks
                start_idx = content_start + script_end + 9;
                found_js = true;
            } else { break; }
        }

        if found_js {
            let commands = crate::kernel::nebula_js::NebulaJS::execute(self.javascript_code.as_str());
            if !commands.is_empty() {
                self.apply_dom_commands(commands);
                // Re-parse styles in case the JS added new tags or changed class/id attributes
                self.parse_styles_from_html();
            }
        }
    }

    fn render_html(&self, fb: &mut framebuffer::Framebuffer, x: isize, y: isize, html: &str, clip: Rect) {
        let mut cur_x = x;
        let mut cur_y = y;
        let mut current_tag = "p";
        let mut current_link: Option<String> = None; // For <a> tags
        let mut current_onclick: Option<String> = None; // For onclick JS
        let mut current_onmouseover: Option<String> = None; // For onmouseover JS
        let mut current_onmouseout: Option<String> = None; // For onmouseout JS
        let mut current_id: Option<String> = None; // Ensure this is Option<String>
        let mut is_italic = false;
        let mut is_underline = false;
        
        let font_height = if gui::LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };

        let mut i = 0;
        let bytes = html.as_bytes();
        while i < bytes.len() {
            if bytes[i] == b'<' {
                let end = html[i..].find('>').unwrap_or(0);
                let tag_content = &html[i + 1..i + end];
                
                // Split tag to handle attributes
                let mut parts = tag_content.split_whitespace();
                let tag_name = parts.next().unwrap_or("");

                // Reset per-tag attributes
                current_id = None;

                // Look for onclick attribute manually to handle spaces in JS code
                if let Some(click_start) = tag_content.find("onclick=\"") {
                    let start = click_start + 9;
                    if let Some(end_offset) = tag_content[start..].find('"') {
                        current_onclick = Some(tag_content[start..start + end_offset].to_string());
                    }
                }
                if let Some(hover_start) = tag_content.find("onmouseover=\"") {
                    let start = hover_start + 13;
                    if let Some(end_offset) = tag_content[start..].find('"') {
                        current_onmouseover = Some(tag_content[start..start + end_offset].to_string());
                    }
                }
                if let Some(out_start) = tag_content.find("onmouseout=\"") {
                    let start = out_start + 12;
                    if let Some(end_offset) = tag_content[start..].find('"') {
                        current_onmouseout = Some(tag_content[start..start + end_offset].to_string());
                    }
                }
                if let Some(id_start) = tag_content.find("id=\"") {
                    let start = id_start + 4;
                    if let Some(end_offset) = tag_content[start..].find('"') {
                        current_id = Some(tag_content[start..start + end_offset].to_string());
                    }
                }

                match tag_name {
                    "h1" => { current_tag = "h1"; }
                    "/h1" => { current_tag = "p"; cur_y += 30; cur_x = x; }
                    "i" => { is_italic = true; }
                    "/i" => { is_italic = false; }
                    "u" => { is_underline = true; }
                    "/u" => { is_underline = false; }
                    "hr" => {
                        cur_y += 10;
                        gui::draw_rect(fb, x, cur_y, win.width - 40, 1, 0x00_44_44_44, Some(clip));
                        cur_y += 10; cur_x = x;
                    }
                    "b" => { current_tag = "b"; }
                    "/b" => { current_tag = "p"; }
                    "button" => { 
                        current_tag = "button";
                        gui::draw_rect(fb, cur_x, cur_y - 2, 80, 20, 0x00_3E_3E_42, Some(clip));
                    }
                    "/button" => { current_tag = "p"; cur_x += 85; current_onclick = None; }
                    "br" => { cur_y += 20; cur_x = x; }
                    "p" => { cur_y += 10; cur_x = x; current_tag = "p"; }
                    "/p" => { cur_y += 20; cur_x = x; current_tag = "p"; }
                    "div" => { cur_y += 10; cur_x = x; current_tag = "div"; }
                    "/div" => { cur_y += 10; cur_x = x; current_tag = "p"; }
                    "span" => { current_tag = "span"; } // Inline, no new line
                    "/span" => { current_tag = "p"; }
                    "ul" => { cur_y += 10; cur_x = x; }
                    "/ul" => { cur_y += 10; cur_x = x; }
                    "li" => { cur_y += 5; cur_x = x + 15; font::draw_string(fb, x + 5, cur_y, "-", 0x00_AAAAAA, Some(clip)); }
                    "a" => {
                        current_tag = "a";
                        if let Some(href_attr) = parts.find(|p| p.starts_with("href=")) {
                            current_link = Some(href_attr.trim_start_matches("href=").trim_matches('"').to_string());
                        }
                    }
                    "/a" => { current_tag = "p"; current_link = None; }
                    "img" => {
                        for attr in parts {
                            if attr.starts_with("src=") {
                                let src = attr.trim_start_matches("src=").trim_matches('"');
                                // Mock Logic: In a full browser, this would trigger a background fetch.
                                // For now, we assume images are embedded or provide logic to handle the data.
                                if src == "nebula_logo.bmp" {
                                    // Placeholder for actual image data access.
                                    // For now, we'll just draw a placeholder rect.
                                    let img_w = 50; let img_h = 50;
                                    gui::draw_rect(fb, cur_x, cur_y, img_w, img_h, 0x00_808080, Some(clip));
                                    font::draw_string(fb, cur_x + 5, cur_y + 20, "[IMG]", 0x00_FFFFFF, Some(clip));
                                    // cur_y += h as isize + 10;
                                    // cur_x = x;
                                }
                            }
                        }
                    }
                    "style" => {
                        if let Some(style_end) = html[i..].find("</style>") {
                            i += style_end + 8;
                            continue;
                        }
                    }
                    "script" => {
                        // Script content is already extracted in parse_styles_from_html
                        // Just skip it during rendering
                        if let Some(script_end) = html[i..].find("</script>") { i += script_end + 9; continue; }
                    }
                    _ => {}
                }
                i += end + 1;
            } else {
                let next_tag = html[i..].find('<').unwrap_or(html.len() - i);
                let text = &html[i..i + next_tag];
                
                let color = self.get_element_color(current_tag, current_id.as_ref().map(|s| s.as_str()));
                font::draw_string(fb, cur_x, cur_y, text, color, Some(clip)); 

                // Record hitboxes for hover/out events
                if let (Some(ref id), Some(ref js_code)) = (current_id.as_ref(), current_onmouseover.as_ref()) {
                    let hitbox = Rect { x: cur_x, y: cur_y, width: font::string_width(text), height: font_height };
                    // We store ID as the second element to associate hover/out
                    self.onmouseover_hitboxes.borrow_mut().push((hitbox, format!("{}|{}", id, js_code)));
                }

                if let (Some(ref id), Some(ref js_code)) = (current_id.as_ref(), current_onmouseout.as_ref()) {
                    let hitbox = Rect { x: cur_x, y: cur_y, width: font::string_width(text), height: font_height };
                    self.onmouseout_hitboxes.borrow_mut().push((hitbox, format!("{}|{}", id, js_code)));
                }

                // Record hitboxes for hover events
                if let Some(ref js_code) = current_onmouseover {
                    let text_w = font::string_width(text);
                    let hitbox = Rect {
                        x: cur_x,
                        y: cur_y,
                        width: text_w,
                        height: font_height,
                    };
                    self.onmouseover_hitboxes.borrow_mut().push((hitbox, js_code.clone()));
                }

                // Record hitbox for onclick event
                if let Some(ref js_code) = current_onclick {
                    let text_w = font::string_width(text);
                    let hitbox = Rect {
                        x: cur_x,
                        y: cur_y,
                        width: if current_tag == "button" { 80 } else { text_w },
                        height: font_height,
                    };
                    self.event_hitboxes.borrow_mut().push((hitbox, js_code.clone()));
                }

                // If inside a link, record the hitbox
                if let Some(ref url) = current_link {
                    let text_w = font::string_width(text);
                    let hitbox = Rect {
                        x: cur_x,
                        y: cur_y,
                        width: text_w,
                        height: font_height,
                    };
                    self.link_hitboxes.borrow_mut().push((hitbox, url.clone()));
                }
                
                cur_x += font::string_width(text) as isize;
                i += next_tag;
            }
        }
    }
}

impl App for NebulaBrowser {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window, dirty_rect: Rect) { // Removed Ordering::SeqCst
        let font_height = if gui::LARGE_TEXT.load(core::sync::atomic::Ordering::SeqCst) { 32 } else { 16 };
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

        // Favorites Toggle Button
        let fav_toggle_btn = Button::new(win.x + 125, toolbar_y + 5, 26, 26, "F");
        fav_toggle_btn.draw(fb, 0, 0, Some(dirty_rect));

        // Settings Button (Right side)
        let settings_btn = Button::new(win.x + win.width as isize - 85, toolbar_y + 5, 30, 26, "S");
        settings_btn.draw(fb, 0, 0, Some(dirty_rect));

        // Add to Favorites Button (+)
        let add_fav_btn = Button::new(win.x + win.width as isize - 120, toolbar_y + 5, 30, 26, "+");
        add_fav_btn.draw(fb, 0, 0, Some(dirty_rect));

        // URL Bar
        let url_bar_x = win.x + 155;
        let url_bar_w = win.width.saturating_sub(280);
        gui::draw_rect(fb, url_bar_x, toolbar_y + 6, url_bar_w, 24, 0x00_12_12_12, Some(dirty_rect));
        
        let text_color = if self.is_editing { 0x00_FFFFFF } else { 0x00_A0_A0_A0 };
        font::draw_string(fb, url_bar_x + 5, toolbar_y + 10, self.url_buffer.as_str(), text_color, Some(dirty_rect));

        if self.is_loading {
            // Draw spinner in place of the Go button
            self.draw_toolbar_spinner(fb, win.x + win.width as isize - 25, toolbar_y + 18, dirty_rect);
            
            // Draw slim progress bar at the bottom of the toolbar
            let pb_y = toolbar_y + toolbar_h as isize - 2;
            let mut pb = gui::progress_bar::ProgressBar::new(win.x, pb_y, win.width, 2, self.loading_progress, 0x00_00_CC_FF);
            pb.orientation = gui::progress_bar::ProgressBarOrientation::Horizontal;
            pb.draw(fb, Some(dirty_rect));
        } else {
            let go_btn = Button::new(win.x + win.width as isize - 45, toolbar_y + 5, 40, 26, "Go");
            go_btn.draw(fb, 0, 0, Some(dirty_rect));
        }

        // Draw typing cursor if focused
        if self.is_editing {
            let cursor_x = url_bar_x + 5 + font::string_width(self.url_buffer.as_str()) as isize;
            if cursor_x < (url_bar_x + url_bar_w as isize - 5) {
                gui::draw_rect(fb, cursor_x, toolbar_y + 10, 2, 16, 0x00_00_7A_CC, Some(dirty_rect));
            }
        }

        let content_y = toolbar_y + toolbar_h as isize + 20;

        if self.show_settings {
            font::draw_string(fb, win.x + 20, content_y, "Browser Settings", 0x00_FF_FF_FF, Some(dirty_rect));
            font::draw_string(fb, win.x + 20, content_y + 30, &format!("Homepage: {}", self.homepage), 0x00_CCCCCC, Some(dirty_rect));
            
            let home_btn = Button::new(win.x + 20, content_y + 60, 180, 25, "Set as Homepage");
            home_btn.draw(fb, 0, 0, Some(dirty_rect));

            let clear_btn = Button::new(win.x + 20, content_y + 95, 150, 25, "Clear History");
            clear_btn.draw(fb, 0, 0, Some(dirty_rect));
            
            if self.history_cleared {
                font::draw_string(fb, win.x + 180, content_y + 100, "Done!", 0x00_00_FF_00, Some(dirty_rect));
            }
            return;
        }

        if self.show_favorites {
            font::draw_string(fb, win.x + 20, content_y, "Favorites", 0x00_FF_FF_FF, Some(dirty_rect));
            let mut fy = content_y + 30;
            if self.favorites.is_empty() {
                font::draw_string(fb, win.x + 20, fy, "No bookmarks yet.", 0x00_888888, Some(dirty_rect));
            } else {
                for fav in &self.favorites {
                    // Fixed: Explicitly call .as_str() to avoid &String vs &str mismatch
                    font::draw_string(fb, win.x + 20, fy, fav.as_str(), 0x00_AAAAFF, Some(dirty_rect));
                    fy += 20;
                }
            }
            return;
        }

        if self.is_loading && self.parsed_current_url != ParsedUrl::Nebula("".to_string()) { // Don't show loading for internal pages
             font::draw_string(fb, win.x + 20, content_y, &format!("Loading: {}", self.url_buffer), 0x00_00_AA_FF, Some(dirty_rect));
        } else {
             self.render_html(fb, win.x + 20, content_y, self.current_page.as_str(), dirty_rect);
        }
    }

    fn handle_event(&mut self, event: &AppEvent, _win: &Window) -> Option<Rect> {
        match event {
            AppEvent::MouseClick { x, y, width, .. } => {
                let toolbar_h = 36;
                let content_y_rel = toolbar_h as isize;
                // Home button hit test
                if *x >= 95 && *x < 121 && *y >= 5 && *y < 31 {
                    self.url_buffer = self.homepage.clone();
                    self.state = BrowserState::Idle;
                    if self.url_buffer == "nebula://welcome" {
                        self.current_page = String::from("<html><style>h1 { color: #00CCFF; } b { color: #FFFFFF; } a { color: #00AAFF; }</style><h1>Welcome to NebulaOS</h1><p>This is a structured HTML rendering test.</p><br><b>Network Status:</b> Online<br><a href=\"nebula://settings\">Browser Settings</a><br><div>This is a div.</div><span>This is a span.</span><img src=\"nebula_logo.bmp\"></html>");
                        self.parse_styles_from_html();
                    }
                    self.is_editing = false;
                    self.show_settings = false;
                    self.show_favorites = false;
                    return Some(_win.rect());
                }

                // Favorites Toggle Button hit test
                if *x >= 125 && *x < 151 && *y >= 5 && *y < 31 {
                    self.show_favorites = !self.show_favorites;
                    self.show_settings = false;
                    return Some(_win.rect());
                }

                // Add Favorite Button hit test
                if *x >= *width as isize - 120 && *x < *width as isize - 90 && *y >= 5 && *y < 31 {
                    if !self.favorites.contains(&self.url_buffer) {
                        self.favorites.push(self.url_buffer.clone());
                    }
                    return Some(_win.rect());
                }

                // Settings Button hit test
                if *x >= *width as isize - 85 && *x < *width as isize - 55 && *y >= 5 && *y < 31 {
                    if self.url_buffer == "nebula://settings" {
                        self.url_buffer = self.homepage.clone();
                    } else {
                        self.url_buffer = "nebula://settings".into();
                    }
                    self.handle_navigation();
                    return Some(_win.rect());
                }

                // Browser Settings Interaction
                if self.show_settings && *x >= 20 && *x < 200 {
                    if *y >= content_y_rel + 60 && *y < content_y_rel + 85 {
                        self.homepage = self.url_buffer.clone();
                        return Some(_win.rect());
                    } else if *y >= content_y_rel + 95 && *y < content_y_rel + 120 {
                        self.history_cleared = true;
                        return Some(_win.rect());
                    }
                }

                // Favorites Interaction
                if self.show_favorites && *x >= 20 && *x < 300 {
                    let fy_rel = content_y_rel + 30;
                    if *y >= fy_rel {
                        let idx = ((*y - fy_rel) / 20) as usize;
                        if idx < self.favorites.len() {
                            self.url_buffer = self.favorites[idx].clone();
                            self.show_favorites = false;
                            self.state = if self.url_buffer.starts_with("nebula://") { BrowserState::Idle } else { BrowserState::ResolvingDns };
                            self.is_loading = self.state != BrowserState::Idle;
                            self.parsed_current_url = Self::parse_url(self.url_buffer.as_str());
                            if !self.is_loading { self.load_internal_page(self.url_buffer.trim_start_matches("nebula://")); }
                            self.parse_styles_from_html();
                            return Some(_win.rect());
                        }
                    }
                }

                let screen_click_x = *x + _win.x;
                let screen_click_y = *y + _win.y + (toolbar_h as isize + title_height as isize);

                // 1. JS Event hit testing (onclick)
                let mut hit_js: Option<String> = None;
                for (rect, code) in self.event_hitboxes.borrow().iter() {
                    if rect.contains(screen_click_x, screen_click_y) {
                        hit_js = Some(code.clone()); // Explicitly create owned String from reference
                        break;
                    }
                }
                if let Some(code) = hit_js {
                    let commands = crate::kernel::nebula_js::NebulaJS::execute(code.as_str());
                    if !commands.is_empty() {
                        self.apply_dom_commands(commands);
                        self.parse_styles_from_html();
                    }
                    return Some(_win.rect());
                }

                // 2. Link hit testing using calculated hitboxes
                let mut hit_url: Option<String> = None;
                for (rect, url) in self.link_hitboxes.borrow().iter() {
                    if rect.contains(screen_click_x, screen_click_y) {
                        hit_url = Some(url.clone()); // Explicitly create owned String from reference
                        break;
                    }
                }
                if let Some(url) = hit_url {
                        self.url_buffer = url;
                        self.handle_navigation(); // Trigger navigation
                        return Some(_win.rect());
                }

                // URL bar hit test (relative to content area)
                let url_bar_w = (*width).saturating_sub(280);
                if *x >= 155 && *x < 155 + url_bar_w as isize && *y >= 6 && *y < 30 {
                    self.is_editing = true;
                    self.show_favorites = false;
                    self.show_settings = false;
                } else {
                    self.is_editing = false;
                }
                return Some(_win.rect());
            }
            AppEvent::MouseMove { x, y, .. } => {
                let font_height = if gui::LARGE_TEXT.load(Ordering::Relaxed) { 32 } else { 16 };
                let title_h = (font_height + 10) as isize;
                let toolbar_h = 36;

                let screen_mouse_x = *x + _win.x;
                let screen_mouse_y = *y + _win.y + toolbar_h as isize + title_h;

                // 1. Mouse Out Detection
                let mut mouseout_js = None;
                // Fix E0308: Dereference Ref guard to get the inner Option<String>
                let last_id: Option<String> = (*self.last_hover_id.borrow()).clone();
                if let Some(ref id) = last_id {
                    let mut still_hovering = false;
                    for (rect, _) in self.onmouseover_hitboxes.borrow().iter() {
                        if rect.contains(screen_mouse_x, screen_mouse_y) {
                            still_hovering = true; break;
                        }
                    }
                    if !still_hovering {
                        // Look for out handler for this ID
                        for (_, data) in self.onmouseout_hitboxes.borrow().iter() {
                            if data.starts_with(id) {
                                mouseout_js = Some(data.split('|').nth(1).unwrap_or("").to_string());
                                break;
                            }
                        }
                        *self.last_hover_id.borrow_mut() = None;
                    }
                }

                if let Some(code) = mouseout_js {
                    let commands = crate::kernel::nebula_js::NebulaJS::execute(code.as_str());
                    self.apply_dom_commands(commands);
                    return Some(_win.rect());
                }

                // 2. Mouse Over Detection
                let mut hit_hover_js: Option<String> = None;
                for (rect, data) in self.onmouseover_hitboxes.borrow().iter() {
                    if rect.contains(screen_mouse_x, screen_mouse_y) {
                        let parts: Vec<&str> = data.as_str().split('|').collect();
                        if parts.len() == 2 {
                            let id = parts[0];
                            // Compare owned Option with dereferenced Ref guard
                            if Some(id.to_string()) != *self.last_hover_id.borrow() {
                                hit_hover_js = Some(parts[1].to_string());
                                *self.last_hover_id.borrow_mut() = Some(id.to_string());
                            }
                        }
                        break;
                    }
                }

                if let Some(code) = hit_hover_js {
                    let commands = crate::kernel::nebula_js::NebulaJS::execute(code.as_str());
                    if !commands.is_empty() {
                        self.apply_dom_commands(commands);
                    }
                    return Some(_win.rect());
                }
                None
            }
            AppEvent::KeyPress { key } if self.is_editing => {
                match *key {
                    '\x08' => { self.url_buffer.pop(); } // Backspace
                    '\n' => { 
                        self.is_editing = false; 
                        self.handle_navigation();
                        return Some(_win.rect());
                    }
                    c if !c.is_control() => { self.url_buffer.push(c); }
                    _ => {}
                }
                return Some(_win.rect());
            }
            AppEvent::Tick { tick_count } => {
                if !self.is_loading || matches!(self.parsed_current_url, ParsedUrl::Nebula(_)) { return None; } // Don't process network for internal pages

                let mut sockets_guard = crate::kernel::net::SOCKET_SET.lock();
                let sockets = sockets_guard.as_mut()?;
                let mut dirty = true; // Ensure redrawing occurs for spinner animation while loading

                // Generic Request Timeout Logic (10 seconds for all phases)
                if *tick_count > self.request_start_tick + 10000 {
                    crate::serial_println!("[Browser] Request timed out.");
                    self.is_loading = false;
                    self.state = BrowserState::Idle;
                    self.tls_conn = None;
                    self.current_page = String::from("<html><h1>Error</h1><p>The request timed out. Please check your connection.</p></html>");
                    self.parse_styles_from_html();
                    return Some(_win.rect());
                }

                match self.state {
                    BrowserState::ResolvingDns => {
                        let handle_lock = crate::kernel::net::DNS_HANDLE.lock();
                        let handle = (*handle_lock)?;
                        let query_handle = self.dns_query_handle?;

                        let dns_socket = if let smoltcp::socket::Socket::Dns(s) = sockets.get_mut(handle) {
                            s
                        } else { return None; };

                        match dns_socket.get_query_result(query_handle) {
                            Ok(addrs) => {
                                if let Some(addr) = addrs.first() {
                                    self.remote_ip = Some(*addr);
                                    self.state = BrowserState::Connecting;
                                    self.loading_progress = 40;
                                    
                                    // Start TCP Connect
                                    let tcp_handle_lock = crate::kernel::net::HTTP_HANDLE.lock();
                                    let tcp_handle = (*tcp_handle_lock)?;
                                    let tcp_socket = if let smoltcp::socket::Socket::Tcp(s) = sockets.get_mut(tcp_handle) {
                                        s
                                    } else { return None; };
                                    
                                    let port = match self.parsed_current_url {
                                        ParsedUrl::Https { .. } => 443,
                                        _ => 80,
                                    };

                                    let local_port = 49152 + (crate::kernel::process::TICKS.load(Ordering::Relaxed) % 16384) as u16;
                                    tcp_socket.connect(crate::kernel::net::INTERFACE.lock().as_mut().unwrap().context(), (*addr, port), local_port).unwrap();
                                    dirty = true;
                                }
                            }
                            Err(smoltcp::socket::dns::GetQueryResultError::Pending) => {},
                            _ => { self.is_loading = false; dirty = true; }
                        }
                    }
                    BrowserState::Connecting => {
                        let handle = (*crate::kernel::net::HTTP_HANDLE.lock())?;
                        self.loading_progress = 65;
                        let tcp_socket = if let smoltcp::socket::Socket::Tcp(s) = sockets.get_mut(handle) {
                            s
                        } else { return None; };
                        self.loading_progress = 50;

                        if tcp_socket.is_active() && tcp_socket.state() == smoltcp::socket::tcp::State::Established {
                            if matches!(self.parsed_current_url, ParsedUrl::Https { .. }) {
                                self.state = BrowserState::TlsHandshake;
                            } else {
                                self.state = BrowserState::SendingRequest;
                            }
                            dirty = true;
                        }
                    }
                    BrowserState::TlsHandshake => {
                        // Example of initializing the rustls client using our global CA store:
                        if let Some(store) = crate::kernel::net::ROOT_CERT_STORE.lock().as_ref() {
                            // let config = rustls::ClientConfig::builder()
                            //     .with_root_certificates(store.clone())
                            //     .with_no_client_auth();
                            // self.state = BrowserState::SendingRequest;
                            // dirty = true;
                        }
                    }
                    BrowserState::SendingRequest => {
                        let handle = (*crate::kernel::net::HTTP_HANDLE.lock())?;
                        let tcp_socket = if let smoltcp::socket::Socket::Tcp(s) = sockets.get_mut(handle) {
                            s
                        } else { return None; };
                        self.loading_progress = 85;

                        if tcp_socket.can_send() {
                            let host = self.url_buffer.trim_start_matches("http://").trim_start_matches("https://").split('/').next().unwrap_or("");
                            let request = format!("GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", host);

                            if let Some(ref mut conn) = self.tls_conn {
                                // Write plaintext to TLS buffer
                                let _ = conn.writer().write_all(request.as_bytes());
                                // Pipe encrypted data to wire
                                tcp_socket.send(|data| {
                                    let mut writer = SliceWriter { data, pos: 0 };
                                    let _ = conn.write_tls(&mut writer);
                                    (writer.pos, ())
                                }).ok();
                            } else {
                                let _ = tcp_socket.send_slice(request.as_bytes());
                            }

                            self.state = BrowserState::ReceivingData;
                            dirty = true;
                        }
                    }
                    BrowserState::ReceivingData => {
                        let handle = (*crate::kernel::net::HTTP_HANDLE.lock())?;
                        let tcp_socket = if let smoltcp::socket::Socket::Tcp(s) = sockets.get_mut(handle) {
                            s
                        } else { return None; };
                        self.loading_progress = 95;

                        if let Some(ref mut conn) = self.tls_conn {
                            if tcp_socket.can_recv() {
                                tcp_socket.recv(|data| {
                                    let mut reader = SliceReader { data, pos: 0 };
                                    let n = conn.read_tls(&mut reader).map(|_| reader.pos).unwrap_or(0);
                                    (n, ())
                                }).ok();
                                let _ = conn.process_new_packets();

                                let mut plaintext = Vec::<u8>::new();
                                if let Ok(_) = conn.reader().read_to_end(&mut plaintext) {
                                    let response = String::from_utf8_lossy(&plaintext[..]);
                                    self.current_page.push_str(response.as_ref());
                                    self.parse_styles_from_html();
                                }
                            }
                        } else if tcp_socket.can_recv() {
                            tcp_socket.recv(|data| {
                                let response = String::from_utf8_lossy(data);
                                
                                // Check for HTTP Redirect (301/302)
                                if response.starts_with("HTTP/1.1 30") || response.starts_with("HTTP/1.0 30") {
                                    if self.redirect_count >= 5 {
                                        self.current_page = String::from("<html><h1>Error</h1><p>Too many redirects.</p></html>");
                                        self.is_loading = false;
                                        self.state = BrowserState::Idle;
                                    } else {
                                        // Look for Location header
                                        let mut new_location = None;
                                        for line in response.lines() {
                                            if line.to_lowercase().starts_with("location:") {
                                                new_location = Some(line[9..].trim().to_string());
                                                break;
                                            }
                                        }

                                        if let Some(loc) = new_location {
                                            self.redirect_count += 1;
                                            self.url_buffer = if loc.starts_with('/') {
                                                // Handle relative redirect
                                                let host = match &self.parsed_current_url {
                                                    ParsedUrl::Http { host, .. } => format!("http://{}", host),
                                                    ParsedUrl::Https { host, .. } => format!("https://{}", host),
                                                    _ => String::new(),
                                                };
                                                format!("{}{}", host, loc)
                                            } else {
                                                loc
                                            };
                                            
                                            // Restart request cycle
                                            self.parsed_current_url = Self::parse_url(self.url_buffer.as_str());
                                            self.state = BrowserState::ResolvingDns;
                                            self.current_page.clear();
                                            crate::serial_println!("[Browser] Redirecting to: {}", self.url_buffer);
                                            return (data.len(), ());
                                        }
                                    }
                                }

                                // Standard content loading
                                let body = if let Some(start) = response.find("\r\n\r\n") { &response[start+4..] } else { response.as_ref() };
                                self.current_page.push_str(body);
                                
                                self.parse_styles_from_html();
                                (data.len(), ())
                            }).unwrap();
                        }

                        if !tcp_socket.is_active() || tcp_socket.state() == smoltcp::socket::tcp::State::Closed {
                            self.is_loading = false;
                            self.state = BrowserState::Idle;
                            self.tls_conn = None;
                            dirty = true;
                        }
                    }
                    _ => {}
                }
                if dirty { return Some(_win.rect()); }
            }
            _ => {}
        }
        None
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }
}

impl NebulaBrowser {
    // Helper to initiate navigation based on current url_buffer
    fn handle_navigation(&mut self) {
        self.request_start_tick = crate::kernel::process::TICKS.load(Ordering::Relaxed);
        self.redirect_count = 0;
        self.parsed_current_url = Self::parse_url(self.url_buffer.as_str());
        self.is_loading = true;
        self.state = BrowserState::Idle; // Reset state for new navigation
        self.current_page.clear(); // Clear previous page content
        self.javascript_code.clear(); // Clear previous JS
        self.tls_conn = None; // Clear TLS connection
        self.link_hitboxes.borrow_mut().clear(); // Clear hitboxes
        self.event_hitboxes.borrow_mut().clear(); // Clear event hitboxes
        self.onmouseover_hitboxes.borrow_mut().clear();
        self.onmouseout_hitboxes.borrow_mut().clear();
        *self.last_hover_id.borrow_mut() = None;
        self.loading_progress = 0;

        match &self.parsed_current_url {
            ParsedUrl::Nebula(page) => {
                self.load_internal_page(page.as_str());
            }
            ParsedUrl::Http { .. } | ParsedUrl::Https { .. } => {
                self.state = BrowserState::ResolvingDns;
            }
            ParsedUrl::Invalid => {
                self.current_page = String::from("<html><h1>Error</h1><p>Invalid URL format.</p></html>");
                self.parse_styles_from_html();
                self.is_loading = false;
            }
        }
    }

    // Helper to handle HTTP status codes (redirects, errors)
    // Returns true if the response was fully handled (e.g., redirect, error page)
    fn handle_http_status(&mut self, status_code: &str, response: &str) -> bool {
        match status_code {
            "301" | "302" | "307" | "308" => {
                if self.redirect_count >= 5 {
                    self.current_page = String::from("<html><h1>Error</h1><p>Too many redirects.</p></html>");
                    self.is_loading = false;
                    self.state = BrowserState::Idle;
                } else {
                    let mut new_location = None;
                    for line in response.lines() {
                        if line.to_lowercase().starts_with("location:") {
                            new_location = Some(line[9..].trim().to_string());
                            break;
                        }
                    }
                    if let Some(loc) = new_location {
                        self.redirect_count += 1;
                        self.url_buffer = if loc.starts_with('/') {
                            let host = match &self.parsed_current_url {
                                ParsedUrl::Http { host, .. } => format!("http://{}", host),
                                ParsedUrl::Https { host, .. } => format!("https://{}", host),
                                _ => String::new(),
                            };
                            format!("{}{}", host, loc)
                        } else { loc };
                        self.handle_navigation(); // Restart navigation
                        return true;
                    }
                }
            }
            "400" => self.current_page = String::from("<html><style>h1 { color: #FF4040; }</style><h1>400 Bad Request</h1><p>The server cannot process the request due to a client error.</p></html>"),
            "401" => self.current_page = String::from("<html><style>h1 { color: #FFD700; }</style><h1>401 Unauthorized</h1><p>Authentication is required to access this resource.</p></html>"),
            "403" => self.current_page = String::from("<html><style>h1 { color: #FF4040; }</style><h1>403 Forbidden</h1><p>You do not have permission to view this page.</p></html>"),
            "404" => self.current_page = String::from("<html><style>h1 { color: #FF4040; }</style><h1>404 Not Found</h1><p>The requested page could not be located on this server.</p></html>"),
            "405" => self.current_page = String::from("<html><style>h1 { color: #FFD700; }</style><h1>405 Method Not Allowed</h1><p>The method specified in the request is not allowed.</p></html>"),
            "500" => self.current_page = String::from("<html><style>h1 { color: #FF8040; }</style><h1>500 Internal Server Error</h1><p>The server encountered an unexpected condition.</p></html>"),
            _ => return false, // Not an error or redirect we handle here
        }
        // If we reached here, it's an error page.
        self.is_loading = false;
        self.state = BrowserState::Idle;
        self.tls_conn = None;
        self.parse_styles_from_html();
        true
    }
}