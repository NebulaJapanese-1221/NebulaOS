#pragma once

namespace drivers::vga {

void initialize();
void clear();
void write(const char* text);
void write_line(const char* text);
void put(char character);
void backspace();

}
