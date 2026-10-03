// Graphical shell for the NebulaOS x86 operating system.
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

#include "shell.hpp"
#include "../drivers/graphics.hpp"
#include "../drivers/keyboard.hpp"
#include "../drivers/mouse.hpp"

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
bool start_menu_open = false;
drivers::mouse::State mouse_state = {0, 0, false, false};
bool mouse_present = false;

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
    const unsigned int screen_width = drivers::graphics::width();
    const unsigned int screen_height = drivers::graphics::height();
    const unsigned int taskbar_y = screen_height - 36;
    const unsigned int window_x = 48;
    const unsigned int window_y = screen_height >= 560 ? 154 : 96;
    const unsigned int window_width = screen_width - 96;
    const unsigned int window_height = taskbar_y - window_y - 28;
    const unsigned int input_y = taskbar_y - 58;
    const unsigned int output_y = window_y + 132;

    drivers::graphics::clear(background);
    drivers::graphics::fill_rect(0, 0, screen_width, 44, title_bar);
    drivers::graphics::fill_rect(24, 12, 20, 20, accent);
    drivers::graphics::draw_text(54, 14, "NEBULA OS", white, 2);
    drivers::graphics::draw_text(screen_width - 138, 17, "DESKTOP", 0x00C7D9E8, 1);

    drivers::graphics::fill_rect(32, 68, 180, 62, 0x002B4964);
    drivers::graphics::draw_text(50, 84, "SYSTEM READY", white, 1);
    drivers::graphics::draw_text(50, 105, "GRUB MULTIBOOT", 0x00A8C0D4, 1);

    drivers::graphics::fill_rect(window_x, window_y, window_width, window_height, panel);
    drivers::graphics::fill_rect(window_x, window_y, window_width, 42, title_bar);
    drivers::graphics::draw_text(window_x + 20, window_y + 14, "NEBULA SHELL", white, 2);
    drivers::graphics::fill_rect(window_x + 20, window_y + 58, window_width - 40, window_height - 82, terminal);
    drivers::graphics::draw_text(window_x + 36, window_y + 72, "WELCOME TO NEBULAOS", 0x0086D7FF, 1);
    drivers::graphics::draw_text(window_x + 36, window_y + 92, "TYPE HELP TO SEE AVAILABLE COMMANDS", 0x00B5C4D2, 1);

    for (unsigned int index = 0; index < output_count; ++index) {
        const unsigned int line_y = output_y + index * 20;
        if (line_y + 8 < input_y) {
            drivers::graphics::draw_text(window_x + 36, line_y, output[index], white, 1);
        }
    }

    drivers::graphics::draw_text(window_x + 36, input_y, "NEBULAOS>", accent, 1);
    drivers::graphics::draw_text(window_x + 102, input_y, command, white, 1);
    drivers::graphics::fill_rect(window_x + 102 + command_length * 6, input_y + 9, 5, 2, white);
    drivers::graphics::fill_rect(0, taskbar_y, screen_width, 36, title_bar);
    drivers::graphics::fill_rect(8, taskbar_y + 4, 124, 28,
                                 mouse_state.x >= 8 && mouse_state.x < 132 &&
                                 mouse_state.y >= taskbar_y + 4 && mouse_state.y < taskbar_y + 32
                                     ? 0x004A6A85 : accent);
    drivers::graphics::fill_rect(16, taskbar_y + 9, 18, 18, title_bar);
    drivers::graphics::draw_text(42, taskbar_y + 11, "START", white, 1);
    drivers::graphics::draw_text(screen_width - 136, taskbar_y + 11, "READY", muted_text, 1);

    if (!mouse_present) {
        drivers::graphics::draw_text(230, 92, "MOUSE NOT AVAILABLE", 0x00FFD080, 1);
    }

    if (start_menu_open) {
        const unsigned int menu_y = taskbar_y - 94;
        drivers::graphics::fill_rect(8, menu_y, 184, 88, panel);
        drivers::graphics::fill_rect(8, menu_y, 184, 24, title_bar);
        drivers::graphics::draw_text(20, menu_y + 8, "NEBULA MENU", white, 1);
        drivers::graphics::draw_text(20, menu_y + 38, "SHELL", dark_text, 1);
        drivers::graphics::draw_text(20, menu_y + 62, "ABOUT NEBULAOS", dark_text, 1);
    }

    if (mouse_present) {
        drivers::graphics::fill_rect(mouse_state.x, mouse_state.y, 2, 14, 0x00000000);
        drivers::graphics::fill_rect(mouse_state.x, mouse_state.y, 10, 2, 0x00000000);
        drivers::graphics::fill_rect(mouse_state.x + 2, mouse_state.y + 2, 2, 8, white);
        drivers::graphics::fill_rect(mouse_state.x + 2, mouse_state.y + 2, 6, 2, white);
        drivers::graphics::fill_rect(mouse_state.x + 4, mouse_state.y + 4, 2, 4, white);
        drivers::graphics::fill_rect(mouse_state.x + 6, mouse_state.y + 6, 2, 2, white);
    }
}

