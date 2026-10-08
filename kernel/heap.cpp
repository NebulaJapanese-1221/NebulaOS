// Kernel Heap Allocator Implementation for NebulaOS
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

#include "heap.hpp"
#include "pmm.hpp"
#include <cstring>

namespace kernel::memory::heap {

namespace {

// Free list head
BlockHeader* free_list = nullptr;

// Statistics
std::size_t total_bytes_count = 0;
std::size_t used_bytes_count = 0;
std::size_t total_blocks = 0;
std::size_t free_blocks = 0;
std::size_t largest_block = 0;

bool heap_initialized = false;

// Align a value up to the next multiple of ALIGNMENT
inline std::size_t align_up(std::size_t value) noexcept {
    return (value + ALIGNMENT - 1) & ~(ALIGNMENT - 1);
}

// Get the header for a payload pointer
inline BlockHeader* header_from_payload(void* payload) noexcept {
    return reinterpret_cast<BlockHeader*>(
        reinterpret_cast<std::uintptr_t>(payload) - sizeof(BlockHeader));
}

// Get the payload pointer for a header
inline void* payload_from_header(BlockHeader* header) noexcept {
    return reinterpret_cast<void*>(
        reinterpret_cast<std::uintptr_t>(header) + sizeof(BlockHeader));
}

// Coalesce a block with following free blocks
BlockHeader* coalesce_forward(BlockHeader* block) {
    while (block->next != nullptr && block->next->magic == MAGIC_FREE) {
        BlockHeader* next = block->next;
        block->size += next->size + sizeof(BlockHeader);
        block->next = next->next;
        if (next->next != nullptr) {
            next->next->prev = block;
        }
        --free_blocks;
    }
    return block;
}

// Insert a free block into the free list (address ordered)
void insert_free_block(BlockHeader* block) {
    if (free_list == nullptr || block < free_list) {
        block->prev = nullptr;
        block->next = free_list;
        if (free_list != nullptr) {
            free_list->prev = block;
        }
        free_list = block;
    } else {
        BlockHeader* current = free_list;
        while (current->next != nullptr && current->next < block) {
            current = current->next;
        }
        block->prev = current;
        block->next = current->next;
        if (current->next != nullptr) {
            current->next->prev = block;
        }
        current->next = block;
    }

    block->magic = MAGIC_FREE;
    ++free_blocks;
    if (block->size > largest_block) {
        largest_block = block->size;
    }
}

// Remove a block from the free list
void remove_free_block(BlockHeader* block) {
    if (block->prev != nullptr) {
        block->prev->next = block->next;
    } else {
        free_list = block->next;
    }
    if (block->next != nullptr) {
        block->next->prev = block->prev;
    }
    block->prev = nullptr;
    block->next = nullptr;
    --free_blocks;
}

// Add a new free block from physical memory
BlockHeader* add_free_block(std::size_t size) {
    // Allocate enough frames to hold the requested size plus header
    const std::size_t total_size = size + sizeof(BlockHeader);
    const std::size_t pages = (total_size + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;

    const auto frame = pmm::allocate_frames(pages, pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        return nullptr;
    }

    BlockHeader* block = reinterpret_cast<BlockHeader*>(frame.value);
    block->prev = nullptr;
    block->next = nullptr;
    block->size = pages * pmm::PAGE_SIZE - sizeof(BlockHeader);
    block->magic = MAGIC_FREE;

    insert_free_block(block);
    return block;
}

} // namespace

bool initialize(std::size_t reserve_bytes) {
    if (heap_initialized) {
        return true;
    }

    free_list = nullptr;
    total_bytes_count = 0;
    used_bytes_count = 0;
    total_blocks = 0;
    free_blocks = 0;
    largest_block = 0;

    // Add an initial free block
    const std::size_t initial_size = reserve_bytes > 0 ? reserve_bytes : 1024 * 1024;
    BlockHeader* initial = add_free_block(initial_size);
    if (initial == nullptr) {
        return false;
    }

    total_bytes_count = initial->size + sizeof(BlockHeader);
    heap_initialized = true;
    return true;
}

bool is_initialized() noexcept {
    return heap_initialized;
}

void* allocate(std::size_t bytes) {
    if (!heap_initialized || bytes == 0) {
        return nullptr;
    }

    const std::size_t aligned_size = align_up(bytes);
    const std::size_t required = aligned_size + sizeof(BlockHeader);

    // First-fit search through free list
    for (BlockHeader* block = free_list; block != nullptr; block = block->next) {
        // Coalesce with following free blocks to maximize size
        coalesce_forward(block);

        if (block->size < aligned_size) {
            continue;
        }

        // Check if we can split the block
        const std::size_t remainder = block->size - aligned_size;
        if (remainder >= sizeof(BlockHeader) + ALIGNMENT) {
            // Split the block
            BlockHeader* new_block = reinterpret_cast<BlockHeader*>(
                reinterpret_cast<std::uintptr_t>(block) + required);
            new_block->prev = block;
            new_block->next = block->next;
            new_block->size = remainder - sizeof(BlockHeader);
            new_block->magic = MAGIC_FREE;

            if (block->next != nullptr) {
                block->next->prev = new_block;
            }
            block->next = new_block;
            block->size = aligned_size;

            // Update largest block
            if (new_block->size > largest_block) {
                largest_block = new_block->size;
            }
        }

        // Remove from free list and mark as used
        remove_free_block(block);
        block->magic = MAGIC_USED;
        ++total_blocks;
        used_bytes_count += block->size;

        // Update largest block if needed
        if (block->size == largest_block) {
            // Recompute largest block
            largest_block = 0;
            for (BlockHeader* fb = free_list; fb != nullptr; fb = fb->next) {
                if (fb->size > largest_block) {
                    largest_block = fb->size;
                }
            }
        }

        return payload_from_header(block);
    }

    // Out of memory - try to grow the heap
    BlockHeader* grown = add_free_block(aligned_size + sizeof(BlockHeader));
    if (grown != nullptr) {
        total_bytes_count += grown->size + sizeof(BlockHeader);
        // Retry allocation
        return allocate(bytes);
    }

    return nullptr;
}

void release(void* ptr) {
    if (!heap_initialized || ptr == nullptr) {
        return;
    }

    BlockHeader* block = header_from_payload(ptr);
    if (block->magic != MAGIC_USED) {
        return;
    }

    // Mark as free
    used_bytes_count -= block->size;
    --total_blocks;
    block->magic = MAGIC_FREE;

    // Coalesce with following free blocks
    coalesce_forward(block);

    // Insert into free list
    insert_free_block(block);

    // Coalesce with previous free block if adjacent
    if (block->prev != nullptr &&
        block->prev->magic == MAGIC_FREE &&
        reinterpret_cast<std::uintptr_t>(block->prev) + sizeof(BlockHeader) + block->prev->size ==
            reinterpret_cast<std::uintptr_t>(block)) {
        coalesce_forward(block->prev);
    }
}

void* reallocate(void* ptr, std::size_t new_size) {
    if (ptr == nullptr) {
        return allocate(new_size);
    }
    if (new_size == 0) {
        release(ptr);
        return nullptr;
    }

    BlockHeader* block = header_from_payload(ptr);
    if (block->magic != MAGIC_USED) {
        return nullptr;
    }

    const std::size_t aligned_size = align_up(new_size);
    if (aligned_size == block->size) {
        return ptr;
    }

    // Try to expand in place if next block is free and large enough
    if (block->next != nullptr && block->next->magic == MAGIC_FREE) {
        const std::size_t combined = block->size + sizeof(BlockHeader) + block->next->size;
        if (combined >= aligned_size) {
            remove_free_block(block->next);
            block->next = block->next->next;
            if (block->next != nullptr) {
                block->next->prev = block;
            }
            // Split if there's enough remainder
            const std::size_t remainder = combined - aligned_size;
            if (remainder >= sizeof(BlockHeader) + ALIGNMENT) {
                BlockHeader* new_block = reinterpret_cast<BlockHeader*>(
                    reinterpret_cast<std::uintptr_t>(block) + sizeof(BlockHeader) + aligned_size);
                new_block->prev = block;
                new_block->next = block->next;
                new_block->size = remainder - sizeof(BlockHeader);
                new_block->magic = MAGIC_FREE;

                if (block->next != nullptr) {
                    block->next->prev = new_block;
                }
                block->next = new_block;
                block->size = aligned_size;
            } else {
                block->size = combined;
            }
            used_bytes_count = used_bytes_count - block->size + aligned_size;
            return ptr;
        }
    }

    // Allocate new block and copy
    void* new_ptr = allocate(new_size);
    if (new_ptr == nullptr) {
        return nullptr;
    }

    const std::size_t copy_size = block->size < new_size ? block->size : new_size;
    std::memcpy(new_ptr, ptr, copy_size);
    release(ptr);
    return new_ptr;
}

void* allocate_zeroed(std::size_t bytes) {
    void* ptr = allocate(bytes);
    if (ptr != nullptr) {
        std::memset(ptr, 0, bytes);
    }
    return ptr;
}

std::size_t total_bytes() noexcept {
    return total_bytes_count;
}

std::size_t used_bytes() noexcept {
    return used_bytes_count;
}

std::size_t free_bytes() noexcept {
    return total_bytes_count - used_bytes_count;
}

std::size_t block_count() noexcept {
    return total_blocks;
}

std::size_t free_block_count() noexcept {
    return free_blocks;
}

std::size_t largest_free_block() noexcept {
    return largest_block;
}

bool validate() noexcept {
    for (BlockHeader* block = free_list; block != nullptr; block = block->next) {
        if (block->magic != MAGIC_FREE) {
            return false;
        }
        if (block->next != nullptr && block->next->prev != block) {
            return false;
        }
    }
    return true;
}

std::uintptr_t base_address() noexcept {
    return free_list != nullptr
               ? reinterpret_cast<std::uintptr_t>(free_list)
               : 0;
}

} // namespace kernel::memory::heap