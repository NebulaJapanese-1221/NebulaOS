// Desktop shell for the NebulaOS x86 operating system.
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
#include "apps/console.hpp"
#include "../drivers/graphics.hpp"
#include "../drivers/keyboard.hpp"
#include "../drivers/mouse.hpp"
#include "../kernel/heap.hpp"
#include "../kernel/paging.hpp"
#include "../kernel/pmm.hpp"
#include "../kernel/timer.hpp"

namespace {
const unsigned int background_color = 0x0015202E;
const unsigned int taskbar_color = 0x00223A54;
const unsigned int start_button_color = 0x00334E68;
const unsigned int start_button_pressed_color = 0x002A4060;
const unsigned int card_fill_color = 0x00203A5A;
const unsigned int card_header_color = 0x002B4C70;
const unsigned int text_primary = 0x00E8F1FA;
const unsigned int text_dim = 0x0098AAB8;
const unsigned int text_accent = 0x0078D0F8;

const unsigned int taskbar_text_x = 148;
const unsigned int taskbar_text_y = 16;

const unsigned int start_button_x = 8;
const unsigned int start_button_y = 6;
const unsigned int start_button_width = 124;
const unsigned int start_button_height = 28;

const unsigned int menu_x = 8;
const unsigned int menu_y = 44;
const unsigned int menu_width = 236;
const unsigned int menu_height = 84;
const unsigned int menu_divider_y = 70;
const unsigned int menu_text_x = 24;
const unsigned int menu_about_y = 78;

const unsigned int panel_x = 24;
const unsigned int panel_y = 56;
const unsigned int panel_width = 300;
const unsigned int panel_header_height = 30;
const unsigned int panel_line_height = 16;

const unsigned int pointer_size = 6;

// The command line is kept in its own buffer rather than being written into
// the console as it is typed, so the caret can be moved through it without
// reprinting the scrollback above it.
const unsigned int input_capacity = 128;

char input_line[input_capacity];
unsigned int input_length = 0;
unsigned int input_cursor = 0;
// Console column the prompt ends at, which is where the editable text starts.
unsigned int prompt_column = 0;

bool needs_render = true;
unsigned int pointer_x = 0;
unsigned int pointer_y = 0;
bool pointer_visible = false;
bool menu_open = false;
int active_panel = -1;

void format_number(char* out, unsigned long long value) {
    unsigned int index = 0;
    if (value == 0) {
        out[index++] = '0';
    } else {
        char digits[24];
        unsigned int count = 0;
        while (value != 0) {
            digits[count++] = static_cast<char>('0' + (value % 10));
            value /= 10;
        }
        while (count != 0) {
            out[index++] = digits[--count];
        }
    }
    out[index] = '\0';
}

void append_number(char* out, unsigned int& index, unsigned int value,
                   unsigned int minimum_digits) {
    char digits[24];
    format_number(digits, value);
    unsigned int length = 0;
    while (digits[length] != '\0') {
        ++length;
    }
    for (unsigned int pad = length; pad < minimum_digits; ++pad) {
        out[index++] = '0';
    }
    for (unsigned int position = 0; position < length; ++position) {
        out[index++] = digits[position];
    }
}

void format_duration(char* out, unsigned int total_seconds) {
    const unsigned int hours = total_seconds / 3600;
    const unsigned int minutes = (total_seconds / 60) % 60;
    const unsigned int seconds = total_seconds % 60;

    unsigned int index = 0;
    // The hour field only appears once there is something to put in it, but
    // minutes and seconds are always two digits so the field widths line up.
    if (hours != 0) {
        append_number(out, index, hours, 1);
        out[index++] = ':';
    }
    append_number(out, index, minutes, 2);
    out[index++] = ':';
    append_number(out, index, seconds, 2);
    out[index] = '\0';
}

bool inside(int left, int top, int right, int bottom) {
    return pointer_x >= static_cast<unsigned int>(left) &&
           pointer_x < static_cast<unsigned int>(right) &&
           pointer_y >= static_cast<unsigned int>(top) &&
           pointer_y < static_cast<unsigned int>(bottom);
}

const char* const panel_titles[] = {"SYSTEM", "MEMORY", "ABOUT"};

void draw_taskbar() {
    drivers::graphics::fill_rect(0, 0, drivers::graphics::width(),
                                 shell::taskbar_height, taskbar_color);
    drivers::graphics::fill_rect(start_button_x, start_button_y,
                                 start_button_width, start_button_height,
                                 menu_open ? start_button_pressed_color
                                           : start_button_color);
    drivers::graphics::draw_text(20, start_button_y + 10, "NEBULA", text_primary, 1);
    drivers::graphics::draw_text(taskbar_text_x, taskbar_text_y,
                                 "NEBULAOS TERMINAL", text_primary, 1);
}

void draw_menu() {
    if (!menu_open) {
        return;
    }
    drivers::graphics::fill_rect(menu_x, menu_y, menu_width, menu_height,
                                 card_fill_color);
    drivers::graphics::fill_rect(menu_x, menu_y, menu_width, 2, card_header_color);
    drivers::graphics::fill_rect(menu_x, menu_y + menu_divider_y, menu_width, 1,
                                 card_header_color);
    drivers::graphics::draw_text(menu_text_x, menu_y + 14, "CONSOLE", text_dim, 1);
    drivers::graphics::draw_text(menu_text_x, menu_y + 36, "SYSTEM", text_dim, 1);
    drivers::graphics::draw_text(menu_text_x, menu_y + menu_about_y, "ABOUT",
                                 text_accent, 1);
}

void draw_panel() {
    if (active_panel < 0) {
        return;
    }
    const unsigned int index = static_cast<unsigned int>(active_panel);
    const unsigned int panel_height =
        panel_header_height + panel_line_height * 4 + 8;
    drivers::graphics::fill_rect(panel_x, panel_y, panel_width, panel_height,
                                 card_fill_color);
    drivers::graphics::fill_rect(panel_x, panel_y, panel_width,
                                 panel_header_height, card_header_color);
    drivers::graphics::draw_text(panel_x + 12, panel_y + 11, panel_titles[index],
                                 text_primary, 1);

    char line[32];
    unsigned int top = panel_y + panel_header_height + 8;
    switch (active_panel) {
    case 0:
        drivers::graphics::draw_text(panel_x + 12, top, "UPTIME", text_dim, 1);
        format_duration(line, kernel::timer::seconds());
        drivers::graphics::draw_text(panel_x + 12, top + panel_line_height, line,
                                     text_accent, 1);
        format_number(line, kernel::timer::ticks());
        drivers::graphics::draw_text(panel_x + 12, top + panel_line_height * 2,
                                     line, text_dim, 1);
        break;
    case 1:
        format_number(line, kernel::memory::pmm::free_frames());
        drivers::graphics::draw_text(panel_x + 12, top, line, text_accent, 1);
        format_number(line, kernel::memory::pmm::total_frames());
        drivers::graphics::draw_text(panel_x + 12, top + panel_line_height, line,
                                     text_dim, 1);
        format_number(line, kernel::memory::heap::free_bytes() / 1024);
        drivers::graphics::draw_text(panel_x + 12, top + panel_line_height * 2,
                                     line, text_dim, 1);
        break;
    default:
        drivers::graphics::draw_text(panel_x + 12, top, "NEBULAOS", text_accent, 1);
        drivers::graphics::draw_text(panel_x + 12, top + panel_line_height,
                                     "32 BIT X86 KERNEL", text_dim, 1);
        drivers::graphics::draw_text(panel_x + 12, top + panel_line_height * 2,
                                     "GNU GPL VERSION 3", text_dim, 1);
        break;
    }
}

void draw_pointer() {
    if (!pointer_visible) {
        return;
    }
    // The cursor is an outline over the desktop colour, which keeps it legible
    // against both the console and the desktop background.
    drivers::graphics::fill_rect(pointer_x, pointer_y, pointer_size, pointer_size,
                                 background_color);
    const unsigned int edge = 1;
    drivers::graphics::fill_rect(pointer_x, pointer_y, pointer_size, edge,
                                 text_primary);
    drivers::graphics::fill_rect(pointer_x, pointer_y + pointer_size - edge,
                                 pointer_size, edge, text_primary);
    drivers::graphics::fill_rect(pointer_x, pointer_y, edge, pointer_size,
                                 text_primary);
    drivers::graphics::fill_rect(pointer_x + pointer_size - edge, pointer_y, edge,
                                 pointer_size, text_primary);
    drivers::graphics::fill_rect(pointer_x + 2, pointer_y + 2, 2, 2, text_primary);
}

void redraw() {
    drivers::graphics::clear(background_color);
    draw_taskbar();
    // The clear above discarded the console along with everything else, so it
    // has to be marked for repaint even if no new output was queued.
    shell::apps::console::invalidate();
    shell::apps::console::render();
    draw_menu();
    draw_panel();
    draw_pointer();
    drivers::graphics::present();
    needs_render = false;
}

void write_number(const char* label, unsigned long long value) {
    char digits[32];
    format_number(digits, value);
    shell::apps::console::write("  ");
    shell::apps::console::write(label);
    shell::apps::console::write(" = ");
    shell::apps::console::write_line(digits);
}

void command_help() {
    shell::apps::console::write_line("AVAILABLE COMMANDS:");
    shell::apps::console::write_line("  HELP       THIS LIST");
    shell::apps::console::write_line("  CLEAR      CLEAR THE SCREEN");
    shell::apps::console::write_line("  UPTIME     TIME SINCE BOOT");
    shell::apps::console::write_line("  MEM        PHYSICAL MEMORY AND HEAP");
    shell::apps::console::write_line("  DISPLAY    FRAMEBUFFER AND PAGING");
    shell::apps::console::write_line("  ECHO TEXT  PRINT TEXT");
    shell::apps::console::write_line("  VER        KERNEL VERSION");
    shell::apps::console::write_line("  ABOUT      PROJECT AND LICENCE");
    shell::apps::console::write_line("  REBOOT     RESET VIA THE PS/2 CONTROLLER");
    shell::apps::console::write_line("");
    shell::apps::console::write_line("ARROW KEYS MOVE THE CARET, HOME AND END JUMP.");
}

void command_uptime() {
    char line[32];
    format_duration(line, kernel::timer::seconds());
    shell::apps::console::write("UPTIME ");
    shell::apps::console::write(line);
    shell::apps::console::write(" (");
    format_number(line, kernel::timer::ticks());
    shell::apps::console::write(line);
    shell::apps::console::write_line(" TICKS)");
}

void command_memory() {
    shell::apps::console::write_line("PHYSICAL MEMORY:");
    write_number("TOTAL FRAMES", kernel::memory::pmm::total_frames());
    write_number("FREE FRAMES", kernel::memory::pmm::free_frames());
    write_number("USED FRAMES",
                 kernel::memory::pmm::total_frames() -
                     kernel::memory::pmm::free_frames());
    shell::apps::console::write_line("KERNEL HEAP:");
    write_number("RESERVED BYTES", kernel::memory::heap::total_bytes());
    write_number("USED BYTES", kernel::memory::heap::used_bytes());
    write_number("FREE BYTES", kernel::memory::heap::free_bytes());
}

void command_display() {
    shell::apps::console::write_line("DISPLAY:");
    write_number("WIDTH", drivers::graphics::width());
    write_number("HEIGHT", drivers::graphics::height());
    write_number("DOUBLE BUFFERED",
                 drivers::graphics::is_double_buffered() ? 1 : 0);
    shell::apps::console::write_line("PAGING:");
    write_number("IDENTITY MAPPED MB",
                 kernel::memory::paging::identity_megabytes());
    write_number("KERNEL VIRTUAL BASE", 0xC0000000ULL);
    write_number("HEAP RESERVED KB",
                 kernel::memory::heap::total_bytes() / 1024);
}

void command_about() {
    shell::apps::console::write_line("NEBULAOS 32 BIT X86 KERNEL");
    shell::apps::console::write_line("");
    shell::apps::console::write_line("COPYRIGHT 2026 NEBULAJAPANESE-1221");
    shell::apps::console::write_line("LICENSED UNDER THE GNU GPL VERSION 3.");
    shell::apps::console::write_line("");
    shell::apps::console::write_line("SEE THE LICENCE FILE FOR FULL TERMS.");
}

// Reset through the PS/2 controller, which keeps the reboot path to the two
// ports that are already driven instead of needing the ACPI tables.
void command_reboot() {
    shell::apps::console::write_line("REBOOTING...");
    shell::apps::console::render();
    drivers::graphics::present();

    for (unsigned int attempt = 0; attempt < 1000000; ++attempt) {
        unsigned char status;
        asm volatile("inb %1, %0" : "=a"(status) : "Nd"(0x64));
        if ((status & 0x02) == 0) {
            break;
        }
    }
    asm volatile("outb %0, %1"
                 :
                 : "a"(static_cast<unsigned char>(0xFE)),
                   "Nd"(static_cast<unsigned short>(0x64)));
    // QEMU resets rather than halting, so reaching this line means the request
    // never made it out.
    shell::apps::console::write_line("REBOOT REQUEST FAILED");
}

// Commands are matched without regard to case. The console font has no
// lowercase glyphs and draws every character as uppercase, so a user typing
// "help" sees HELP on screen and would be baffled by an unknown command.
char to_upper(char character) {
    return (character >= 'a' && character <= 'z')
               ? static_cast<char>(character - 'a' + 'A')
               : character;
}

bool matches(const char* line, const char* command) {
    unsigned int index = 0;
    while (command[index] != '\0' && to_upper(line[index]) == command[index]) {
        ++index;
    }
    return command[index] == '\0' && (line[index] == ' ' || line[index] == '\0');
}

const char* argument(const char* line) {
    while (*line != '\0' && *line != ' ') {
        ++line;
    }
    while (*line == ' ') {
        ++line;
    }
    return line;
}

void run_command(const char* line) {
    if (matches(line, "HELP")) {
        command_help();
    } else if (matches(line, "CLEAR")) {
        shell::apps::console::clear();
    } else if (matches(line, "UPTIME")) {
        command_uptime();
    } else if (matches(line, "MEM")) {
        command_memory();
    } else if (matches(line, "DISPLAY")) {
        command_display();
    } else if (matches(line, "ECHO")) {
        shell::apps::console::write_line(argument(line));
    } else if (matches(line, "VER")) {
        shell::apps::console::write_line("NEBULAOS 0.0.1");
    } else if (matches(line, "ABOUT")) {
        command_about();
    } else if (matches(line, "REBOOT")) {
        command_reboot();
    } else if (line[0] != '\0') {
        shell::apps::console::write("UNKNOWN COMMAND: ");
        shell::apps::console::write_line(line);
        shell::apps::console::write_line("TYPE HELP FOR THE LIST.");
    }
}

void show_prompt() {
    shell::apps::console::write("> ");
    prompt_column = shell::apps::console::cursor_column();
}

void repaint_input() {
    // Rewind to the prompt, reprint the buffer, then walk the caret back to
    // where it belongs. Repainting the whole line keeps every edit path the
    // same instead of each one having to patch the tail it disturbed.
    while (shell::apps::console::cursor_column() > prompt_column) {
        shell::apps::console::erase_previous();
    }
    for (unsigned int index = 0; index < input_length; ++index) {
        shell::apps::console::write_char(input_line[index]);
    }
    for (unsigned int index = input_length; index > input_cursor; --index) {
        shell::apps::console::erase_previous();
    }
}

unsigned int input_limit() {
    // One column is held back so the caret has somewhere to sit at the end of
    // a full line, and one byte is held back so the buffer stays terminated.
    const unsigned int columns = shell::apps::console::columns();
    const unsigned int room = columns > prompt_column + 1
                                  ? columns - prompt_column - 1
                                  : 1;
    return room < input_capacity - 1 ? room : input_capacity - 1;
}

void handle_key(const drivers::keyboard::Event& event) {
    if (event.released) {
        return;
    }

    if (event.extended) {
        switch (event.code) {
        case drivers::keyboard::key_left:
            if (input_cursor > 0) {
                --input_cursor;
            }
            break;
        case drivers::keyboard::key_right:
            if (input_cursor < input_length) {
                ++input_cursor;
            }
            break;
        case drivers::keyboard::key_home:
            input_cursor = 0;
            break;
        case drivers::keyboard::key_end:
            input_cursor = input_length;
            break;
        case drivers::keyboard::key_delete:
            if (input_cursor < input_length) {
                for (unsigned int index = input_cursor; index + 1 < input_length;
                     ++index) {
                    input_line[index] = input_line[index + 1];
                }
                --input_length;
            }
            break;
        default:
            return;
        }
        repaint_input();
        return;
    }

    if (event.code == drivers::keyboard::key_enter) {
        input_line[input_length] = '\0';
        shell::apps::console::write_line(input_line);
        run_command(input_line);
        input_length = 0;
        input_cursor = 0;
        show_prompt();
        return;
    }

    // Control combinations arrive as the classic control codes the keyboard
    // driver produces, so they need no scancode of their own.
    if (event.character == 0x15) {
        input_length = 0;
        input_cursor = 0;
        repaint_input();
        return;
    }
    if (event.character == 0x0C) {
        input_length = 0;
        input_cursor = 0;
        shell::apps::console::clear();
        show_prompt();
        return;
    }
    if (event.character == 0x03) {
        input_length = 0;
        input_cursor = 0;
        shell::apps::console::write_line("");
        show_prompt();
        return;
    }

    if (event.code == drivers::keyboard::key_backspace) {
        if (input_cursor > 0) {
            for (unsigned int index = input_cursor - 1; index + 1 < input_length;
                 ++index) {
                input_line[index] = input_line[index + 1];
            }
            --input_length;
            --input_cursor;
        }
        repaint_input();
        return;
    }

    if (event.character == '\0' || input_length >= input_limit()) {
        return;
    }
    for (unsigned int index = input_length; index > input_cursor; --index) {
        input_line[index] = input_line[index - 1];
    }
    input_line[input_cursor] = event.character;
    ++input_length;
    ++input_cursor;
    repaint_input();
}

void handle_mouse() {
    drivers::mouse::State state;
    while (drivers::mouse::poll(state)) {
        pointer_x = state.x;
        pointer_y = state.y;
        pointer_visible = true;
        needs_render = true;

        if (!state.left_clicked) {
            continue;
        }
        if (inside(start_button_x, start_button_y,
                   start_button_x + static_cast<int>(start_button_width),
                   start_button_y + static_cast<int>(start_button_height))) {
            menu_open = !menu_open;
            active_panel = -1;
            continue;
        }
        if (menu_open &&
            inside(menu_x, menu_y, menu_x + static_cast<int>(menu_width),
                   menu_y + static_cast<int>(menu_height))) {
            menu_open = false;
            if (pointer_y < menu_y + static_cast<int>(menu_divider_y)) {
                shell::apps::console::write_line("");
                shell::apps::console::write_line("TYPE HELP AND PRESS ENTER.");
                shell::apps::console::write_line("");
                show_prompt();
            } else {
                active_panel = 2;
            }
            continue;
        }
        if (menu_open) {
            menu_open = false;
        }
    }
}
}

namespace shell {

void run() {
    apps::console::initialize(taskbar_height + 16);
    apps::console::write_line("");
    apps::console::write_line("NEBULAOS 32 BIT KERNEL - CONSOLE READY.");
    apps::console::write_line("TYPE HELP AND PRESS ENTER.");
    apps::console::write_line("");
    show_prompt();

    for (;;) {
        drivers::keyboard::Event event;
        while (drivers::keyboard::try_read_event(event)) {
            handle_key(event);
            needs_render = true;
        }
        handle_mouse();
        if (needs_render) {
            redraw();
        }
        asm volatile("pause");
    }
}

}
