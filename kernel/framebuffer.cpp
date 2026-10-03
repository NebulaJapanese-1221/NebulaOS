// GRUB framebuffer setup and pixel access for the NebulaOS x86 kernel.
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

namespace {
struct __attribute__((packed)) MultibootInfo {
    unsigned int flags;
    unsigned int memory_lower;
    unsigned int memory_upper;
    unsigned int boot_device;
    unsigned int command_line;
    unsigned int modules_count;
    unsigned int modules_address;
    unsigned int symbols[4];
    unsigned int memory_map_length;
    unsigned int memory_map_address;
    unsigned int drives_length;
    unsigned int drives_address;
    unsigned int configuration_table;
    unsigned int boot_loader_name;
    unsigned int apm_table;
    unsigned int vbe_control_info;
    unsigned int vbe_mode_info;
    unsigned short vbe_mode;
    unsigned short vbe_interface_segment;
    unsigned short vbe_interface_offset;
    unsigned short vbe_interface_length;
    unsigned long long framebuffer_address;
    unsigned int framebuffer_pitch;
    unsigned int framebuffer_width;
    unsigned int framebuffer_height;
    unsigned char framebuffer_bpp;
    unsigned char framebuffer_type;
    unsigned char red_position;
    unsigned char red_mask_size;
    unsigned char green_position;
    unsigned char green_mask_size;
    unsigned char blue_position;
    unsigned char blue_mask_size;
};

struct ChannelMask {
    unsigned int position;
    unsigned int size;
};

volatile unsigned char* pixels = nullptr;
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
}

namespace kernel::framebuffer {

bool initialize(unsigned int multiboot_info_address, const char** reason) {
    set_reason(reason, "FRAMEBUFFER UNAVAILABLE");

    if (multiboot_info_address == 0) {
        set_reason(reason, "NO MULTIBOOT INFORMATION");
        return false;
    }

    const MultibootInfo* info =
        reinterpret_cast<const MultibootInfo*>(multiboot_info_address);
    if ((info->flags & (1U << 12)) == 0) {
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
    if (!masks_usable(info->framebuffer_bpp, red, green, blue)) {
        if (!standard_masks(info->framebuffer_bpp, red, green, blue)) {
            set_reason(reason, "FRAMEBUFFER CHANNEL MASKS ARE UNUSABLE");
            return false;
        }
    }

    pixels = reinterpret_cast<volatile unsigned char*>(
        static_cast<unsigned int>(info->framebuffer_address));
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
    set_reason(reason, nullptr);
    return true;
}

void clear(unsigned int color) {
    fill_rect(0, 0, pixel_width, pixel_height, color);
}

void fill_rect(unsigned int x, unsigned int y, unsigned int rect_width,
               unsigned int rect_height, unsigned int color) {
    if (pixels == nullptr || x >= pixel_width || y >= pixel_height) {
        return;
    }
    if (rect_width > pixel_width - x) {
        rect_width = pixel_width - x;
    }
    if (rect_height > pixel_height - y) {
        rect_height = pixel_height - y;
    }

    const unsigned int pixel_color = encode_color(color);
    for (unsigned int row = y; row < y + rect_height; ++row) {
        volatile unsigned char* destination =
            pixels + row * pitch_bytes + x * bytes_per_pixel;
        for (unsigned int column = 0; column < rect_width; ++column) {
            volatile unsigned char* pixel = destination + column * bytes_per_pixel;
            pixel[0] = static_cast<unsigned char>(pixel_color);
            pixel[1] = static_cast<unsigned char>(pixel_color >> 8);
            pixel[2] = static_cast<unsigned char>(pixel_color >> 16);
            if (bytes_per_pixel == 4) {
                pixel[3] = static_cast<unsigned char>(pixel_color >> 24);
            }
        }
    }
}

unsigned int width() {
    return pixel_width;
}

unsigned int height() {
    return pixel_height;
}

}
