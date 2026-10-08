// Compositor for NebulaOS
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

namespace kernel::compositor {

// Surface types
enum class SurfaceType : std::uint8_t {
    NONE       = 0,
    WINDOW     = 1,
    CURSOR     = 2,
    PANEL      = 3,
    POPUP      = 4,
    NOTIFICATION = 5,
    BACKGROUND = 6,
};

// Surface flags
enum SurfaceFlags : std::uint32_t {
    FLAG_NONE        = 0,
    FLAG_OPAQUE      = 1 << 0,   // Opaque (no alpha blending)
    FLAG_VISIBLE     = 1 << 1,   // Visible
    FLAG_DIRTY       = 1 << 2,   // Needs redraw
    FLAG_INPUT       = 1 << 3,   // Accepts input
    FLAG_FOCUSABLE   = 1 << 4,   // Can receive focus
    FLAG_DECORATED   = 1 << 5,   // Has window decorations
    FLAG_MOVABLE     = 1 << 6,   // Can be moved
    FLAG_RESIZABLE   = 1 << 7,   // Can be resized
    FLAG_MINIMIZABLE = 1 << 8,   // Can be minimized
    FLAG_MAXIMIZABLE = 1 << 9,   // Can be maximized
    FLAG_CLOSABLE    = 1 << 10,  // Can be closed
    FLAG_ALWAYS_ON_TOP = 1 << 11, // Always on top
    FLAG_FULLSCREEN  = 1 << 12,  // Fullscreen
    FLAG_ALPHA       = 1 << 13,  // Has alpha channel
    FLAG_SHADOW      = 1 << 14,  // Has drop shadow
    FLAG_ROUNDED     = 1 << 15,  // Rounded corners
};

// Surface structure (a drawable region)
struct Surface {
    std::uint32_t id;
    SurfaceType type;
    std::uint32_t flags;

    // Position and size
    int x;
    int y;
    std::uint32_t width;
    std::uint32_t height;

    // Backing buffer
    std::uint32_t* buffer;
    std::size_t buffer_size;
    std::uint32_t buffer_width;
    std::uint32_t buffer_height;

    // Z-order (higher = on top)
    int z_order;

    // Alpha (0-255)
    std::uint8_t alpha;

    // Damage region (for partial updates)
    int damage_x;
    int damage_y;
    std::uint32_t damage_width;
    std::uint32_t damage_height;
    bool has_damage;

    // Parent surface (for window hierarchies)
    Surface* parent;

    // Linked list
    Surface* next;
    Surface* prev;

    // User data
    void* user_data;

    // Surface operations (virtual function table style)
    void (*paint)(Surface* surface, int x, int y, int width, int height);
    void (*close)(Surface* surface);
    void (*focus)(Surface* surface);
    void (*blur)(Surface* surface);
    void (*resize)(Surface* surface, std::uint32_t width, std::uint32_t height);
    void (*move)(Surface* surface, int x, int y);
};

// Compositor configuration
struct Config {
    std::uint32_t screen_width;
    std::uint32_t screen_height;
    std::uint32_t screen_bpp;

    // Background color
    std::uint32_t background_color;

    // Compositor features
    bool double_buffering;
    bool vsync;
    bool damage_tracking;
    bool alpha_blending;
    bool shadows;

    // Cursor
    std::uint32_t cursor_x;
    std::uint32_t cursor_y;
    bool cursor_visible;
    Surface* cursor_surface;
};

// Compositor statistics
struct Stats {
    std::uint64_t frames_rendered;
    std::uint64_t total_render_time;
    std::uint32_t surfaces;
    std::uint32_t visible_surfaces;
    std::uint32_t dirty_surfaces;
    std::uint32_t fps;
};

// Initialize the compositor
bool initialize(const Config& config);

// Shutdown the compositor
void shutdown();

// Create a new surface
Surface* create_surface(SurfaceType type,
                           int x, int y,
                           std::uint32_t width, std::uint32_t height,
                           std::uint32_t flags = FLAG_VISIBLE | FLAG_INPUT | FLAG_FOCUSABLE);

// Destroy a surface
void destroy_surface(Surface* surface);

// Set surface position
void set_position(Surface* surface, int x, int y);

// Set surface size
void set_size(Surface* surface, std::uint32_t width, std::uint32_t height);

// Set surface flags
void set_flags(Surface* surface, std::uint32_t flags);

// Set surface alpha
void set_alpha(Surface* surface, std::uint8_t alpha);

// Set surface z-order
void set_z_order(Surface* surface, int z_order);

// Raise surface to top
void raise(Surface* surface);

// Lower surface to bottom
void lower(Surface* surface);

// Show a surface
void show(Surface* surface);

// Hide a surface
void hide(Surface* surface);

// Mark a region of a surface as damaged (needs redraw)
void damage(Surface* surface, int x, int y,
               std::uint32_t width, std::uint32_t height);

// Mark entire surface as damaged
void damage_all(Surface* surface);

// Present the composited frame to the screen
void present();

// Render all surfaces to the screen
void render();

// Get surface at a position
Surface* surface_at(int x, int y);

// Get focused surface
Surface* focused_surface() noexcept;

// Set focused surface
void set_focus(Surface* surface);

// Get the screen buffer
std::uint32_t* screen_buffer() noexcept;

// Get screen dimensions
std::uint32_t screen_width() noexcept;
std::uint32_t screen_height() noexcept;

// Set cursor position
void set_cursor(int x, int y);

// Get cursor position
void cursor_position(int* x, int* y) noexcept;

// Show/hide cursor
void show_cursor(bool visible);

// Check if cursor is visible
bool cursor_visible() noexcept;

// Compositor statistics
Stats stats() noexcept;

// Register a paint callback
using PaintCallback = void (*)(Surface*, int, int, int, int);
void set_paint_callback(Surface* surface, PaintCallback callback);

// Blit a surface to the screen (internal)
void blit(Surface* surface, int dx, int dy,
             int sx, int sy,
             std::uint32_t width, std::uint32_t height);

// Fill a rectangle on the screen
void fill_rect(int x, int y, std::uint32_t width, std::uint32_t height,
                  std::uint32_t color);

// Draw a pixel
void set_pixel(int x, int y, std::uint32_t color);

// Get a pixel
std::uint32_t get_pixel(int x, int y) noexcept;

// Alpha blend two colors
std::uint32_t alpha_blend(std::uint32_t src, std::uint32_t dst,
                              std::uint8_t alpha) noexcept;

} // namespace kernel::compositor