// Minimal FAT12/16 read-only filesystem for NebulaOS.
#pragma once
#include <cstdint>
#include <cstddef>
namespace kernel::fs::fat {
bool mount(unsigned int start_lba);
bool read_file(const char* path, void* buffer, std::size_t* size);
}
