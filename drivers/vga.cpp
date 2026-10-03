// VGA text output support for the NebulaOS x86 operating system.
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

#include "vga.hpp"

namespace {
volatile unsigned short* const buffer = reinterpret_cast<volatile unsigned short*>(0xB8000);
unsigned int row = 0;
unsigned int column = 0;
const unsigned char color = 0x0F;

void newline() {
    column = 0;
    ++row;
    if (row >= 25) {
        row = 0;
    }
}
}

namespace drivers::vga {

void initialize() {
    clear();
}

void clear() {
    for (unsigned int index = 0; index < 80 * 25; ++index) {
        buffer[index] = static_cast<unsigned short>(color << 8) | ' ';
    }
    row = 0;
    column = 0;
}

void put(char character) {
    if (character == '\n') {
        newline();
        return;
    }
    if (character == '\r') {
        column = 0;
        return;
    }
    buffer[row * 80 + column] = static_cast<unsigned short>(color << 8) | static_cast<unsigned char>(character);
    ++column;
    if (column >= 80) {
        newline();
    }
}

void write(const char* text) {
    for (unsigned int index = 0; text[index] != '\0'; ++index) {
        put(text[index]);
    }
}

void write_line(const char* text) {
    write(text);
    put('\n');
}

void backspace() {
    if (column == 0) {
        return;
    }
    --column;
    buffer[row * 80 + column] = static_cast<unsigned short>(color << 8) | ' ';
}

}
