// Physical page frame allocator interface for the NebulaOS x86 kernel.
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

const unsigned int page_size = 4096;

// Frames are tracked for physical memory below this ceiling. Anything at or
// above it is never handed out, which keeps the tracking bitmap small and
// leaves large memory-mapped device windows alone.
const unsigned long long managed_ceiling = 1024ULL * 1024ULL * 1024ULL;

bool initialize(unsigned int multiboot_info_address);
bool is_initialized();

void reserve(unsigned int physical_base, unsigned int length);
void* allocate_frame();
bool release_frame(void* frame);

unsigned int total_frames();
unsigned int free_frames();
unsigned long long total_bytes();
unsigned long long free_bytes();
unsigned long long managed_bytes();
unsigned int highest_managed_address();

}