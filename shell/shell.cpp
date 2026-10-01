#include "shell.hpp"
#include "../drivers/keyboard.hpp"
#include "../drivers/vga.hpp"

namespace {
char command[64];
unsigned int length = 0;

bool equals(const char* left, const char* right) {
    unsigned int index = 0;
    while (left[index] != '\0' && right[index] != '\0') {
        if (left[index] != right[index]) {
            return false;
        }
        ++index;
    }
    return left[index] == right[index];
}

void prompt() {
    drivers::vga::write("NebulaBoot> ");
}

void execute() {
    command[length] = '\0';
    drivers::vga::put('\n');
    if (equals(command, "help")) {
        drivers::vga::write_line("Commands: help, clear, about, echo");
    } else if (equals(command, "clear")) {
        drivers::vga::clear();
    } else if (equals(command, "about")) {
        drivers::vga::write_line("NebulaOS C++ x86 BIOS kernel");
    } else if (length >= 5 && command[0] == 'e' && command[1] == 'c' && command[2] == 'h' && command[3] == 'o' && command[4] == ' ') {
        drivers::vga::write_line(command + 5);
    } else if (length != 0) {
        drivers::vga::write_line("Unknown command. Type help.");
    }
    length = 0;
    prompt();
}
}

namespace shell {

[[noreturn]] void run() {
    drivers::vga::write_line("NebulaOS text shell");
    drivers::vga::write_line("Type help for commands.");
    prompt();
    for (;;) {
        char character = drivers::keyboard::read_character();
        if (character == '\b') {
            if (length != 0) {
                --length;
                drivers::vga::backspace();
            }
        } else if (character == '\n') {
            execute();
        } else if (length + 1 < sizeof(command) && character >= 0x20 && character <= 0x7E) {
            command[length++] = character;
            drivers::vga::put(character);
        }
    }
}

}
