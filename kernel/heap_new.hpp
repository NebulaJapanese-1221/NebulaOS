// Kernel Heap Allocator for NebulaOS
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
#include <new>

namespace kernel::memory::heap {

// Alignment for heap allocations
constexpr std::size_t ALIGNMENT = 16;

// Block header structure
// Magic numbers for debugging
constexpr std::uint32_t MAGIC_USED = 0xC0DE0001;
constexpr std::uint32_t MAGIC_FREE = 0xFEEE0002;
constexpr std::uint32_t MAGIC_POISONED = 0xDEADBEEF;

struct BlockHeader {
    BlockHeader* prev;      // Previous block in list
    BlockHeader* next;      // Next block in list
    std::size_t size;        // Size of the payload (not including header)
    std::uint32_t magic;     // Magic for validation
};

// Initialize the heap with a reserve of memory
bool initialize(std::size_t reserve_bytes);

// Check if heap is initialized
bool is_initialized() noexcept;

// Allocate memory
void* allocate(std::size_t bytes);

// Free memory
void release(void* ptr);

// Reallocate memory
void* reallocate(void* ptr, std::size_t new_size);

// Allocate zeroed memory
void* allocate_zeroed(std::size_t bytes);

// Statistics
std::size_t total_bytes() noexcept;
std::size_t used_bytes() noexcept;
std::size_t free_bytes() noexcept;
std::size_t block_count() noexcept;
std::size_t free_block_count() noexcept;
std::size_t largest_free_block() noexcept;

// Validate heap integrity (for debugging)
bool validate() noexcept;

// Get the heap base address
std::uintptr_t base_address() noexcept;

// C++ operator new/delete for kernel
inline void* operator new(std::size_t size) {
    return allocate(size);
}

inline void* operator new[](std::size_t size) {
    return allocate(size);
}

inline void operator delete(void* ptr) noexcept {
    release(ptr);
}

inline void operator delete[](void* ptr) noexcept {
    release(ptr);
}

inline void operator delete(void* ptr, std::size_t) noexcept {
    release(ptr);
}

inline void operator delete[](void* ptr, std::size_t) noexcept {
    release(ptr);
}

} // namespace kernel::memory::heap