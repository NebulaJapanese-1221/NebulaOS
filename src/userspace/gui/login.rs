use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::draw_large_string;
use crate::gui::widgets::{Button, WidgetContainer};
use crate::rtc;
use crate::services::power;
use alloc::boxed::Box;

/// Login state tracking whether user is logged in
pub struct LoginState {
    pub logged_in: bool,
    widget_container: WidgetContainer,
    button_bounds: Rect,
    shutdown_bounds: Rect,
    restart_bounds: Rect,
    pub shutdown_requested: bool,
    pub restart_requested: bool,
}

impl LoginState {
    pub fn new() -> Self {
        let mut container = WidgetContainer::new();

        // Login button
        let mut login_btn = Button::new("Login");
        login_btn.enabled = true;
        container.add_widget(Box::new(login_btn));

        LoginState {
            logged_in: false,
            widget_container: container,
            button_bounds: Rect { x: 0, y: 0, width: 0, height: 0 },
            shutdown_bounds: Rect { x: 0, y: 0, width: 0, height: 0 },
            restart_bounds: Rect { x: 0, y: 0, width: 0, height: 0 },
            shutdown_requested: false,
            restart_requested: false,
        }
    }

    /// Draw the full-screen login interface
    pub fn draw(&mut self, fb: &mut Framebuffer) {
        // Dark gradient-like background
        fb.draw_rect(0, 0, fb.width, fb.height, 0x00000000);

        // Draw a subtle border glow
        let border_color = 0x00113355;
        fb.draw_rect(0, 0, fb.width, 2, border_color);
        fb.draw_rect(0, fb.height - 2, fb.width, 2, border_color);
        fb.draw_rect(0, 0, 2, fb.height, border_color);
        fb.draw_rect(fb.width - 2, 0, 2, fb.height, border_color);

        // Center coordinates
        let center_x = fb.width / 2;
        let center_y = fb.height / 2;

        // Draw "NebulaOS" title
        let title_scale: usize = 6;
        let title_width = 8 * 8 * title_scale;
        let title_x = center_x.saturating_sub(title_width / 2);
        let title_y = center_y.saturating_sub(120);
        draw_large_string(fb, title_x, title_y, "NebulaOS", 0x0078D7FF, title_scale);

        // Draw subtitle
        let subtitle_scale: usize = 3;
        let subtitle_width = 5 * 8 * subtitle_scale;
        let subtitle_x = center_x.saturating_sub(subtitle_width / 2);
        let subtitle_y = center_y.saturating_sub(40);
        draw_large_string(fb, subtitle_x, subtitle_y, "v0.0.1", 0x00AAAAAA, subtitle_scale);

        // Draw clock at the bottom-right area
        let time = rtc::get_time();
        let clock_scale: usize = 3;
        let time_str = alloc::format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second);
        let clock_str_width = time_str.len() * 8 * clock_scale;
        let clock_x = (fb.width as usize).saturating_sub(clock_str_width).saturating_sub(30);
        let clock_y = (fb.height as usize).saturating_sub(80);
        draw_large_string(fb, clock_x, clock_y, &time_str, 0x00CCCCCC, clock_scale);

        // Draw date below clock
        let date_scale: usize = 2;
        let _date_str = alloc::format!("{}/{}/{}", time.day, time.month, time.year);
        let date_str_width = 17 * 8 * date_scale;
        let date_x = (fb.width as usize).saturating_sub(date_str_width).saturating_sub(30);
        let date_y = clock_y.saturating_add(30);
        draw_large_string(fb, date_x, date_y, "NebulaOS  Desktop", 0x00666666, date_scale);

        // --- Draw Login button using Widget system ---
        let btn_width = 160u32;
        let btn_height = 40u32;
        let btn_x = (center_x as u32).saturating_sub(btn_width / 2);
        let btn_y = (center_y as u32).saturating_add(40);

        self.button_bounds = Rect {
            x: btn_x,
            y: btn_y,
            width: btn_width,
            height: btn_height,
        };

        // Draw the button using the existing Button widget
        if let Some(button) = self.widget_container.widgets.get_mut(0) {
            button.draw(fb, self.button_bounds, true);
        }

        // --- Draw Shutdown button below Login ---
        let shutdown_btn_width = 160u32;
        let shutdown_btn_height = 40u32;
        let shutdown_btn_x = (center_x as u32).saturating_sub(shutdown_btn_width / 2);
        let shutdown_btn_y = btn_y + btn_height + 10;

        self.shutdown_bounds = Rect {
            x: shutdown_btn_x,
            y: shutdown_btn_y,
            width: shutdown_btn_width,
            height: shutdown_btn_height,
        };

