// Kernel heap interface for the NebulaOS x86 operating system.
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

namespace kernel::memory {

// First-fit allocator backed by frames from the physical page frame manager.
// Every block is tracked in one doubly linked list so adjacent free blocks can
// be merged on release.
void initialize(unsigned int reserve_megabytes);
bool is_initialized();

void* allocate(unsigned int bytes);
void release(void* pointer);

unsigned int total_bytes();
unsigned int used_bytes();
unsigned int free_bytes();

}