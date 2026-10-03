// First-fit kernel heap for the NebulaOS x86 operating system.
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

namespace {
const unsigned int block_magic_used = 0xC0DE0001;
const unsigned int block_magic_free = 0xFEEE0002;
const unsigned int alignment = 16;

// The magic doubles as the in-use flag: a block is free when it still carries
// the free magic, and poisoned blocks are rejected instead of corrupting the
// list. Sixteen bytes keeps payload pointers suitably aligned without padding.
struct BlockHeader {
    BlockHeader* previous;
    BlockHeader* next;
    unsigned int size;
    unsigned int magic;
};

BlockHeader* first_block = nullptr;
unsigned int reserved_bytes = 0;
bool heap_ready = false;

unsigned int align_up(unsigned int bytes) {
    return (bytes + alignment - 1) & ~(alignment - 1);
}

void tally(unsigned int* used, unsigned int* unused) {
    unsigned int used_total = 0;
    unsigned int free_total = 0;
    for (BlockHeader* block = first_block; block != nullptr; block = block->next) {
        if (block->magic == block_magic_free) {
            free_total += block->size;
        } else {
            used_total += block->size;
        }
    }
    *used = used_total;
    *unused = free_total;
}
}

namespace kernel::memory {

void initialize(unsigned int reserve_megabytes) {
    if (heap_ready) {
        return;
    }
    BlockHeader* previous = nullptr;
    unsigned int collected = 0;
    while (collected < reserve_megabytes * 1024U * 1024U) {
        void* const frame = pmm::allocate_frame();
        if (frame == nullptr) {
            break;
        }
        BlockHeader* const block = static_cast<BlockHeader*>(frame);
        block->size = page_size;
        block->magic = block_magic_free;
        block->previous = previous;
        block->next = nullptr;
        if (previous != nullptr) {
            previous->next = block;
        } else {
            first_block = block;
        }
        previous = block;
        collected += page_size;
    }
    reserved_bytes = collected;
    heap_ready = first_block != nullptr;
}

bool is_initialized() {
    return heap_ready;
}

void* allocate(unsigned int bytes) {
    if (!heap_ready) {
        return nullptr;
    }
    unsigned int wanted = align_up(bytes);
    if (wanted == 0) {
        wanted = alignment;
    }
    const unsigned int required = wanted + sizeof(BlockHeader);

    for (BlockHeader* block = first_block; block != nullptr; block = block->next) {
        if (block->magic != block_magic_free || block->size < required) {
            continue;
        }
        const unsigned int remainder = block->size - required;
        if (remainder >= sizeof(BlockHeader) + alignment) {
            BlockHeader* const split = reinterpret_cast<BlockHeader*>(
                reinterpret_cast<unsigned char*>(block) + required);
            split->size = remainder;
            split->magic = block_magic_free;
            split->previous = block;
            split->next = block->next;
            if (block->next != nullptr) {
                block->next->previous = split;
            }
            block->next = split;
            block->size = required;
        }
        block->magic = block_magic_used;
        return reinterpret_cast<unsigned char*>(block) + sizeof(BlockHeader);
    }
    return nullptr;
}

void release(void* pointer) {
    if (!heap_ready || pointer == nullptr) {
        return;
    }
    BlockHeader* block = reinterpret_cast<BlockHeader*>(
        static_cast<unsigned char*>(pointer) - sizeof(BlockHeader));
    if (block->magic != block_magic_used) {
        return;
    }
    block->magic = block_magic_free;

    BlockHeader* const following = block->next;
    if (following != nullptr && following->magic == block_magic_free) {
        block->size += following->size;
        block->next = following->next;
        if (following->next != nullptr) {
            following->next->previous = block;
        }
    }

    BlockHeader* const preceding = block->previous;
    if (preceding != nullptr && preceding->magic == block_magic_free) {
        preceding->size += block->size;
        preceding->next = block->next;
        if (block->next != nullptr) {
            block->next->previous = preceding;
        }
    }
}

unsigned int total_bytes() {
    return reserved_bytes;
}

unsigned int used_bytes() {
    unsigned int used = 0;
    unsigned int unused = 0;
    tally(&used, &unused);
    return used;
}

unsigned int free_bytes() {
    unsigned int used = 0;
    unsigned int unused = 0;
    tally(&used, &unused);
    return unused;
}

}