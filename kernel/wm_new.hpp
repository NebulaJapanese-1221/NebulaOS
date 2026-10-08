// Window Manager for NebulaOS
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

#include <cstdint>
#include <cstddef>
#include "compositor.hpp"

namespace kernel::wm {

// Window types
enum class WindowType : std::uint8_t {
    NORMAL     = 0,
    DIALOG     = 1,
    UTILITY    = 2,
    DESKTOP    = 3,
    DOCK       = 4,
    MENU       = 5,
    TOOLTIP    = 6,
    SPLASH     = 7,
    NOTIFICATION = 8,
};

// Window states
enum class WindowState : std::uint8_t {
    NORMAL     = 0,
    MINIMIZED  = 1,
    MAXIMIZED  = 2,
    FULLSCREEN = 3,
    SHADED     = 4,  // Roll-up
    STICKY     = 5,  // Visible on all workspaces
};

// Window flags
enum WindowFlags : std::uint32_t {
    FLAG_NONE          = 0,
    FLAG_DECORATED     = 1 << 0,   // Has title bar and borders
    FLAG_RESIZABLE     = 1 << 1,   // Can be resized
    FLAG_MOVABLE       = 1 << 2,   // Can be moved
    FLAG_MINIMIZABLE   = 1 << 3,   // Can be minimized
    FLAG_MAXIMIZABLE   = 1 << 4,   // Can be maximized
    FLAG_CLOSABLE      = 1 << 5,   // Can be closed
    FLAG_FOCUSABLE     = 1 << 6,   // Can receive focus
    FLAG_MODAL         = 1 << 7,   // Modal dialog
    FLAG_SKIP_TASKBAR  = 1 << 8,   // Don't show in taskbar
    FLAG_SKIP_PAGER    = 1 << 9,   // Don't show in pager
    FLAG_ABOVE         = 1 << 10,  // Always on top
    FLAG_BELOW         = 1 << 11,  // Always on bottom
    FLAG_FULLSCREEN    = 1 << 12,  // Fullscreen
    FLAG_NO_INPUT      = 1 << 13,  // Ignore input
    FLAG_KEEP_ASPECT   = 1 << 14,  // Keep aspect ratio
};

// Window event types
enum class EventType : std::uint8_t {
    NONE           = 0,
    CREATE         = 1,
    DESTROY        = 2,
    SHOW           = 3,
    HIDE           = 4,
    MOVE           = 5,
    RESIZE         = 6,
    FOCUS          = 7,
    BLUR           = 8,
    MINIMIZE       = 9,
    MAXIMIZE       = 10,
    RESTORE        = 11,
    CLOSE_REQUEST  = 12,
    KEY_DOWN       = 13,
    KEY_UP         = 14,
    MOUSE_MOVE     = 15,
    MOUSE_DOWN     = 16,
    MOUSE_UP       = 17,
    MOUSE_WHEEL    = 18,
    PAINT          = 19,
    EXPOSURE       = 20,
};

// Window event
struct Event {
    EventType type;
    std::uint32_t window_id;
    std::uint32_t timestamp;

    union {
        struct {
            int x;
            int y;
        } move;

        struct {
            std::uint32_t width;
            std::uint32_t height;
        } resize;

        struct {
            int x;
            int y;
            std::uint32_t button;
        } mouse;

        struct {
            int dx;
            int dy;
            std::uint32_t button;
        } wheel;

        struct {
            std::uint32_t key_code;
            bool extended;
            char character;
        } key;
    };
};

// Window title bar buttons
enum class TitleButton : std::uint8_t {
    NONE     = 0,
    MINIMIZE = 1,
    MAXIMIZE = 2,
    CLOSE    = 3,
    SHADE    = 4,
    STICKY   = 5,
};

// Window structure
struct Window {
    std::uint32_t id;
    WindowType type;
    WindowState state;
    WindowFlags flags;

    // Position and size (client area)
    int x;
    int y;
    std::uint32_t width;
    std::uint32_t height;

    // Saved geometry (for restore)
    int saved_x;
    int saved_y;
    std::uint32_t saved_width;
    std::uint32_t saved_height;

    // Minimum and maximum sizes
    std::uint32_t min_width;
    std::uint32_t min_height;
    std::uint32_t max_width;
    std::uint32_t max_height;

    // Compositor surface
    compositor::Surface* surface;

    // Title
    char title[128];
    std::size_t title_length;

    // Window decorations
    bool decorated;
    std::uint32_t titlebar_height;
    std::uint32_t border_width;

    // Title bar buttons
    bool has_minimize;
    bool has_maximize;
    bool has_close;

