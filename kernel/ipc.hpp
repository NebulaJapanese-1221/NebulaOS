// IPC (Inter-Process Communication) for NebulaOS
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

namespace kernel::ipc {

// Maximum message size
constexpr std::size_t MAX_MESSAGE_SIZE = 4096;

// Maximum number of IPC ports
constexpr std::size_t MAX_PORTS = 256;

// Maximum number of shared memory regions
constexpr std::size_t MAX_SHARED_REGIONS = 64;

// Message types
enum class MessageType : std::uint32_t {
    NONE       = 0,
    REQUEST    = 1,
    RESPONSE   = 2,
    NOTIFY     = 3,
    SIGNAL     = 4,
};

// Message header
struct MessageHeader {
    std::uint32_t type;        // MessageType
    std::uint32_t source_id;   // Sender thread/process ID
    std::uint32_t target_id;   // Receiver thread/process ID
    std::uint32_t port;        // Port number
    std::size_t size;          // Payload size
};

// Message structure
struct Message {
    MessageHeader header;
    std::uint8_t data[MAX_MESSAGE_SIZE];
};

// Port structure (message queue)
struct Port {
    std::uint32_t id;
    bool in_use;

    // Message queue (circular buffer)
    Message* messages;
    std::size_t head;
    std::size_t tail;
    std::size_t count;
    std::size_t capacity;

    // Wait queue for receivers
    void* wait_queue;

    // Owner
    std::uint32_t owner_id;

    Port* next;
};

// Shared memory region
struct SharedRegion {
    std::uint32_t id;
    bool in_use;
    std::uintptr_t kernel_address;  // Kernel virtual address
    std::uintptr_t physical_address; // Physical address
    std::size_t size;
    std::uint32_t owner_id;
    std::uint32_t permissions;  // Bitmask of allowed threads
};

// IPC result codes
enum class Result : std::int32_t {
    SUCCESS        = 0,
    INVALID_PORT   = -1,
    INVALID_SIZE   = -2,
    NO_MEMORY      = -3,
    TIMEOUT        = -4,
    WOULD_BLOCK    = -5,
    DESTROYED      = -6,
};

// Initialize IPC subsystem
bool initialize();

// Create a port (message queue)
Port* create_port(std::uint32_t capacity);

// Destroy a port
void destroy_port(Port* port);

// Send a message to a port (blocking)
Result send(Port* port, const MessageHeader& header,
              const void* data, std::size_t size);

// Send a message to a port (non-blocking)
Result try_send(Port* port, const MessageHeader& header,
                  const void* data, std::size_t size);

// Receive a message from a port (blocking)
Result receive(Port* port, Message& out_message);

// Receive a message from a port (non-blocking)
Result try_receive(Port* port, Message& out_message);

// Receive a message with timeout
Result receive_timeout(Port* port, Message& out_message,
                         std::uint32_t timeout_ms);

// Reply to a message (send response back to sender)
Result reply(const Message& request, const void* data, std::size_t size);

// Shared memory
SharedRegion* create_shared_region(std::size_t size,
                                     std::uint32_t permissions);
void destroy_shared_region(SharedRegion* region);

// Map a shared region into a thread's address space
Result map_shared_region(SharedRegion* region,
                          std::uintptr_t* virtual_address);

// Unmap a shared region from a thread's address space
Result unmap_shared_region(SharedRegion* region);

// Get statistics
struct IpcStats {
    std::uint32_t total_ports;
    std::uint32_t total_messages;
    std::uint32_t total_shared_regions;
    std::uint32_t messages_sent;
    std::uint32_t messages_received;
};

IpcStats stats() noexcept;

} // namespace kernel::ipc