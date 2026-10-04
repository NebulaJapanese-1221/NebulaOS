// Framebuffer text console for the NebulaOS x86 operating system.
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

#include "console.hpp"
#include "../../drivers/graphics.hpp"

namespace {
// One pixel of spacing after the 5 pixel glyph, and one blank row between
// lines, which keeps the grid readable at the same scale the desktop uses.
const unsigned int cell_width = 6;
const unsigned int cell_height = 8;

// The grid is bounded rather than sized from the screen so the backing store
// stays a fixed object instead of needing an allocation during start up.
const unsigned int maximum_columns = 160;
const unsigned int maximum_rows = 60;

const unsigned int screen_margin = 16;
const unsigned int bottom_margin = 12;

const unsigned int console_background = 0x000B1220;
const unsigned int console_border = 0x00223A54;
const unsigned int console_text = 0x00C7D9E8;
const unsigned int console_bright = 0x00FFFFFF;
const unsigned int console_cursor = 0x0038BDF8;
const unsigned int console_shadow = 0x00000000;

char cell[maximum_columns * maximum_rows];
unsigned int used_columns = 0;
unsigned int used_rows = 0;
unsigned int origin_x = 0;
unsigned int origin_y = 0;
unsigned int cursor_x = 0;
unsigned int cursor_y = 0;
bool cursor_shown = true;
bool dirty = true;

unsigned int at(unsigned int row, unsigned int column) {
    return row * maximum_columns + column;
}

void advance_line() {
    if (cursor_y + 1 < used_rows) {
        ++cursor_y;
        return;
    }
    // The cursor is on the last row, so the grid scrolls and the cursor stays
    // where it is rather than moving off the bottom.
    for (unsigned int row = 1; row < used_rows; ++row) {
        for (unsigned int column = 0; column < used_columns; ++column) {
            cell[at(row - 1, column)] = cell[at(row, column)];
        }
    }
    for (unsigned int column = 0; column < used_columns; ++column) {
        cell[at(used_rows - 1, column)] = ' ';
    }
    cursor_y = used_rows - 1;
}

void blank(unsigned int row, unsigned int column) {
    cell[at(row, column)] = ' ';
    dirty = true;
}
}

namespace shell::apps::console {

void initialize(unsigned int top) {
    origin_x = screen_margin;
    origin_y = top;

    const unsigned int screen_width = drivers::graphics::width();
    const unsigned int screen_height = drivers::graphics::height();
    const unsigned int usable_width =
        screen_width > 2 * screen_margin ? screen_width - 2 * screen_margin : 0;
    const unsigned int usable_height =
        screen_height > top + bottom_margin ? screen_height - top - bottom_margin : 0;

    used_columns = usable_width / cell_width;
    used_rows = usable_height / cell_height;
    if (used_columns > maximum_columns) {
        used_columns = maximum_columns;
    }
    if (used_rows > maximum_rows) {
        used_rows = maximum_rows;
    }
    clear();
}

void clear() {
    for (unsigned int index = 0; index < maximum_columns * maximum_rows; ++index) {
        cell[index] = ' ';
    }
    cursor_x = 0;
    cursor_y = 0;
    dirty = true;
}

void write_char(char character) {
    if (used_columns == 0 || used_rows == 0) {
        return;
    }
    if (character == '\n') {
        cursor_x = 0;
        advance_line();
        dirty = true;
        return;
    }
    if (character == '\r') {
        cursor_x = 0;
        dirty = true;
        return;
    }
    if (character < ' ') {
        return;
    }
    if (cursor_x >= used_columns) {
        cursor_x = 0;
        advance_line();
    }
    cell[at(cursor_y, cursor_x)] = character;
    ++cursor_x;
    dirty = true;
}

void write(const char* text) {
    for (unsigned int index = 0; text != nullptr && text[index] != '\0'; ++index) {
        write_char(text[index]);
    }
}

void write_line(const char* text) {
    write(text);
    write_char('\n');
}

void carriage_return() {
    cursor_x = 0;
    dirty = true;
}

void erase_previous() {
    if (used_columns == 0 || used_rows == 0) {
        return;
    }
    if (cursor_x > 0) {
        --cursor_x;
    } else if (cursor_y > 0) {
        cursor_y = cursor_y - 1;
        cursor_x = used_columns - 1;
    } else {
        return;
    }
    blank(cursor_y, cursor_x);
}

void render() {
    if (!dirty || used_columns == 0 || used_rows == 0) {
        return;
    }

    const unsigned int grid_width = used_columns * cell_width;
    const unsigned int grid_height = used_rows * cell_height;

    // A drop shadow plus a border separates the console from the desktop
    // without needing a compositor.
    drivers::graphics::fill_rect(origin_x + 3, origin_y + 3, grid_width + 4,
                                 grid_height + 4, console_shadow);
    drivers::graphics::fill_rect(origin_x, origin_y, grid_width, grid_height,
                                 console_background);
    drivers::graphics::fill_rect(origin_x, origin_y, grid_width, 2,
                                 console_border);
    drivers::graphics::fill_rect(origin_x, origin_y + grid_height - 2,
                                 grid_width, 2, console_border);
    drivers::graphics::fill_rect(origin_x, origin_y, 2, grid_height,
                                 console_border);
    drivers::graphics::fill_rect(origin_x + grid_width - 2, origin_y, 2,
                                 grid_height, console_border);

    for (unsigned int row = 0; row < used_rows; ++row) {
        for (unsigned int column = 0; column < used_columns; ++column) {
            const char character = cell[at(row, column)];
            if (character == ' ') {
                continue;
            }
            const unsigned int left = origin_x + 2 + column * cell_width;
            const unsigned int top = origin_y + 2 + row * cell_height;
            const bool on_cursor_line =
                cursor_shown && row == cursor_y && column == cursor_x;
            if (on_cursor_line) {
                // The cursor is drawn as an inverse cell so it stays visible
                // over both blank and text cells.
                drivers::graphics::fill_rect(left, top, cell_width - 1,
                                             cell_height - 1, console_cursor);
                drivers::graphics::draw_text(left, top, &character,
                                            console_background, 1);
                continue;
            }
            // The row holding the caret is the active input line and reads brighter than
// the scrollback above it.
const unsigned int color =
    row == cursor_y ? console_bright : console_text;
drivers::graphics::draw_text(left, top, &character, color, 1);
        }
    }

    dirty = false;
}

void invalidate() {
    dirty = true;
}

unsigned int columns() {
    return used_columns;
}

unsigned int rows() {
    return used_rows;
}

unsigned int cursor_column() {
    return cursor_x;
}

unsigned int cursor_row() {
    return cursor_y;
}

}
