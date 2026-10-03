// PS/2 keyboard input for the NebulaOS x86 operating system.
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

#include "keyboard.hpp"

namespace {
unsigned char read_port(unsigned short port) {
    unsigned char value;
    asm volatile("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

char translate(unsigned char code) {
    static const char table[] = "\0\0" "1234567890-=\b\t" "qwertyuiop[]\n\0" "asdfghjkl;'`\0" "\\zxcvbnm,./\0";
    if (code >= 0x02 && code <= 0x35) {
        return table[code - 0x00];
    }
    if (code == 0x39) {
        return ' ';
    }
    return '\0';
}
}

namespace drivers::keyboard {

void initialize() {
    while ((read_port(0x64) & 0x01) != 0) {
        read_port(0x60);
    }
}

bool try_read_character(char& character) {
    for (;;) {
        const unsigned char status = read_port(0x64);
        if ((status & 0x01) == 0) {
            return false;
        }
        if ((status & 0x20) != 0) {
            return false;
        }
        unsigned char code = read_port(0x60);
        if ((code & 0x80) != 0) {
            continue;
        }
        character = translate(code);
        if (character != '\0') {
            return true;
        }
    }
}

}
