// Multiboot 1 structures shared by the NebulaOS x86 kernel.
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

namespace kernel::multiboot {

const unsigned int handoff_magic = 0x2BADB002;
const unsigned int flag_memory_map = 1U << 0;
const unsigned int flag_framebuffer = 1U << 12;

const unsigned int map_type_available = 1;

// Field offsets follow the Multiboot 1 specification exactly. Keeping a
// single definition here avoids silent ABI drift between the framebuffer and
// memory managers, which would otherwise misread the boot information.
struct __attribute__((packed)) Information {
    unsigned int flags;
    unsigned int memory_lower;
    unsigned int memory_upper;
    unsigned int boot_device;
    unsigned int command_line;
    unsigned int modules_count;
    unsigned int modules_address;
    unsigned int symbols[4];
    unsigned int memory_map_length;
    unsigned int memory_map_address;
    unsigned int drives_length;
    unsigned int drives_address;
    unsigned int configuration_table;
    unsigned int boot_loader_name;
    unsigned int apm_table;
    unsigned int vbe_control_info;
    unsigned int vbe_mode_info;
    unsigned short vbe_mode;
    unsigned short vbe_interface_segment;
    unsigned short vbe_interface_offset;
    unsigned short vbe_interface_length;
    unsigned long long framebuffer_address;
    unsigned int framebuffer_pitch;
    unsigned int framebuffer_width;
    unsigned int framebuffer_height;
    unsigned char framebuffer_bpp;
    unsigned char framebuffer_type;
    unsigned char red_position;
    unsigned char red_mask_size;
    unsigned char green_position;
    unsigned char green_mask_size;
    unsigned char blue_position;
    unsigned char blue_mask_size;
};

struct __attribute__((packed)) MapEntry {
    unsigned int size;
    unsigned long long base;
    unsigned long long length;
    unsigned int type;
};

inline const Information* information(unsigned int address) {
    if (address == 0) {
        return nullptr;
    }
    return reinterpret_cast<const Information*>(address);
}

}