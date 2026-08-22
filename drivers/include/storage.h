#ifndef NEBULAOS_STORAGE_H
#define NEBULAOS_STORAGE_H

#include "../common/include/stdint.h"

#define ATA_PRIMARY 0x1F0
#define ATA_SECONDARY 0x170
#define ATA_MASTER 0x00
#define ATA_SLAVE 0x10

#define ATA_SECTOR_SIZE 512

#define ATA_CMD_READ_PIO 0x20
#define ATA_CMD_WRITE_PIO 0x30
#define ATA_CMD_IDENTIFY 0xEC

int ata_init(void);
int ata_read_sectors(uint32_t lba, uint8_t count, void* buffer);
int ata_write_sectors(uint32_t lba, uint8_t count, const void* buffer);
int ata_get_device_count(void);

#endif