    // Z-order
    int z_order;

    // Workspace (for multi-desktop)
    std::uint32_t workspace;

    // Parent window (for dialogs)
    Window* parent;

    // Transient windows (children)
    Window* children[8];
    std::size_t child_count;

    // User data
    void* user_data;

    // Event callback
    void (*event_callback)(Window* window, const Event& event);

    // Paint callback
    void (*paint_callback)(Window* window, int x, int y,
                               std::uint32_t width, std::uint32_t height);

    // Linked list
    Window* next;
    Window* prev;
};

// Window manager configuration
struct Config {
    // Colors
    std::uint32_t active_border_color;
    std::uint32_t inactive_border_color;
    std::uint32_t active_titlebar_color;
    std::uint32_t inactive_titlebar_color;
    std::uint32_t active_titlebar_text_color;
    std::uint32_t inactive_titlebar_text_color;
    std::uint32_t close_button_color;
    std::uint32_t maximize_button_color;
    std::uint32_t minimize_button_color;
    std::uint32_t button_text_color;
    std::uint32_t background_color;

    // Dimensions
    std::uint32_t border_width;
    std::uint32_t titlebar_height;
    std::uint32_t button_size;
    std::uint32_t resize_handle_size;
    std::uint32_t min_window_width;
    std::uint32_t min_window_height;

    // Behavior
    bool focus_follows_mouse;
    bool raise_on_focus;
    bool snap_to_edges;
    std::uint32_t snap_distance;
    bool double_click_maximize;
    std::uint32_t double_click_time;
};

// Window manager statistics
struct Stats {
    std::uint32_t total_windows;
    std::uint32_t visible_windows;
    std::uint32_t focused_window_id;
    std::uint32_t workspaces;
    std::uint32_t current_workspace;
};

// Initialize the window manager
bool initialize(const Config& config);

// Shutdown the window manager
void shutdown();

// Create a window
Window* create_window(WindowType type,
                           const char* title,
                           int x, int y,
                           std::uint32_t width, std::uint32_t height,
                           WindowFlags flags = FLAG_DECORATED |
                               FLAG_RESIZABLE | FLAG_MOVABLE |
                               FLAG_MINIMIZABLE | FLAG_MAXIMIZABLE |
                               FLAG_CLOSABLE | FLAG_FOCUSABLE);

// Destroy a window
void destroy_window(Window* window);

// Show a window
void show_window(Window* window);

// Hide a window
void hide_window(Window* window);

// Move a window
void move_window(Window* window, int x, int y);

// Resize a window
void resize_window(Window* window, std::uint32_t width, std::uint32_t height);

// Minimize a window
void minimize_window(Window* window);

// Maximize a window
void maximize_window(Window* window);

// Restore a window
void restore_window(Window* window);

// Close a window (requests close)
void close_window(Window* window);

// Set window title
void set_title(Window* window, const char* title);

// Set window state
void set_state(Window* window, WindowState state);

// Set window flags
void set_flags(Window* window, WindowFlags flags);

// Raise a window to the top
void raise_window(Window* window);

// Lower a window to the bottom
void lower_window(Window* window);

// Focus a window
void focus_window(Window* window);

// Get the focused window
Window* focused_window() noexcept;

// Get window at a position
Window* window_at(int x, int y);

// Get window by ID
Window* get_window(std::uint32_t id) noexcept;

// Get the desktop window
Window* desktop_window() noexcept;

// Set the event callback
void set_event_callback(Window* window,
                           void (*callback)(Window*, const Event&));

// Set the paint callback
void set_paint_callback(Window* window,
                           void (*callback)(Window*, int, int,
                                                std::uint32_t, std::uint32_t));

// Send an event to a window
void send_event(Window* window, const Event& event);

// Process input events
void process_mouse_move(int x, int y);
void process_mouse_button(int x, int y, std::uint32_t button, bool pressed);
void process_mouse_wheel(int x, int y, int dx, int dy, std::uint32_t button);
void process_key(std::uint32_t key_code, bool pressed, bool extended, char character);

// Window manager statistics
Stats stats() noexcept;

// Workspace management
void set_workspace(std::uint32_t workspace);
std::uint32_t current_workspace() noexcept;
std::uint32_t workspace_count() noexcept;

// Get list of windows
std::size_t window_list(Window** out, std::size_t max) noexcept;

// Update the window manager (render, etc.)
void update();

// Register a window class (for future use)
struct WindowClass {
    const char* name;
    void (*paint)(Window*);
    void (*event)(Window*, const Event&);
    void (*destroy)(Window*);
};

void register_class(const WindowClass& window_class);

} // namespace kernel::wm