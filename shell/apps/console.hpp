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

#pragma once

namespace shell::apps::console {

// A character grid drawn straight into the desktop framebuffer, sharing the
// 5x7 font and the 6 pixel advance that drivers::graphics already uses.

// Reserves the region below top for the console and sizes the grid to it.
void initialize(unsigned int top);

void clear();
void write(const char* text);
void write_line(const char* text);
void write_char(char character);

// Moves the cursor to the start of the line and blanks the cell before it,
// which is what a shell needs to erase a character it just echoed.
void carriage_return();
void erase_previous();

// Redraws the grid if anything was written since the last frame.
void render();

// Forces the next render() to repaint. The desktop clears the whole framebuffer
// before drawing itself, which discards the console just as thoroughly as new
// output would, so a full repaint has to be asked for explicitly.
void invalidate();

unsigned int columns();
unsigned int rows();
unsigned int cursor_column();
unsigned int cursor_row();

}
