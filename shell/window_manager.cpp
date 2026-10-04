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

#include "window_manager.hpp"
#include "apps/console.hpp"
#include "../drivers/graphics.hpp"
#include "../drivers/mouse.hpp"
#include "../kernel/heap.hpp"

namespace shell::wm {

namespace {
const uint32_t MAX_WINDOWS = 64;

Window windows[MAX_WINDOWS];
uint32_t window_count = 0;
uint32_t next_window_id = 1;
Window* focused_window = nullptr;
Window* grabbed_window = nullptr;
Window* resized_window = nullptr;

int32_t grab_offset_x = 0;
int32_t grab_offset_y = 0;
int32_t resize_start_x = 0;
int32_t resize_start_y = 0;
uint32_t resize_start_width = 0;
uint32_t resize_start_height = 0;
bool resizing_right = false;
bool resizing_bottom = false;
bool resizing_corner = false;

Config config = {
    2,          // border_width
    24,         // titlebar_height
    16,         // resize_handle_size
    160,        // default_min_width
    100,        // default_min_height
    1920,       // default_max_width
    1080,       // default_max_height
    0x00223A54, // default_border_color
    0x002B4C70, // default_titlebar_color
    0x00E8F1FA, // default_titlebar_text_color
    0x0038BDF8, // focused_border_color
    0x003D8FD8, // focused_titlebar_color
    0x00FFFFFF, // focused_titlebar_text_color
};

uint32_t screen_w = 0;
uint32_t screen_h = 0;
uint32_t desktop_left_pos = 0;
uint32_t desktop_top_pos = 0;
uint32_t desktop_w = 0;
uint32_t desktop_h = 0;

void clamp_window(Window* window) {
    if (window->x < static_cast<int32_t>(desktop_left_pos)) {
        window->x = desktop_left_pos;
    }
    if (window->y < static_cast<int32_t>(desktop_top_pos)) {
        window->y = desktop_top_pos;
    }
    if (window->x + static_cast<int32_t>(window->width) > 
        static_cast<int32_t>(desktop_left_pos + desktop_w)) {
        window->x = desktop_left_pos + desktop_w - window->width;
    }
    if (window->y + static_cast<int32_t>(window->height) > 
        static_cast<int32_t>(desktop_top_pos + desktop_h)) {
        window->y = desktop_top_pos + desktop_h - window->height;
    }
}

void update_window_geometry(Window* window) {
    if (window->width < window->min_width) window->width = window->min_width;
    if (window->height < window->min_height) window->height = window->min_height;
    if (window->width > window->max_width) window->width = window->max_width;
    if (window->height > window->max_height) window->height = window->max_height;
    clamp_window(window);
}

void draw_window_border(Window* window) {
    uint32_t border_color = window->focused ? config.focused_border_color : window->border_color;
    uint32_t titlebar_color = window->focused ? config.focused_titlebar_color : window->titlebar_color;
    uint32_t titlebar_text_color = window->focused ? config.focused_titlebar_text_color : window->titlebar_text_color;
    
    uint32_t x = window->x;
    uint32_t y = window->y;
    uint32_t w = window->width;
    uint32_t h = window->height;
    uint32_t bw = config.border_width;
    uint32_t tbh = config.titlebar_height;
    
    // Draw border
    drivers::graphics::fill_rect(x, y, w, bw, border_color);                                    // Top
    drivers::graphics::fill_rect(x, y + h - bw, w, bw, border_color);                          // Bottom
    drivers::graphics::fill_rect(x, y, bw, h, border_color);                                    // Left
    drivers::graphics::fill_rect(x + w - bw, y, bw, h, border_color);                          // Right
    
    if (window->decorated) {
        // Draw titlebar
        drivers::graphics::fill_rect(x + bw, y + bw, w - 2 * bw, tbh, titlebar_color);
        
        // Draw title text
        drivers::graphics::draw_text(x + bw + 8, y + bw + 4, window->title, titlebar_text_color, 1);
        
        // Draw close button (top-right)
        uint32_t close_x = x + w - bw - tbh - 4;
        uint32_t close_y = y + bw + 2;
        drivers::graphics::fill_rect(close_x, close_y, tbh - 4, tbh - 4, 0x00CC3333);
        drivers::graphics::draw_text(close_x + 4, close_y + 2, "X", 0x00FFFFFF, 1);
        
        // Draw maximize button
        uint32_t max_x = close_x - tbh;
        drivers::graphics::fill_rect(max_x, close_y, tbh - 4, tbh - 4, 0x003366CC);
        drivers::graphics::draw_text(max_x + 4, close_y + 2, "[ ]", 0x00FFFFFF, 1);
        
        // Draw minimize button
        uint32_t min_x = max_x - tbh;
        drivers::graphics::fill_rect(min_x, close_y, tbh - 4, tbh - 4, 0x0033CC33);
        drivers::graphics::draw_text(min_x + 4, close_y + 2, "_", 0x00FFFFFF, 1);
    }
    
    // Draw resize handle (bottom-right corner)
    if (window->resizable) {
        uint32_t handle_size = config.resize_handle_size;
        uint32_t handle_x = x + w - handle_size;
        uint32_t handle_y = y + h - handle_size;
        drivers::graphics::fill_rect(handle_x, handle_y, handle_size, handle_size, 0x00444444);
        // Draw diagonal lines for resize handle
        for (uint32_t i = 0; i < handle_size; i += 4) {
            drivers::graphics::draw_line(handle_x + i, handle_y + handle_size - 2, 
                                         handle_x + handle_size - 2, handle_y + i, 0x00888888);
        }
    }
}

bool hit_test_titlebar(Window* window, int32_t x, int32_t y) {
    if (!window->decorated) return false;
    uint32_t bw = config.border_width;
    uint32_t tbh = config.titlebar_height;
    return x >= window->x + bw && 
           x < static_cast<int32_t>(window->x + window->width - bw) &&
           y >= window->y + bw && 
           y < static_cast<int32_t>(window->y + bw + tbh);
}

bool hit_test_close(Window* window, int32_t x, int32_t y) {
    if (!window->decorated) return false;
    uint32_t bw = config.border_width;
    uint32_t tbh = config.titlebar_height;
    uint32_t close_x = window->x + window->width - bw - tbh - 4;
    uint32_t close_y = window->y + bw + 2;
    return x >= static_cast<int32_t>(close_x) && 
           x < static_cast<int32_t>(close_x + tbh - 4) &&
           y >= static_cast<int32_t>(close_y) && 
           y < static_cast<int32_t>(close_y + tbh - 4);
}

bool hit_test_maximize(Window* window, int32_t x, int32_t y) {
    if (!window->decorated) return false;
    uint32_t bw = config.border_width;
    uint32_t tbh = config.titlebar_height;
    uint32_t close_x = window->x + window->width - bw - tbh - 4;
    uint32_t max_x = close_x - tbh;
    uint32_t close_y = window->y + bw + 2;
    return x >= static_cast<int32_t>(max_x) && 
           x < static_cast<int32_t>(max_x + tbh - 4) &&
           y >= static_cast<int32_t>(close_y) && 
           y < static_cast<int32_t>(close_y + tbh - 4);
}

bool hit_test_minimize(Window* window, int32_t x, int32_t y) {
    if (!window->decorated) return false;
    uint32_t bw = config.border_width;
    uint32_t tbh = config.titlebar_height;
    uint32_t close_x = window->x + window->width - bw - tbh - 4;
    uint32_t max_x = close_x - tbh;
    uint32_t min_x = max_x - tbh;
    uint32_t close_y = window->y + bw + 2;
    return x >= static_cast<int32_t>(min_x) && 
           x < static_cast<int32_t>(min_x + tbh - 4) &&
           y >= static_cast<int32_t>(close_y) && 
           y < static_cast<int32_t>(close_y + tbh - 4);
}

bool hit_test_resize(Window* window, int32_t x, int32_t y) {
    if (!window->resizable) return false;
    uint32_t handle_size = config.resize_handle_size;
    uint32_t handle_x = window->x + window->width - handle_size;
    uint32_t handle_y = window->y + window->height - handle_size;
    return x >= static_cast<int32_t>(handle_x) && 
           x < static_cast<int32_t>(handle_x + handle_size) &&
           y >= static_cast<int32_t>(handle_y) && 
           y < static_cast<int32_t>(handle_y + handle_size);
}

} // anonymous namespace

void initialize() {
    screen_w = drivers::graphics::width();
    screen_h = drivers::graphics::height();
    
    // Default desktop area (full screen minus taskbar area)
    desktop_left_pos = 0;
    desktop_top_pos = 40; // taskbar height
    desktop_w = screen_w;
    desktop_h = screen_h > desktop_top_pos ? screen_h - desktop_top_pos : 0;
    
    // Initialize window array
    for (uint32_t i = 0; i < MAX_WINDOWS; ++i) {
        windows[i].id = 0;
        windows[i].visible = false;
    }
}

Window* create_window(const char* title, int32_t x, int32_t y, 
                      uint32_t width, uint32_t height, 
                      WindowType type) {
    if (window_count >= MAX_WINDOWS) {
        return nullptr;
    }
    
    // Find free slot
    Window* window = nullptr;
    for (uint32_t i = 0; i < MAX_WINDOWS; ++i) {
        if (windows[i].id == 0) {
            window = &windows[i];
            break;
        }
    }
    
    if (!window) return nullptr;
    
    // Initialize window
    window->id = next_window_id++;
    window->x = x;
    window->y = y;
    window->width = width;
    window->height = height;
    window->min_width = config.default_min_width;
    window->min_height = config.default_min_height;
    window->max_width = config.default_max_width;
    window->max_height = config.default_max_height;
    window->type = type;
    window->state = WindowState::NORMAL;
    window->visible = true;
    window->focused = false;
    window->decorated = true;
    window->resizable = true;
    window->movable = true;
    window->border_color = config.default_border_color;
    window->titlebar_color = config.default_titlebar_color;
    window->titlebar_text_color = config.default_titlebar_text_color;
    window->z_order = window_count;
    window->close_requested = false;
    window->user_data = nullptr;
    
    // Copy title
    for (uint32_t i = 0; i < 63 && title[i] != '\0'; ++i) {
        window->title[i] = title[i];
    }
    window->title[63] = '\0';
    
    update_window_geometry(window);
    
    // Bring to front
    window->z_order = window_count++;
    set_focus(window);
    
    return window;
}

void destroy_window(Window* window) {
    if (!window || window->id == 0) return;
    
    if (focused_window == window) {
        focused_window = nullptr;
    }
    if (grabbed_window == window) {
        grabbed_window = nullptr;
    }
    if (resized_window == window) {
        resized_window = nullptr;
    }
    
    window->id = 0;
    window->visible = false;
    window->focused = false;
    window_count = 0;
    for (uint32_t i = 0; i < MAX_WINDOWS; ++i) {
        if (windows[i].id != 0) window_count++;
    }
}

Window* get_focused_window() {
    return focused_window;
}

void set_focus(Window* window) {
    if (focused_window == window) return;
    
    if (focused_window) {
        focused_window->focused = false;
    }
    
    focused_window = window;
    
    if (window) {
        window->focused = true;
        bring_to_front(window);
    }
}

void move_window(Window* window, int32_t x, int32_t y) {
    if (!window) return;
    window->x = x;
    window->y = y;
    clamp_window(window);
}

void resize_window(Window* window, uint32_t width, uint32_t height) {
    if (!window) return;
    window->width = width;
    window->height = height;
    update_window_geometry(window);
}

void minimize_window(Window* window) {
    if (!window) return;
    window->state = WindowState::MINIMIZED;
    window->visible = false;
    if (focused_window == window) {
        set_focus(nullptr);
    }
}

void maximize_window(Window* window) {
    if (!window) return;
    window->state = WindowState::MAXIMIZED;
    window->x = desktop_left_pos;
    window->y = desktop_top_pos;
    window->width = desktop_w;
    window->height = desktop_h;
}

void restore_window(Window* window) {
    if (!window) return;
    window->state = WindowState::NORMAL;
    window->visible = true;
    // Note: We don't restore previous position/size here for simplicity
    // A full implementation would store the previous geometry
}

void set_window_title(Window* window, const char* title) {
    if (!window || !title) return;
    for (uint32_t i = 0; i < 63 && title[i] != '\0'; ++i) {
        window->title[i] = title[i];
    }
    window->title[63] = '\0';
}

void show_window(Window* window) {
    if (!window) return;
    window->visible = true;
}

void hide_window(Window* window) {
    if (!window) return;
    window->visible = false;
    if (focused_window == window) {
        set_focus(nullptr);
    }
}

void bring_to_front(Window* window) {
    if (!window) return;
    
    int32_t max_z = -1;
    for (uint32_t i = 0; i < MAX_WINDOWS; ++i) {
        if (windows[i].id != 0 && windows[i].z_order > max_z) {
            max_z = windows[i].z_order;
        }
    }
    window->z_order = max_z + 1;
}

void send_to_back(Window* window) {
    if (!window) return;
    
    int32_t min_z = 0x7FFFFFFF;
    for (uint32_t i = 0; i < MAX_WINDOWS; ++i) {
        if (windows[i].id != 0 && windows[i].z_order < min_z) {
            min_z = windows[i].z_order;
        }
    }
    window->z_order = min_z - 1;
}

bool hit_test(Window* window, int32_t x, int32_t y) {
    if (!window || !window->visible) return false;
    return x >= window->x && 
           x < static_cast<int32_t>(window->x + window->width) &&
           y >= window->y && 
           y < static_cast<int32_t>(window->y + window->height);
}

Window* window_at(int32_t x, int32_t y) {
    // Find topmost window at position
    Window* topmost = nullptr;
    int32_t max_z = -1;
    
    for (uint32_t i = 0; i < MAX_WINDOWS; ++i) {
        if (windows[i].id != 0 && windows[i].visible && hit_test(&windows[i], x, y)) {
            if (windows[i].z_order > max_z) {
                max_z = windows[i].z_order;
                topmost = &windows[i];
            }
        }
    }
    
    return topmost;
}

void process_mouse_event(int32_t x, int32_t y, bool left_down, bool right_down, bool moved) {
    (void)right_down; // unused for now
    
    Window* target = window_at(x, y);
    
    if (left_down) {
        if (target) {
            set_focus(target);
            
            if (hit_test_close(target, x, y)) {
                target->close_requested = true;
                return;
            }
            
            if (hit_test_maximize(target, x, y)) {
                if (target->state == WindowState::MAXIMIZED) {
                    restore_window(target);
                } else {
                    maximize_window(target);
                }
                return;
            }
            
            if (hit_test_minimize(target, x, y)) {
                minimize_window(target);
                return;
            }
            
            if (hit_test_resize(target, x, y)) {
                resized_window = target;
                resize_start_x = x;
                resize_start_y = y;
                resize_start_width = target->width;
                resize_start_height = target->height;
                resizing_right = true;
                resizing_bottom = true;
                resizing_corner = true;
                return;
            }
            
            if (hit_test_titlebar(target, x, y) && target->movable) {
                grabbed_window = target;
                grab_offset_x = x - target->x;
                grab_offset_y = y - target->y;
                return;
            }
        } else {
            set_focus(nullptr);
        }
    } else if (moved) {
        if (grabbed_window) {
            move_window(grabbed_window, x - grab_offset_x, y - grab_offset_y);
        } else if (resized_window) {
            int32_t dx = x - resize_start_x;
            int32_t dy = y - resize_start_y;
            
            uint32_t new_width = resize_start_width;
            uint32_t new_height = resize_start_height;
            
            if (resizing_right) {
                new_width = (dx > 0) ? resize_start_width + dx : 
                             (resize_start_width > static_cast<uint32_t>(-dx)) ? 
                             resize_start_width + dx : resized_window->min_width;
            }
            if (resizing_bottom) {
                new_height = (dy > 0) ? resize_start_height + dy : 
                              (resize_start_height > static_cast<uint32_t>(-dy)) ? 
                              resize_start_height + dy : resized_window->min_height;
            }
            
            resize_window(resized_window, new_width, new_height);
        }
    } else { // left_up
        grabbed_window = nullptr;
        resized_window = nullptr;
    }
}

void process_keyboard_event(uint32_t key, bool pressed, bool extended) {
    (void)key; (void)pressed; (void)extended;
    // TODO: Handle keyboard shortcuts (Alt+Tab, etc.)
}

void render() {
    // Sort windows by z-order
    Window* sorted[MAX_WINDOWS];
    uint32_t sorted_count = 0;
    
    for (uint32_t i = 0; i < MAX_WINDOWS; ++i) {
        if (windows[i].id != 0 && windows[i].visible) {
            sorted[sorted_count++] = &windows[i];
        }
    }
    
    // Simple bubble sort by z-order (small array, so it's fine)
    for (uint32_t i = 0; i < sorted_count; ++i) {
        for (uint32_t j = i + 1; j < sorted_count; ++j) {
            if (sorted[i]->z_order > sorted[j]->z_order) {
                Window* temp = sorted[i];
                sorted[i] = sorted[j];
                sorted[j] = temp;
            }
        }
    }
    
    // Render windows back to front
    for (uint32_t i = 0; i < sorted_count; ++i) {
        Window* window = sorted[i];
        
        // Draw window background
        uint32_t bw = config.border_width;
        uint32_t tbh = window->decorated ? config.titlebar_height : 0;
        
        drivers::graphics::fill_rect(
            window->x + bw, 
            window->y + bw + tbh, 
            window->width - 2 * bw, 
            window->height - 2 * bw - tbh,
            0x001A2A3A
        );
        
        // Draw window border and decorations
        draw_window_border(window);
        
        // TODO: Draw window contents (would need a callback or virtual function)
        // For now, just draw a placeholder
        if (window->type == WindowType::NORMAL) {
            char buf[32];
            buf[0] = 'W';
            buf[1] = 'I';
            buf[2] = 'N';
            buf[3] = 'D';
            buf[4] = 'O';
            buf[5] = 'W';
            buf[6] = ' ';
            buf[7] = '0' + (window->id / 100);
            buf[8] = '0' + ((window->id / 10) % 10);
            buf[9] = '0' + (window->id % 10);
            buf[10] = '\0';
            drivers::graphics::draw_text(window->x + bw + 8, window->y + bw + tbh + 8, buf, 0x00888888, 1);
        }
    }
}

const Config& get_config() {
    return config;
}

void set_config(const Config& new_config) {
    config = new_config;
}

uint32_t screen_width() {
    return screen_w;
}

uint32_t screen_height() {
    return screen_h;
}

uint32_t desktop_left() {
    return desktop_left_pos;
}

uint32_t desktop_top() {
    return desktop_top_pos;
}

uint32_t desktop_width() {
    return desktop_w;
}

uint32_t desktop_height() {
    return desktop_h;
}

} // namespace shell::wm