// Window Manager Implementation for NebulaOS
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

#include "wm.hpp"
#include "heap.hpp"
#include "../drivers/serial.hpp"
#include <cstring>

namespace kernel::wm {

namespace {

// Window list
Window* window_list = nullptr;
std::uint32_t window_count = 0;

// Focused window
Window* focused = nullptr;

// Desktop window
Window* desktop = nullptr;

// Configuration
Config config = {};

// Next window ID
std::uint32_t next_window_id = 1;

// Workspace
std::uint32_t current_workspace = 0;
constexpr std::uint32_t MAX_WORKSPACES = 16;

// Mouse state
int mouse_x = 0;
int mouse_y = 0;
bool mouse_buttons[8] = {};
Window* mouse_window = nullptr;
Window* drag_window = nullptr;
int drag_offset_x = 0;
int drag_offset_y = 0;
bool dragging = false;
Window* resize_window_ptr = nullptr;
int resize_start_x = 0;
int resize_start_y = 0;
std::uint32_t resize_start_width = 0;
std::uint32_t resize_start_height = 0;
bool resizing = false;

// Last click time for double-click detection
std::uint64_t last_click_time = 0;
Window* double_click_window = nullptr;

// Window classes
WindowClass classes[32];
std::size_t class_count = 0;

// Helper: get the title bar height for a window
std::uint32_t titlebar_height(const Window* window) {
    if (window == nullptr || !window->decorated) {
        return 0;
    }
    return config.titlebar_height;
}

// Helper: get the border width for a window
std::uint32_t border_width(const Window* window) {
    if (window == nullptr || !window->decorated) {
        return 0;
    }
    return config.border_width;
}

// Helper: check if a point is in the title bar
bool in_titlebar(const Window* window, int x, int y) {
    if (window == nullptr || !window->decorated) {
        return false;
    }
    const std::uint32_t bw = border_width(window);
    const std::uint32_t tbh = titlebar_height(window);
    return x >= window->x + static_cast<int>(bw) &&
           x < window->x + static_cast<int>(window->width + 2 * bw) &&
           y >= window->y + static_cast<int>(bw) &&
           y < window->y + static_cast<int>(bw + tbh);
}

// Helper: check if a point is in a title bar button
bool in_button(const Window* window, int x, int y,
                   TitleButton button) {
    if (window == nullptr || !window->decorated) {
        return false;
    }

    const std::uint32_t bw = border_width(window);
    const std::uint32_t tbh = titlebar_height(window);
    const std::uint32_t bs = config.button_size;

    // Buttons are in the top-right corner
    const int right = window->x + static_cast<int>(window->width + 2 * bw);
    const int top = window->y + static_cast<int>(bw);

    int button_x = 0;
    switch (button) {
        case TitleButton::CLOSE:
            button_x = right - static_cast<int>(bs) - 4;
            break;
        case TitleButton::MAXIMIZE:
            button_x = right - static_cast<int>(bs) * 2 - 8;
            break;
        case TitleButton::MINIMIZE:
            button_x = right - static_cast<int>(bs) * 3 - 12;
            break;
        default:
            return false;
    }

    return x >= button_x && x < button_x + static_cast<int>(bs) &&
           y >= top + 2 && y < top + 2 + static_cast<int>(bs);
}

// Helper: check if a point is in the resize handle
bool in_resize_handle(const Window* window, int x, int y) {
    if (window == nullptr || (window->flags & FLAG_RESIZABLE) == 0) {
        return false;
    }

    const std::uint32_t hs = config.resize_handle_size;
    const int right = window->x + static_cast<int>(window->width);
    const int bottom = window->y + static_cast<int>(window->height);

    return x >= right - static_cast<int>(hs) &&
           y >= bottom - static_cast<int>(hs);
}

// Helper: sort windows by z-order
void sort_windows() {
    if (window_list == nullptr) {
        return;
    }

    bool swapped = true;
    while (swapped) {
        swapped = false;
        Window* prev = nullptr;
        Window* current = window_list;

        while (current != nullptr && current->next != nullptr) {
            if (current->z_order > current->next->z_order) {
                Window* next = current->next;
                current->next = next->next;
                next->next = current;

                if (prev == nullptr) {
                    window_list = next;
                } else {
                    prev->next = next;
                }

                if (current->next != nullptr) {
                    current->next->prev = current;
                }
                next->prev = prev;
                current->prev = next;

                swapped = true;
            } else {
                prev = current;
                current = current->next;
            }
        }
    }
}

// Helper: draw window decorations
void draw_decorations(Window* window) {
    if (window == nullptr || !window->decorated) {
        return;
    }

    const bool is_focused = (window == focused);
    const std::uint32_t bw = border_width(window);
    const std::uint32_t tbh = titlebar_height(window);
    const std::uint32_t bs = config.button_size;

    const std::uint32_t border_color = is_focused
                                            ? config.active_border_color
                                            : config.inactive_border_color;
    const std::uint32_t titlebar_color = is_focused
                                              ? config.active_titlebar_color
                                              : config.inactive_titlebar_color;
    const std::uint32_t text_color = is_focused
                                          ? config.active_titlebar_text_color
                                          : config.inactive_titlebar_text_color;

    const int x = window->x;
    const int y = window->y;
    const int w = static_cast<int>(window->width + 2 * bw);
    const int h = static_cast<int>(window->height + 2 * bw);

    // Draw border
    compositor::fill_rect(x, y, w, static_cast<std::uint32_t>(bw), border_color);
    compositor::fill_rect(x, y + h - static_cast<int>(bw), w, static_cast<std::uint32_t>(bw), border_color);
    compositor::fill_rect(x, y, static_cast<std::uint32_t>(bw), h, border_color);
    compositor::fill_rect(x + w - static_cast<int>(bw), y, static_cast<std::uint32_t>(bw), h, border_color);

    // Draw title bar
    compositor::fill_rect(x + static_cast<int>(bw), y + static_cast<int>(bw),
                               w - 2 * static_cast<int>(bw), tbh, titlebar_color);

    // Draw title text
    // (In a real implementation, we'd use a font renderer)

    // Draw title bar buttons
    if (window->has_close) {
        const int close_x = x + w - static_cast<int>(bw) - static_cast<int>(bs) - 4;
        const int close_y = y + static_cast<int>(bw) + 2;
        compositor::fill_rect(close_x, close_y, bs, bs, config.close_button_color);
        // Draw X
        compositor::set_pixel(close_x + 4, close_y + 4, config.button_text_color);
        compositor::set_pixel(close_x + 5, close_y + 5, config.button_text_color);
        compositor::set_pixel(close_x + 6, close_y + 6, config.button_text_color);
        compositor::set_pixel(close_x + 7, close_y + 7, config.button_text_color);
        compositor::set_pixel(close_x + 8, close_y + 4, config.button_text_color);
        compositor::set_pixel(close_x + 9, close_y + 5, config.button_text_color);
        compositor::set_pixel(close_x + 10, close_y + 6, config.button_text_color);
        compositor::set_pixel(close_x + 11, close_y + 7, config.button_text_color);
        compositor::set_pixel(close_x + 12, close_y + 4, config.button_text_color);
        compositor::set_pixel(close_x + 4, close_y + 8, config.button_text_color);
        compositor::set_pixel(close_x + 5, close_y + 9, config.button_text_color);
        compositor::set_pixel(close_x + 6, close_y + 10, config.button_text_color);
        compositor::set_pixel(close_x + 7, close_y + 11, config.button_text_color);
        compositor::set_pixel(close_x + 8, close_y + 8, config.button_text_color);
        compositor::set_pixel(close_x + 9, close_y + 9, config.button_text_color);
        compositor::set_pixel(close_x + 10, close_y + 10, config.button_text_color);
        compositor::set_pixel(close_x + 11, close_y + 11, config.button_text_color);
        compositor::set_pixel(close_x + 12, close_y + 8, config.button_text_color);
    }

    if (window->has_maximize) {
        const int max_x = x + w - static_cast<int>(bw) - static_cast<int>(bs) * 2 - 8;
        const int max_y = y + static_cast<int>(bw) + 2;
        compositor::fill_rect(max_x, max_y, bs, bs, config.maximize_button_color);
        // Draw square
        compositor::set_pixel(max_x + 4, max_y + 4, config.button_text_color);
        compositor::set_pixel(max_x + 5, max_y + 4, config.button_text_color);
        compositor::set_pixel(max_x + 6, max_y + 4, config.button_text_color);
        compositor::set_pixel(max_x + 7, max_y + 4, config.button_text_color);
        compositor::set_pixel(max_x + 4, max_y + 5, config.button_text_color);
        compositor::set_pixel(max_x + 7, max_y + 5, config.button_text_color);
        compositor::set_pixel(max_x + 4, max_y + 6, config.button_text_color);
        compositor::set_pixel(max_x + 7, max_y + 6, config.button_text_color);
        compositor::set_pixel(max_x + 4, max_y + 7, config.button_text_color);
        compositor::set_pixel(max_x + 5, max_y + 7, config.button_text_color);
        compositor::set_pixel(max_x + 6, max_y + 7, config.button_text_color);
        compositor::set_pixel(max_x + 7, max_y + 7, config.button_text_color);
    }

    if (window->has_minimize) {
        const int min_x = x + w - static_cast<int>(bw) - static_cast<int>(bs) * 3 - 12;
        const int min_y = y + static_cast<int>(bw) + 2;
        compositor::fill_rect(min_x, min_y, bs, bs, config.minimize_button_color);
        // Draw underscore
        compositor::set_pixel(min_x + 4, min_y + 7, config.button_text_color);
        compositor::set_pixel(min_x + 5, min_y + 7, config.button_text_color);
        compositor::set_pixel(min_x + 6, min_y + 7, config.button_text_color);
        compositor::set_pixel(min_x + 7, min_y + 7, config.button_text_color);
        compositor::set_pixel(min_x + 8, min_y + 7, config.button_text_color);
        compositor::set_pixel(min_x + 9, min_y + 7, config.button_text_color);
        compositor::set_pixel(min_x + 10, min_y + 7, config.button_text_color);
        compositor::set_pixel(min_x + 11, min_y + 7, config.button_text_color);
    }

    // Draw resize handle
    if (window->flags & FLAG_RESIZABLE) {
        const std::uint32_t hs = config.resize_handle_size;
        const int hx = x + w - static_cast<int>(hs);
        const int hy = y + h - static_cast<int>(hs);
        compositor::fill_rect(hx, hy, hs, hs, 0x00444444);
        // Draw diagonal lines
        for (std::uint32_t i = 0; i < hs; i += 4) {
            compositor::set_pixel(hx + i, hy + hs - 2, 0x00888888);
            compositor::set_pixel(hx + i + 1, hy + hs - 3, 0x00888888);
            compositor::set_pixel(hx + i + 2, hy + hs - 4, 0x00888888);
        }
    }
}

// Helper: update the compositor surface for a window
void update_surface(Window* window) {
    if (window == nullptr || window->surface == nullptr) {
        return;
    }

    compositor::set_position(window->surface, window->x, window->y);
    compositor::set_size(window->surface, window->width, window->height);
    compositor::set_z_order(window->surface, window->z_order);
    compositor::damage_all(window->surface);
}

} // namespace

bool initialize(const Config& new_config) {
    config = new_config;

    window_list = nullptr;
    window_count = 0;
    focused = nullptr;
    desktop = nullptr;
    next_window_id = 1;
    current_workspace = 0;

    std::memset(mouse_buttons, 0, sizeof(mouse_buttons));
    mouse_window = nullptr;
    drag_window = nullptr;
    dragging = false;
    resize_window_ptr = nullptr;
    resizing = false;

    last_click_time = 0;
    double_click_window = nullptr;

    std::memset(classes, 0, sizeof(classes));
    class_count = 0;

    // Create the desktop window
    desktop = create_window(WindowType::DESKTOP,
                                 "Desktop",
                                 0, 0,
                                 config.screen_width,
                                 config.screen_height,
                                 FLAG_NONE);
    if (desktop == nullptr) {
        return false;
    }

    return true;
}

void shutdown() {
    // Destroy all windows
    Window* current = window_list;
    while (current != nullptr) {
        Window* next = current->next;
        destroy_window(current);
        current = next;
    }

    window_list = nullptr;
    window_count = 0;
    focused = nullptr;
    desktop = nullptr;
}

Window* create_window(WindowType type,
                           const char* title,
                           int x, int y,
                           std::uint32_t width, std::uint32_t height,
                           WindowFlags flags) {
    if (width == 0 || height == 0) {
        return nullptr;
    }

    // Allocate window structure
    Window* window = static_cast<Window*>(
        heap::allocate(sizeof(Window)));
    if (window == nullptr) {
        return nullptr;
    }

    std::memset(window, 0, sizeof(Window));

    // Create compositor surface
    compositor::SurfaceType surface_type = compositor::SurfaceType::WINDOW;
    std::uint32_t surface_flags = compositor::FLAG_VISIBLE |
                                       compositor::FLAG_INPUT |
                                       compositor::FLAG_FOCUSABLE;

    if ((flags & FLAG_DECORATED) != 0) {
        surface_flags |= compositor::FLAG_DECORATED;
    }

    window->surface = compositor::create_surface(
        surface_type, x, y, width, height, surface_flags);
    if (window->surface == nullptr) {
        heap::release(window);
        return nullptr;
    }

    // Initialize window
    window->id = next_window_id++;
    window->type = type;
    window->state = WindowState::NORMAL;
    window->flags = flags;

    window->x = x;
    window->y = y;
    window->width = width;
    window->height = height;

    window->saved_x = x;
    window->saved_y = y;
    window->saved_width = width;
    window->saved_height = height;

    window->min_width = config.min_window_width;
    window->min_height = config.min_window_height;
    window->max_width = 1920;
    window->max_height = 1080;

    window->decorated = (flags & FLAG_DECORATED) != 0;
    window->titlebar_height = config.titlebar_height;
    window->border_width = config.border_width;

    window->has_minimize = (flags & FLAG_MINIMIZABLE) != 0;
    window->has_maximize = (flags & FLAG_MAXIMIZABLE) != 0;
    window->has_close = (flags & FLAG_CLOSABLE) != 0;

    window->z_order = static_cast<int>(window_count);
    window->workspace = current_workspace;
    window->parent = nullptr;
    window->child_count = 0;
    window->user_data = nullptr;
    window->event_callback = nullptr;
    window->paint_callback = nullptr;

    // Copy title
    if (title != nullptr) {
        std::strncpy(window->title, title, sizeof(window->title) - 1);
        window->title[sizeof(window->title) - 1] = '\0';
        window->title_length = std::strlen(window->title);
    }

    // Set surface user data
    window->surface->user_data = window;

    // Set paint callback to draw decorations
    if (window->decorated) {
        compositor::set_paint_callback(window->surface,
            [](compositor::Surface* surface, int, int,
                  std::uint32_t, std::uint32_t) {
                Window* win = static_cast<Window*>(surface->user_data);
                draw_decorations(win);
            });
    }

    // Add to window list
    if (window_list == nullptr) {
        window_list = window;
    } else {
        Window* tail = window_list;
        while (tail->next != nullptr) {
            tail = tail->next;
        }
        tail->next = window;
        window->prev = tail;
    }

    ++window_count;
    ++stats_data.total_windows;

    return window;
}

void destroy_window(Window* window) {
    if (window == nullptr) {
        return;
    }

    // Remove from list
    if (window->prev != nullptr) {
        window->prev->next = window->next;
    } else {
        window_list = window->next;
    }
    if (window->next != nullptr) {
        window->next->prev = window->prev;
    }

    // Clear focus
    if (focused == window) {
        focused = nullptr;
    }

    // Clear mouse window
    if (mouse_window == window) {
        mouse_window = nullptr;
    }

    // Destroy compositor surface
    if (window->surface != nullptr) {
        compositor::destroy_surface(window->surface);
    }

    heap::release(window);
    --window_count;
}

void show_window(Window* window) {
    if (window == nullptr) {
        return;
    }
    if (window->surface != nullptr) {
        compositor::show(window->surface);
    }
}

void hide_window(Window* window) {
    if (window == nullptr) {
        return;
    }
    if (window->surface != nullptr) {
        compositor::hide(window->surface);
    }
}

void move_window(Window* window, int x, int y) {
    if (window == nullptr) {
        return;
    }

    // Snap to edges if enabled
    if (config.snap_to_edges) {
        if (x < static_cast<int>(config.snap_distance)) {
            x = 0;
        }
        if (y < static_cast<int>(config.snap_distance)) {
            y = 0;
        }
        if (x + static_cast<int>(window->width) >
            static_cast<int>(config.screen_width - config.snap_distance)) {
            x = static_cast<int>(config.screen_width - window->width);
        }
        if (y + static_cast<int>(window->height) >
            static_cast<int>(config.screen_height - config.snap_distance)) {
            y = static_cast<int>(config.screen_height - window->height);
        }
    }

    window->x = x;
    window->y = y;
    update_surface(window);
}

void resize_window(Window* window, std::uint32_t width, std::uint32_t height) {
    if (window == nullptr) {
        return;
    }

    // Enforce minimum size
    if (width < window->min_width) {
        width = window->min_width;
    }
    if (height < window->min_height) {
        height = window->min_height;
    }

    // Enforce maximum size
    if (width > window->max_width) {
        width = window->max_width;
    }
    if (height > window->max_height) {
        height = window->max_height;
    }

    window->width = width;
    window->height = height;
    update_surface(window);
}

void minimize_window(Window* window) {
    if (window == nullptr) {
        return;
    }

    // Save geometry
    window->saved_x = window->x;
    window->saved_y = window->y;
    window->saved_width = window->width;
    window->saved_height = window->height;

    window->state = WindowState::MINIMIZED;

    if (window->surface != nullptr) {
        compositor::hide(window->surface);
    }

    if (focused == window) {
        focused = nullptr;
    }

    // Send minimize event
    Event event;
    event.type = EventType::MINIMIZE;
    event.window_id = window->id;
    event.timestamp = 0;
    send_event(window, event);
}

void maximize_window(Window* window) {
    if (window == nullptr) {
        return;
    }

    // Save geometry
    window->saved_x = window->x;
    window->saved_y = window->y;
    window->saved_width = window->width;
    window->saved_height = window->height;

    window->state = WindowState::MAXIMIZED;
    window->x = 0;
    window->y = 0;
    window->width = config.screen_width;
    window->height = config.screen_height;

    update_surface(window);

    // Send maximize event
    Event event;
    event.type = EventType::MAXIMIZE;
    event.window_id = window->id;
    event.timestamp = 0;
    send_event(window, event);
}

void restore_window(Window* window) {
    if (window == nullptr) {
        return;
    }

    window->state = WindowState::NORMAL;
    window->x = window->saved_x;
    window->y = window->saved_y;
    window->width = window->saved_width;
    window->height = window->saved_height;

    update_surface(window);

    // Send restore event
    Event event;
    event.type = EventType::RESTORE;
    event.window_id = window->id;
    event.timestamp = 0;
    send_event(window, event);
}

void close_window(Window* window) {
    if (window == nullptr) {
        return;
    }

    // Send close request event
    Event event;
    event.type = EventType::CLOSE_REQUEST;
    event.window_id = window->id;
    event.timestamp = 0;
    send_event(window, event);
}

void set_title(Window* window, const char* title) {
    if (window == nullptr || title == nullptr) {
        return;
    }
    std::strncpy(window->title, title, sizeof(window->title) - 1);
    window->title[sizeof(window->title) - 1] = '\0';
    window->title_length = std::strlen(window->title);
}

void set_state(Window* window, WindowState state) {
    if (window == nullptr) {
        return;
    }

    switch (state) {
        case WindowState::NORMAL:
            restore_window(window);
            break;
        case WindowState::MINIMIZED:
            minimize_window(window);
            break;
        case WindowState::MAXIMIZED:
            maximize_window(window);
            break;
        case WindowState::FULLSCREEN:
            // Save geometry
            window->saved_x = window->x;
            window->saved_y = window->y;
            window->saved_width = window->width;
            window->saved_height = window->height;
            window->state = WindowState::FULLSCREEN;
            window->x = 0;
            window->y = 0;
            window->width = config.screen_width;
            window->height = config.screen_height;
            update_surface(window);
            break;
        default:
            break;
    }
}

void set_flags(Window* window, WindowFlags flags) {
    if (window == nullptr) {
        return;
    }
    window->flags = flags;
}

void raise_window(Window* window) {
    if (window == nullptr) {
        return;
    }

    // Find maximum z-order
    int max_z = window->z_order;
    for (Window* w = window_list; w != nullptr; w = w->next) {
        if (w->z_order > max_z) {
            max_z = w->z_order;
        }
    }

    window->z_order = max_z + 1;
    sort_windows();
    update_surface(window);
}

void lower_window(Window* window) {
    if (window == nullptr) {
        return;
    }

    // Find minimum z-order
    int min_z = window->z_order;
    for (Window* w = window_list; w != nullptr; w = w->next) {
        if (w->z_order < min_z) {
            min_z = w->z_order;
        }
    }

    window->z_order = min_z - 1;
    sort_windows();
    update_surface(window);
}

void focus_window(Window* window) {
    if (window == nullptr || window == focused) {
        return;
    }

    // Blur old window
    if (focused != nullptr) {
        Event blur_event;
        blur_event.type = EventType::BLUR;
        blur_event.window_id = focused->id;
        blur_event.timestamp = 0;
        send_event(focused, blur_event);
    }

    focused = window;

    // Raise window
    if (config.raise_on_focus) {
        raise_window(window);
    }

    // Send focus event
    Event focus_event;
    focus_event.type = EventType::FOCUS;
    focus_event.window_id = window->id;
    focus_event.timestamp = 0;
    send_event(window, focus_event);
}

Window* focused_window() noexcept {
    return focused;
}

Window* window_at(int x, int y) {
    // Search from top to bottom
    Window* found = nullptr;
    for (Window* window = window_list; window != nullptr; window = window->next) {
        if (window->surface == nullptr ||
            (window->surface->flags & compositor::FLAG_VISIBLE) == 0) {
            continue;
        }
        if ((window->flags & FLAG_NO_INPUT) != 0) {
            continue;
        }
        if (x >= window->x &&
            x < window->x + static_cast<int>(window->width) &&
            y >= window->y &&
            y < window->y + static_cast<int>(window->height)) {
            found = window;
        }
    }
    return found;
}

Window* get_window(std::uint32_t id) noexcept {
    for (Window* window = window_list; window != nullptr; window = window->next) {
        if (window->id == id) {
            return window;
        }
    }
    return nullptr;
}

Window* desktop_window() noexcept {
    return desktop;
}

void set_event_callback(Window* window,
                           void (*callback)(Window*, const Event&)) {
    if (window == nullptr) {
        return;
    }
    window->event_callback = callback;
}

void set_paint_callback(Window* window,
                           void (*callback)(Window*, int, int,
                                                std::uint32_t, std::uint32_t)) {
    if (window == nullptr) {
        return;
    }
    window->paint_callback = callback;
    if (window->surface != nullptr) {
        compositor::set_paint_callback(window->surface,
            [callback](compositor::Surface* surface, int x, int y,
                          std::uint32_t width, std::uint32_t height) {
                Window* win = static_cast<Window*>(surface->user_data);
                if (callback != nullptr) {
                    callback(win, x, y, width, height);
                }
            });
    }
}

void send_event(Window* window, const Event& event) {
    if (window == nullptr) {
        return;
    }

    if (window->event_callback != nullptr) {
        window->event_callback(window, event);
    }
}

void process_mouse_move(int x, int y) {
    mouse_x = x;
    mouse_y = y;

    // Find the window under the mouse
    Window* new_mouse_window = window_at(x, y);

    if (new_mouse_window != mouse_window) {
        mouse_window = new_mouse_window;

        if (config.focus_follows_mouse && new_mouse_window != nullptr) {
            focus_window(new_mouse_window);
        }
    }

    // Handle dragging
    if (dragging && drag_window != nullptr) {
        move_window(drag_window,
                        x - drag_offset_x,
                        y - drag_offset_y);
        return;
    }

    // Handle resizing
    if (resizing && resize_window_ptr != nullptr) {
        const int dx = x - resize_start_x;
        const int dy = y - resize_start_y;

        std::uint32_t new_width = resize_start_width;
        std::uint32_t new_height = resize_start_height;

        if (dx > 0) {
            new_width = resize_start_width + static_cast<std::uint32_t>(dx);
        } else if (dx < 0 && resize_start_width > static_cast<std::uint32_t>(-dx)) {
            new_width = resize_start_width + static_cast<std::uint32_t>(dx);
        }

        if (dy > 0) {
            new_height = resize_start_height + static_cast<std::uint32_t>(dy);
        } else if (dy < 0 && resize_start_height > static_cast<std::uint32_t>(-dy)) {
            new_height = resize_start_height + static_cast<std::uint32_t>(dy);
        }

        resize_window(resize_window_ptr, new_width, new_height);
        return;
    }

    // Send mouse move event
    if (mouse_window != nullptr) {
        Event event;
        event.type = EventType::MOUSE_MOVE;
        event.window_id = mouse_window->id;
        event.timestamp = 0;
        event.mouse.x = x - mouse_window->x;
        event.mouse.y = y - mouse_window->y;
        event.mouse.button = 0;
        send_event(mouse_window, event);
    }
}

void process_mouse_button(int x, int y, std::uint32_t button, bool pressed) {
    if (button >= sizeof(mouse_buttons) / sizeof(mouse_buttons[0])) {
        return;
    }

    mouse_buttons[button] = pressed;

    Window* target = window_at(x, y);

    if (pressed) {
        // Check for double-click
        const bool is_double_click =
            (target == double_click_window) &&
            (target != nullptr);

        if (target != nullptr) {
            focus_window(target);

            // Check title bar buttons
            if (target->decorated) {
                if (target->has_close &&
                    in_button(target, x, y, TitleButton::CLOSE)) {
                    close_window(target);
                    return;
                }
                if (target->has_maximize &&
                    in_button(target, x, y, TitleButton::MAXIMIZE)) {
                    if (target->state == WindowState::MAXIMIZED) {
                        restore_window(target);
                    } else {
                        maximize_window(target);
                    }
                    return;
                }
                if (target->has_minimize &&
                    in_button(target, x, y, TitleButton::MINIMIZE)) {
                    minimize_window(target);
                    return;
                }
            }

            // Check title bar for dragging
            if ((target->flags & FLAG_MOVABLE) != 0 &&
                in_titlebar(target, x, y)) {
                drag_window = target;
                drag_offset_x = x - target->x;
                drag_offset_y = y - target->y;
                dragging = true;
                return;
            }

            // Check resize handle
            if ((target->flags & FLAG_RESIZABLE) != 0 &&
                in_resize_handle(target, x, y)) {
                resize_window_ptr = target;
                resize_start_x = x;
                resize_start_y = y;
                resize_start_width = target->width;
                resize_start_height = target->height;
                resizing = true;
                return;
            }
        }
    } else {
        dragging = false;
        drag_window = nullptr;
        resizing = false;
        resize_window_ptr = nullptr;

        // Record for double-click detection
        if (button == 0) {  // Left button
            double_click_window = target;
        }
    }

    // Send mouse button event
    if (target != nullptr) {
        Event event;
        event.type = pressed ? EventType::MOUSE_DOWN : EventType::MOUSE_UP;
        event.window_id = target->id;
        event.timestamp = 0;
        event.mouse.x = x - target->x;
        event.mouse.y = y - target->y;
        event.mouse.button = button;
        send_event(target, event);
    }
}

void process_mouse_wheel(int x, int y, int dx, int dy, std::uint32_t button) {
    Window* target = window_at(x, y);
    if (target == nullptr) {
        return;
    }

    Event event;
    event.type = EventType::MOUSE_WHEEL;
    event.window_id = target->id;
    event.timestamp = 0;
    event.wheel.dx = dx;
    event.wheel.dy = dy;
    event.wheel.button = button;
    send_event(target, event);
}

void process_key(std::uint32_t key_code, bool pressed, bool extended, char character) {
    if (focused == nullptr) {
        return;
    }

    Event event;
    event.type = pressed ? EventType::KEY_DOWN : EventType::KEY_UP;
    event.window_id = focused->id;
    event.timestamp = 0;
    event.key.key_code = key_code;
    event.key.extended = extended;
    event.key.character = character;
    send_event(focused, event);
}

Stats stats() noexcept {
    Stats s = {};
    s.total_windows = window_count;
    s.focused_window_id = focused != nullptr ? focused->id : 0;
    s.workspaces = MAX_WORKSPACES;
    s.current_workspace = current_workspace;

    for (Window* window = window_list; window != nullptr; window = window->next) {
        if (window->surface != nullptr &&
            (window->surface->flags & compositor::FLAG_VISIBLE) != 0) {
            ++s.visible_windows;
        }
    }

    return s;
}

void set_workspace(std::uint32_t workspace) {
    if (workspace >= MAX_WORKSPACES) {
        return;
    }
    current_workspace = workspace;

    // Show/hide windows based on workspace
    for (Window* window = window_list; window != nullptr; window = window->next) {
        if (window->surface == nullptr) {
            continue;
        }
        if (window->workspace == current_workspace ||
            (window->state == WindowState::STICKY)) {
            compositor::show(window->surface);
        } else {
            compositor::hide(window->surface);
        }
    }
}

std::uint32_t current_workspace() noexcept {
    return current_workspace;
}

std::uint32_t workspace_count() noexcept {
    return MAX_WORKSPACES;
}

std::size_t window_list(Window** out, std::size_t max) noexcept {
    if (out == nullptr || max == 0) {
        return 0;
    }

    std::size_t count = 0;
    for (Window* window = window_list; window != nullptr && count < max; window = window->next) {
        out[count++] = window;
    }

    return count;
}

void update() {
    // Render all windows
    compositor::present();
}

void register_class(const WindowClass& window_class) {
    if (class_count >= sizeof(classes) / sizeof(classes[0])) {
        return;
    }
    classes[class_count++] = window_class;
}

} // namespace kernel::wm