// GRUB framebuffer setup, double buffering and pixel access for NebulaOS.
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

#include "framebuffer.hpp"
#include "heap.hpp"
#include "multiboot.hpp"
#include "paging.hpp"

namespace {
struct ChannelMask {
    unsigned int position;
    unsigned int size;
};

unsigned char* display_pointer = nullptr;
unsigned char* back_buffer = nullptr;
unsigned int pixel_width = 0;
unsigned int pixel_height = 0;
unsigned int pitch_bytes = 0;
unsigned int bytes_per_pixel = 0;
unsigned int red_position = 16;
unsigned int red_mask_size = 8;
unsigned int green_position = 8;
unsigned int green_mask_size = 8;
unsigned int blue_position = 0;
unsigned int blue_mask_size = 8;

void set_reason(const char** reason, const char* text) {
    if (reason != nullptr) {
        *reason = text;
    }
}

bool masks_overlap(ChannelMask left, ChannelMask right) {
    if (left.size == 0 || right.size == 0) {
        return true;
    }
    const unsigned int left_last = left.position + left.size - 1;
    const unsigned int right_last = right.position + right.size - 1;
    return left.position <= right_last && right.position <= left_last;
}

bool masks_usable(unsigned int bits, ChannelMask red, ChannelMask green, ChannelMask blue) {
    if (red.size == 0 || green.size == 0 || blue.size == 0) {
        return false;
    }
    if (red.size > 8 || green.size > 8 || blue.size > 8) {
        return false;
    }
    if (red.position + red.size > bits || green.position + green.size > bits ||
        blue.position + blue.size > bits) {
        return false;
    }
    return !masks_overlap(red, green) && !masks_overlap(green, blue) &&
           !masks_overlap(red, blue);
}

// Some boot loaders report usable geometry but leave the per-channel masks
// empty or inconsistent. Fall back to the layouts VGA and VBE expose in
// practice so a valid framebuffer is never discarded over cosmetic metadata.
bool standard_masks(unsigned int bits, ChannelMask& red, ChannelMask& green,
                    ChannelMask& blue) {
    if (bits == 24 || bits == 32) {
        red = ChannelMask{16, 8};
        green = ChannelMask{8, 8};
        blue = ChannelMask{0, 8};
        return true;
    }
    return false;
}

unsigned int convert_channel(unsigned int channel, unsigned int mask_size,
                             unsigned int position) {
    if (mask_size == 0 || mask_size > 8 || position >= 32 ||
        mask_size + position > 32) {
        return 0;
    }
    const unsigned int maximum = (1U << mask_size) - 1;
    return ((channel * maximum + 127) / 255) << position;
}

unsigned int encode_color(unsigned int color) {
    return convert_channel((color >> 16) & 0xFF, red_mask_size, red_position) |
           convert_channel((color >> 8) & 0xFF, green_mask_size, green_position) |
           convert_channel(color & 0xFF, blue_mask_size, blue_position);
}

// Hardware can sit above the identity mapping, in which case paging exposes it
// through the reserved device window.
bool resolve_display_pointer(unsigned int physical_base, unsigned int length,
                             unsigned int* virtual_base) {
    const unsigned int identity_limit =
        static_cast<unsigned int>(kernel::memory::paging::identity_megabytes()) * 1024U * 1024U;
    if (physical_base < identity_limit && physical_base + length <= identity_limit) {
        *virtual_base = physical_base;
        return true;
    }
    return kernel::memory::paging::map_device_range(physical_base, length, virtual_base);
}
}

