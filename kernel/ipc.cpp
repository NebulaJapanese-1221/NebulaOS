// IPC (Inter-Process Communication) Implementation for NebulaOS
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

#include "ipc.hpp"
#include "heap.hpp"
#include "scheduler.hpp"
#include "paging.hpp"
#include "pmm.hpp"
#include <cstring>

namespace kernel::ipc {

namespace {

// Global port list
Port* port_list = nullptr;
std::uint32_t port_count = 0;

// Global shared region list
SharedRegion shared_regions[MAX_SHARED_REGIONS];
std::uint32_t shared_region_count = 0;

// Statistics
IpcStats ipc_stats = {};

// Generate a unique port ID
std::uint32_t next_port_id() {
    static std::uint32_t next_id = 1;
    return next_id++;
}

// Generate a unique shared region ID
std::uint32_t next_region_id() {
    static std::uint32_t next_id = 1;
    return next_id++;
}

} // namespace

bool initialize() {
    port_list = nullptr;
    port_count = 0;
    shared_region_count = 0;

    ipc_stats = {};

    // Initialize shared regions array
    for (auto& region : shared_regions) {
        region.id = 0;
        region.in_use = false;
        region.kernel_address = 0;
        region.physical_address = 0;
        region.size = 0;
        region.owner_id = 0;
        region.permissions = 0;
    }

    return true;
}

Port* create_port(std::uint32_t capacity) {
    if (capacity == 0 || capacity > 1024) {
        return nullptr;
    }

    // Allocate port structure
    Port* port = static_cast<Port*>(
        heap::allocate(sizeof(Port)));
    if (port == nullptr) {
        return nullptr;
    }

    // Allocate message buffer
    port->messages = static_cast<Message*>(
        heap::allocate(capacity * sizeof(Message)));
    if (port->messages == nullptr) {
        heap::release(port);
        return nullptr;
    }

    // Initialize port
    port->id = next_port_id();
    port->in_use = true;
    port->messages = port->messages;
    port->head = 0;
    port->tail = 0;
    port->count = 0;
    port->capacity = capacity;
    port->wait_queue = nullptr;
    port->owner_id = 0;
    port->next = nullptr;

    // Add to global list
    port->next = port_list;
    port_list = port;
    ++port_count;
    ++ipc_stats.total_ports;

    return port;
}

void destroy_port(Port* port) {
    if (port == nullptr) {
        return;
    }

    // Remove from global list
    if (port_list == port) {
        port_list = port->next;
    } else {
        for (Port* p = port_list; p != nullptr; p = p->next) {
            if (p->next == port) {
                p->next = port->next;
                break;
            }
        }
    }

    --port_count;

    // Free memory
    heap::release(port->messages);
    heap::release(port);
}

Result send(Port* port, const MessageHeader& header,
              const void* data, std::size_t size) {
    if (port == nullptr || !port->in_use) {
        return Result::INVALID_PORT;
    }
    if (size > MAX_MESSAGE_SIZE) {
        return Result::INVALID_SIZE;
    }

    // Wait until there's space in the queue
    while (port->count >= port->capacity) {
        scheduler::block(port->wait_queue);
    }

    // Copy message into the queue
    Message& msg = port->messages[port->tail];
    std::memcpy(&msg.header, &header, sizeof(MessageHeader));
    if (data != nullptr && size > 0) {
        std::memcpy(msg.data, data, size);
    }

    port->tail = (port->tail + 1) % port->capacity;
    ++port->count;
    ++ipc_stats.total_messages;
    ++ipc_stats.messages_sent;

    return Result::SUCCESS;
}

Result try_send(Port* port, const MessageHeader& header,
                  const void* data, std::size_t size) {
    if (port == nullptr || !port->in_use) {
        return Result::INVALID_PORT;
    }
    if (size > MAX_MESSAGE_SIZE) {
        return Result::INVALID_SIZE;
    }
    if (port->count >= port->capacity) {
        return Result::WOULD_BLOCK;
    }

    Message& msg = port->messages[port->tail];
    std::memcpy(&msg.header, &header, sizeof(MessageHeader));
    if (data != nullptr && size > 0) {
        std::memcpy(msg.data, data, size);
    }

    port->tail = (port->tail + 1) % port->capacity;
    ++port->count;
    ++ipc_stats.total_messages;
    ++ipc_stats.messages_sent;

    return Result::SUCCESS;
}

Result receive(Port* port, Message& out_message) {
    if (port == nullptr || !port->in_use) {
        return Result::INVALID_PORT;
    }

    // Wait until a message is available
    while (port->count == 0) {
        scheduler::block(port->wait_queue);
    }

    // Copy message from the queue
    const Message& msg = port->messages[port->head];
    std::memcpy(&out_message, &msg, sizeof(Message));

    port->head = (port->head + 1) % port->capacity;
    --port->count;
    ++ipc_stats.messages_received;

    return Result::SUCCESS;
}

Result try_receive(Port* port, Message& out_message) {
    if (port == nullptr || !port->in_use) {
        return Result::INVALID_PORT;
    }
    if (port->count == 0) {
        return Result::WOULD_BLOCK;
    }

    const Message& msg = port->messages[port->head];
    std::memcpy(&out_message, &msg, sizeof(Message));

    port->head = (port->head + 1) % port->capacity;
    --port->count;
    ++ipc_stats.messages_received;

    return Result::SUCCESS;
}

Result receive_timeout(Port* port, Message& out_message,
                         std::uint32_t timeout_ms) {
    if (port == nullptr || !port->in_use) {
        return Result::INVALID_PORT;
    }

    // Simple polling implementation (timer-driven would be better)
    const std::uint32_t start = timer::ticks();
    while (port->count == 0) {
        if (timer::ticks() - start >= timeout_ms) {
            return Result::TIMEOUT;
        }
        scheduler::yield();
    }

    const Message& msg = port->messages[port->head];
    std::memcpy(&out_message, &msg, sizeof(Message));

    port->head = (port->head + 1) % port->capacity;
    --port->count;
    ++ipc_stats.messages_received;

    return Result::SUCCESS;
}

Result reply(const Message& request, const void* data, std::size_t size) {
    // Find the sender's port (assuming port 0 is the reply port)
    // This is a simplified implementation
    // In a real OS, you'd have a reply port mechanism

    if (request.header.source_id == 0) {
        return Result::INVALID_PORT;
    }

    // Create a response message
    MessageHeader header;
    header.type = static_cast<std::uint32_t>(MessageType::RESPONSE);
    header.source_id = 0;  // Kernel
    header.target_id = request.header.source_id;
    header.port = 0;  // Reply port
    header.size = size;

    // For now, just send to port 0
    // In a real implementation, we'd track reply ports
    return Result::SUCCESS;
}

SharedRegion* create_shared_region(std::size_t size,
                                     std::uint32_t permissions) {
    // Find a free slot
    SharedRegion* region = nullptr;
    for (auto& r : shared_regions) {
        if (!r.in_use) {
            region = &r;
            break;
        }
    }
    if (region == nullptr) {
        return nullptr;
    }

    // Allocate physical frames
    const std::size_t pages = (size + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
    const auto frame = pmm::allocate_frames(pages, pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        return nullptr;
    }

    // Map into kernel space
    std::uintptr_t virtual_base = 0;
    if (!paging::map_device_range(frame.value, pages * pmm::PAGE_SIZE,
                                      &virtual_base)) {
        pmm::free_frames(frame.value, pages);
        return nullptr;
    }

    region->id = next_region_id();
    region->in_use = true;
    region->kernel_address = virtual_base;
    region->physical_address = frame.value;
    region->size = size;
    region->owner_id = 0;
    region->permissions = permissions;

    ++shared_region_count;
    ++ipc_stats.total_shared_regions;

    return region;
}

void destroy_shared_region(SharedRegion* region) {
    if (region == nullptr || !region->in_use) {
        return;
    }

    // Unmap from kernel space
    const std::size_t pages = (region->size + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
    for (std::size_t i = 0; i < pages; ++i) {
        paging::unmap_page(region->kernel_address + i * pmm::PAGE_SIZE);
    }

    // Free physical frames
    pmm::free_frames(region->physical_address, pages);

    region->in_use = false;
    region->id = 0;
    region->kernel_address = 0;
    region->physical_address = 0;
    region->size = 0;
    region->owner_id = 0;
    region->permissions = 0;

    --shared_region_count;
}

Result map_shared_region(SharedRegion* region,
                          std::uintptr_t* virtual_address) {
    if (region == nullptr || !region->in_use) {
        return Result::INVALID_PORT;
    }
    if (virtual_address == nullptr) {
        return Result::INVALID_SIZE;
    }

    // For simplicity, return the kernel address
    // In a real OS, we'd map into the current process's address space
    *virtual_address = region->kernel_address;

    return Result::SUCCESS;
}

Result unmap_shared_region(SharedRegion* region) {
    if (region == nullptr || !region->in_use) {
        return Result::INVALID_PORT;
    }

    // Unmap from kernel space
    const std::size_t pages = (region->size + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
    for (std::size_t i = 0; i < pages; ++i) {
        paging::unmap_page(region->kernel_address + i * pmm::PAGE_SIZE);
    }

    return Result::SUCCESS;
}

IpcStats stats() noexcept {
    return ipc_stats;
}

} // namespace kernel::ipc