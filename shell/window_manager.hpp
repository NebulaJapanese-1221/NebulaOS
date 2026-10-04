// Window manager for the NebulaOS x86 operating system.
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// See LICENCE for the full license text.

#pragma once

typedef unsigned int uint32_t;
typedef int int32_t;

namespace shell::wm {

// Window types
enum class WindowType {
    NORMAL,
    DIALOG,
    UTILITY,
    DESKTOP
};

// Window state
enum class WindowState {
    NORMAL,
    MINIMIZED,
    MAXIMIZED,
    FULLSCREEN
};

// Window structure
struct Window {
    uint32_t id;
    int32_t x;
    int32_t y;
    uint32_t width;
    uint32_t height;
    uint32_t min_width;
    uint32_t min_height;
    uint32_t max_width;
    uint32_t max_height;
    WindowType type;
    WindowState state;
    bool visible;
    bool focused;
    bool decorated;
    bool resizable;
    bool movable;
    uint32_t border_color;
    uint32_t titlebar_color;
    uint32_t titlebar_text_color;
    char title[64];
    void* user_data;
    
    // Window z-order (higher = on top)
    int32_t z_order;
    
    // Window flags
    bool close_requested;
};

// Window manager configuration
struct Config {
    uint32_t border_width;
    uint32_t titlebar_height;
    uint32_t resize_handle_size;
    uint32_t default_min_width;
    uint32_t default_min_height;
    uint32_t default_max_width;
    uint32_t default_max_height;
    uint32_t default_border_color;
    uint32_t default_titlebar_color;
    uint32_t default_titlebar_text_color;
    uint32_t focused_border_color;
    uint32_t focused_titlebar_color;
    uint32_t focused_titlebar_text_color;
};

// Initialize the window manager
void initialize();

// Create a new window
Window* create_window(const char* title, int32_t x, int32_t y, 
                      uint32_t width, uint32_t height, 
                      WindowType type = WindowType::NORMAL);

// Destroy a window
void destroy_window(Window* window);

// Get the focused window
Window* get_focused_window();

// Set window focus
void set_focus(Window* window);

// Move a window
void move_window(Window* window, int32_t x, int32_t y);

// Resize a window
void resize_window(Window* window, uint32_t width, uint32_t height);

// Minimize a window
void minimize_window(Window* window);

// Maximize a window
void maximize_window(Window* window);

// Restore a window
void restore_window(Window* window);

// Set window title
void set_window_title(Window* window, const char* title);

// Show/hide a window
void show_window(Window* window);
void hide_window(Window* window);

// Bring window to front
void bring_to_front(Window* window);

// Send window to back
void send_to_back(Window* window);

// Check if point is in window
bool hit_test(Window* window, int32_t x, int32_t y);

// Get window at position
Window* window_at(int32_t x, int32_t y);

// Process mouse events
void process_mouse_event(int32_t x, int32_t y, bool left_down, bool right_down, bool moved);

// Process keyboard events
void process_keyboard_event(uint32_t key, bool pressed, bool extended);

// Render all windows
void render();

// Get the window manager config
const Config& get_config();
void set_config(const Config& config);

// Get screen dimensions
uint32_t screen_width();
uint32_t screen_height();

// Desktop area (excluding taskbar)
uint32_t desktop_left();
uint32_t desktop_top();
uint32_t desktop_width();
uint32_t desktop_height();

} // namespace shell::wm