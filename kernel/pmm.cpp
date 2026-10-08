// Physical Memory Manager Implementation for NebulaOS
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

#include "pmm_new.hpp"
#include "multiboot.hpp"
#include <cstring>

extern "C" {
extern unsigned char kernel_image_start[];
extern unsigned char kernel_image_end[];
}

namespace kernel::memory::pmm {

namespace {

// Bitmap for tracking frame allocation
// Each bit represents one 4 KiB frame
constexpr std::size_t BITMAP_WORDS = (MAX_FRAMES + 31) / 32;

alignas(16) std::uint32_t frame_bitmap[BITMAP_WORDS];

// Statistics
std::size_t total_frame_count = 0;
std::size_t free_frame_count = 0;
std::uintptr_t highest_addr = 0;
bool pmm_ready = false;

// Memory regions from multiboot
MemoryRegion regions[64];
std::size_t region_count = 0;

// First-fit hint for faster allocation
std::size_t allocation_hint = 0;

// Low reserved boundary (BIOS, VGA, bootloader data)
constexpr std::uintptr_t LOW_RESERVED = 0x100000; // 1 MiB

// Kernel margin for alignment slack
constexpr std::uintptr_t KERNEL_MARGIN = 0x10000; // 64 KiB

inline bool frame_is_valid(std::size_t frame) noexcept {
    return frame < total_frame_count;
}

inline bool frame_is_free(std::size_t frame) noexcept {
    if (!frame_is_valid(frame)) {
        return false;
    }
    const std::uint32_t mask = 1U << (frame % 32);
    return (frame_bitmap[frame / 32] & mask) == 0;
}

inline void frame_set_free(std::size_t frame, bool is_free) noexcept {
    if (!frame_is_valid(frame)) {
        return;
    }
    const std::uint32_t mask = 1U << (frame % 32);
    if (is_free) {
        frame_bitmap[frame / 32] &= ~mask;
    } else {
        frame_bitmap[frame / 32] |= mask;
    }
}

inline std::size_t frame_from_address(std::uintptr_t addr) noexcept {
    return addr / PAGE_SIZE;
}

inline std::uintptr_t address_from_frame(std::size_t frame) noexcept {
    return frame * PAGE_SIZE;
}

void mark_range(std::uintptr_t base, std::size_t size, bool is_free) {
    if (size == 0 || base >= MAX_PHYS_ADDR) {
        return;
    }
    const std::uintptr_t aligned_base = base & PAGE_MASK;
    const std::uintptr_t end = (base + size + PAGE_SIZE - 1) & PAGE_MASK;
    const std::uintptr_t clamped_end = end > MAX_PHYS_ADDR ? MAX_PHYS_ADDR : end;

    for (std::uintptr_t addr = aligned_base; addr < clamped_end; addr += PAGE_SIZE) {
        const std::size_t frame = frame_from_address(addr);
        const bool was_free = frame_is_free(frame);
        frame_set_free(frame, is_free);
        if (is_free && !was_free) {
            ++free_frame_count;
        } else if (!is_free && was_free) {
            --free_frame_count;
        }
    }
}

void parse_memory_map(const kernel::multiboot::Information* info) {
    region_count = 0;

    if ((info->flags & kernel::multiboot::flag_memory_map) == 0 ||
        info->memory_map_address == 0 || info->memory_map_length == 0) {
        // Fallback to coarse memory_lower/memory_upper report
        if (info->memory_lower > 0) {
            regions[region_count++] = {0x0, static_cast<std::size_t>(info->memory_lower) * 1024, MemoryRegion::Type::AVAILABLE};
        }
        if (info->memory_upper > 0) {
            regions[region_count++] = {0x100000, static_cast<std::size_t>(info->memory_upper) * 1024, MemoryRegion::Type::AVAILABLE};
        }
        return;
    }

    const std::uintptr_t cursor_start = info->memory_map_address;
    const std::uintptr_t cursor_end = cursor_start + info->memory_map_length;
    std::uintptr_t cursor = cursor_start;

    while (cursor + sizeof(kernel::multiboot::MapEntry) <= cursor_end) {
        const auto* entry = reinterpret_cast<const kernel::multiboot::MapEntry*>(cursor);
        if (entry->size < 2 * sizeof(std::uint32_t)) {
            break;
        }

        MemoryRegion region;
        region.base = static_cast<std::uintptr_t>(entry->base);
        region.size = static_cast<std::size_t>(entry->length);

        switch (entry->type) {
            case kernel::multiboot::map_type_available:
                region.type = MemoryRegion::Type::AVAILABLE;
                break;
            case 2:
                region.type = MemoryRegion::Type::RESERVED;
                break;
            case 3:
                region.type = MemoryRegion::Type::ACPI;
                break;
            case 4:
                region.type = MemoryRegion::Type::NVS;
                break;
            default:
                region.type = MemoryRegion::Type::BAD;
                break;
        }

        if (region_count < sizeof(regions) / sizeof(regions[0])) {
            regions[region_count++] = region;
        }

        // Honour advertised stride, but fall back to struct size
        const std::size_t stride = entry->size >= sizeof(kernel::multiboot::MapEntry)
                                        ? entry->size
                                        : sizeof(kernel::multiboot::MapEntry);
        cursor += stride;
    }
}

void reserve_framebuffer(const kernel::multiboot::Information* info) {
    if ((info->flags & kernel::multiboot::flag_framebuffer) == 0) {
        return;
    }
    if (info->framebuffer_address == 0 ||
        info->framebuffer_address >= MAX_PHYS_ADDR) {
        return;
    }
    if (info->framebuffer_pitch == 0 || info->framebuffer_height == 0) {
        return;
    }
    const std::uintptr_t base = static_cast<std::uintptr_t>(info->framebuffer_address);
    const std::size_t size = static_cast<std::size_t>(info->framebuffer_pitch) * info->framebuffer_height;
    if (base + size <= MAX_PHYS_ADDR) {
        reserve_range(base, size);
    }
}

} // namespace

bool initialize(std::uintptr_t multiboot_info_addr) {
    const auto* info = kernel::multiboot::information(multiboot_info_addr);
    if (info == nullptr) {
        return false;
    }

    // Initialize bitmap: all frames marked as used (not free)
    for (std::size_t i = 0; i < BITMAP_WORDS; ++i) {
        frame_bitmap[i] = 0xFFFFFFFFU;
    }

    total_frame_count = MAX_FRAMES;
    free_frame_count = 0;
    highest_addr = 0;

    // Parse memory map and mark available regions
    parse_memory_map(info);
    for (std::size_t i = 0; i < region_count; ++i) {
        if (regions[i].type == MemoryRegion::Type::AVAILABLE) {
            const std::uintptr_t region_end = regions[i].base + regions[i].size;
            if (region_end > highest_addr && region_end <= MAX_PHYS_ADDR) {
                highest_addr = region_end;
            }
            // Only mark as free if it's above the low reserved area
            if (regions[i].base + regions[i].size > LOW_RESERVED) {
                const std::uintptr_t free_base = regions[i].base > LOW_RESERVED
                                                    ? regions[i].base
                                                    : LOW_RESERVED;
                const std::size_t free_size = regions[i].size - (free_base - regions[i].base);
                mark_range(free_base, free_size, true);
            }
        }
    }

    // Reserve the low megabyte (BIOS, VGA, bootloader data)
    reserve_range(0, LOW_RESERVED);

    // Reserve the kernel image
    const std::uintptr_t kernel_start = reinterpret_cast<std::uintptr_t>(kernel_image_start);
    const std::uintptr_t kernel_end = reinterpret_cast<std::uintptr_t>(kernel_image_end);
    if (kernel_end > kernel_start) {
        reserve_range(kernel_start, kernel_end - kernel_start + KERNEL_MARGIN);
    }

    // Reserve the framebuffer
    reserve_framebuffer(info);

    allocation_hint = frame_from_address(LOW_RESERVED);
    pmm_ready = (free_frame_count > 0);
    return pmm_ready;
}

bool is_initialized() noexcept {
    return pmm_ready;
}

FrameResult allocate_frame(FrameFlags flags) {
    if (!pmm_ready) {
        return {0, false};
    }

    const std::size_t start_hint = allocation_hint;
    for (std::size_t offset = 0; offset < total_frame_count; ++offset) {
        const std::size_t frame = (start_hint + offset) % total_frame_count;
        if (frame_is_free(frame)) {
            frame_set_free(frame, false);
            --free_frame_count;
            allocation_hint = (frame + 1) % total_frame_count;

            std::uintptr_t addr = address_from_frame(frame);
            if ((flags & FrameFlags::ZEROED) != FrameFlags::NONE) {
                std::memset(reinterpret_cast<void*>(addr), 0, PAGE_SIZE);
            }
            return {addr, true};
        }
    }

    return {0, false};
}

FrameResult allocate_frames(std::size_t count, FrameFlags flags) {
    if (!pmm_ready || count == 0) {
        return {0, false};
    }

    // Scan for contiguous free frames
    std::size_t consecutive = 0;
    std::size_t start_frame = 0;

    for (std::size_t frame = 0; frame < total_frame_count; ++frame) {
        if (frame_is_free(frame)) {
            if (consecutive == 0) {
                start_frame = frame;
            }
            ++consecutive;
            if (consecutive == count) {
                // Mark all as used
                for (std::size_t i = 0; i < count; ++i) {
                    frame_set_free(start_frame + i, false);
                }
                free_frame_count -= count;
                allocation_hint = (start_frame + count) % total_frame_count;

                std::uintptr_t addr = address_from_frame(start_frame);
                if ((flags & FrameFlags::ZEROED) != FrameFlags::NONE) {
                    std::memset(reinterpret_cast<void*>(addr), 0, count * PAGE_SIZE);
                }
                return {addr, true};
            }
        } else {
            consecutive = 0;
        }
    }

    return {0, false};
}

void free_frame(std::uintptr_t physical_addr) {
    if (!pmm_ready || physical_addr == 0) {
        return;
    }
    if ((physical_addr % PAGE_SIZE) != 0) {
        return;
    }
    const std::size_t frame = frame_from_address(physical_addr);
    if (!frame_is_valid(frame)) {
        return;
    }
    if (!frame_is_free(frame)) {
        frame_set_free(frame, true);
        ++free_frame_count;
    }
}

void free_frames(std::uintptr_t physical_addr, std::size_t count) {
    if (!pmm_ready || count == 0) {
        return;
    }
    for (std::size_t i = 0; i < count; ++i) {
        free_frame(physical_addr + i * PAGE_SIZE);
    }
}

void reserve_range(std::uintptr_t base, std::size_t size) {
    if (!pmm_ready) {
        return;
    }
    mark_range(base, size, false);
}

void release_range(std::uintptr_t base, std::size_t size) {
    if (!pmm_ready) {
        return;
    }
    mark_range(base, size, true);
}

std::size_t total_frames() noexcept {
    return total_frame_count;
}

std::size_t free_frames() noexcept {
    return free_frame_count;
}

std::size_t used_frames() noexcept {
    return total_frame_count - free_frame_count;
}

std::uintptr_t total_bytes() noexcept {
    return total_frame_count * PAGE_SIZE;
}

std::uintptr_t free_bytes() noexcept {
    return free_frame_count * PAGE_SIZE;
}

std::uintptr_t used_bytes() noexcept {
    return (total_frame_count - free_frame_count) * PAGE_SIZE;
}

std::uintptr_t highest_managed_address() noexcept {
    return highest_addr;
}

std::size_t get_memory_map(MemoryRegion* out_regions, std::size_t max_regions) noexcept {
    if (out_regions == nullptr || max_regions == 0) {
        return 0;
    }
    const std::size_t copy_count = region_count < max_regions ? region_count : max_regions;
    for (std::size_t i = 0; i < copy_count; ++i) {
        out_regions[i] = regions[i];
    }
    return copy_count;
}

} // namespace kernel::memory::pmm