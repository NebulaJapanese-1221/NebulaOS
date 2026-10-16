// ATA PIO disk driver for NebulaOS.
#pragma once
namespace drivers::ata {
bool initialize();
bool read_sector(unsigned int lba, void* buffer);
bool write_sector(unsigned int lba, const void* buffer);
unsigned int sector_count();
}
