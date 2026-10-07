// Kernel window manager for the NebulaOS x86 operating system.
// Manages window state, backing buffers, event queues, and compositing.
// Userspace applications interact with it through kernel::syscall.
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or (at your
// option) any later version.
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

namespace kernel::wm {

// Window types
enum class WindowType {
    NORMAL,
    DIALOG,
    UTILITY
};

// Window state
enum class WindowState {
    NORMAL,
    MINIMIZED,
    MAXIMIZED
};

// Event types delivered to userspace windows
enum class EventType {
    NONE = 0,
    PAINT = 1,
    CLOSE = 2,
    KEY_DOWN = 3,
    KEY_UP = 4,
    MOUSE_MOVE = 5,
    MOUSE_DOWN = 6,
    MOUSE_UP = 7,
    FOCUS_LOST = 8
};

struct WindowEvent {
    EventType type;
    unsigned int key_code;
    bool extended;
    int mouse_x;
    int mouse_y;
    unsigned int mouse_buttons;
};

struct WindowInfo {
    unsigned int id;
    int x;
    int y;
    unsigned int width;
    unsigned int height;
    unsigned int buffer;
    unsigned int buffer_width;
    unsigned int buffer_height;
    bool visible;
    bool focused;
    char title[64];
};

struct ScreenInfo {
    unsigned int width;
    unsigned int height;
    unsigned int desktop_left;
    unsigned int desktop_top;
    unsigned int desktop_width;
    unsigned int desktop_height;
    unsigned int taskbar_height;
};

void initialize(unsigned int taskbar_height);

// Window lifecycle
unsigned int create_window(const char* title, int x, int y, unsigned int width, unsigned int height);
void destroy_window(unsigned int window_id);
void show_window(unsigned int window_id);
void hide_window(unsigned int window_id);

// Window state
void set_title(unsigned int window_id, const char* title);
void move_window(unsigned int window_id, int x, int y);
void resize_window(unsigned int window_id, unsigned int width, unsigned int height);
void minimize_window(unsigned int window_id);
void maximize_window(unsigned int window_id);
void restore_window(unsigned int window_id);

// Drawing API (draws into the window's backing buffer)
void clear_window(unsigned int window_id, unsigned int color);
void fill_rect(unsigned int window_id, int x, int y, unsigned int w, unsigned int h, unsigned int color);
void draw_text(unsigned int window_id, int x, int y, const char* text, unsigned int color, unsigned int scale);
void present_window(unsigned int window_id);

// Buffer access
unsigned int get_buffer(unsigned int window_id);
unsigned int get_buffer_width(unsigned int window_id);
unsigned int get_buffer_height(unsigned int window_id);

// Event handling
bool poll_event(unsigned int window_id, WindowEvent& event);
bool wait_event(unsigned int window_id, WindowEvent& event, unsigned int timeout_ticks);

// Info
bool get_window_info(unsigned int window_id, WindowInfo& info);
void get_screen_info(ScreenInfo& info);

// Input injection from drivers
void inject_mouse(int x, int y, bool left_down, bool right_down, bool moved);
void inject_key(unsigned int code, bool pressed, bool extended, char character);

// Composite all visible windows onto the framebuffer
void composite();

// Update focused window based on mouse position
void update_focus(int x, int y);

} // namespace kernel::wm