#include "shell.hpp"
#include "../drivers/graphics.hpp"
#include "../drivers/keyboard.hpp"

namespace {
const unsigned int background = 0x00223A54;
const unsigned int panel = 0x00F1F5F9;
const unsigned int title_bar = 0x00334E68;
const unsigned int accent = 0x0038BDF8;
const unsigned int dark_text = 0x00182736;
const unsigned int muted_text = 0x00748798;
const unsigned int white = 0x00FFFFFF;
const unsigned int terminal = 0x000D1724;

char command[48];
char output[8][48];
unsigned int command_length = 0;
unsigned int output_count = 0;

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

void copy_text(char* destination, const char* source) {
    unsigned int index = 0;
    while (source[index] != '\0' && index < 47) {
        destination[index] = source[index];
        ++index;
    }
    destination[index] = '\0';
}

void add_output(const char* text) {
    if (output_count == 8) {
        for (unsigned int index = 1; index < 8; ++index) {
            copy_text(output[index - 1], output[index]);
        }
        output_count = 7;
    }
    copy_text(output[output_count++], text);
}

void render() {
    drivers::graphics::clear(background);
    drivers::graphics::fill_rect(0, 0, drivers::graphics::width(), 44, title_bar);
    drivers::graphics::fill_rect(24, 12, 20, 20, accent);
    drivers::graphics::draw_text(54, 14, "NEBULA OS", white, 2);
    drivers::graphics::draw_text(612, 17, "DESKTOP", 0x00C7D9E8, 1);

    drivers::graphics::fill_rect(32, 68, 180, 62, 0x002B4964);
    drivers::graphics::draw_text(50, 84, "SYSTEM READY", white, 1);
    drivers::graphics::draw_text(50, 105, "GRUB MULTIBOOT", 0x00A8C0D4, 1);

    drivers::graphics::fill_rect(48, 154, 704, 382, panel);
    drivers::graphics::fill_rect(48, 154, 704, 42, title_bar);
    drivers::graphics::draw_text(68, 168, "NEBULA SHELL", white, 2);
    drivers::graphics::fill_rect(68, 212, 664, 300, terminal);
    drivers::graphics::draw_text(84, 226, "WELCOME TO NEBULAOS", 0x0086D7FF, 1);
    drivers::graphics::draw_text(84, 246, "TYPE HELP TO SEE AVAILABLE COMMANDS", 0x00B5C4D2, 1);

    for (unsigned int index = 0; index < output_count; ++index) {
        drivers::graphics::draw_text(84, 286 + index * 20, output[index], white, 1);
    }

    const unsigned int input_y = 468;
    drivers::graphics::draw_text(84, input_y, "NEBULAOS>", accent, 1);
    drivers::graphics::draw_text(150, input_y, command, white, 1);
    drivers::graphics::fill_rect(150 + command_length * 6, input_y + 9, 5, 2, white);
    drivers::graphics::fill_rect(0, drivers::graphics::height() - 36, drivers::graphics::width(), 36, title_bar);
    drivers::graphics::fill_rect(16, drivers::graphics::height() - 27, 18, 18, accent);
    drivers::graphics::draw_text(46, drivers::graphics::height() - 25, "NEBULA", white, 1);
    drivers::graphics::draw_text(664, drivers::graphics::height() - 25, "READY", muted_text, 1);
}

void execute() {
    command[command_length] = '\0';
    if (equals(command, "help")) {
        add_output("COMMANDS: HELP CLEAR ABOUT ECHO");
    } else if (equals(command, "clear")) {
        output_count = 0;
    } else if (equals(command, "about")) {
        add_output("NEBULAOS GUI SHELL - X86");
    } else if (command_length >= 5 && command[0] == 'e' && command[1] == 'c' &&
               command[2] == 'h' && command[3] == 'o' && command[4] == ' ') {
        add_output(command + 5);
    } else if (command_length != 0) {
        add_output("UNKNOWN COMMAND. TYPE HELP.");
    }
    command_length = 0;
    render();
}
}

namespace shell {

[[noreturn]] void run() {
    render();
    for (;;) {
        const char character = drivers::keyboard::read_character();
        if (character == '\b') {
            if (command_length != 0) {
                command[--command_length] = '\0';
                render();
            }
        } else if (character == '\n') {
            execute();
        } else if (command_length + 1 < sizeof(command) &&
                   character >= 0x20 && character <= 0x7E) {
            command[command_length++] = character;
            command[command_length] = '\0';
            render();
        }
    }
}

}