        // Draw shutdown button manually (red-tinted)
        let shutdown_bg = 0x00552222;
        fb.draw_rect(
            self.shutdown_bounds.x as usize,
            self.shutdown_bounds.y as usize,
            self.shutdown_bounds.width as usize,
            self.shutdown_bounds.height as usize,
            shutdown_bg,
        );
        // Draw border
        fb.draw_rect(self.shutdown_bounds.x as usize, self.shutdown_bounds.y as usize, self.shutdown_bounds.width as usize, 1, 0x00FF4444);
        fb.draw_rect(self.shutdown_bounds.x as usize, (self.shutdown_bounds.y + self.shutdown_bounds.height - 1) as usize, self.shutdown_bounds.width as usize, 1, 0x00FF4444);
        fb.draw_rect(self.shutdown_bounds.x as usize, self.shutdown_bounds.y as usize, 1, self.shutdown_bounds.height as usize, 0x00FF4444);
        fb.draw_rect((self.shutdown_bounds.x + self.shutdown_bounds.width - 1) as usize, self.shutdown_bounds.y as usize, 1, self.shutdown_bounds.height as usize, 0x00FF4444);
        // Draw text
        let shutdown_text_x = self.shutdown_bounds.x as usize + ((self.shutdown_bounds.width as usize) - (7 * 8)) / 2;
        let shutdown_text_y = self.shutdown_bounds.y as usize + ((self.shutdown_bounds.height as usize) - 8) / 2;
        crate::gui::draw_string(fb, shutdown_text_x, shutdown_text_y, "Shutdown", 0x00FF8888);

        // --- Draw Restart button below Shutdown ---
        let restart_btn_width = 160u32;
        let restart_btn_height = 40u32;
        let restart_btn_x = (center_x as u32).saturating_sub(restart_btn_width / 2);
        let restart_btn_y = shutdown_btn_y + shutdown_btn_height + 10;

        self.restart_bounds = Rect {
            x: restart_btn_x,
            y: restart_btn_y,
            width: restart_btn_width,
            height: restart_btn_height,
        };

        // Draw restart button manually (orange-tinted)
        let restart_bg = 0x00554422;
        fb.draw_rect(
            self.restart_bounds.x as usize,
            self.restart_bounds.y as usize,
            self.restart_bounds.width as usize,
            self.restart_bounds.height as usize,
            restart_bg,
        );
        // Draw border
        fb.draw_rect(self.restart_bounds.x as usize, self.restart_bounds.y as usize, self.restart_bounds.width as usize, 1, 0x00FFAA44);
        fb.draw_rect(self.restart_bounds.x as usize, (self.restart_bounds.y + self.restart_bounds.height - 1) as usize, self.restart_bounds.width as usize, 1, 0x00FFAA44);
        fb.draw_rect(self.restart_bounds.x as usize, self.restart_bounds.y as usize, 1, self.restart_bounds.height as usize, 0x00FFAA44);
        fb.draw_rect((self.restart_bounds.x + self.restart_bounds.width - 1) as usize, self.restart_bounds.y as usize, 1, self.restart_bounds.height as usize, 0x00FFAA44);
        // Draw text
        let restart_text_x = self.restart_bounds.x as usize + ((self.restart_bounds.width as usize) - (7 * 8)) / 2;
        let restart_text_y = self.restart_bounds.y as usize + ((self.restart_bounds.height as usize) - 8) / 2;
        crate::gui::draw_string(fb, restart_text_x, restart_text_y, "Restart", 0x00FFBB66);

        // Draw a hint text below the last button
        let hint_scale: usize = 2;
        let hint_width = 19 * 8 * hint_scale;
        let hint_x = center_x.saturating_sub(hint_width / 2);
        let hint_y = (restart_btn_y as usize).saturating_add(restart_btn_height as usize).saturating_add(30);
        draw_large_string(fb, hint_x, hint_y, "Click Login to continue", 0x00666666, hint_scale);

        fb.present();
    }

    /// Handle mouse click on the login screen
    pub fn handle_click(&mut self, mx: i32, my: i32) -> bool {
        // Check if Login button is clicked
        let bx = self.button_bounds.x as i32;
        let by = self.button_bounds.y as i32;
        let bw = self.button_bounds.width as i32;
        let bh = self.button_bounds.height as i32;

        if mx >= bx && mx < bx + bw && my >= by && my < by + bh {
            self.logged_in = true;
            return true;
        }

        // Check if Shutdown button is clicked
        let sx = self.shutdown_bounds.x as i32;
        let sy = self.shutdown_bounds.y as i32;
        let sw = self.shutdown_bounds.width as i32;
        let sh = self.shutdown_bounds.height as i32;

        if mx >= sx && mx < sx + sw && my >= sy && my < sy + sh {
            let power_svc = power::get_power_service();
            power_svc.lock().request_shutdown();
            self.shutdown_requested = true;
            return true;
        }

        // Check if Restart button is clicked
        let rx = self.restart_bounds.x as i32;
        let ry = self.restart_bounds.y as i32;
        let rw = self.restart_bounds.width as i32;
        let rh = self.restart_bounds.height as i32;

        if mx >= rx && mx < rx + rw && my >= ry && my < ry + rh {
            let power_svc = power::get_power_service();
            power_svc.lock().request_restart();
            self.restart_requested = true;
            return true;
        }

        false
    }
}

