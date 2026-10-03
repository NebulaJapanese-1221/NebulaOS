// x86 paging interface for the NebulaOS x86 kernel.
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

const unsigned int page_present = 0x1;
const unsigned int page_writable = 0x2;
const unsigned int page_user = 0x4;

// Builds the identity mapping over the low megabytes. Nothing is activated
// until enable() runs, so the caller can install an IDT that survives a fault.
bool initialize(unsigned int identity_megabytes);
bool enable();
bool is_enabled();

// Number of low megabytes currently covered by the identity mapping.
unsigned int identity_megabytes();

bool map_page(unsigned int virtual_address, unsigned int physical_address,
              unsigned int flags);
bool unmap_page(unsigned int virtual_address);
unsigned int translate(unsigned int virtual_address);

// Maps a physical byte range into a reserved high window so memory-mapped
// devices above the identity mapping stay reachable in 32-bit mode.
bool map_device_range(unsigned int physical_base, unsigned int length,
                      unsigned int* virtual_base);

}