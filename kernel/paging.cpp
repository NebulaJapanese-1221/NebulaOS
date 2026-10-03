// x86 4 KiB paging for the NebulaOS x86 kernel.
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

#include "paging.hpp"
#include "pmm.hpp"

namespace {
const unsigned int directory_entries = 1024;
const unsigned int table_entries = 1024;
const unsigned int index_mask = 0x3FF;
const unsigned int address_mask = 0xFFFFF000;

// The kernel and every heap allocation live in the low identity mapping, so
// device windows are placed far above it and can never collide.
const unsigned int device_window_base = 0xF0000000;
const unsigned int device_window_limit = 0xF0400000;

const unsigned int maximum_identity_megabytes = 256;

unsigned int* page_directory = nullptr;
unsigned int* page_tables[directory_entries];
unsigned int mapped_megabytes = 0;
unsigned int device_cursor = device_window_base;
bool paging_active = false;

unsigned int directory_index_of(unsigned int virtual_address) {
    return (virtual_address >> 22) & index_mask;
}

unsigned int table_index_of(unsigned int virtual_address) {
    return (virtual_address >> 12) & index_mask;
}

unsigned int* table_for(unsigned int directory_index) {
    if (page_tables[directory_index] != nullptr) {
        return page_tables[directory_index];
    }
    void* const frame = pmm::allocate_frame();
    if (frame == nullptr) {
        return nullptr;
    }
    unsigned int* const table = static_cast<unsigned int*>(frame);
    for (unsigned int index = 0; index < table_entries; ++index) {
        table[index] = 0;
    }
    page_tables[directory_index] = table;
    page_directory[directory_index] = reinterpret_cast<unsigned int>(table) |
                                      page_present | page_writable;
    return table;
}

bool table_is_empty(const unsigned int* table) {
    for (unsigned int index = 0; index < table_entries; ++index) {
        if ((table[index] & page_present) != 0) {
            return false;
        }
    }
    return true;
}
}

namespace kernel::memory {

bool initialize(unsigned int identity_megabytes) {
    if (page_directory != nullptr) {
        return true;
    }
    if (identity_megabytes == 0) {
        identity_megabytes = 1;
    }
    if (identity_megabytes > maximum_identity_megabytes) {
        identity_megabytes = maximum_identity_megabytes;
    }

    void* const directory_frame = pmm::allocate_frame();
    if (directory_frame == nullptr) {
        return false;
    }
    page_directory = static_cast<unsigned int*>(directory_frame);
    for (unsigned int index = 0; index < directory_entries; ++index) {
        page_directory[index] = 0;
        page_tables[index] = nullptr;
    }
    device_cursor = device_window_base;

    const unsigned long long span =
        static_cast<unsigned long long>(identity_megabytes) * 1024ULL * 1024ULL;
    for (unsigned long long address = 0; address < span; address += page_size) {
        if (!map_page(static_cast<unsigned int>(address),
                      static_cast<unsigned int>(address),
                      page_present | page_writable)) {
            return false;
        }
    }
    mapped_megabytes = identity_megabytes;
    return true;
}

bool enable() {
    if (page_directory == nullptr) {
        return false;
    }
    unsigned int control = 0;
    asm volatile("mov %0, %%cr3" : : "r"(reinterpret_cast<unsigned int>(page_directory)));
    asm volatile("mov %%cr0, %0" : "=r"(control));
    if ((control & (1U << 31)) == 0) {
        control |= 1U << 31;
        asm volatile("mov %0, %%cr0" : : "r"(control));
    }
    paging_active = true;
    return true;
}

bool is_enabled() {
    return paging_active;
}

unsigned int identity_megabytes() {
    return mapped_megabytes;
}

bool map_page(unsigned int virtual_address, unsigned int physical_address,
              unsigned int flags) {
    if (page_directory == nullptr) {
        return false;
    }
    unsigned int* const table = table_for(directory_index_of(virtual_address));
    if (table == nullptr) {
        return false;
    }
    table[table_index_of(virtual_address)] =
        (physical_address & address_mask) | flags | page_present;
    return true;
}

bool unmap_page(unsigned int virtual_address) {
    if (page_directory == nullptr) {
        return false;
    }
    const unsigned int directory_index = directory_index_of(virtual_address);
    unsigned int* const table = page_tables[directory_index];
    if (table == nullptr) {
        return false;
    }
    table[table_index_of(virtual_address)] = 0;
    if (table_is_empty(table)) {
        page_directory[directory_index] = 0;
        page_tables[directory_index] = nullptr;
        pmm::release_frame(table);
    }
    return true;
}

unsigned int translate(unsigned int virtual_address) {
    if (page_directory == nullptr) {
        return 0;
    }
    const unsigned int directory_index = directory_index_of(virtual_address);
    const unsigned int directory_entry = page_directory[directory_index];
    if ((directory_entry & page_present) == 0) {
        return 0;
    }
    const unsigned int* const table =
        reinterpret_cast<const unsigned int*>(directory_entry & address_mask);
    const unsigned int entry = table[table_index_of(virtual_address)];
    if ((entry & page_present) == 0) {
        return 0;
    }
    return (entry & address_mask) | (virtual_address & ~address_mask);
}

bool map_device_range(unsigned int physical_base, unsigned int length,
                      unsigned int* virtual_base) {
    if (page_directory == nullptr || length == 0) {
        return false;
    }
    const unsigned int first_page = physical_base & address_mask;
    const unsigned int last_page =
        (physical_base + length + address_mask) & address_mask;
    const unsigned int span = last_page - first_page;
    if (device_cursor + span > device_window_limit) {
        return false;
    }

    const unsigned int base = device_cursor;
    for (unsigned int address = first_page; address < last_page; address += page_size) {
        if (!map_page(device_cursor, address, page_present | page_writable)) {
            return false;
        }
        device_cursor += page_size;
    }
    if (virtual_base != nullptr) {
        *virtual_base = base;
    }
    return true;
}

}