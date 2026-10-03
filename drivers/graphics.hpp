#pragma once

namespace drivers::graphics {

bool initialize(unsigned int multiboot_info_address);
void clear(unsigned int color);
void fill_rect(unsigned int x, unsigned int y, unsigned int width, unsigned int height, unsigned int color);
void draw_text(unsigned int x, unsigned int y, const char* text, unsigned int color, unsigned int scale);
unsigned int width();
unsigned int height();

}
