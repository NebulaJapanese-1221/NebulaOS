// Virtual Memory Manager for NebulaOS
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

namespace kernel::memory::paging {

// Page size
constexpr std::uintptr_t PAGE_SIZE = 4096;
constexpr std::uintptr_t PAGE_SHIFT = 12;
constexpr std::uintptr_t PAGE_MASK = ~(PAGE_SIZE - 1);

// Page table entry flags
enum PageFlags : std::uint32_t {
    PAGE_NONE        = 0,
    PAGE_PRESENT     = 1 << 0,
    PAGE_WRITABLE    = 1 << 1,
    PAGE_USER        = 1 << 2,
    PAGE_WRITE_THROUGH = 1 << 3,
    PAGE_NO_CACHE    = 1 << 4,
    PAGE_ACCESSED    = 1 << 5,
    PAGE_DIRTY       = 1 << 6,
    PAGE_4MB         = 1 << 7,  // 4 MiB page (PSE)
    PAGE_GLOBAL      = 1 << 8,
};

// Address space structure
struct AddressSpace {
    std::uintptr_t page_directory_phys;  // Physical address of page directory
    std::uintptr_t kernel_cr3;           // Saved CR3 for kernel space
};

// Result type
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

using MapResult = Result<void*>;

// Initialize paging subsystem
bool initialize(std::size_t identity_megabytes);

// Enable paging (activate the page directory)
bool enable();

// Check if paging is enabled
bool is_enabled() noexcept;

// Get current identity mapping size in MiB
std::size_t identity_megabytes() noexcept;

// Create a new address space (for user processes)
Result<AddressSpace> create_address_space();

// Destroy an address space and free its resources
void destroy_address_space(AddressSpace& space);

// Switch to a different address space
void switch_address_space(const AddressSpace& space);

// Map a virtual address to a physical address
bool map_page(std::uintptr_t virtual_addr, std::uintptr_t physical_addr,
              std::uint32_t flags);

// Unmap a virtual address
bool unmap_page(std::uintptr_t virtual_addr);

// Translate a virtual address to physical
std::uintptr_t translate(std::uintptr_t virtual_addr) noexcept;

// Map a physical range into the device window (high memory)
bool map_device_range(std::uintptr_t physical_base, std::size_t length,
                      std::uintptr_t* virtual_base);

// Map a range of virtual addresses to contiguous physical frames
bool map_range(std::uintptr_t virtual_base, std::size_t length,
               std::uintptr_t physical_base, std::uint32_t flags);

// Get page fault information
struct PageFaultInfo {
    std::uintptr_t address;    // Faulting address (CR2)
    std::uint32_t error_code;  // CPU error code
    bool present;              // Page was present (protection fault)
    bool write;                // Write access caused fault
    bool user;                 // User mode caused fault
    bool instruction_fetch;    // Instruction fetch caused fault
};

// Handle a page fault (called from interrupt handler)
void handle_page_fault(const PageFaultInfo& fault);

// Register a page fault handler for user space
using PageFaultHandler = void (*)(const PageFaultInfo&);
void set_page_fault_handler(PageFaultHandler handler);

// Virtual address space layout
namespace layout {
    constexpr std::uintptr_t KERNEL_BASE = 0xC0000000;  // 3 GiB
    constexpr std::uintptr_t USER_BASE = 0x00000000;    // 0
    constexpr std::uintptr_t USER_TOP = 0xBFFFFFFF;     // 3 GiB - 1
    constexpr std::uintptr_t DEVICE_WINDOW_BASE = 0xF0000000;  // High device mapping
    constexpr std::uintptr_t DEVICE_WINDOW_LIMIT = 0xF8000000;
} // namespace layout

} // namespace kernel::memory::paging