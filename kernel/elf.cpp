// Dynamic Linker Implementation for NebulaOS
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

#include "elf.hpp"
#include "paging_new.hpp"
#include "pmm_new.hpp"
#include "heap_new.hpp"
#include "../drivers/serial.hpp"
#include <cstring>

namespace kernel::elf {

namespace {

// Loaded modules
constexpr std::size_t MAX_MODULES = 32;
ModuleInfo modules[MAX_MODULES];
std::size_t module_count_value = 0;

// Dynamic linker initialized
bool elf_initialized = false;

// Helper: check if a pointer is valid
inline bool valid_pointer(const void* ptr, std::size_t size,
                                 const void* data, std::size_t data_size) {
    if (ptr == nullptr) {
        return false;
    }
    const std::uintptr_t start = reinterpret_cast<std::uintptr_t>(ptr);
    const std::uintptr_t end = start + size;
    const std::uintptr_t data_start = reinterpret_cast<std::uintptr_t>(data);
    const std::uintptr_t data_end = data_start + data_size;
    return start >= data_start && end <= data_end;
}

// Helper: get symbol type
inline std::uint8_t symbol_type(const Elf32_Sym& sym) {
    return sym.st_info & 0xF;
}

// Helper: get symbol binding
inline std::uint8_t symbol_binding(const Elf32_Sym& sym) {
    return sym.st_info >> 4;
}

// Helper: get relocation type
inline std::uint8_t relocation_type(std::uint32_t info) {
    return info & 0xFF;
}

// Helper: get relocation symbol index
inline std::uint32_t relocation_symbol(std::uint32_t info) {
    return info >> 8;
}

// Helper: get the string at an offset in a string table
const char* get_string(const char* strtab, std::size_t strtab_size,
                            std::uint32_t offset) {
    if (strtab == nullptr || offset >= strtab_size) {
        return nullptr;
    }
    return strtab + offset;
}

} // namespace

bool initialize() {
    if (elf_initialized) {
        return true;
    }

    for (auto& module : modules) {
        module.base = 0;
        module.entry = 0;
        module.dynamic = 0;
        module.name = nullptr;
        module.handle = nullptr;
    }

    module_count_value = 0;
    elf_initialized = true;

    return true;
}

Result<ModuleInfo> load_executable(const void* data, std::size_t size,
                                       const char* name) {
    if (data == nullptr || size < sizeof(Elf32_Ehdr)) {
        return {{}, false};
    }

    const auto* ehdr = static_cast<const Elf32_Ehdr*>(data);

    // Check ELF magic
    if (ehdr->e_ident[0] != 0x7F ||
        ehdr->e_ident[1] != 'E' ||
        ehdr->e_ident[2] != 'L' ||
        ehdr->e_ident[3] != 'F') {
        return {{}, false};
    }

    // Check architecture
    if (ehdr->e_machine != EM_386) {
        return {{}, false};
    }

    // Check type
    if (ehdr->e_type != ET_EXEC && ehdr->e_type != ET_DYN) {
        return {{}, false};
    }

    // Validate program header table
    if (ehdr->e_phoff == 0 || ehdr->e_phentsize < sizeof(Elf32_Phdr)) {
        return {{}, false};
    }
    if (ehdr->e_phnum == 0 || ehdr->e_phnum > 64) {
        return {{}, false};
    }

    // Calculate the total memory needed
    std::uintptr_t min_addr = 0xFFFFFFFF;
    std::uintptr_t max_addr = 0;

    const auto* phdrs = reinterpret_cast<const Elf32_Phdr*>(
        reinterpret_cast<const std::uint8_t*>(data) + ehdr->e_phoff);

    for (std::uint32_t i = 0; i < ehdr->e_phnum; ++i) {
        const auto& phdr = phdrs[i];
        if (phdr.p_type != PT_LOAD) {
            continue;
        }
        if (phdr.p_vaddr < min_addr) {
            min_addr = phdr.p_vaddr;
        }
        if (phdr.p_vaddr + phdr.p_memsz > max_addr) {
            max_addr = phdr.p_vaddr + phdr.p_memsz;
        }
    }

    if (min_addr == 0xFFFFFFFF || max_addr == 0) {
        return {{}, false};
    }

    // Align the base address
    const std::uintptr_t load_size = max_addr - min_addr;
    const std::uintptr_t aligned_size =
        (load_size + pmm::PAGE_SIZE - 1) & pmm::PAGE_MASK;

    // Allocate frames for the executable
    const std::size_t pages = aligned_size / pmm::PAGE_SIZE;
    const auto frame = pmm::allocate_frames(pages, pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        return {{}, false};
    }

    const std::uintptr_t base = frame.value;

    // Map the executable into the kernel address space
    // (In a real implementation, we'd map into a user address space)
    std::uintptr_t virtual_base = 0;
    if (!paging::map_device_range(base, aligned_size, &virtual_base)) {
        pmm::free_frames(base, pages);
        return {{}, false};
    }

    // Load each PT_LOAD segment
    for (std::uint32_t i = 0; i < ehdr->e_phnum; ++i) {
        const auto& phdr = phdrs[i];
        if (phdr.p_type != PT_LOAD) {
            continue;
        }

        const std::uintptr_t dest = virtual_base + (phdr.p_vaddr - min_addr);
        const std::uintptr_t src = reinterpret_cast<std::uintptr_t>(data) + phdr.p_offset;

        // Copy file contents
        if (phdr.p_filesz > 0) {
            std::memcpy(reinterpret_cast<void*>(dest),
                           reinterpret_cast<const void*>(src),
                           phdr.p_filesz);
        }

        // Zero the BSS
        if (phdr.p_memsz > phdr.p_filesz) {
            std::memset(reinterpret_cast<void*>(dest + phdr.p_filesz), 0,
                           phdr.p_memsz - phdr.p_filesz);
        }
    }

    // Create module info
    ModuleInfo info;
    info.base = base;
    info.entry = virtual_base + (ehdr->e_entry - min_addr);
    info.dynamic = 0;
    info.name = name;
    info.handle = reinterpret_cast<void*>(virtual_base);

    // Find PT_DYNAMIC
    for (std::uint32_t i = 0; i < ehdr->e_phnum; ++i) {
        const auto& phdr = phdrs[i];
        if (phdr.p_type == PT_DYNAMIC) {
            info.dynamic = virtual_base + (phdr.p_vaddr - min_addr);
            break;
        }
    }

    // Register the module
    if (module_count_value < MAX_MODULES) {
        modules[module_count_value++] = info;
    }

    return {info, true};
}

Result<ModuleInfo> load_library(const void* data, std::size_t size,
                                    const char* name) {
    if (data == nullptr || size < sizeof(Elf32_Ehdr)) {
        return {{}, false};
    }

    const auto* ehdr = static_cast<const Elf32_Ehdr*>(data);

    // Check ELF magic
    if (ehdr->e_ident[0] != 0x7F ||
        ehdr->e_ident[1] != 'E' ||
        ehdr->e_ident[2] != 'L' ||
        ehdr->e_ident[3] != 'F') {
        return {{}, false};
    }

    // Check architecture
    if (ehdr->e_machine != EM_386) {
        return {{}, false};
    }

    // Check type (shared library is ET_DYN)
    if (ehdr->e_type != ET_DYN) {
        return {{}, false};
    }

    // Validate program header table
    if (ehdr->e_phoff == 0 || ehdr->e_phentsize < sizeof(Elf32_Phdr)) {
        return {{}, false};
    }
    if (ehdr->e_phnum == 0 || ehdr->e_phnum > 64) {
        return {{}, false};
    }

    // Calculate the total memory needed
    std::uintptr_t min_addr = 0xFFFFFFFF;
    std::uintptr_t max_addr = 0;

    const auto* phdrs = reinterpret_cast<const Elf32_Phdr*>(
        reinterpret_cast<const std::uint8_t*>(data) + ehdr->e_phoff);

    for (std::uint32_t i = 0; i < ehdr->e_phnum; ++i) {
        const auto& phdr = phdrs[i];
        if (phdr.p_type != PT_LOAD) {
            continue;
        }
        if (phdr.p_vaddr < min_addr) {
            min_addr = phdr.p_vaddr;
        }
        if (phdr.p_vaddr + phdr.p_memsz > max_addr) {
            max_addr = phdr.p_vaddr + phdr.p_memsz;
        }
    }

    if (min_addr == 0xFFFFFFFF || max_addr == 0) {
        return {{}, false};
    }

    // Align the base address
    const std::uintptr_t load_size = max_addr - min_addr;
    const std::uintptr_t aligned_size =
        (load_size + pmm::PAGE_SIZE - 1) & pmm::PAGE_MASK;

    // Allocate frames for the library
    const std::size_t pages = aligned_size / pmm::PAGE_SIZE;
    const auto frame = pmm::allocate_frames(pages, pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        return {{}, false};
    }

    const std::uintptr_t base = frame.value;

    // Map the library into the kernel address space
    std::uintptr_t virtual_base = 0;
    if (!paging::map_device_range(base, aligned_size, &virtual_base)) {
        pmm::free_frames(base, pages);
        return {{}, false};
    }

    // Load each PT_LOAD segment
    for (std::uint32_t i = 0; i < ehdr->e_phnum; ++i) {
        const auto& phdr = phdrs[i];
        if (phdr.p_type != PT_LOAD) {
            continue;
        }

        const std::uintptr_t dest = virtual_base + (phdr.p_vaddr - min_addr);
        const std::uintptr_t src = reinterpret_cast<std::uintptr_t>(data) + phdr.p_offset;

        // Copy file contents
        if (phdr.p_filesz > 0) {
            std::memcpy(reinterpret_cast<void*>(dest),
                           reinterpret_cast<const void*>(src),
                           phdr.p_filesz);
        }

        // Zero the BSS
        if (phdr.p_memsz > phdr.p_filesz) {
            std::memset(reinterpret_cast<void*>(dest + phdr.p_filesz), 0,
                           phdr.p_memsz - phdr.p_filesz);
        }
    }

    // Create module info
    ModuleInfo info;
    info.base = base;
    info.entry = 0;  // Libraries don't have an entry point
    info.dynamic = 0;
    info.name = name;
    info.handle = reinterpret_cast<void*>(virtual_base);

    // Find PT_DYNAMIC
    for (std::uint32_t i = 0; i < ehdr->e_phnum; ++i) {
        const auto& phdr = phdrs[i];
        if (phdr.p_type == PT_DYNAMIC) {
            info.dynamic = virtual_base + (phdr.p_vaddr - min_addr);
            break;
        }
    }

    // Apply relocations
    if (!apply_relocations(info, false)) {
        // Unmap and free
        const std::size_t rel_pages =
            (aligned_size + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
        pmm::free_frames(base, rel_pages);
        return {{}, false};
    }

    // Register the module
    if (module_count_value < MAX_MODULES) {
        modules[module_count_value++] = info;
    }

    return {info, true};
}

void unload_module(ModuleInfo& module) {
    if (module.base == 0) {
        return;
    }

    // Unregister from the module list
    for (std::size_t i = 0; i < module_count_value; ++i) {
        if (modules[i].base == module.base) {
            // Shift remaining modules
            for (std::size_t j = i; j < module_count_value - 1; ++j) {
                modules[j] = modules[j + 1];
            }
            --module_count_value;
            break;
        }
    }

    // Free the frames
    // (In a real implementation, we'd unmap the pages and free the frames)
    // For now, we just mark it as unloaded
    module.base = 0;
    module.entry = 0;
    module.dynamic = 0;
    module.handle = nullptr;
}

Result<void*> resolve_symbol(ModuleInfo& module, const char* name) {
    if (module.base == 0 || name == nullptr) {
        return {{}, false};
    }

    // Find the dynamic section
    if (module.dynamic == 0) {
        return {{}, false};
    }

    const auto* dynamic = reinterpret_cast<const Elf32_Dyn*>(module.dynamic);

    // Find the symbol table and string table
    const Elf32_Sym* symtab = nullptr;
    const char* strtab = nullptr;
    std::size_t sym_count = 0;
    std::size_t str_size = 0;

    for (std::size_t i = 0; dynamic[i].d_tag != DT_NULL; ++i) {
        switch (dynamic[i].d_tag) {
            case DT_SYMTAB:
                symtab = reinterpret_cast<const Elf32_Sym*>(
                    dynamic[i].d_un.d_ptr);
                break;
            case DT_STRTAB:
                strtab = reinterpret_cast<const char*>(
                    dynamic[i].d_un.d_ptr);
                break;
            case DT_STRSZ:
                str_size = dynamic[i].d_un.d_val;
                break;
            default:
                break;
        }
    }

    if (symtab == nullptr || strtab == nullptr) {
        return {{}, false};
    }

    // Count symbols (we don't have a direct count, so we estimate)
    // In a real implementation, we'd use the hash table
    sym_count = 1024;  // Conservative estimate

    // Lookup the symbol
    const Elf32_Sym* sym = lookup_symbol(symtab, strtab, sym_count, name);
    if (sym == nullptr) {
        return {{}, false};
    }

    // Return the address
    void* address = reinterpret_cast<void*>(
        module.base + sym->st_value);

    return {address, true};
}

Result<void*> get_symbol(const char* module_name, const char* name) {
    if (module_name == nullptr || name == nullptr) {
        return {{}, false};
    }

    // Find the module
    ModuleInfo* module = find_library(module_name);
    if (module == nullptr) {
        return {{}, false};
    }

    return resolve_symbol(*module, name);
}

void register_library(ModuleInfo& module) {
    if (module_count_value < MAX_MODULES) {
        modules[module_count_value++] = module;
    }
}

void unregister_library(ModuleInfo& module) {
    for (std::size_t i = 0; i < module_count_value; ++i) {
        if (modules[i].base == module.base) {
            // Shift remaining modules
            for (std::size_t j = i; j < module_count_value - 1; ++j) {
                modules[j] = modules[j + 1];
            }
            --module_count_value;
            break;
        }
    }
}

std::size_t module_count() noexcept {
    return module_count_value;
}

ModuleInfo* get_module(std::size_t index) noexcept {
    if (index >= module_count_value) {
        return nullptr;
    }
    return &modules[index];
}

ModuleInfo* find_library(const char* name) noexcept {
    if (name == nullptr) {
        return nullptr;
    }

    for (std::size_t i = 0; i < module_count_value; ++i) {
        if (modules[i].name != nullptr &&
            std::strcmp(modules[i].name, name) == 0) {
            return &modules[i];
        }
    }

    return nullptr;
}

bool apply_relocations(ModuleInfo& module, bool lazy) {
    if (module.base == 0 || module.dynamic == 0) {
        return false;
    }

    const auto* dynamic = reinterpret_cast<const Elf32_Dyn*>(module.dynamic);

    // Find relocation tables
    const Elf32_Rel* rel = nullptr;
    const Elf32_Rela* rela = nullptr;
    std::size_t rel_size = 0;
    std::size_t rela_size = 0;
    const Elf32_Sym* symtab = nullptr;
    const char* strtab = nullptr;

    for (std::size_t i = 0; dynamic[i].d_tag != DT_NULL; ++i) {
        switch (dynamic[i].d_tag) {
            case DT_REL:
                rel = reinterpret_cast<const Elf32_Rel*>(
                    dynamic[i].d_un.d_ptr);
                break;
            case DT_RELSZ:
                rel_size = dynamic[i].d_un.d_val;
                break;
            case DT_RELA:
                rela = reinterpret_cast<const Elf32_Rela*>(
                    dynamic[i].d_un.d_ptr);
                break;
            case DT_RELASZ:
                rela_size = dynamic[i].d_un.d_val;
                break;
            case DT_SYMTAB:
                symtab = reinterpret_cast<const Elf32_Sym*>(
                    dynamic[i].d_un.d_ptr);
                break;
            case DT_STRTAB:
                strtab = reinterpret_cast<const char*>(
                    dynamic[i].d_un.d_ptr);
                break;
            default:
                break;
        }
    }

    // Apply REL relocations
    if (rel != nullptr && rel_size > 0 && symtab != nullptr && strtab != nullptr) {
        const std::size_t rel_count = rel_size / sizeof(Elf32_Rel);

        for (std::size_t i = 0; i < rel_count; ++i) {
            const auto& r = rel[i];
            const std::uint8_t type = relocation_type(r.r_info);
            const std::uint32_t sym_idx = relocation_symbol(r.r_info);

            // Get the target address
            std::uintptr_t* target = reinterpret_cast<std::uintptr_t*>(
                module.base + r.r_offset);

            switch (type) {
                case R_386_NONE:
                    break;

                case R_386_32: {
                    // S + A
                    const Elf32_Sym& sym = symtab[sym_idx];
                    *target += module.base + sym.st_value;
                    break;
                }

                case R_386_PC32: {
                    // S + A - P
                    const Elf32_Sym& sym = symtab[sym_idx];
                    *target += module.base + sym.st_value -
                                   reinterpret_cast<std::uintptr_t>(target);
                    break;
                }

                case R_386_GLOB_DAT:
                case R_386_JMP_SLOT: {
                    // S
                    const Elf32_Sym& sym = symtab[sym_idx];
                    if (sym.st_shndx != SHN_UNDEF) {
                        *target = module.base + sym.st_value;
                    } else {
                        // External symbol - need to resolve from other modules
                        const char* name = strtab + sym.st_name;
                        auto result = get_symbol(nullptr, name);
                        if (result.ok) {
                            *target = reinterpret_cast<std::uintptr_t>(result.value);
                        }
                    }
                    break;
                }

                case R_386_RELATIVE: {
                    // B + A
                    *target = module.base + r.r_addend;
                    break;
                }

                default:
                    // Unknown relocation type
                    break;
            }
        }
    }

    // Apply RELA relocations
    if (rela != nullptr && rela_size > 0 && symtab != nullptr && strtab != nullptr) {
        const std::size_t rela_count = rela_size / sizeof(Elf32_Rela);

        for (std::size_t i = 0; i < rela_count; ++i) {
            const auto& r = rela[i];
            const std::uint8_t type = relocation_type(r.r_info);
            const std::uint32_t sym_idx = relocation_symbol(r.r_info);

            std::uintptr_t* target = reinterpret_cast<std::uintptr_t*>(
                module.base + r.r_offset);

            switch (type) {
                case R_386_NONE:
                    break;

                case R_386_32: {
                    const Elf32_Sym& sym = symtab[sym_idx];
                    *target = module.base + sym.st_value + r.r_addend;
                    break;
                }

                case R_386_PC32: {
                    const Elf32_Sym& sym = symtab[sym_idx];
                    *target = module.base + sym.st_value + r.r_addend -
                                   reinterpret_cast<std::uintptr_t>(target);
                    break;
                }

                case R_386_GLOB_DAT:
                case R_386_JMP_SLOT: {
                    const Elf32_Sym& sym = symtab[sym_idx];
                    if (sym.st_shndx != SHN_UNDEF) {
                        *target = module.base + sym.st_value;
                    } else {
                        const char* name = strtab + sym.st_name;
                        auto result = get_symbol(nullptr, name);
                        if (result.ok) {
                            *target = reinterpret_cast<std::uintptr_t>(result.value);
                        }
                    }
                    break;
                }

                case R_386_RELATIVE: {
                    *target = module.base + r.r_addend;
                    break;
                }

                default:
                    break;
            }
        }
    }

    (void)lazy;
    return true;
}

std::uint32_t elf_hash(const char* name) noexcept {
    if (name == nullptr) {
        return 0;
    }

    std::uint32_t hash = 0;
    while (*name != '\0') {
        hash = (hash << 4) + static_cast<unsigned char>(*name++);
        const std::uint32_t g = hash & 0xF0000000;
        if (g != 0) {
            hash ^= g >> 24;
        }
        hash &= ~g;
    }

    return hash;
}

const Elf32_Sym* lookup_symbol(const Elf32_Sym* symtab,
                                    const char* strtab,
                                    std::size_t sym_count,
                                    const char* name) noexcept {
    if (symtab == nullptr || strtab == nullptr || name == nullptr) {
        return nullptr;
    }

    for (std::size_t i = 0; i < sym_count; ++i) {
        const auto& sym = symtab[i];
        if (sym.st_name == 0) {
            continue;
        }

        const char* sym_name = strtab + sym.st_name;
        if (std::strcmp(sym_name, name) == 0) {
            // Check if it's a defined symbol
            if (sym.st_shndx != SHN_UNDEF ||
                symbol_binding(sym) == STB_WEAK) {
                return &sym;
            }
        }
    }

    return nullptr;
}

} // namespace kernel::elf