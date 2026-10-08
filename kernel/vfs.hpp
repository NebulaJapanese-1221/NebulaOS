// Virtual File System for NebulaOS
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

namespace kernel::vfs {

// Maximum path length
constexpr std::size_t MAX_PATH_LENGTH = 4096;

// Maximum filename length
constexpr std::size_t MAX_NAME_LENGTH = 255;

// Maximum number of mounted filesystems
constexpr std::size_t MAX_MOUNTS = 16;

// Maximum number of open files
constexpr std::size_t MAX_FILES = 256;

// Maximum number of file descriptors per process
constexpr std::size_t MAX_FDS = 32;

// File types
enum class FileType : std::uint8_t {
    NONE     = 0,
    REGULAR  = 1,
    DIRECTORY = 2,
    SYMLINK  = 3,
    CHARDEV  = 4,
    BLOCKDEV = 5,
    FIFO     = 6,
    SOCKET   = 7,
};

// File permissions
enum FilePermissions : std::uint32_t {
    PERM_NONE   = 0,
    PERM_EXEC   = 1 << 0,
    PERM_WRITE  = 1 << 1,
    PERM_READ   = 1 << 2,
    PERM_ALL    = PERM_READ | PERM_WRITE | PERM_EXEC,
};

// File open flags
enum FileFlags : std::uint32_t {
    FLAG_NONE    = 0,
    FLAG_READ    = 1 << 0,
    FLAG_WRITE   = 1 << 1,
    FLAG_APPEND  = 1 << 2,
    FLAG_CREATE  = 1 << 3,
    FLAG_TRUNC   = 1 << 4,
    FLAG_EXCL    = 1 << 5,
    FLAG_NONBLOCK = 1 << 6,
};

// File seek whence
enum SeekWhence : std::uint32_t {
    SEEK_SET = 0,
    SEEK_CUR = 1,
    SEEK_END = 2,
};

// File status
struct Stat {
    std::uint32_t dev;
    std::uint32_t ino;
    FileType type;
    std::uint32_t mode;
    std::uint32_t nlink;
    std::uint32_t uid;
    std::uint32_t gid;
    std::uint32_t rdev;
    std::uint64_t size;
    std::uint64_t atime;
    std::uint64_t mtime;
    std::uint64_t ctime;
    std::uint32_t blksize;
    std::uint64_t blocks;
};

// Directory entry
struct Dirent {
    std::uint32_t ino;
    std::uint32_t type;
    char name[MAX_NAME_LENGTH + 1];
};

// File operations interface
struct FileOperations {
    // Open the file
    int (*open)(void* node, std::uint32_t flags);

    // Close the file
    int (*close)(void* node);

    // Read from the file
    ssize_t (*read)(void* node, std::uint64_t offset,
                        void* buffer, std::size_t size);

    // Write to the file
    ssize_t (*write)(void* node, std::uint64_t offset,
                         const void* buffer, std::size_t size);

    // Seek within the file
    off_t (*seek)(void* node, off_t offset,
                      std::uint32_t whence);

    // Get file status
    int (*stat)(void* node, Stat* stat);

    // IO control
    int (*ioctl)(void* node, std::uint32_t request,
                     void* arg);

    // Read directory entry
    Dirent* (*readdir)(void* node, std::uint32_t index);

    // Find a directory entry
    void* (*finddir)(void* node, const char* name);

    // Create a file
    int (*create)(void* node, const char* name,
                      std::uint32_t mode);

    // Delete a file
    int (*unlink)(void* node, const char* name);

    // Create a directory
    int (*mkdir)(void* node, const char* name,
                     std::uint32_t mode);

    // Remove a directory
    int (*rmdir)(void* node, const char* name);

    // Rename a file
    int (*rename)(void* node, const char* old_name,
                      const char* new_name);

    // Get file attributes
    int (*getattr)(void* node, void* attr);

    // Set file attributes
    int (*setattr)(void* node, const void* attr);