void show_about() {
    add_output("NEBULAOS COPYRIGHT (C) 2026");
    add_output("BY NEBULAJAPANESE-1221");
    add_output("GPLV3 OR LATER. FREE TO SHARE AND CHANGE.");
    add_output("CONTACT: NEBULAJAPANESE@GMAIL.COM");
    add_output("NO POSTAL ADDRESS AVAILABLE.");
    add_output("FULL LICENSE TEXT: LICENCE IN SOURCE TREE.");
}

void handle_mouse_click() {
    if (!mouse_state.left_clicked) {
        return;
    }

    const unsigned int taskbar_y = drivers::graphics::height() - 36;
    if (mouse_state.x >= 8 && mouse_state.x < 132 &&
        mouse_state.y >= taskbar_y + 4 && mouse_state.y < taskbar_y + 32) {
        start_menu_open = !start_menu_open;
        return;
    }

    if (!start_menu_open) {
        return;
    }

    const unsigned int menu_y = taskbar_y - 94;
    if (mouse_state.x >= 8 && mouse_state.x < 192 &&
        mouse_state.y >= menu_y + 26 && mouse_state.y < menu_y + 54) {
        add_output("NEBULA SHELL IS ALREADY OPEN.");
        start_menu_open = false;
    } else if (mouse_state.x >= 8 && mouse_state.x < 192 &&
               mouse_state.y >= menu_y + 54 && mouse_state.y < menu_y + 88) {
        show_about();
        start_menu_open = false;
    } else {
        start_menu_open = false;
    }
}

void execute() {
    command[command_length] = '\0';
    if (equals(command, "help")) {
        add_output("COMMANDS: HELP CLEAR ABOUT ECHO SHOW W SHOW C");
    } else if (equals(command, "clear")) {
        output_count = 0;
    } else if (equals(command, "about")) {
        show_about();
    } else if (equals(command, "show w")) {
        add_output("ABSOLUTELY NO WARRANTY.");
        add_output("PROGRAM PROVIDED AS IS; SEE LICENCE SECTION 15.");
    } else if (equals(command, "show c")) {
        add_output("FREE TO REDISTRIBUTE AND MODIFY UNDER GPL.");
        add_output("GPLV3 OR LATER; SEE LICENCE FOR FULL TERMS.");
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

[[noreturn]] void run(bool has_mouse) {
    mouse_present = has_mouse;
    add_output("NEBULAOS COPYRIGHT (C) 2026 NEBULAJAPANESE-1221");
    add_output("ABSOLUTELY NO WARRANTY. TYPE SHOW W FOR DETAILS.");
    add_output("FREE SOFTWARE: TYPE SHOW C FOR GPL TERMS.");
    render();
    for (;;) {
        bool redraw = false;
        char character;
        if (drivers::keyboard::try_read_character(character)) {
            if (character == '\b') {
                if (command_length != 0) {
                    command[--command_length] = '\0';
                    redraw = true;
                }
            } else if (character == '\n') {
                execute();
                redraw = false;
            } else if (command_length + 1 < sizeof(command) &&
                       character >= 0x20 && character <= 0x7E) {
                command[command_length++] = character;
                command[command_length] = '\0';
                redraw = true;
            }
        }

        if (mouse_present && drivers::mouse::poll(mouse_state)) {
            handle_mouse_click();
            redraw = true;
        }
        if (redraw) {
            render();
        }
        asm volatile("pause");
    }
}

}
