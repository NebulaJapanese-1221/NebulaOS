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
