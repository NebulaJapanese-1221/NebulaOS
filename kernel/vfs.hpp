// Virtual filesystem interface for the NebulaOS x86 kernel.
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

#include "../drivers/serial.hpp"

namespace kernel::vfs {

// File types
enum class FileType {
    REGULAR,
    DIRECTORY,
    DEVICE,
    SYMLINK,
    FIFO
};

// File open flags
enum class OpenFlags {
    READ = 0x1,
    WRITE = 0x2,
    APPEND = 0x4,
    CREATE = 0x8,
    TRUNCATE = 0x10
};

// File status structure
struct Stat {
    unsigned int type;        // FileType value
    unsigned int size;        // File size in bytes
    unsigned int blocks;      // Number of blocks allocated
    unsigned int mode;        // Permission bits
    unsigned int uid;         // User ID
    unsigned int gid;         // Group ID
    unsigned int atime;       // Access time
    unsigned int mtime;       // Modification time
    unsigned int ctime;       // Creation time
};

// File descriptor structure
struct File {
    int fd;                    // File descriptor number
    FileType type;             // File type
    unsigned int position;     // Current read/write position
    unsigned int flags;        // Open flags
    void* inode;               // Inode pointer (filesystem-specific)
    void* fs_data;             // Filesystem-specific data
    bool in_use;               // Whether this slot is occupied
};

// Maximum open files
const int MAX_OPEN_FILES = 64;
const int MAX_PATH_LENGTH = 256;

// Initialize the VFS
void initialize();

// Register a filesystem driver
bool register_filesystem(const char* name, void* fs);

// Mount a filesystem at a mount point
bool mount(const char* source, const char* target, const char* fstype);

// File operations
int open(const char* path, int flags);
int close(int fd);
int read(int fd, void* buf, unsigned int count);
int write(int fd, const void* buf, unsigned int count);
int lseek(int fd, int offset, int whence);
int fstat(int fd, struct Stat* stat);

// Directory operations
int mkdir(const char* path, unsigned int mode);

// Directory entry
struct DirEntry {
    char name[MAX_PATH_LENGTH];
    FileType type;
    unsigned int size;
};

int readdir(const char* path, DirEntry* entry, unsigned int index);

// Path operations
bool exists(const char* path);
bool is_directory(const char* path);
bool is_regular(const char* path);

// Device operations
int register_device(const char* name, unsigned int major, void* ops);

} // namespace kernel::vfs