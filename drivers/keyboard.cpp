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

char read_character() {
    for (;;) {
        while ((read_port(0x64) & 0x01) == 0) {
            asm volatile("hlt");
        }
        unsigned char code = read_port(0x60);
        if ((code & 0x80) != 0) {
            continue;
        }
        char character = translate(code);
        if (character != '\0') {
            return character;
        }
    }
}

}
