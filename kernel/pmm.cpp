// Physical page frame allocator for the NebulaOS x86 kernel.
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

#include "pmm.hpp"
#include "multiboot.hpp"

extern "C" {
extern unsigned char kernel_image_start[];
extern unsigned char kernel_image_end[];
}

namespace {
const unsigned int maximum_frames =
    static_cast<unsigned int>(managed_ceiling / page_size);
const unsigned int bitmap_words = (maximum_frames + 31) / 32;

// The low megabyte holds BIOS data, VGA windows and the boot information
// structures, so it is never available to the allocator.
const unsigned long long low_reserved_boundary = 0x100000ULL;

// entry.asm parks the boot stack directly below this address.
const unsigned int stack_base = 0x80000;
const unsigned int stack_top = 0x90000;

// Slop covering sections the linker script places outside the tracked image
// range, such as .eh_frame.
const unsigned int kernel_margin = 0x10000;

unsigned int frame_bitmap[bitmap_words];
unsigned int managed_frames = 0;
unsigned int allocation_hint = 0;
bool memory_ready = false;

bool within_managed(unsigned long long base, unsigned long long length) {
    return base < managed_ceiling && length != 0;
}

unsigned int clamp_frames(unsigned long long base, unsigned long long length) {
    unsigned long long end = base + length;
    if (end < end) {
        end = managed_ceiling;
    }
    if (end > managed_ceiling) {
        end = managed_ceiling;
    }
    return static_cast<unsigned int>(end / page_size);
}

void set_frame_used(unsigned int frame, bool used) {
    if (frame >= managed_frames) {
        return;
    }
    const unsigned int mask = 1U << (frame % 32);
    if (used) {
        frame_bitmap[frame / 32] |= mask;
    } else {
        frame_bitmap[frame / 32] &= ~mask;
    }
}

bool frame_is_free(unsigned int frame) {
    if (frame >= managed_frames) {
        return false;
    }
    return (frame_bitmap[frame / 32] & (1U << (frame % 32))) == 0;
}

void release_range(unsigned long long base, unsigned long long length) {
    if (!within_managed(base, length)) {
        return;
    }
    const unsigned int last = clamp_frames(base, length);
    const unsigned int first = static_cast<unsigned int>(base / page_size);
    for (unsigned int frame = first; frame < last; ++frame) {
        set_frame_used(frame, false);
    }
}

void mark_available_from_map(const kernel::multiboot::Information* info) {
    if ((info->flags & kernel::multiboot::flag_memory_map) == 0 ||
        info->memory_map_address == 0 || info->memory_map_length == 0) {
        // No map: fall back to the coarse memory_lower/memory_upper report.
        release_range(0x100000ULL, static_cast<unsigned long long>(info->memory_upper) * 1024ULL);
        release_range(0x10000ULL, static_cast<unsigned long long>(info->memory_lower) * 1024ULL);
        return;
    }

    const unsigned char* cursor =
        reinterpret_cast<const unsigned char*>(info->memory_map_address);
    const unsigned char* limit = cursor + info->memory_map_length;
    while (cursor + sizeof(kernel::multiboot::MapEntry) <= limit) {
        const kernel::multiboot::MapEntry* entry =
            reinterpret_cast<const kernel::multiboot::MapEntry*>(cursor);
        if (entry->size < sizeof(kernel::multiboot::MapEntry)) {
            break;
        }
        if (entry->type == kernel::multiboot::map_type_available) {
            release_range(entry->base, entry->length);
        }
        cursor += entry->size;
    }
}

void reserve_framebuffer(const kernel::multiboot::Information* info) {
    if ((info->flags & kernel::multiboot::flag_framebuffer) == 0) {
        return;
    }
    if (info->framebuffer_address == 0 ||
        info->framebuffer_address >= managed_ceiling) {
        return;
    }
    if (info->framebuffer_pitch == 0 || info->framebuffer_height == 0) {
        return;
    }
    const unsigned long long base = info->framebuffer_address;
    const unsigned long long length =
        static_cast<unsigned long long>(info->framebuffer_pitch) * info->framebuffer_height;
    if (within_managed(base, length)) {
        reserve(static_cast<unsigned int>(base), static_cast<unsigned int>(length));
    }
}
}

namespace kernel::memory {

bool initialize(unsigned int multiboot_info_address) {
    const kernel::multiboot::Information* info =
        kernel::multiboot::information(multiboot_info_address);
    if (info == nullptr) {
        return false;
    }

    for (unsigned int word = 0; word < bitmap_words; ++word) {
        frame_bitmap[word] = 0xFFFFFFFFU;
    }

    managed_frames = maximum_frames;
    mark_available_from_map(info);

    // Everything below the kernel is firmware territory, and the kernel image,
    // its boot stack and any framebuffer living in low memory are handed to the
    // kernel already in use.
    reserve(0, static_cast<unsigned int>(low_reserved_boundary));
    const unsigned int kernel_start = reinterpret_cast<unsigned int>(kernel_image_start);
    const unsigned int kernel_end = reinterpret_cast<unsigned int>(kernel_image_end);
    reserve(kernel_start, kernel_end - kernel_start + kernel_margin);
    reserve(stack_base, stack_top - stack_base);
    reserve_framebuffer(info);

    allocation_hint = static_cast<unsigned int>(low_reserved_boundary / page_size);
    memory_ready = true;
    return true;
}

bool is_initialized() {
    return memory_ready;
}

void reserve(unsigned int physical_base, unsigned int length) {
    const unsigned int last =
        clamp_frames(physical_base, static_cast<unsigned long long>(length));
    const unsigned int first = physical_base / page_size;
    for (unsigned int frame = first; frame < last; ++frame) {
        set_frame_used(frame, true);
    }
}

void* allocate_frame() {
    if (!memory_ready) {
        return nullptr;
    }
    for (unsigned int offset = 0; offset < managed_frames; ++offset) {
        const unsigned int frame = (allocation_hint + offset) % managed_frames;
        if (frame_is_free(frame)) {
            set_frame_used(frame, true);
            allocation_hint = (frame + 1) % managed_frames;
            return reinterpret_cast<void*>(frame * page_size);
        }
    }
    return nullptr;
}

bool release_frame(void* frame) {
    if (!memory_ready || frame == nullptr) {
        return false;
    }
    const unsigned int address = reinterpret_cast<unsigned int>(frame);
    if ((address % page_size) != 0) {
        return false;
    }
    const unsigned int index = address / page_size;
    if (index >= managed_frames) {
        return false;
    }
    set_frame_used(index, false);
    return true;
}

unsigned int total_frames() {
    return managed_frames;
}

unsigned int free_frames() {
    unsigned int count = 0;
    for (unsigned int frame = 0; frame < managed_frames; ++frame) {
        if (frame_is_free(frame)) {
            ++count;
        }
    }
    return count;
}

unsigned long long total_bytes() {
    return static_cast<unsigned long long>(managed_frames) * page_size;
}

unsigned long long free_bytes() {
    return static_cast<unsigned long long>(free_frames()) * page_size;
}

unsigned long long managed_bytes() {
    return managed_ceiling;
}

unsigned int highest_managed_address() {
    return managed_frames * page_size;
}

}