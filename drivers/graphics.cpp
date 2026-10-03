#include "graphics.hpp"

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
    unsigned long long framebuffer_address;
    unsigned int framebuffer_pitch;
    unsigned int framebuffer_width;
    unsigned int framebuffer_height;
    unsigned char framebuffer_bpp;
    unsigned char framebuffer_type;
    unsigned char color_info[6];
};

volatile unsigned int* framebuffer = nullptr;
unsigned int screen_width = 0;
unsigned int screen_height = 0;
unsigned int screen_pitch = 0;
unsigned int red_position = 16;
unsigned int green_position = 8;
unsigned int blue_position = 0;

unsigned int rgb(unsigned char red, unsigned char green, unsigned char blue) {
    return (static_cast<unsigned int>(red) << red_position) |
           (static_cast<unsigned int>(green) << green_position) |
           (static_cast<unsigned int>(blue) << blue_position);
}

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

bool initialize(unsigned int multiboot_info_address) {
    const MultibootInfo* info = reinterpret_cast<const MultibootInfo*>(multiboot_info_address);
    if ((info->flags & (1U << 12)) == 0 || info->framebuffer_type != 1 ||
        info->framebuffer_bpp != 32 || info->framebuffer_width < 640 ||
        info->framebuffer_height < 480 || info->framebuffer_pitch < info->framebuffer_width * 4) {
        return false;
    }

    framebuffer = reinterpret_cast<volatile unsigned int*>(
        static_cast<unsigned int>(info->framebuffer_address));
    screen_width = info->framebuffer_width;
    screen_height = info->framebuffer_height;
    screen_pitch = info->framebuffer_pitch / 4;
    red_position = info->color_info[0];
    green_position = info->color_info[2];
    blue_position = info->color_info[4];
    return true;
}

void clear(unsigned int color) {
    for (unsigned int y = 0; y < screen_height; ++y) {
        for (unsigned int x = 0; x < screen_width; ++x) {
            framebuffer[y * screen_pitch + x] = color;
        }
    }
}

void fill_rect(unsigned int x, unsigned int y, unsigned int rect_width, unsigned int rect_height, unsigned int color) {
    if (x >= screen_width || y >= screen_height) {
        return;
    }
    if (rect_width > screen_width - x) {
        rect_width = screen_width - x;
    }
    if (rect_height > screen_height - y) {
        rect_height = screen_height - y;
    }
    for (unsigned int row = y; row < y + rect_height; ++row) {
        for (unsigned int column = x; column < x + rect_width; ++column) {
            framebuffer[row * screen_pitch + column] = color;
        }
    }
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
                    fill_rect(left + column * scale, y + row * scale, scale, scale, color);
                }
            }
        }
    }
}

unsigned int width() {
    return screen_width;
}

unsigned int height() {
    return screen_height;
}

}
