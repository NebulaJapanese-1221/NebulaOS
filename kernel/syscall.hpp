// Syscall interface for the NebulaOS x86 operating system.
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

namespace kernel::syscall {

void initialize();

// Registers a new file descriptor backed by a userspace buffer. Used by the
// userspace runtime to expose the framebuffer and the serial port without
// requiring a real filesystem in the kernel.
int open_device(const char* name, unsigned int buffer, unsigned int length);
void close_device(int fd);

// Fills the userspace struct with the current framebuffer geometry so a
// graphical userspace program can draw straight into the shared buffer.
bool get_framebuffer_info(void* info);

// Returns the number of whole seconds since boot and the tick count.
unsigned long long get_time_seconds();
unsigned int get_ticks();

// Yields the calling thread for the given number of ticks.
void sleep_ticks(unsigned int ticks);

} // namespace kernel::syscall