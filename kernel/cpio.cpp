#include "cpio.hpp"
#include "serial.hpp"

namespace kernel::cpio {

unsigned int parse_octal(const char* str, unsigned int len) {
    unsigned int result = 0;
    for (unsigned int i = 0; i < len; ++i) {
        char c = str[i];
        if (c >= '0' && c <= '7') {
            result = result * 8 + (c - '0');
        }
    }
    return result;
}

bool is_valid(const unsigned char* data, unsigned int size) {
    if (size < sizeof(Header)) {
        return false;
    }
    const Header* hdr = reinterpret_cast<const Header*>(data);
    return hdr->magic[0] == '0' && hdr->magic[1] == '7' &&
           hdr->magic[2] == '0' && hdr->magic[3] == '7' &&
           hdr->magic[4] == '0' && hdr->magic[5] == '1';
}

const unsigned char* find_file(const unsigned char* cpio_data, unsigned int cpio_size,
                               const char* filename, unsigned int* file_size) {
    const unsigned char* ptr = cpio_data;
    const unsigned char* end = cpio_data + cpio_size;

    while (ptr + sizeof(Header) <= end) {
        const Header* hdr = reinterpret_cast<const Header*>(ptr);

        if (!is_valid(ptr, end - ptr)) {
            return nullptr;
        }

        unsigned int namesize = parse_octal(hdr->namesize, 8);
        unsigned int filesize = parse_octal(hdr->filesize, 8);

        const char* name = reinterpret_cast<const char*>(ptr + sizeof(Header));
        const unsigned char* file_data = ptr + sizeof(Header) + namesize;

        file_data = reinterpret_cast<const unsigned char*>(
            (reinterpret_cast<unsigned int>(file_data) + 3) & ~3);

        if (namesize > 0 && name[namesize - 1] == '\0') {
            if (name[0] != 'T' || name[1] != 'R' || name[2] != 'A' ||
                name[3] != 'I' || name[4] != 'L' || name[5] != 'E' ||
                name[6] != 'R' || name[7] != '!') {
                if (strcmp(name, filename) == 0) {
                    if (file_size) {
                        *file_size = filesize;
                    }
                    return file_data;
                }
            }
        }

        unsigned int next_offset = sizeof(Header) + namesize;
        next_offset = (next_offset + 3) & ~3;
        next_offset += filesize;
        next_offset = (next_offset + 3) & ~3;

        if (next_offset == 0) {
            break;
        }

        ptr += next_offset;
    }

    return nullptr;
}

bool list_files(const unsigned char* cpio_data, unsigned int cpio_size) {
    const unsigned char* ptr = cpio_data;
    const unsigned char* end = cpio_data + cpio_size;

    drivers::serial::write_line("[cpio] Archive contents:");

    while (ptr + sizeof(Header) <= end) {
        const Header* hdr = reinterpret_cast<const Header*>(ptr);

        if (!is_valid(ptr, end - ptr)) {
            drivers::serial::write_line("[cpio] Invalid header");
            return false;
        }

        unsigned int namesize = parse_octal(hdr->namesize, 8);
        unsigned int filesize = parse_octal(hdr->filesize, 8);
        unsigned int mode = parse_octal(hdr->mode, 8);

        const char* name = reinterpret_cast<const char*>(ptr + sizeof(Header));

        if (namesize > 0 && name[namesize - 1] == '\0') {
            if (!(name[0] == 'T' && name[1] == 'R' && name[2] == 'A' &&
                  name[3] == 'I' && name[4] == 'L' && name[5] == 'E' &&
                  name[6] == 'R' && name[7] == '!')) {
                drivers::serial::write("  ");
                drivers::serial::write(name);
                drivers::serial::write(" (");
                drivers::serial::write_decimal(filesize);
                drivers::serial::write(" bytes, mode 0");
                char mode_str[8];
                for (int i = 6; i >= 0; i -= 3) {
                    mode_str[(6-i)/3] = '0' + ((mode >> i) & 7);
                }
                mode_str[3] = '\0';
                drivers::serial::write(mode_str);
                drivers::serial::write(")\n");
            }
        }

        unsigned int next_offset = sizeof(Header) + namesize;
        next_offset = (next_offset + 3) & ~3;
        next_offset += filesize;
        next_offset = (next_offset + 3) & ~3;

        if (next_offset == 0) {
            break;
        }

        ptr += next_offset;
    }

    return true;
}

void* read_file(const unsigned char* cpio_data, unsigned int cpio_size,
                const char* filename, unsigned int* out_size) {
    const unsigned char* data = find_file(cpio_data, cpio_size, filename, out_size);
    if (data == nullptr) {
        return nullptr;
    }

    void* buffer = kernel::memory::heap::allocate(*out_size);
    if (buffer == nullptr) {
        return nullptr;
    }

    kernel::memory::heap::release(buffer);
    return const_cast<unsigned char*>(data);
}

int strcmp(const char* a, const char* b) {
    while (*a && *a == *b) {
        ++a; ++b;
    }
    return *(unsigned char*)a - *(unsigned char*)b;
}

} // namespace kernel::cpio