    // Truncate the file
    int (*truncate)(void* node, std::uint64_t length);

    // Sync the file
    int (*sync)(void* node);
};

// Mount point
struct Mount {
    char source[MAX_PATH_LENGTH];
    char target[MAX_PATH_LENGTH];
    char filesystem[64];
    void* root;
    FileOperations* ops;
    void* data;

    Mount* next;
};

// File descriptor
struct FileDescriptor {
    void* node;
    FileOperations* ops;
    std::uint64_t offset;
    std::uint32_t flags;
    int ref_count;
    bool is_open;
};

// Filesystem interface
struct Filesystem {
    const char* name;

    // Mount the filesystem
    int (*mount)(const char* source, const char* target,
                     const char* filesystem,
                     std::uint32_t flags,
                     const void* data);

    // Unmount the filesystem
    int (*unmount)(const char* target);

    // Get filesystem statistics
    int (*statfs)(const char* path, void* buf);

    Filesystem* next;
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

// Initialize the VFS
bool initialize();

// Mount a filesystem
int mount(const char* source, const char* target,
              const char* filesystem,
              std::uint32_t flags,
              const void* data);

// Unmount a filesystem
int unmount(const char* target);

// Register a filesystem
void register_filesystem(Filesystem* fs);

// Unregister a filesystem
void unregister_filesystem(Filesystem* fs);

// Open a file
Result<FileDescriptor*> open(const char* path,
                                  std::uint32_t flags);

// Close a file descriptor
int close(FileDescriptor* fd);

// Read from a file descriptor
ssize_t read(FileDescriptor* fd, void* buffer,
                  std::size_t size);

// Write to a file descriptor
ssize_t write(FileDescriptor* fd, const void* buffer,
                   std::size_t size);

// Seek within a file
off_t seek(FileDescriptor* fd, off_t offset,
               std::uint32_t whence);

// Get file status
int stat(const char* path, Stat* stat);

// Get file status by descriptor
int fstat(FileDescriptor* fd, Stat* stat);

// Read a directory entry
Dirent* readdir(FileDescriptor* fd, std::uint32_t index);

// Open a directory
Result<FileDescriptor*> opendir(const char* path);

// Create a file
int create(const char* path, std::uint32_t mode);

// Delete a file
int unlink(const char* path);

// Create a directory
int mkdir(const char* path, std::uint32_t mode);

// Remove a directory
int rmdir(const char* path);

// Rename a file
int rename(const char* old_path, const char* new_path);

// Truncate a file
int truncate(const char* path, std::uint64_t length);

// Sync a file
int sync(FileDescriptor* fd);

// Sync all filesystems
int sync_all();

// Resolve a path to a node
Result<void*> resolve(const char* path);

// Get the current working directory
Result<char*> getcwd(char* buffer, std::size_t size);

// Change the current working directory
int chdir(const char* path);

// Get the root directory
void* get_root() noexcept;

// Check if a path is absolute
bool is_absolute(const char* path) noexcept;

// Normalize a path
Result<char*> normalize(const char* path);

// Join two paths
Result<char*> join(const char* base, const char* relative);

// Get the parent directory
Result<char*> dirname(const char* path);

// Get the filename from a path
Result<char*> basename(const char* path);

// File descriptor management
int allocate_fd(FileDescriptor* fd);
void release_fd(int fd);
FileDescriptor* get_fd(int fd) noexcept;

// Get the number of open files
std::size_t open_file_count() noexcept;

// Get the number of mounted filesystems
std::size_t mount_count() noexcept;

// Get filesystem statistics
struct FilesystemStats {
    std::uint64_t total_blocks;
    std::uint64_t free_blocks;
    std::uint64_t used_blocks;
    std::uint64_t total_inodes;
    std::uint64_t free_inodes;
    std::uint64_t block_size;
    std::uint64_t inode_size;
};

Result<FilesystemStats> statfs(const char* path);

} // namespace kernel::vfs