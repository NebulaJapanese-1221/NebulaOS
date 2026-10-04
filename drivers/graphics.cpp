// Text and shape drawing for the NebulaOS x86 operating system.
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

#include "graphics.hpp"
#include "../kernel/framebuffer.hpp"

namespace {
unsigned char glyph_column(char character, unsigned int column) {
    static const unsigned char glyphs[36][5] = {
        {0x7E, 0x11, 0x11, 0x11, 0x7E}, {0x7F, 0x49, 0x49, 0x49, 0x36},
        {0x3E, 0x41, 0x41, 0x41, 0x22}, {0x7F, 0x41, 0x41, 0x22, 0x1C},
        {0x7F, 0x49, 0x49, 0x49, 0x41}, {0x7F, 0x09, 0x09, 0x09, 0x01},
        {0x3E, 0x41, 0x49, 0x49, 0x7A}, {0x7F, 0x08, 0x08, 0x08, 0x7F},
        {0x00, 0x41, 0x7F, 0x41, 0x00}, {0x20, 0x40, 0x41, 0x3F, 0x01},
        {0x7F, 0x08, 0x14, 0x22, 0x41}, {0x7F, 0x40, 0x40, 0x40, 0x40},
        {0x7F, 0x02, 0x0C, 0x02, 0x7F}, {0x7F, 0x04, 0x08, 0x10, 0x7F},
        {0x3E, 0x41, 0x41, 0x41, 0x3E}, {0x7F, 0x09, 0x09, 0x09, 0x06},
        {0x3E, 0x41, 0x51, 0x21, 0x5E}, {0x7F, 0x09, 0x19, 0x29, 0x46},
        {0x46, 0x49, 0x49, 0x49, 0x31}, {0x01, 0x01, 0x7F, 0x01, 0x01},
        {0x3F, 0x40, 0x40, 0x40, 0x3F}, {0x1F, 0x20, 0x40, 0x20, 0x1F},
        {0x3F, 0x40, 0x38, 0x40, 0x3F}, {0x63, 0x14, 0x08, 0x14, 0x63},
        {0x07, 0x08, 0x70, 0x08, 0x07}, {0x61, 0x51, 0x49, 0x45, 0x43},
        {0x3E, 0x51, 0x49, 0x45, 0x3E}, {0x00, 0x42, 0x7F, 0x40, 0x00},
        {0x42, 0x61, 0x51, 0x49, 0x46}, {0x21, 0x41, 0x45, 0x4B, 0x31},
        {0x18, 0x14, 0x12, 0x7F, 0x10}, {0x27, 0x45, 0x45, 0x45, 0x39},
        {0x3C, 0x4A, 0x49, 0x49, 0x30}, {0x01, 0x71, 0x09, 0x05, 0x03},
        {0x36, 0x49, 0x49, 0x49, 0x36}, {0x06, 0x49, 0x49, 0x29, 0x1E}
    };

    if (column >= 5) {
        return 0;
    }
    if (character >= 'a' && character <= 'z') {
        character = static_cast<char>(character - 'a' + 'A');
    }
    if (character >= 'A' && character <= 'Z') {
        return glyphs[character - 'A'][column];
    }
    if (character >= '0' && character <= '9') {
        return glyphs[26 + character - '0'][column];
    }

    switch (character) {
    case ':': return column == 1 || column == 3 ? 0x14 : 0;
    case '-': return column == 1 || column == 2 || column == 3 ? 0x08 : 0;
    case '>': return column == 0 ? 0x41 : column == 1 ? 0x22 : column == 2 ? 0x14 : column == 3 ? 0x08 : 0;
    case '!': return column == 2 ? 0x5F : 0;
    case '.': return column == 2 ? 0x40 : 0;
    case '\'': return column == 2 ? 0x03 : 0;
    case '/': return column == 0 ? 0x60 : column == 1 ? 0x18 : column == 2 ? 0x06 : 0;
    case '?': return column == 0 ? 0x02 : column == 1 ? 0x01 : column == 2 ? 0x51 : column == 3 ? 0x09 : column == 4 ? 0x06 : 0;
    case '_': return column == 0 || column == 4 ? 0x40 : 0;
    default: return 0;
    }
}
}

namespace drivers::graphics {

bool initialize(unsigned int multiboot_info_address, const char** reason) {
    return kernel::framebuffer::initialize(multiboot_info_address, reason);
}

void clear(unsigned int color) {
    kernel::framebuffer::clear(color);
}

void fill_rect(unsigned int x, unsigned int y, unsigned int rect_width, unsigned int rect_height, unsigned int color) {
    kernel::framebuffer::fill_rect(x, y, rect_width, rect_height, color);
}

void draw_text(unsigned int x, unsigned int y, const char* text, unsigned int color, unsigned int scale) {
    if (scale == 0) {
        return;
    }
    for (unsigned int index = 0; text[index] != '\0'; ++index) {
        const unsigned int left = x + index * 6 * scale;
        for (unsigned int column = 0; column < 5; ++column) {
            const unsigned char pixels = glyph_column(text[index], column);
            for (unsigned int row = 0; row < 7; ++row) {
                if ((pixels & (1U << row)) != 0) {
                    kernel::framebuffer::fill_rect(
                        left + column * scale, y + row * scale, scale, scale, color);
                }
            }
        }
    }
}

unsigned int width() {
    return kernel::framebuffer::width();
}

unsigned int height() {
    return kernel::framebuffer::height();
}

void present() {
    kernel::framebuffer::present();
}

bool is_double_buffered() {
    return kernel::framebuffer::is_double_buffered();
}

void draw_line(int x0, int y0, int x1, int y1, unsigned int color) {
    // Bresenham, so the whole thing stays in integer arithmetic. A floating
    // point version would pull the soft float library into a freestanding
    // kernel for no gain, and rounding it per step makes steep lines ragged.
    const int dx = x1 > x0 ? x1 - x0 : x0 - x1;
    const int dy = y1 > y0 ? y1 - y0 : y0 - y1;
    const int step_x = x0 < x1 ? 1 : -1;
    const int step_y = y0 < y1 ? 1 : -1;

    int error = dx - dy;
    for (;;) {
        if (x0 >= 0 && y0 >= 0) {
            kernel::framebuffer::fill_rect(static_cast<unsigned int>(x0),
                                           static_cast<unsigned int>(y0), 1, 1, color);
        }
        if (x0 == x1 && y0 == y1) {
            return;
        }
        // Doubling the error decides whether to step both axes or only one,
        // which is what keeps a near diagonal from stair stepping.
        const int doubled = error * 2;
        if (doubled > -dy) {
            error -= dy;
            x0 += step_x;
        }
        if (doubled < dx) {
            error += dx;
            y0 += step_y;
        }
    }
}

}
