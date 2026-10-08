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

#include "initramfs.hpp"
#include "../heap.hpp"
#include "../../drivers/serial.hpp"

namespace kernel::initramfs {

namespace {

// Copies a bounded string into a fixed size buffer and
// always null terminates it.
void copy_name(char* destination, const char* source, std::size_t limit) {
    std::size_t index = 0;
    while (index < limit && source[index] != '\0') {
        destination[index] = source[index];
        ++index;
    }
    destination[index] = '\0';
}

// Maps the cpio mode word onto the VFS file type.
vfs::FileType type_from_mode(std::uint32_t mode) {
    switch (mode & 0xF000) {
    case 0x4000: return vfs::FileType::DIRECTORY;
    case 0xA000: return vfs::FileType::SYMLINK;
    case 0x2000: return vfs::FileType::CHARDEV;
    case 0x6000: return vfs::FileType::BLOCKDEV;
    case 0x1000: return vfs::FileType::FIFO;
    case 0xC000: return vfs::FileType::SOCKET;
    case 0x8000:
    default:     return vfs::FileType::REGULAR;
    }
}

// Allocates and zeroes a node.
Node* allocate_node() {
    void* raw = memory::heap::allocate(sizeof(Node));
    if (raw == nullptr) {
        return nullptr;
    }
    Node* node = static_cast<Node*>(raw);
    for (std::size_t byte = 0; byte < sizeof(Node); ++byte) {
        static_cast<unsigned char*>(raw)[byte] = 0;
    }
    return node;
}

// Finds or creates the child with the given name under parent.
Node* find_or_create_child(Node* parent, const char* name,
                                    vfs::FileType type) {
    for (Node* child = parent->children; child != nullptr;
         child = child->next) {
        if (strcmp(child->name, name) == 0) {
            return child;
        }
    }

    Node* child = allocate_node();
    if (child == nullptr) {
        return nullptr;
    }
    copy_name(child->name, name, vfs::MAX_NAME_LENGTH);
    child->type = type;
    child->mode = type == vfs::FileType::DIRECTORY ? 0755 : 0644;
    child->parent = parent;
    child->next = parent->children;
    parent->children = child;
    return child;
}

// Splits a full path into its components and creates the
// intermediate directories, then returns the deepest node.
// The final component is created with the given type.
Node* ensure_path(Node* root, const char* path, vfs::FileType type,
                       std::uint32_t mode, std::uint32_t uid,
                       std::uint32_t gid, std::uint64_t size,
                       std::uint64_t mtime,
                       const unsigned char* data) {
    char buffer[vfs::MAX_PATH_LENGTH];
    copy_name(buffer, path, vfs::MAX_PATH_LENGTH - 1);

    Node* current = root;
    char* component = buffer;
    while (*component == '/') {
        ++component;
    }

    while (*component != '\0') {
        char* slash = component;
        while (*slash != '\0' && *slash != '/') {
            ++slash;
        }
        const bool last = (*slash == '\0');
        const char saved = *slash;
        *slash = '\0';

        if (*component != '\0') {
            if (last) {
                Node* child = find_or_create_child(current, component, type);
                if (child == nullptr) {
                    return nullptr;
                }
                child->type = type;
                child->mode = mode;
                child->uid = uid;
                child->gid = gid;
                child->size = size;
                child->mtime = mtime;
                child->data = data;
                return child;
            }
            current = find_or_create_child(current, component,
                                           vfs::FileType::DIRECTORY);
            if (current == nullptr) {
                return nullptr;
            }
        }

        *slash = saved;
        component = slash;
        while (*component == '/') {
            ++component;
        }
    }

    return current;
}

// Counts the entries in the archive so the tree can be built
// without reallocating. The count is only used for reporting.
std::size_t count_entries(const unsigned char* archive,
                                    std::size_t size) {
    std::size_t count = 0;
    const unsigned char* ptr = archive;
    const unsigned char* end = archive + size;
    while (ptr + sizeof(cpio::Header) <= end) {
        if (!cpio::is_valid(ptr, end - ptr)) {
            break;
        }
        const cpio::Header* header =
            reinterpret_cast<const cpio::Header*>(ptr);
        const std::size_t namesize =
            cpio::parse_octal(header->namesize, 8);
        const std::size_t filesize =
            cpio::parse_octal(header->filesize, 8);
        const char* name =
            reinterpret_cast<const char*>(ptr + sizeof(cpio::Header));
        const bool trailer = namesize > 0 &&
            strcmp(name, "TRAILER!!!") == 0;
        if (trailer) {
            break;
        }
        ++count;
        std::size_t offset = sizeof(cpio::Header) + namesize;
        offset = (offset + 3) & ~static_cast<std::size_t>(3);
        offset += filesize;
        offset = (offset + 3) & ~static_cast<std::size_t>(3);
        if (offset == 0) {
            break;
        }
        ptr += offset;
    }
    return count;
}

// Walks the archive once and inserts every entry into the tree.
bool populate(Node* root, const unsigned char* archive,
                       std::size_t size) {
    const unsigned char* ptr = archive;
    const unsigned char* end = archive + size;
    std::size_t entries = 0;

    while (ptr + sizeof(cpio::Header) <= end) {
        if (!cpio::is_valid(ptr, end - ptr)) {
            break;
        }
        const cpio::Header* header =
            reinterpret_cast<const cpio::Header*>(ptr);
        const std::uint32_t mode =
            cpio::parse_octal(header->mode, 8);
        const std::uint32_t uid =
            cpio::parse_octal(header->uid, 8);
        const std::uint32_t gid =
            cpio::parse_octal(header->gid, 8);
        const std::uint32_t nlink =
            cpio::parse_octal(header->nlink, 8);
        const std::uint64_t mtime =
            cpio::parse_octal(header->mtime, 8);
        const std::uint64_t filesize =
            cpio::parse_octal(header->filesize, 8);
        const std::uint32_t rdevmajor =
            cpio::parse_octal(header->rdevmajor, 8);
        const std::uint32_t rdevminor =
            cpio::parse_octal(header->rdevminor, 8);
        const std::size_t namesize =
            cpio::parse_octal(header->namesize, 8);

        const char* name =
            reinterpret_cast<const char*>(ptr + sizeof(cpio::Header));

        const unsigned char* data = ptr + sizeof(cpio::Header) + namesize;
        data = reinterpret_cast<const unsigned char*>(
            (reinterpret_cast<std::size_t>(data) + 3) &
            ~static_cast<std::size_t>(3));

        const bool trailer = namesize > 0 &&
            strcmp(name, "TRAILER!!!") == 0;
        if (trailer) {
            break;
        }

        // Strip a leading "./" so archive entries resolve the
        // same way whether or not the packer emitted it.
        const char* clean = name;
        while (clean[0] == '.' && clean[1] == '/') {
            clean += 2;
        }

        if (clean[0] != '\0' && strcmp(clean, ".") != 0) {
            const vfs::FileType type = type_from_mode(mode);
            Node* node = ensure_path(root, clean, type, mode, uid, gid,
                                     filesize, mtime,
                                     type == vfs::FileType::REGULAR
                                         ? data : nullptr);
            if (node != nullptr) {
                // Character and block devices carry their device
                // number in the rdev fields, which the VFS reports
                // through Stat::rdev.
                if (type == vfs::FileType::CHARDEV ||
                    type == vfs::FileType::BLOCKDEV) {
                    node->size = filesize;
                    (void)rdevmajor;
                    (void)rdevminor;
                    (void)nlink;
                }
                ++entries;
            }
        }

        std::size_t offset = sizeof(cpio::Header) + namesize;
        offset = (offset + 3) & ~static_cast<std::size_t>(3);
        offset += filesize;
        offset = (offset + 3) & ~static_cast<std::size_t>(3);
        if (offset == 0) {
            break;
        }
        ptr += offset;
    }

    drivers::serial::write("[initramfs] mounted ");
    drivers::serial::write_decimal(entries);
    drivers::serial::write_line(" entries");
    return true;
}

int node_open(void* node, std::uint32_t flags) {
    (void)node;
    (void)flags;
    return 0;
}

int node_close(void* node) {
    (void)node;
    return 0;
}

ssize_t node_read(void* node, std::uint64_t offset,
                        void* buffer, std::size_t size) {
    const Node* file = static_cast<Node*>(node);
    if (file == nullptr || file->type != vfs::FileType::REGULAR) {
        return -1;
    }
    if (file->data == nullptr) {
        return 0;
    }
    if (offset >= file->size) {
        return 0;
    }
    std::size_t available =
        static_cast<std::size_t>(file->size - offset);
    if (size > available) {
        size = available;
    }
    unsigned char* destination = static_cast<unsigned char*>(buffer);
    for (std::size_t index = 0; index < size; ++index) {
        destination[index] = file->data[offset + index];
    }
    return static_cast<ssize_t>(size);
}

ssize_t node_write(void* node, std::uint64_t offset,
                         const void* buffer, std::size_t size) {
    (void)node;
    (void)offset;
    (void)buffer;
    (void)size;
    // The initramfs is read only: its data lives in the
    // bootloader loaded archive.
    return -1;
}

off_t node_seek(void* node, off_t offset, std::uint32_t whence) {
    (void)node;
    (void)offset;
    (void)whence;
    return -1;
}

int node_stat(void* node, vfs::Stat* stat) {
    const Node* file = static_cast<Node*>(node);
    if (file == nullptr || stat == nullptr) {
        return -1;
    }
    stat->dev = 1;
    stat->ino = 0;
    stat->type = file->type;
    stat->mode = file->mode;
    stat->nlink = 1;
    stat->uid = file->uid;
    stat->gid = file->gid;
    stat->rdev = 0;
    stat->size = file->size;
    stat->atime = file->mtime;
    stat->mtime = file->mtime;
    stat->ctime = file->mtime;
    stat->blksize = 4096;
    stat->blocks = (file->size + 4095) / 4096;
    return 0;
}

int node_ioctl(void* node, std::uint32_t request, void* arg) {
    (void)node;
    (void)request;
    (void)arg;
    return -1;
}

vfs::Dirent* node_readdir(void* node, std::uint32_t index) {
    const Node* directory = static_cast<Node*>(node);
    if (directory == nullptr ||
        directory->type != vfs::FileType::DIRECTORY) {
        return nullptr;
    }

    static vfs::Dirent entry;
    Node* child = directory->children;
    std::uint32_t position = 0;
    while (child != nullptr && position < index) {
        child = child->next;
        ++position;
    }
    if (child == nullptr) {
        return nullptr;
    }

    entry.ino = 0;
    entry.type = static_cast<std::uint32_t>(child->type);
    copy_name(entry.name, child->name, vfs::MAX_NAME_LENGTH);
    return &entry;
}

void* node_finddir(void* node, const char* name) {
    const Node* directory = static_cast<Node*>(node);
    if (directory == nullptr ||
        directory->type != vfs::FileType::DIRECTORY) {
        return nullptr;
    }
    for (Node* child = directory->children; child != nullptr;
         child = child->next) {
        if (strcmp(child->name, name) == 0) {
            return child;
        }
    }
    return nullptr;
}

int node_create(void* node, const char* name, std::uint32_t mode) {
    (void)node;
    (void)name;
    (void)mode;
    return -1;
}

int node_unlink(void* node, const char* name) {
    (void)node;
    (void)name;
    return -1;
}

int node_mkdir(void* node, const char* name, std::uint32_t mode) {
    (void)node;
    (void)name;
    (void)mode;
    return -1;
}

int node_rmdir(void* node, const char* name) {
    (void)node;
    (void)name;
    return -1;
}

int node_rename(void* node, const char* old_name,
                      const char* new_name) {
    (void)node;
    (void)old_name;
    (void)new_name;
    return -1;
}

int node_getattr(void* node, void* attr) {
    return node_stat(node, static_cast<vfs::Stat*>(attr));
}

int node_setattr(void* node, const void* attr) {
    (void)node;
    (void)attr;
    return -1;
}

int node_truncate(void* node, std::uint64_t length) {
    (void)node;
    (void)length;
    return -1;
}

int node_sync(void* node) {
    (void)node;
    return 0;
}

} // namespace

vfs::FileOperations operations = {
    node_open,
    node_close,
    node_read,
    node_write,
    node_seek,
    node_stat,
    node_ioctl,
    node_readdir,
    node_finddir,
    node_create,
    node_unlink,
    node_mkdir,
    node_rmdir,
    node_rename,
    node_getattr,
    node_setattr,
    node_truncate,
    node_sync,
};

namespace {

int fs_mount(const char* source, const char* target,
                   const char* filesystem, std::uint32_t flags,
                   const void* data) {
    (void)source;
    (void)filesystem;
    (void)flags;
    if (data == nullptr) {
        return -1;
    }
    // The data pointer carries a pointer to the mount
    // description: the archive pointer and its size.
    const Node* root = static_cast<const Node*>(data);
    (void)root;
    (void)target;
    return 0;
}

int fs_unmount(const char* target) {
    (void)target;
    return 0;
}

int fs_statfs(const char* path, void* buf) {
    (void)path;
    (void)buf;
    return -1;
}

} // namespace

vfs::Filesystem filesystem = {
    "initramfs",
    fs_mount,
    fs_unmount,
    fs_statfs,
    nullptr,
};

Node* build(const unsigned char* archive, std::size_t size) {
    if (archive == nullptr || size < sizeof(cpio::Header)) {
        return nullptr;
    }
    if (!cpio::is_valid(archive, size)) {
        return nullptr;
    }

    Node* root = allocate_node();
    if (root == nullptr) {
        return nullptr;
    }
    copy_name(root->name, "/", vfs::MAX_NAME_LENGTH);
    root->type = vfs::FileType::DIRECTORY;
    root->mode = 0755;
    root->size = count_entries(archive, size);

    if (!populate(root, archive, size)) {
        destroy(root);
        return nullptr;
    }
    return root;
}

void destroy(Node* root) {
    if (root == nullptr) {
        return;
    }
    Node* child = root->children;
    while (child != nullptr) {
        Node* next = child->next;
        destroy(child);
        child = next;
    }
    memory::heap::release(root);
}

int mount(const unsigned char* archive, std::size_t size,
              const char* target) {
    Node* root = build(archive, size);
    if (root == nullptr) {
        return -1;
    }
    const int result = vfs::mount("initramfs", target, "initramfs",
                                  0, root);
    if (result != 0) {
        destroy(root);
        return result;
    }
    return 0;
}

} // namespace kernel::initramfs

// strcmp is provided by the C library for userspace, but the
// kernel needs its own copy before the heap and the VFS come up.
extern "C" int strcmp(const char* a, const char* b) {
    while (*a && *a == *b) {
        ++a;
        ++b;
    }
    return static_cast<int>(
        *reinterpret_cast<const unsigned char*>(a) -
        *reinterpret_cast<const unsigned char*>(b));
}