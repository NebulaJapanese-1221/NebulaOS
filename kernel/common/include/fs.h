// NebulaOS - Filesystem Interface
// ================================
//
// Generic filesystem interface definitions

#ifndef NEBULAOS_FS_H
#define NEBULAOS_FS_H

#include "nebula.h"
#include "stdint.h"

// Filesystem error codes
#define FS_SUCCESS  0
#define FS_ERROR    -1
#define FS_ENOENT   -2
#define FS_EIO      -3

// File access flags
#define FS_FLAG_READ   0x01
#define FS_FLAG_WRITE  0x02

// Maximum filename length
#define FS_MAX_NAME 255

// File/directory type flags
#define FS_TYPE_FILE    0x00
#define FS_TYPE_DIR     0x01

// Forward declarations
typedef struct fs_file fs_file_t;
typedef struct fs_dir  fs_dir_t;

// File descriptor structure
struct fs_file {
    char     name[FS_MAX_NAME + 1];
    uint32_t size;
    uint32_t offset;
    uint32_t start_cluster;
    uint32_t flags;
};

// Directory entry structure
typedef struct {
    char     name[FS_MAX_NAME + 1];
    uint32_t size;
    uint8_t  type;
} fs_dirent_t;

// Directory handle structure
struct fs_dir {
    fs_dirent_t* entries;
    uint32_t     count;
    uint32_t     position;
    void*        internal;
};

// -----------------------------------------------------------------------------
// Filesystem Functions
// -----------------------------------------------------------------------------

// Initialize the filesystem subsystem
void fs_init(void);

// Mount the filesystem
int fs_mount(void);

// Read data from an open file
int fs_read(fs_file_t* file, void* buffer, uint32_t size);

// Write data to an open file
int fs_write(fs_file_t* file, const void* buffer, uint32_t size);

// Open a file by path
int fs_open(const char* path, fs_file_t* file, uint32_t flags);

// Close an open file
int fs_close(fs_file_t* file);

// List directory contents
int fs_list(const char* path, fs_dir_t* dir);

#endif // NEBULAOS_FS_H
