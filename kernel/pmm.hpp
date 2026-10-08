// Physical Memory Manager for NebulaOS
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

#include <cstdint>
#include <cstddef>

namespace kernel::memory::pmm {

// Page size in bytes (4 KiB)
constexpr std::uintptr_t PAGE_SIZE = 4096;
constexpr std::uintptr_t PAGE_SHIFT = 12;
constexpr std::uintptr_t PAGE_MASK = ~(PAGE_SIZE - 1);

// Maximum physical address we manage (1 GiB for 32-bit)
constexpr std::uintptr_t MAX_PHYS_ADDR = 0x40000000;
constexpr std::size_t MAX_FRAMES = MAX_PHYS_ADDR / PAGE_SIZE;

// Frame allocation flags
enum class FrameFlags : std::uint32_t {
    NONE        = 0,
    ZEROED      = 1 << 0,  // Frame should be zeroed
    DMA         = 1 << 1,  // Suitable for DMA (below 16 MiB)
    HIGH        = 1 << 2,  // Can be above 4 GiB (for PAE)
    NO_RESERVE  = 1 << 3,  // Don't reserve, just allocate
};

// Result type for fallible operations
template<typename T>
struct Result {
    T value;
    bool ok;

    constexpr operator bool() const noexcept { return ok; }
    constexpr T& operator*() noexcept { return value; }
    constexpr const T& operator*() const noexcept { return value; }
    constexpr T* operator->() noexcept { return &value; }
    constexpr const T* operator->() const noexcept { return &value; }
};

using FrameResult = Result<std::uintptr_t>;

// Initialize PMM from multiboot info
bool initialize(std::uintptr_t multiboot_info_addr);

// Check if PMM is initialized
bool is_initialized() noexcept;

// Allocate a single physical frame
FrameResult allocate_frame(FrameFlags flags = FrameFlags::NONE);

// Allocate contiguous physical frames
FrameResult allocate_frames(std::size_t count, FrameFlags flags = FrameFlags::NONE);

// Free a previously allocated frame
void free_frame(std::uintptr_t physical_addr);

// Free contiguous frames
void free_frames(std::uintptr_t physical_addr, std::size_t count);

// Reserve a physical memory range (mark as used)
void reserve_range(std::uintptr_t base, std::size_t size);

// Release a reserved range back to allocator
void release_range(std::uintptr_t base, std::size_t size);

// Statistics
std::size_t total_frames() noexcept;
std::size_t free_frames() noexcept;
std::size_t used_frames() noexcept;
std::uintptr_t total_bytes() noexcept;
std::uintptr_t free_bytes() noexcept;
std::uintptr_t used_bytes() noexcept;

// Physical address of the highest managed frame
std::uintptr_t highest_managed_address() noexcept;

// Memory map entry for querying
struct MemoryRegion {
    std::uintptr_t base;
    std::size_t size;
    enum class Type : std::uint32_t {
        AVAILABLE = 1,
        RESERVED  = 2,
        ACPI      = 3,
        NVS       = 4,
        BAD       = 5,
    } type;
};

// Get memory map (for diagnostics)
std::size_t get_memory_map(MemoryRegion* out_regions, std::size_t max_regions) noexcept;

} // namespace kernel::memory::pmm