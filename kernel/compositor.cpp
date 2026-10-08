// Compositor Implementation for NebulaOS
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

#include "compositor.hpp"
#include "heap_new.hpp"
#include "pmm_new.hpp"
#include "../drivers/serial.hpp"
#include "../framebuffer.hpp"
#include <cstring>

namespace kernel::compositor {

namespace {

// Screen buffer (back buffer)
std::uint32_t* screen_buffer_ptr = nullptr;
std::size_t screen_buffer_size = 0;

// Configuration
Config config = {};

// Surface list (sorted by z-order)
Surface* surface_list = nullptr;
Surface* surface_tail = nullptr;
std::uint32_t surface_count = 0;

// Focused surface
Surface* focused = nullptr;

// Cursor
int cursor_x = 0;
int cursor_y = 0;
bool cursor_visible_flag = true;
Surface* cursor_surface = nullptr;

// Statistics
Stats stats_data = {};

// Next surface ID
std::uint32_t next_surface_id = 1;

// Vsync counter
std::uint64_t frame_count = 0;

// Helper: clamp a value
inline int clamp(int value, int min, int max) {
    if (value < min) return min;
    if (value > max) return max;
    return value;
}

// Helper: check if a rectangle intersects the screen
bool intersects_screen(int x, int y, std::uint32_t width, std::uint32_t height) {
    if (x < 0) {
        width = static_cast<std::uint32_t>(
            static_cast<int>(width) + x);
        x = 0;
    }
    if (y < 0) {
        height = static_cast<std::uint32_t>(
            static_cast<int>(height) + y);
        y = 0;
    }
    if (x + static_cast<int>(width) > static_cast<int>(config.screen_width)) {
        width = config.screen_width - x;
    }
    if (y + static_cast<int>(height) > static_cast<int>(config.screen_height)) {
        height = config.screen_height - y;
    }
    return width > 0 && height > 0;
}

// Helper: sort surfaces by z-order (simple insertion sort)
void sort_surfaces() {
    if (surface_list == nullptr) {
        return;
    }

    // Bubble sort (fine for small lists)
    bool swapped = true;
    while (swapped) {
        swapped = false;
        Surface* prev = nullptr;
        Surface* current = surface_list;

        while (current != nullptr && current->next != nullptr) {
            if (current->z_order > current->next->z_order) {
                // Swap
                Surface* next = current->next;
                current->next = next->next;
                next->next = current;

                if (prev == nullptr) {
                    surface_list = next;
                } else {
                    prev->next = next;
                }

                // Fix prev pointers
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

    // Update tail
    surface_tail = surface_list;
    while (surface_tail != nullptr && surface_tail->next != nullptr) {
        surface_tail = surface_tail->next;
    }
}

} // namespace

bool initialize(const Config& new_config) {
    config = new_config;

    // Allocate the screen buffer
    const std::size_t buffer_bytes =
        static_cast<std::size_t>(config.screen_width) *
        config.screen_height * 4;
    screen_buffer_size = buffer_bytes;

    // Allocate physical frames for the screen buffer
    const std::size_t pages = (buffer_bytes + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
    const auto frame = pmm::allocate_frames(pages, pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        return false;
    }

    screen_buffer_ptr = reinterpret_cast<std::uint32_t*>(frame.value);

    // Initialize the screen buffer with the background color
    fill_rect(0, 0, config.screen_width, config.screen_height,
                 config.background_color);

    surface_list = nullptr;
    surface_tail = nullptr;
    surface_count = 0;
    focused = nullptr;
    cursor_x = 0;
    cursor_y = 0;
    cursor_visible_flag = true;
    cursor_surface = nullptr;

    stats_data = {};
    next_surface_id = 1;
    frame_count = 0;

    return true;
}

void shutdown() {
    // Destroy all surfaces
    Surface* current = surface_list;
    while (current != nullptr) {
        Surface* next = current->next;
        if (current->buffer != nullptr) {
            // Free the buffer frames
            const std::size_t buffer_bytes =
                current->buffer_width * current->buffer_height * 4;
            const std::size_t pages =
                (buffer_bytes + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
            pmm::free_frames(
                reinterpret_cast<std::uintptr_t>(current->buffer), pages);
        }
        heap::release(current);
        current = next;
    }

    // Free the screen buffer
    if (screen_buffer_ptr != nullptr) {
        pmm::free_frames(
            reinterpret_cast<std::uintptr_t>(screen_buffer_ptr),
            screen_buffer_size / pmm::PAGE_SIZE);
        screen_buffer_ptr = nullptr;
    }

    surface_list = nullptr;
    surface_tail = nullptr;
    surface_count = 0;
    focused = nullptr;
}

Surface* create_surface(SurfaceType type,
                           int x, int y,
                           std::uint32_t width, std::uint32_t height,
                           std::uint32_t flags) {
    if (width == 0 || height == 0) {
        return nullptr;
    }

    // Allocate surface structure
    Surface* surface = static_cast<Surface*>(
        heap::allocate(sizeof(Surface)));
    if (surface == nullptr) {
        return nullptr;
    }

    std::memset(surface, 0, sizeof(Surface));

    // Allocate backing buffer
    const std::size_t buffer_bytes =
        static_cast<std::size_t>(width) * height * 4;
    const std::size_t pages =
        (buffer_bytes + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;

    const auto frame = pmm::allocate_frames(pages, pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        heap::release(surface);
        return nullptr;
    }

    surface->id = next_surface_id++;
    surface->type = type;
    surface->flags = flags;
    surface->x = x;
    surface->y = y;
    surface->width = width;
    surface->height = height;
    surface->buffer = reinterpret_cast<std::uint32_t*>(frame.value);
    surface->buffer_size = buffer_bytes;
    surface->buffer_width = width;
    surface->buffer_height = height;
    surface->z_order = static_cast<int>(surface_count);
    surface->alpha = 255;
    surface->has_damage = true;
    surface->damage_x = 0;
    surface->damage_y = 0;
    surface->damage_width = width;
    surface->damage_height = height;
    surface->parent = nullptr;
    surface->next = nullptr;
    surface->prev = nullptr;
    surface->user_data = nullptr;
    surface->paint = nullptr;
    surface->close = nullptr;
    surface->focus = nullptr;
    surface->blur = nullptr;
    surface->resize = nullptr;
    surface->move = nullptr;

    // Add to surface list
    if (surface_list == nullptr) {
        surface_list = surface;
        surface_tail = surface;
    } else {
        surface_tail->next = surface;
        surface->prev = surface_tail;
        surface_tail = surface;
    }

    ++surface_count;
    ++stats_data.surfaces;

    return surface;
}

void destroy_surface(Surface* surface) {
    if (surface == nullptr) {
        return;
    }

    // Remove from list
    if (surface->prev != nullptr) {
        surface->prev->next = surface->next;
    } else {
        surface_list = surface->next;
    }
    if (surface->next != nullptr) {
        surface->next->prev = surface->prev;
    } else {
        surface_tail = surface->prev;
    }

    // If focused, clear focus
    if (focused == surface) {
        focused = nullptr;
    }

    // Free backing buffer
    if (surface->buffer != nullptr) {
        const std::size_t pages =
            (surface->buffer_size + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
        pmm::free_frames(
            reinterpret_cast<std::uintptr_t>(surface->buffer), pages);
    }

    heap::release(surface);
    --surface_count;
}

void set_position(Surface* surface, int x, int y) {
    if (surface == nullptr) {
        return;
    }
    surface->x = x;
    surface->y = y;
    surface->has_damage = true;
}

void set_size(Surface* surface, std::uint32_t width, std::uint32_t height) {
    if (surface == nullptr || width == 0 || height == 0) {
        return;
    }

    // For simplicity, just update the size
    // In a real implementation, we'd reallocate the buffer
    surface->width = width;
    surface->height = height;
    surface->buffer_width = width;
    surface->buffer_height = height;
    surface->has_damage = true;
}

void set_flags(Surface* surface, std::uint32_t flags) {
    if (surface == nullptr) {
        return;
    }
    surface->flags = flags;
}

void set_alpha(Surface* surface, std::uint8_t alpha) {
    if (surface == nullptr) {
        return;
    }
    surface->alpha = alpha;
}

void set_z_order(Surface* surface, int z_order) {
    if (surface == nullptr) {
        return;
    }
    surface->z_order = z_order;
    sort_surfaces();
}

void raise(Surface* surface) {
    if (surface == nullptr) {
        return;
    }
    // Find the maximum z-order
    int max_z = surface->z_order;
    for (Surface* s = surface_list; s != nullptr; s = s->next) {
        if (s->z_order > max_z) {
            max_z = s->z_order;
        }
    }
    surface->z_order = max_z + 1;
    sort_surfaces();
}

void lower(Surface* surface) {
    if (surface == nullptr) {
        return;
    }
    // Find the minimum z-order
    int min_z = surface->z_order;
    for (Surface* s = surface_list; s != nullptr; s = s->next) {
        if (s->z_order < min_z) {
            min_z = s->z_order;
        }
    }
    surface->z_order = min_z - 1;
    sort_surfaces();
}

void show(Surface* surface) {
    if (surface == nullptr) {
        return;
    }
    surface->flags |= FLAG_VISIBLE;
    surface->has_damage = true;
}

void hide(Surface* surface) {
    if (surface == nullptr) {
        return;
    }
    surface->flags &= ~FLAG_VISIBLE;
}

void damage(Surface* surface, int x, int y,
               std::uint32_t width, std::uint32_t height) {
    if (surface == nullptr) {
        return;
    }

    // Expand damage region
    if (!surface->has_damage) {
        surface->damage_x = x;
        surface->damage_y = y;
        surface->damage_width = width;
        surface->damage_height = height;
        surface->has_damage = true;
    } else {
        const int old_right = surface->damage_x + static_cast<int>(surface->damage_width);
        const int old_bottom = surface->damage_y + static_cast<int>(surface->damage_height);
        const int new_right = x + static_cast<int>(width);
        const int new_bottom = y + static_cast<int>(height);

        const int new_left = x < surface->damage_x ? x : surface->damage_x;
        const int new_top = y < surface->damage_y ? y : surface->damage_y;
        const int new_right_max = new_right > old_right ? new_right : old_right;
        const int new_bottom_max = new_bottom > old_bottom ? new_bottom : old_bottom;

        surface->damage_x = new_left;
        surface->damage_y = new_top;
        surface->damage_width = static_cast<std::uint32_t>(new_right_max - new_left);
        surface->damage_height = static_cast<std::uint32_t>(new_bottom_max - new_top);
    }
}

void damage_all(Surface* surface) {
    if (surface == nullptr) {
        return;
    }
    surface->damage_x = 0;
    surface->damage_y = 0;
    surface->damage_width = surface->width;
    surface->damage_height = surface->height;
    surface->has_damage = true;
}

void present() {
    // Render all surfaces to the screen buffer
    render();

    // Copy the screen buffer to the framebuffer
    if (screen_buffer_ptr != nullptr && kernel::framebuffer::buffer() != nullptr) {
        const std::size_t copy_bytes = screen_buffer_size;
        std::memcpy(kernel::framebuffer::buffer(),
                       screen_buffer_ptr,
                       copy_bytes);
        kernel::framebuffer::present();
    }

    ++frame_count;
    ++stats_data.frames_rendered;
}

void render() {
    // Clear the screen buffer with the background color
    fill_rect(0, 0, config.screen_width, config.screen_height,
                 config.background_color);

    // Render surfaces in z-order
    for (Surface* surface = surface_list; surface != nullptr; surface = surface->next) {
        if ((surface->flags & FLAG_VISIBLE) == 0) {
            continue;
        }

        // Call the paint callback if set
        if (surface->paint != nullptr) {
            surface->paint(surface, surface->damage_x, surface->damage_y,
                               surface->damage_width, surface->damage_height);
        }

        // Blit the surface to the screen buffer
        blit(surface, surface->x, surface->y,
                surface->damage_x, surface->damage_y,
                surface->damage_width, surface->damage_height);

        // Clear damage region
        surface->has_damage = false;
        ++stats_data.dirty_surfaces;
    }

    // Draw the cursor
    if (cursor_visible_flag && cursor_surface != nullptr) {
        blit(cursor_surface, cursor_x, cursor_y, 0, 0,
                cursor_surface->width, cursor_surface->height);
    }
}

Surface* surface_at(int x, int y) {
    // Search from top to bottom (highest z-order first)
    Surface* found = nullptr;
    for (Surface* surface = surface_list; surface != nullptr; surface = surface->next) {
        if ((surface->flags & FLAG_VISIBLE) == 0) {
            continue;
        }
        if ((surface->flags & FLAG_INPUT) == 0) {
            continue;
        }
        if (x >= surface->x &&
            x < surface->x + static_cast<int>(surface->width) &&
            y >= surface->y &&
            y < surface->y + static_cast<int>(surface->height)) {
            found = surface;
            // Don't break, continue to find the topmost
        }
    }
    return found;
}

Surface* focused_surface() noexcept {
    return focused;
}

void set_focus(Surface* surface) {
    if (focused == surface) {
        return;
    }

    // Blur the old surface
    if (focused != nullptr && focused->blur != nullptr) {
        focused->blur(focused);
    }

    focused = surface;

    // Focus the new surface
    if (focused != nullptr && focused->focus != nullptr) {
        focused->focus(focused);
    }
}

std::uint32_t* screen_buffer() noexcept {
    return screen_buffer_ptr;
}

std::uint32_t screen_width() noexcept {
    return config.screen_width;
}

std::uint32_t screen_height() noexcept {
    return config.screen_height;
}

void set_cursor(int x, int y) {
    cursor_x = x;
    cursor_y = y;
}

void cursor_position(int* x, int* y) noexcept {
    if (x != nullptr) *x = cursor_x;
    if (y != nullptr) *y = cursor_y;
}

void show_cursor(bool visible) {
    cursor_visible_flag = visible;
}

bool cursor_visible() noexcept {
    return cursor_visible_flag;
}

Stats stats() noexcept {
    return stats_data;
}

void set_paint_callback(Surface* surface, PaintCallback callback) {
    if (surface == nullptr) {
        return;
    }
    surface->paint = callback;
}

void blit(Surface* surface, int dx, int dy,
             int sx, int sy,
             std::uint32_t width, std::uint32_t height) {
    if (surface == nullptr || surface->buffer == nullptr ||
        screen_buffer_ptr == nullptr) {
        return;
    }

    // Clip to surface bounds
    if (sx < 0) {
        dx -= sx;
        width = static_cast<std::uint32_t>(
            static_cast<int>(width) + sx);
        sx = 0;
    }
    if (sy < 0) {
        dy -= sy;
        height = static_cast<std::uint32_t>(
            static_cast<int>(height) + sy);
        sy = 0;
    }
    if (sx + static_cast<int>(width) > static_cast<int>(surface->buffer_width)) {
        width = surface->buffer_width - sx;
    }
    if (sy + static_cast<int>(height) > static_cast<int>(surface->buffer_height)) {
        height = surface->buffer_height - sy;
    }

    // Clip to screen bounds
    if (dx < 0) {
        width = static_cast<std::uint32_t>(
            static_cast<int>(width) + dx);
        dx = 0;
    }
    if (dy < 0) {
        height = static_cast<std::uint32_t>(
            static_cast<int>(height) + dy);
        dy = 0;
    }
    if (dx + static_cast<int>(width) > static_cast<int>(config.screen_width)) {
        width = config.screen_width - dx;
    }
    if (dy + static_cast<int>(height) > static_cast<int>(config.screen_height)) {
        height = config.screen_height - dy;
    }

    if (width == 0 || height == 0) {
        return;
    }

    const std::uint8_t alpha = surface->alpha;
    const bool opaque = (surface->flags & FLAG_OPAQUE) != 0 || alpha == 255;

    // Copy row by row
    for (std::uint32_t row = 0; row < height; ++row) {
        const std::uint32_t* src = surface->buffer +
            (sy + row) * surface->buffer_width + sx;
        std::uint32_t* dst = screen_buffer_ptr +
            (dy + row) * config.screen_width + dx;

        if (opaque) {
            std::memcpy(dst, src, width * 4);
        } else {
            // Alpha blend
            for (std::uint32_t col = 0; col < width; ++col) {
                dst[col] = alpha_blend(src[col], dst[col], alpha);
            }
        }
    }
}

void fill_rect(int x, int y, std::uint32_t width, std::uint32_t height,
                  std::uint32_t color) {
    if (screen_buffer_ptr == nullptr) {
        return;
    }

    // Clip to screen bounds
    if (x < 0) {
        width = static_cast<std::uint32_t>(
            static_cast<int>(width) + x);
        x = 0;
    }
    if (y < 0) {
        height = static_cast<std::uint32_t>(
            static_cast<int>(height) + y);
        y = 0;
    }
    if (x + static_cast<int>(width) > static_cast<int>(config.screen_width)) {
        width = config.screen_width - x;
    }
    if (y + static_cast<int>(height) > static_cast<int>(config.screen_height)) {
        height = config.screen_height - y;
    }

    if (width == 0 || height == 0) {
        return;
    }

    for (std::uint32_t row = 0; row < height; ++row) {
        std::uint32_t* dst = screen_buffer_ptr +
            (y + row) * config.screen_width + x;
        for (std::uint32_t col = 0; col < width; ++col) {
            dst[col] = color;
        }
    }
}

void set_pixel(int x, int y, std::uint32_t color) {
    if (screen_buffer_ptr == nullptr ||
        x < 0 || y < 0 ||
        x >= static_cast<int>(config.screen_width) ||
        y >= static_cast<int>(config.screen_height)) {
        return;
    }
    screen_buffer_ptr[y * config.screen_width + x] = color;
}

std::uint32_t get_pixel(int x, int y) noexcept {
    if (screen_buffer_ptr == nullptr ||
        x < 0 || y < 0 ||
        x >= static_cast<int>(config.screen_width) ||
        y >= static_cast<int>(config.screen_height)) {
        return 0;
    }
    return screen_buffer_ptr[y * config.screen_width + x];
}

std::uint32_t alpha_blend(std::uint32_t src, std::uint32_t dst,
                              std::uint8_t alpha) noexcept {
    if (alpha == 0) {
        return dst;
    }
    if (alpha == 255) {
        return src;
    }

    const std::uint8_t src_alpha = alpha;
    const std::uint8_t dst_alpha = static_cast<std::uint8_t>(255 - alpha);

    const std::uint32_t src_r = (src >> 16) & 0xFF;
    const std::uint32_t src_g = (src >> 8) & 0xFF;
    const std::uint32_t src_b = src & 0xFF;

    const std::uint32_t dst_r = (dst >> 16) & 0xFF;
    const std::uint32_t dst_g = (dst >> 8) & 0xFF;
    const std::uint32_t dst_b = dst & 0xFF;

    const std::uint32_t r = (src_r * src_alpha + dst_r * dst_alpha) / 255;
    const std::uint32_t g = (src_g * src_alpha + dst_g * dst_alpha) / 255;
    const std::uint32_t b = (src_b * src_alpha + dst_b * dst_alpha) / 255;

    return (r << 16) | (g << 8) | b;
}

} // namespace kernel::compositor