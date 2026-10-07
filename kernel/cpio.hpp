#pragma once

#include "heap.hpp"

namespace kernel::cpio {

struct Header {
    char magic[6];
    char inode[8];
    char mode[8];
    char uid[8];
    char gid[8];
    char nlink[8];
    char mtime[8];
    char filesize[8];
    char devmajor[8];
    char devminor[8];
    char rdevmajor[8];
    char rdevminor[8];
    char namesize[8];
    char check[8];
};

bool is_valid(const unsigned char* data, unsigned int size);
unsigned int parse_octal(const char* str, unsigned int len);
const unsigned char* find_file(const unsigned char* cpio_data, unsigned int cpio_size,
                               const char* filename, unsigned int* file_size);
bool list_files(const unsigned char* cpio_data, unsigned int cpio_size);
void* read_file(const unsigned char* cpio_data, unsigned int cpio_size,
                const char* filename, unsigned int* out_size);

} // namespace kernel::cpio