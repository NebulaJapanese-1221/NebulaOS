// Kernel framebuffer interface for the NebulaOS x86 operating system.
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

namespace kernel::framebuffer {

bool initialize(unsigned int multiboot_info_address, const char** reason = nullptr);

// All drawing targets the back buffer when one could be allocated. Call
// present() once a frame is complete so the update reaches the screen in one
// pass instead of tearing as each primitive lands in device memory.
void clear(unsigned int color);
void fill_rect(unsigned int x, unsigned int y, unsigned int width, unsigned int height, unsigned int color);
void present();
void present_rect(unsigned int x, unsigned int y, unsigned int width, unsigned int height);

bool is_double_buffered();
unsigned char* buffer();
unsigned int width();
unsigned int height();

}