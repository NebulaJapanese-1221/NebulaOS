// Virtual Memory Manager Implementation for NebulaOS
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

#include "paging_new.hpp"
#include "pmm_new.hpp"
#include "../drivers/serial.hpp"

extern "C" {
extern unsigned char kernel_image_start[];
extern unsigned char kernel_image_end[];
}

namespace kernel::memory::paging {

namespace {

// Page directory for kernel space
alignas(PAGE_SIZE) std::uint32_t kernel_page_directory[1024];

// Track allocated page tables (physical addresses)
std::uint32_t* page_tables[1024];

// State
std::size_t identity_mib = 0;
bool paging_enabled = false;
std::uintptr_t device_cursor = layout::DEVICE_WINDOW_BASE;

// Page fault handler
PageFaultHandler fault_handler = nullptr;

// Page table entry counts for statistics
std::size_t mapped_pages = 0;

// Helper: get directory index from virtual address
inline std::size_t directory_index(std::uintptr_t vaddr) noexcept {
    return (vaddr >> 22) & 0x3FF;
}

// Helper: get table index from virtual address
inline std::size_t table_index(std::uintptr_t vaddr) noexcept {
    return (vaddr >> 12) & 0x3FF;
}

// Helper: check if a table is empty
bool table_is_empty(const std::uint32_t* table) noexcept {
    for (std::size_t i = 0; i < 1024; ++i) {
        if ((table[i] & PAGE_PRESENT) != 0) {
            return false;
        }
    }
    return true;
}

// Helper: get or create a page table for a directory entry
std::uint32_t* get_or_create_table(std::size_t dir_index,
                                       std::uint32_t dir_flags) {
    if (page_tables[dir_index] != nullptr) {
        return page_tables[dir_index];
    }

    // Allocate a new page table frame
    const auto frame = pmm::allocate_frame(pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        return nullptr;
    }

    auto* table = reinterpret_cast<std::uint32_t*>(frame.value);
    // Zero the table
    for (std::size_t i = 0; i < 1024; ++i) {
        table[i] = 0;
    }

    page_tables[dir_index] = table;
    kernel_page_directory[dir_index] = frame.value | dir_flags | PAGE_PRESENT;
    return table;
}

// Helper: extract flags from a page table entry
inline std::uint32_t extract_flags(std::uint32_t entry) noexcept {
    return entry & 0xFFF;
}

} // namespace

bool initialize(std::size_t identity_mib) {
    if (identity_mib == 0) {
        identity_mib = 1;
    }
    // Cap at 256 MiB for safety
    if (identity_mib > 256) {
        identity_mib = 256;
    }

    // Clear the kernel page directory
    for (std::size_t i = 0; i < 1024; ++i) {
        kernel_page_directory[i] = 0;
        page_tables[i] = nullptr;
    }

    device_cursor = layout::DEVICE_WINDOW_BASE;
    mapped_pages = 0;

    // Identity map the low memory
    const std::uintptr_t span = static_cast<std::uintptr_t>(identity_mib) * 1024 * 1024;
    for (std::uintptr_t addr = 0; addr < span; addr += PAGE_SIZE) {
        if (!map_page(addr, addr, PAGE_PRESENT | PAGE_WRITABLE)) {
            return false;
        }
    }

    // Mirror the kernel image at KERNEL_BASE
    const std::uintptr_t image_start = reinterpret_cast<std::uintptr_t>(kernel_image_start);
    const std::uintptr_t image_end = reinterpret_cast<std::uintptr_t>(kernel_image_end);
    for (std::uintptr_t addr = image_start; addr < image_end; addr += PAGE_SIZE) {
        if (!map_page(addr + layout::KERNEL_BASE, addr,
                      PAGE_PRESENT | PAGE_WRITABLE)) {
            return false;
        }
    }

    identity_mib = identity_mib;
    return true;
}

bool enable() {
    // Set CR3 to the kernel page directory
    const std::uintptr_t pd_phys = reinterpret_cast<std::uintptr_t>(kernel_page_directory);
    asm volatile("mov %0, %%cr3" : : "r"(pd_phys));

    // Enable paging (PG bit) and write protection (WP bit)
    std::uintptr_t cr0 = 0;
    asm volatile("mov %%cr0, %0" : "=r"(cr0));
    cr0 |= (1U << 31) | (1U << 16);
    asm volatile("mov %0, %%cr0" : : "r"(cr0));

    paging_enabled = true;
    return true;
}

bool is_enabled() noexcept {
    return paging_enabled;
}

std::size_t identity_megabytes() noexcept {
    return identity_mib;
}

Result<AddressSpace> create_address_space() {
    // Allocate a new page directory
    const auto frame = pmm::allocate_frame(pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        return {{}, false};
    }

    AddressSpace space;
    space.page_directory_phys = frame.value;
    space.kernel_cr3 = reinterpret_cast<std::uintptr_t>(kernel_page_directory);

    auto* dir = reinterpret_cast<std::uint32_t*>(frame.value);
    // Copy kernel mappings from the kernel page directory
    // (entries 768-1023 cover the high half)
    for (std::size_t i = 768; i < 1024; ++i) {
        dir[i] = kernel_page_directory[i];
    }

    return {space, true};
}

void destroy_address_space(AddressSpace& space) {
    if (space.page_directory_phys == 0) {
        return;
    }
    // Free the page directory frame
    pmm::free_frame(space.page_directory_phys);
    space.page_directory_phys = 0;
}

void switch_address_space(const AddressSpace& space) {
    asm volatile("mov %0, %%cr3" : : "r"(space.page_directory_phys));
}

bool map_page(std::uintptr_t virtual_addr, std::uintptr_t physical_addr,
              std::uint32_t flags) {
    const std::size_t dir_idx = directory_index(virtual_addr);
    const std::size_t tbl_idx = table_index(virtual_addr);

    // Determine directory flags (user bit propagates)
    const std::uint32_t dir_flags = (flags & PAGE_USER) ? PAGE_USER : 0;

    std::uint32_t* table = nullptr;
    if (paging_enabled) {
        // If paging is enabled, we're modifying the active directory
        table = get_or_create_table(dir_idx, dir_flags);
    } else {
        // Before paging is enabled, only kernel directory exists
        table = get_or_create_table(dir_idx, dir_flags);
    }

    if (table == nullptr) {
        return false;
    }

    // Check if this page was previously unmapped
    const bool was_mapped = (table[tbl_idx] & PAGE_PRESENT) != 0;

    table[tbl_idx] = (physical_addr & PAGE_MASK) | flags | PAGE_PRESENT;

    if (!was_mapped) {
        ++mapped_pages;
    }

    // Invalidate the TLB entry for this address
    asm volatile("invlpg (%0)" : : "r"(virtual_addr) : "memory");

    return true;
}

bool unmap_page(std::uintptr_t virtual_addr) {
    const std::size_t dir_idx = directory_index(virtual_addr);
    const std::size_t tbl_idx = table_index(virtual_addr);

    std::uint32_t* table = page_tables[dir_idx];
    if (table == nullptr) {
        return false;
    }

    const bool was_mapped = (table[tbl_idx] & PAGE_PRESENT) != 0;
    table[tbl_idx] = 0;

    if (was_mapped) {
        --mapped_pages;
    }

    // If the table is now empty, free it
    if (table_is_empty(table)) {
        kernel_page_directory[dir_idx] = 0;
        page_tables[dir_idx] = nullptr;
        pmm::free_frame(reinterpret_cast<std::uintptr_t>(table));
    }

    asm volatile("invlpg (%0)" : : "r"(virtual_addr) : "memory");
    return true;
}

std::uintptr_t translate(std::uintptr_t virtual_addr) noexcept {
    const std::size_t dir_idx = directory_index(virtual_addr);
    const std::size_t tbl_idx = table_index(virtual_addr);

    const std::uint32_t dir_entry = kernel_page_directory[dir_idx];
    if ((dir_entry & PAGE_PRESENT) == 0) {
        return 0;
    }

    const auto* table = reinterpret_cast<const std::uint32_t*>(dir_entry & PAGE_MASK);
    const std::uint32_t entry = table[tbl_idx];
    if ((entry & PAGE_PRESENT) == 0) {
        return 0;
    }

    return (entry & PAGE_MASK) | (virtual_addr & ~PAGE_MASK);
}

bool map_device_range(std::uintptr_t physical_base, std::size_t length,
                      std::uintptr_t* virtual_base) {
    if (length == 0) {
        return false;
    }

    // Compute page-aligned span
    const std::uintptr_t first_page = physical_base & PAGE_MASK;
    const std::uintptr_t last_page = (physical_base + length + PAGE_SIZE - 1) & PAGE_MASK;
    const std::uintptr_t span = last_page - first_page;

    if (last_page > 0x100000000ULL) {
        return false;
    }
    if (device_cursor + span > layout::DEVICE_WINDOW_LIMIT) {
        return false;
    }

    const std::uintptr_t base = device_cursor;
    for (std::uintptr_t addr = first_page; addr < last_page; addr += PAGE_SIZE) {
        if (!map_page(device_cursor, addr,
                      PAGE_PRESENT | PAGE_WRITABLE | PAGE_NO_CACHE)) {
            return false;
        }
        device_cursor += PAGE_SIZE;
    }

    if (virtual_base != nullptr) {
        *virtual_base = base;
    }
    return true;
}

bool map_range(std::uintptr_t virtual_base, std::size_t length,
               std::uintptr_t physical_base, std::uint32_t flags) {
    const std::uintptr_t end = virtual_base + length;
    std::uintptr_t phys = physical_base;

    for (std::uintptr_t addr = virtual_base; addr < end; addr += PAGE_SIZE) {
        if (!map_page(addr, phys, flags)) {
            return false;
        }
        phys += PAGE_SIZE;
    }
    return true;
}

void handle_page_fault(const PageFaultInfo& fault) {
    if (fault_handler != nullptr) {
        fault_handler(fault);
    }
}

void set_page_fault_handler(PageFaultHandler handler) {
    fault_handler = handler;
}

} // namespace kernel::memory::paging