// Virtual File System Implementation for NebulaOS
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

#include "vfs.hpp"
#include "heap.hpp"
#include "../drivers/serial.hpp"
#include <cstring>

namespace kernel::vfs {

namespace {

// Root node (from the initramfs or first mounted filesystem)
void* root_node = nullptr;
FileOperations* root_ops = nullptr;

// Mount list
Mount* mount_list = nullptr;
std::size_t mount_count_value = 0;

// Filesystem list
Filesystem* filesystem_list = nullptr;

// File descriptor table
FileDescriptor file_table[MAX_FILES];
std::size_t open_file_count_value = 0;

// Current working directory
char current_directory[MAX_PATH_LENGTH] = "/";

// VFS initialized
bool vfs_initialized = false;

// Helper: check if a character is a path separator
inline bool is_separator(char c) {
    return c == '/' || c == '\\';
}

// Helper: skip leading separators
inline const char* skip_separators(const char* path) {
    while (is_separator(*path)) {
        ++path;
    }
    return path;
}

// Helper: find the next path component
inline const char* next_component(const char* path) {
    while (*path != '\0' && !is_separator(*path)) {
        ++path;
    }
    return path;
}

// Helper: copy a path component
inline std::size_t copy_component(char* dest, std::size_t dest_size,
                                          const char* src) {
    std::size_t len = 0;
    while (len < dest_size - 1 && src[len] != '\0' &&
           !is_separator(src[len])) {
        dest[len] = src[len];
        ++len;
    }
    dest[len] = '\0';
    return len;
}

// Helper: resolve a path to a node
Result<void*> resolve_path(const char* path) {
    if (path == nullptr) {
        return {{}, false};
    }

    // Start from the root or current directory
    void* current = root_node;
    FileOperations* current_ops = root_ops;

    if (current == nullptr || current_ops == nullptr) {
        return {{}, false};
    }

    // If path is relative, start from current directory
    const char* p = path;
    if (!is_absolute(path)) {
        p = current_directory;
        // For simplicity, we only handle absolute paths in this implementation
        if (!is_absolute(p)) {
            return {{}, false};
        }
    }

    p = skip_separators(p);

    while (*p != '\0') {
        // Get the next component
        char component[MAX_NAME_LENGTH + 1];
        const char* end = next_component(p);
        std::size_t len = end - p;
        if (len > MAX_NAME_LENGTH) {
            len = MAX_NAME_LENGTH;
        }
        std::memcpy(component, p, len);
        component[len] = '\0';

        // Skip to the next component
        p = skip_separators(end);

        // Handle special components
        if (std::strcmp(component, ".") == 0) {
            continue;
        }
        if (std::strcmp(component, "..") == 0) {
            // Go to parent (not implemented in this simple version)
            continue;
        }

        // Look up the component in the current directory
        if (current_ops->finddir == nullptr) {
            return {{}, false};
        }

        void* next = current_ops->finddir(current, component);
        if (next == nullptr) {
            return {{}, false};
        }

        current = next;
    }

    return {current, true};
}

// Helper: find the mount point for a path
Mount* find_mount(const char* path) {
    if (path == nullptr) {
        return nullptr;
    }

    Mount* best = nullptr;
    std::size_t best_len = 0;

    for (Mount* mount = mount_list; mount != nullptr;
         mount = mount->next) {
        const std::size_t target_len = std::strlen(mount->target);
        if (std::strncmp(path, mount->target, target_len) == 0) {
            if (target_len > best_len) {
                best = mount;
                best_len = target_len;
            }
        }
    }

    return best;
}

// Helper: get the path relative to a mount point
const char* relative_path(const char* path,
                                 const char* mount_point) {
    const std::size_t mount_len = std::strlen(mount_point);
    const char* p = path + mount_len;
    while (*p != '\0' && is_separator(*p)) {
        ++p;
    }
    if (*p == '\0') {
        return "/";
    }
    return p;
}

// Helper: find an empty file descriptor slot
int find_empty_fd() {
    for (std::size_t i = 0; i < MAX_FILES; ++i) {
        if (!file_table[i].is_open) {
            return static_cast<int>(i);
        }
    }
    return -1;
}

} // namespace

bool initialize() {
    if (vfs_initialized) {
        return true;
    }

    root_node = nullptr;
    root_ops = nullptr;
    mount_list = nullptr;
    mount_count_value = 0;
    filesystem_list = nullptr;
    open_file_count_value = 0;

    std::memset(file_table, 0, sizeof(file_table));
    std::strcpy(current_directory, "/");

    vfs_initialized = true;

    return true;
}

int mount(const char* source, const char* target,
              const char* filesystem,
              std::uint32_t flags,
              const void* data) {
    if (source == nullptr || target == nullptr ||
        filesystem == nullptr) {
        return -1;
    }

    // Find the filesystem
    Filesystem* fs = nullptr;
    for (Filesystem* f = filesystem_list; f != nullptr;
         f = f->next) {
        if (std::strcmp(f->name, filesystem) == 0) {
            fs = f;
            break;
        }
    }

    if (fs == nullptr || fs->mount == nullptr) {
        return -1;
    }

    // Allocate a mount structure
    Mount* mount = static_cast<Mount*>(
        heap::allocate(sizeof(Mount)));
    if (mount == nullptr) {
        return -1;
    }

    std::memset(mount, 0, sizeof(Mount));

    std::strncpy(mount->source, source, MAX_PATH_LENGTH - 1);
    std::strncpy(mount->target, target, MAX_PATH_LENGTH - 1);
    std::strncpy(mount->filesystem, filesystem, 63);

    // Mount the filesystem
    int result = fs->mount(source, target, filesystem,
                                flags, data);
    if (result != 0) {
        heap::release(mount);
        return result;
    }

    // Get the root node and operations from the mount
    // (This is filesystem-specific, so we need a way to get them)
    // For now, we'll set them from the data parameter
    // (In a real implementation, we'd have a mount structure
    // returned by the filesystem)

    // Add to the mount list
    mount->next = mount_list;
    mount_list = mount;
    ++mount_count_value;

    return 0;
}

int unmount(const char* target) {
    if (target == nullptr) {
        return -1;
    }

    // Find the mount
    Mount* prev = nullptr;
    Mount* mount = mount_list;
    while (mount != nullptr) {
        if (std::strcmp(mount->target, target) == 0) {
            break;
        }
        prev = mount;
        mount = mount->next;
    }

    if (mount == nullptr) {
        return -1;
    }

    // Find the filesystem
    Filesystem* fs = nullptr;
    for (Filesystem* f = filesystem_list; f != nullptr;
         f = f->next) {
        if (std::strcmp(f->name, mount->filesystem) == 0) {
            fs = f;
            break;
        }
    }

    if (fs != nullptr && fs->unmount != nullptr) {
        fs->unmount(target);
    }

    // Remove from the mount list
    if (prev == nullptr) {
        mount_list = mount->next;
    } else {
        prev->next = mount->next;
    }

    heap::release(mount);
    --mount_count_value;

    return 0;
}

void register_filesystem(Filesystem* fs) {
    if (fs == nullptr) {
        return;
    }
    fs->next = filesystem_list;
    filesystem_list = fs;
}

void unregister_filesystem(Filesystem* fs) {
    if (fs == nullptr) {
        return;
    }

    if (filesystem_list == fs) {
        filesystem_list = fs->next;
    } else {
        for (Filesystem* f = filesystem_list; f != nullptr;
             f = f->next) {
            if (f->next == fs) {
                f->next = fs->next;
                break;
            }
        }
    }

    fs->next = nullptr;
}

Result<FileDescriptor*> open(const char* path,
                                  std::uint32_t flags) {
    if (path == nullptr) {
        return {{}, false};
    }

    // Resolve the path
    auto node_result = resolve_path(path);
    if (!node_result.ok) {
        // If the file doesn't exist and we're creating it,
        // we need to create it in the parent directory
        if ((flags & FLAG_CREATE) == 0) {
            return {{}, false};
        }

        // Get the parent directory
        auto parent_result = dirname(path);
        if (!parent_result.ok) {
            return {{}, false};
        }

        auto parent_node = resolve_path(parent_result.value);
        heap::release(parent_result.value);

        if (!parent_node.ok) {
            return {{}, false};
        }

        // Get the filename
        auto name_result = basename(path);
        if (!name_result.ok) {
            return {{}, false};
        }

        // Create the file
        if (root_ops == nullptr || root_ops->create == nullptr) {
            heap::release(name_result.value);
            return {{}, false};
        }

        void* node = nullptr;
        // Create in the parent directory
        // (In a real implementation, we'd use the parent's ops->create)
        int result = root_ops->create(parent_node.value,
                                           name_result.value,
                                           PERM_READ | PERM_WRITE);
        heap::release(name_result.value);

        if (result != 0) {
            return {{}, false};
        }

        // Resolve the newly created file
        node_result = resolve_path(path);
        if (!node_result.ok) {
            return {{}, false};
        }
    }

    // Find an empty file descriptor
    const int fd_index = find_empty_fd();
    if (fd_index < 0) {
        return {{}, false};
    }

    // Get the file operations for the node
    // (In a real implementation, we'd look up the filesystem
    // that contains the node)
    FileOperations* ops = root_ops;
    if (ops == nullptr) {
        return {{}, false};
    }

    // Open the file
    if (ops->open != nullptr) {
        int result = ops->open(node_result.value, flags);
        if (result != 0) {
            return {{}, false};
        }
    }

    // Initialize the file descriptor
    FileDescriptor* fd = &file_table[fd_index];
    fd->node = node_result.value;
    fd->ops = ops;
    fd->offset = 0;
    fd->flags = flags;
    fd->ref_count = 1;
    fd->is_open = true;

    ++open_file_count_value;

    return {fd, true};
}

int close(FileDescriptor* fd) {
    if (fd == nullptr || !fd->is_open) {
        return -1;
    }

    // Close the file
    if (fd->ops != nullptr && fd->ops->close != nullptr) {
        fd->ops->close(fd->node);
    }

    fd->is_open = false;
    fd->node = nullptr;
    fd->ops = nullptr;
    fd->offset = 0;
    fd->flags = 0;
    fd->ref_count = 0;

    --open_file_count_value;

    return 0;
}

ssize_t read(FileDescriptor* fd, void* buffer,
                  std::size_t size) {
    if (fd == nullptr || !fd->is_open || buffer == nullptr ||
        size == 0) {
        return -1;
    }

    if (fd->ops == nullptr || fd->ops->read == nullptr) {
        return -1;
    }

    // Check read permission
    if ((fd->flags & FLAG_READ) == 0) {
        return -1;
    }

    const ssize_t result = fd->ops->read(fd->node,
                                                fd->offset,
                                                buffer, size);
    if (result > 0) {
        fd->offset += result;
    }

    return result;
}

ssize_t write(FileDescriptor* fd, const void* buffer,
                   std::size_t size) {
    if (fd == nullptr || !fd->is_open || buffer == nullptr ||
        size == 0) {
        return -1;
    }

    if (fd->ops == nullptr || fd->ops->write == nullptr) {
        return -1;
    }

    // Check write permission
    if ((fd->flags & FLAG_WRITE) == 0) {
        return -1;
    }

    const ssize_t result = fd->ops->write(fd->node,
                                                 fd->offset,
                                                 buffer, size);
    if (result > 0) {
        fd->offset += result;
    }

    return result;
}

off_t seek(FileDescriptor* fd, off_t offset,
               std::uint32_t whence) {
    if (fd == nullptr || !fd->is_open) {
        return -1;
    }

    if (fd->ops != nullptr && fd->ops->seek != nullptr) {
        const off_t result = fd->ops->seek(fd->node, offset,
                                                  whence);
        if (result >= 0) {
            fd->offset = result;
        }
        return result;
    }

    // Simple seek implementation
    switch (whence) {
        case SEEK_SET:
            fd->offset = offset;
            break;
        case SEEK_CUR:
            fd->offset += offset;
            break;
        case SEEK_END: {
            // Get the file size
            Stat stat;
            if (fd->ops != nullptr && fd->ops->stat != nullptr) {
                if (fd->ops->stat(fd->node, &stat) == 0) {
                    fd->offset = stat.size + offset;
                }
            }
            break;
        }
        default:
            return -1;
    }

    return fd->offset;
}

int stat(const char* path, Stat* stat) {
    if (path == nullptr || stat == nullptr) {
        return -1;
    }

    auto node_result = resolve_path(path);
    if (!node_result.ok) {
        return -1;
    }

    if (root_ops == nullptr || root_ops->stat == nullptr) {
        return -1;
    }

    return root_ops->stat(node_result.value, stat);
}

int fstat(FileDescriptor* fd, Stat* stat) {
    if (fd == nullptr || !fd->is_open || stat == nullptr) {
        return -1;
    }

    if (fd->ops == nullptr || fd->ops->stat == nullptr) {
        return -1;
    }

    return fd->ops->stat(fd->node, stat);
}

Dirent* readdir(FileDescriptor* fd, std::uint32_t index) {
    if (fd == nullptr || !fd->is_open) {
        return nullptr;
    }

    if (fd->ops == nullptr || fd->ops->readdir == nullptr) {
        return nullptr;
    }

    return fd->ops->readdir(fd->node, index);
}

Result<FileDescriptor*> opendir(const char* path) {
    if (path == nullptr) {
        return {{}, false};
    }

    auto node_result = resolve_path(path);
    if (!node_result.ok) {
        return {{}, false};
    }

    // Check if it's a directory
    if (root_ops == nullptr || root_ops->stat == nullptr) {
        return {{}, false};
    }

    Stat stat;
    if (root_ops->stat(node_result.value, &stat) != 0) {
        return {{}, false};
    }

    if (stat.type != FileType::DIRECTORY) {
        return {{}, false};
    }

    // Find an empty file descriptor
    const int fd_index = find_empty_fd();
    if (fd_index < 0) {
        return {{}, false};
    }

    FileDescriptor* fd = &file_table[fd_index];
    fd->node = node_result.value;
    fd->ops = root_ops;
    fd->offset = 0;
    fd->flags = FLAG_READ;
    fd->ref_count = 1;
    fd->is_open = true;

    ++open_file_count_value;

    return {fd, true};
}

int create(const char* path, std::uint32_t mode) {
    if (path == nullptr) {
        return -1;
    }

    // Get the parent directory
    auto parent_result = dirname(path);
    if (!parent_result.ok) {
        return -1;
    }

    auto parent_node = resolve_path(parent_result.value);
    heap::release(parent_result.value);

    if (!parent_node.ok) {
        return -1;
    }

    // Get the filename
    auto name_result = basename(path);
    if (!name_result.ok) {
        return -1;
    }

    if (root_ops == nullptr || root_ops->create == nullptr) {
        heap::release(name_result.value);
        return -1;
    }

    int result = root_ops->create(parent_node.value,
                                       name_result.value,
                                       mode);
    heap::release(name_result.value);

    return result;
}

int unlink(const char* path) {
    if (path == nullptr) {
        return -1;
    }

    // Get the parent directory
    auto parent_result = dirname(path);
    if (!parent_result.ok) {
        return -1;
    }

    auto parent_node = resolve_path(parent_result.value);
    heap::release(parent_result.value);

    if (!parent_node.ok) {
        return -1;
    }

    // Get the filename
    auto name_result = basename(path);
    if (!name_result.ok) {
        return -1;
    }

    if (root_ops == nullptr || root_ops->unlink == nullptr) {
        heap::release(name_result.value);
        return -1;
    }

    int result = root_ops->unlink(parent_node.value,
                                       name_result.value);
    heap::release(name_result.value);

    return result;
}

int mkdir(const char* path, std::uint32_t mode) {
    if (path == nullptr) {
        return -1;
    }

    // Get the parent directory
    auto parent_result = dirname(path);
    if (!parent_result.ok) {
        return -1;
    }

    auto parent_node = resolve_path(parent_result.value);
    heap::release(parent_result.value);

    if (!parent_node.ok) {
        return -1;
    }

    // Get the filename
    auto name_result = basename(path);
    if (!name_result.ok) {
        return -1;
    }

    if (root_ops == nullptr || root_ops->mkdir == nullptr) {
        heap::release(name_result.value);
        return -1;
    }

    int result = root_ops->mkdir(parent_node.value,
                                      name_result.value,
                                      mode);
    heap::release(name_result.value);

    return result;
}

int rmdir(const char* path) {
    if (path == nullptr) {
        return -1;
    }

    // Get the parent directory
    auto parent_result = dirname(path);
    if (!parent_result.ok) {
        return -1;
    }

    auto parent_node = resolve_path(parent_result.value);
    heap::release(parent_result.value);

    if (!parent_node.ok) {
        return -1;
    }

    // Get the filename
    auto name_result = basename(path);
    if (!name_result.ok) {
        return -1;
    }

    if (root_ops == nullptr || root_ops->rmdir == nullptr) {
        heap::release(name_result.value);
        return -1;
    }

    int result = root_ops->rmdir(parent_node.value,
                                      name_result.value);
    heap::release(name_result.value);

    return result;
}

int rename(const char* old_path, const char* new_path) {
    if (old_path == nullptr || new_path == nullptr) {
        return -1;
    }

    if (root_ops == nullptr || root_ops->rename == nullptr) {
        return -1;
    }

    return root_ops->rename(root_node, old_path, new_path);
}

int truncate(const char* path, std::uint64_t length) {
    if (path == nullptr) {
        return -1;
    }

    auto node_result = resolve_path(path);
    if (!node_result.ok) {
        return -1;
    }

    if (root_ops == nullptr || root_ops->truncate == nullptr) {
        return -1;
    }

    return root_ops->truncate(node_result.value, length);
}

int sync(FileDescriptor* fd) {
    if (fd == nullptr || !fd->is_open) {
        return -1;
    }

    if (fd->ops == nullptr || fd->ops->sync == nullptr) {
        return -1;
    }

    return fd->ops->sync(fd->node);
}

int sync_all() {
    int result = 0;
    for (std::size_t i = 0; i < MAX_FILES; ++i) {
        if (file_table[i].is_open) {
            if (sync(&file_table[i]) != 0) {
                result = -1;
            }
        }
    }
    return result;
}

Result<void*> resolve(const char* path) {
    return resolve_path(path);
}

Result<char*> getcwd(char* buffer, std::size_t size) {
    if (buffer == nullptr || size == 0) {
        return {{}, false};
    }

    const std::size_t len = std::strlen(current_directory);
    if (len >= size) {
        return {{}, false};
    }

    std::memcpy(buffer, current_directory, len + 1);
    return {buffer, true};
}

int chdir(const char* path) {
    if (path == nullptr) {
        return -1;
    }

    auto node_result = resolve_path(path);
    if (!node_result.ok) {
        return -1;
    }

    // Check if it's a directory
    if (root_ops == nullptr || root_ops->stat == nullptr) {
        return -1;
    }

    Stat stat;
    if (root_ops->stat(node_result.value, &stat) != 0) {
        return -1;
    }

    if (stat.type != FileType::DIRECTORY) {
        return -1;
    }

    // Update the current directory
    std::strncpy(current_directory, path, MAX_PATH_LENGTH - 1);
    current_directory[MAX_PATH_LENGTH - 1] = '\0';

    return 0;
}

void* get_root() noexcept {
    return root_node;
}

bool is_absolute(const char* path) noexcept {
    if (path == nullptr) {
        return false;
    }
    return path[0] == '/';
}

Result<char*> normalize(const char* path) {
    if (path == nullptr) {
        return {{}, false};
    }

    char* result = static_cast<char*>(
        heap::allocate(MAX_PATH_LENGTH));
    if (result == nullptr) {
        return {{}, false};
    }

    std::size_t out = 0;
    const char* p = path;

    // Handle absolute paths
    if (is_separator(*p)) {
        result[out++] = '/';
        p = skip_separators(p);
    }

    while (*p != '\0') {
        // Get the next component
        char component[MAX_NAME_LENGTH + 1];
        const char* end = next_component(p);
        std::size_t len = end - p;
        if (len > MAX_NAME_LENGTH) {
            len = MAX_NAME_LENGTH;
        }
        std::memcpy(component, p, len);
        component[len] = '\0';

        p = skip_separators(end);

        // Handle special components
        if (std::strcmp(component, ".") == 0) {
            continue;
        }
        if (std::strcmp(component, "..") == 0) {
            // Go up one level
            if (out > 0) {
                while (out > 0 && result[out - 1] != '/') {
                    --out;
                }
                if (out > 0) {
                    --out;
                }
            }
            continue;
        }

        // Add the component
        if (out > 0 && result[out - 1] != '/') {
            result[out++] = '/';
        }
        for (std::size_t i = 0; i < len; ++i) {
            result[out++] = component[i];
        }
    }

    if (out == 0) {
        result[out++] = '/';
    }
    result[out] = '\0';

    return {result, true};
}

Result<char*> join(const char* base, const char* relative) {
    if (base == nullptr || relative == nullptr) {
        return {{}, false};
    }

    char* result = static_cast<char*>(
        heap::allocate(MAX_PATH_LENGTH));
    if (result == nullptr) {
        return {{}, false};
    }

    std::size_t out = 0;

    // Copy the base path
    for (std::size_t i = 0; base[i] != '\0' &&
         out < MAX_PATH_LENGTH - 1; ++i) {
        result[out++] = base[i];
    }

    // Add a separator if needed
    if (out > 0 && result[out - 1] != '/' &&
        !is_separator(relative[0])) {
        result[out++] = '/';
    }

    // Copy the relative path
    for (std::size_t i = 0; relative[i] != '\0' &&
         out < MAX_PATH_LENGTH - 1; ++i) {
        result[out++] = relative[i];
    }

    result[out] = '\0';

    return {result, true};
}

Result<char*> dirname(const char* path) {
    if (path == nullptr) {
        return {{}, false};
    }

    char* result = static_cast<char*>(
        heap::allocate(MAX_PATH_LENGTH));
    if (result == nullptr) {
        return {{}, false};
    }

    // Find the last separator
    const char* last_sep = nullptr;
    for (const char* p = path; *p != '\0'; ++p) {
        if (is_separator(*p)) {
            last_sep = p;
        }
    }

    if (last_sep == nullptr) {
        // No separator, return "."
        result[0] = '.';
        result[1] = '\0';
        return {result, true};
    }

    // Copy everything up to the last separator
    std::size_t len = last_sep - path;
    if (len == 0) {
        result[0] = '/';
        result[1] = '\0';
    } else {
        std::memcpy(result, path, len);
        result[len] = '\0';
    }

    return {result, true};
}

Result<char*> basename(const char* path) {
    if (path == nullptr) {
        return {{}, false};
    }

    char* result = static_cast<char*>(
        heap::allocate(MAX_PATH_LENGTH));
    if (result == nullptr) {
        return {{}, false};
    }

    // Find the last separator
    const char* last_sep = nullptr;
    for (const char* p = path; *p != '\0'; ++p) {
        if (is_separator(*p)) {
            last_sep = p + 1;
        }
    }

    const char* start = last_sep != nullptr ? last_sep : path;

    // Copy the basename
    std::size_t i = 0;
    while (start[i] != '\0' && i < MAX_PATH_LENGTH - 1) {
        result[i] = start[i];
        ++i;
    }
    result[i] = '\0';

    // Handle trailing separator
    if (result[0] == '\0') {
        result[0] = '/';
        result[1] = '\0';
    }

    return {result, true};
}

int allocate_fd(FileDescriptor* fd) {
    if (fd == nullptr) {
        return -1;
    }

    const int index = find_empty_fd();
    if (index < 0) {
        return -1;
    }

    file_table[index] = *fd;
    file_table[index].is_open = true;
    ++open_file_count_value;

    return index;
}

void release_fd(int fd) {
    if (fd < 0 || static_cast<std::size_t>(fd) >= MAX_FILES) {
        return;
    }

    if (file_table[fd].is_open) {
        file_table[fd].is_open = false;
        file_table[fd].node = nullptr;
        file_table[fd].ops = nullptr;
        file_table[fd].offset = 0;
        file_table[fd].flags = 0;
        file_table[fd].ref_count = 0;
        --open_file_count_value;
    }
}

FileDescriptor* get_fd(int fd) noexcept {
    if (fd < 0 || static_cast<std::size_t>(fd) >= MAX_FILES) {
        return nullptr;
    }
    if (!file_table[fd].is_open) {
        return nullptr;
    }
    return &file_table[fd];
}

std::size_t open_file_count() noexcept {
    return open_file_count_value;
}

std::size_t mount_count() noexcept {
    return mount_count_value;
}

Result<FilesystemStats> statfs(const char* path) {
    (void)path;
    return {{}, false};
}

} // namespace kernel::vfs