namespace kernel::framebuffer {

bool initialize(unsigned int multiboot_info_address, const char** reason) {
    set_reason(reason, "FRAMEBUFFER UNAVAILABLE");

    const kernel::multiboot::Information* info =
        kernel::multiboot::information(multiboot_info_address);
    if (info == nullptr) {
        set_reason(reason, "NO MULTIBOOT INFORMATION");
        return false;
    }
    if ((info->flags & kernel::multiboot::flag_framebuffer) == 0) {
        set_reason(reason, "BOOTLOADER SUPPLIED NO FRAMEBUFFER");
        return false;
    }
    if (info->framebuffer_type != 1) {
        set_reason(reason, "FRAMEBUFFER IS NOT DIRECT COLOUR");
        return false;
    }
    if (info->framebuffer_bpp != 24 && info->framebuffer_bpp != 32) {
        set_reason(reason, "FRAMEBUFFER DEPTH IS NOT 24 OR 32 BPP");
        return false;
    }
    if (info->framebuffer_address == 0 ||
        info->framebuffer_address > 0xFFFFFFFFULL) {
        set_reason(reason, "FRAMEBUFFER ADDRESS IS OUT OF RANGE");
        return false;
    }
    if (info->framebuffer_width < 640 || info->framebuffer_height < 480) {
        set_reason(reason, "FRAMEBUFFER RESOLUTION IS TOO SMALL");
        return false;
    }
    if (info->framebuffer_pitch <
        info->framebuffer_width * ((info->framebuffer_bpp + 7) / 8)) {
        set_reason(reason, "FRAMEBUFFER PITCH IS TOO SMALL");
        return false;
    }

    ChannelMask red = {info->red_position, info->red_mask_size};
    ChannelMask green = {info->green_position, info->green_mask_size};
    ChannelMask blue = {info->blue_position, info->blue_mask_size};
    if (!masks_usable(info->framebuffer_bpp, red, green, blue) &&
        !standard_masks(info->framebuffer_bpp, red, green, blue)) {
        set_reason(reason, "FRAMEBUFFER CHANNEL MASKS ARE UNUSABLE");
        return false;
    }

    pixel_width = info->framebuffer_width;
    pixel_height = info->framebuffer_height;
    pitch_bytes = info->framebuffer_pitch;
    bytes_per_pixel = (info->framebuffer_bpp + 7) / 8;
    red_position = red.position;
    red_mask_size = red.size;
    green_position = green.position;
    green_mask_size = green.size;
    blue_position = blue.position;
    blue_mask_size = blue.size;

    const unsigned long long physical = info->framebuffer_address;
    const unsigned long long length =
        static_cast<unsigned long long>(pitch_bytes) * pixel_height;
    unsigned int virtual_base = 0;
    if (length > 0xFFFFFFFFULL ||
        !resolve_display_pointer(static_cast<unsigned int>(physical),
                                 static_cast<unsigned int>(length),
                                 &virtual_base)) {
        set_reason(reason, "FRAMEBUFFER COULD NOT BE MAPPED");
        return false;
    }
    display_pointer = reinterpret_cast<unsigned char*>(virtual_base);

    // A back buffer keeps a whole frame in ordinary memory so the visible
    // update is a single sequential copy rather than a scatter of writes to
    // device memory. If the heap cannot satisfy it the framebuffer still works,
    // it just draws straight through.
    back_buffer = static_cast<unsigned char*>(
        memory::heap::allocate(static_cast<unsigned int>(length)));
    if (back_buffer != nullptr) {
        for (unsigned int offset = 0; offset < static_cast<unsigned int>(length); ++offset) {
            back_buffer[offset] = 0;
        }
    }

    set_reason(reason, nullptr);
    return true;
}

void clear(unsigned int color) {
    if (display_pointer == nullptr) {
        return;
    }
    if (back_buffer == nullptr) {
        fill_rect(0, 0, pixel_width, pixel_height, color);
        return;
    }

    // Widen the pattern to the pixel stride so full-screen clears do not need a
    // write per channel. Only the 32bpp case has a row pitch that is always a
    // multiple of the wider store, so narrower formats keep the byte loop.
    const unsigned int pixel_color = encode_color(color);
    for (unsigned int row = 0; row < pixel_height; ++row) {
        unsigned char* const destination = back_buffer + row * pitch_bytes;
        unsigned int column = 0;
        if (bytes_per_pixel == 4) {
            unsigned int* const words = reinterpret_cast<unsigned int*>(destination);
            for (; column + 4 <= pixel_width; column += 4) {
                words[column] = pixel_color;
            }
        }
        for (; column < pixel_width; ++column) {
            unsigned char* pixel = destination + column * bytes_per_pixel;
            pixel[0] = static_cast<unsigned char>(pixel_color);
            pixel[1] = static_cast<unsigned char>(pixel_color >> 8);
            pixel[2] = static_cast<unsigned char>(pixel_color >> 16);
            if (bytes_per_pixel == 4) {
                pixel[3] = static_cast<unsigned char>(pixel_color >> 24);
            }
        }
    }
}

void fill_rect(unsigned int x, unsigned int y, unsigned int rect_width,
               unsigned int rect_height, unsigned int color) {
    if (display_pointer == nullptr || x >= pixel_width || y >= pixel_height) {
        return;
    }
    if (rect_width > pixel_width - x) {
        rect_width = pixel_width - x;
    }
    if (rect_height > pixel_height - y) {
        rect_height = pixel_height - y;
    }

    unsigned char* const target = back_buffer != nullptr ? back_buffer : display_pointer;
    const unsigned int pixel_color = encode_color(color);
    for (unsigned int row = y; row < y + rect_height; ++row) {
        unsigned char* destination = target + row * pitch_bytes + x * bytes_per_pixel;
        if (bytes_per_pixel == 4) {
            unsigned int* const words = reinterpret_cast<unsigned int*>(destination);
            for (unsigned int column = 0; column < rect_width; ++column) {
                words[column] = pixel_color;
            }
            continue;
        }
        for (unsigned int column = 0; column < rect_width; ++column) {
            unsigned char* pixel = destination + column * bytes_per_pixel;
            pixel[0] = static_cast<unsigned char>(pixel_color);
            pixel[1] = static_cast<unsigned char>(pixel_color >> 8);
            pixel[2] = static_cast<unsigned char>(pixel_color >> 16);
        }
    }
}

void present() {
    present_rect(0, 0, pixel_width, pixel_height);
}

void present_rect(unsigned int x, unsigned int y, unsigned int rect_width,
                  unsigned int rect_height) {
    if (back_buffer == nullptr || display_pointer == nullptr) {
        return;
    }
    if (x >= pixel_width || y >= pixel_height) {
        return;
    }
    if (rect_width > pixel_width - x) {
        rect_width = pixel_width - x;
    }
    if (rect_height > pixel_height - y) {
        rect_height = pixel_height - y;
    }

    const unsigned int row_bytes = rect_width * bytes_per_pixel;
    for (unsigned int row = y; row < y + rect_height; ++row) {
        const unsigned char* source = back_buffer + row * pitch_bytes + x * bytes_per_pixel;
        volatile unsigned char* destination =
            display_pointer + row * pitch_bytes + x * bytes_per_pixel;
        for (unsigned int offset = 0; offset < row_bytes; ++offset) {
            destination[offset] = source[offset];
        }
    }
}

bool is_double_buffered() {
    return back_buffer != nullptr;
}

unsigned char* buffer() {
    return back_buffer != nullptr ? back_buffer : display_pointer;
}

unsigned int width() {
    return pixel_width;
}

unsigned int height() {
    return pixel_height;
}

}