// Initramfs filesystem for NebulaOS
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

#include "../vfs.hpp"
#include "../cpio.hpp"

namespace kernel::initramfs {

// A node in the initramfs tree. The archive is a flat list of
// entries with full paths, so the tree is built once at mount
// time and the file data stays where the bootloader put it.
struct Node {
    char name[vfs::MAX_NAME_LENGTH + 1];
    vfs::FileType type;
    std::uint32_t mode;
    std::uint32_t uid;
    std::uint32_t gid;
    std::uint64_t size;
    std::uint64_t mtime;
    const unsigned char* data;

    Node* parent;
    Node* children;
    Node* next;
};

// Builds the tree from a cpio archive and returns its root.
// Returns null when the archive is not a valid newc image.
Node* build(const unsigned char* archive, std::size_t size);

// Frees the tree built by build. The archive itself is not
// touched, because the nodes only point into it.
void destroy(Node* root);

// Mounts the archive at the given target path.
int mount(const unsigned char* archive, std::size_t size,
              const char* target);

// The filesystem descriptor handed to the VFS.
extern vfs::Filesystem filesystem;

// File operations for initramfs nodes.
extern vfs::FileOperations operations;

} // namespace kernel::initramfs