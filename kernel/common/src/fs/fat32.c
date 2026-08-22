// NebulaOS - FAT32 Filesystem Driver
// ====================================
//
// FAT32 filesystem driver implementation
// Reads BPB from sector 0, parses FAT and root directory
// Implements file open/read/close and directory listing

#include "../include/fs.h"
#include "../include/nebula.h"
#include "../include/stdint.h"
#include "../include/memory.h"
#include "../../../lib/include/string.h"

// ATA read function (provided by ATA driver; weak stub here)
int WEAK ata_read_sectors(uint32_t lba, uint8_t count, void* buffer) {
    (void)lba;
    (void)count;
    (void)buffer;
    return -1;
}

// Sector size
#define FAT32_SECTOR_SIZE 512
#define FAT32_MAX_CLUSTER_CHAIN 64
#define FAT32_MAX_DIR_ENTRIES 256

// Directory entry structure (FAT32 on-disk format)
typedef struct PACKED {
    uint8_t  dir_name[11];
    uint8_t  dir_attr;
    uint8_t  dir_ntres;
    uint8_t  dir_crt_time_tenth;
    uint16_t dir_crt_time;
    uint16_t dir_crt_date;
    uint16_t dir_last_acc_date;
    uint16_t dir_fst_cluster_hi;
    uint16_t dir_write_time;
    uint16_t dir_write_date;
    uint16_t dir_fst_cluster_lo;
    uint32_t dir_file_size;
} fat32_dir_entry_t;

// BPB structure (first 71 bytes of sector 0)
typedef struct PACKED {
    uint8_t  bs_jmpboot[3];
    uint8_t  bs_oemname[8];
    uint16_t bpb_bytes_per_sector;
    uint8_t  bpb_sectors_per_cluster;
    uint16_t bpb_reserved_sector_count;
    uint8_t  bpb_num_fats;
    uint16_t bpb_root_entry_count;
    uint16_t bpb_total_sectors_16;
    uint8_t  bpb_media_type;
    uint16_t bpb_fat_size_16;
    uint16_t bpb_sectors_per_track;
    uint16_t bpb_num_heads;
    uint32_t bpb_hidden_sector_count;
    uint32_t bpb_total_sectors_32;
    uint32_t bpf32_fat_size;
    uint16_t bpf32_ext_flags;
    uint16_t bpf32_fs_version;
    uint32_t bpf32_root_cluster;
    uint16_t bpf32_fs_info;
    uint16_t bpf32_backup_boot_sector;
    uint8_t  bpf32_reserved[12];
    uint8_t  bsd_drive_number;
    uint8_t  bsd_reserved;
    uint8_t  bsd_boot_signature;
    uint32_t bsd_volume_id;
    uint8_t  bsd_volume_label[11];
    uint8_t  bsd_fs_type[8];
} fat32_bpb_t;

// FAT32 driver state
typedef struct {
    uint32_t sectors_per_cluster;
    uint32_t reserved_sectors;
    uint32_t num_fats;
    uint32_t fat_size;
    uint32_t root_cluster;
    uint32_t total_sectors;
    uint32_t bytes_per_sector;
    uint32_t fat_start_sector;
    uint32_t data_start_sector;
    uint8_t  fat_buffer[FAT32_SECTOR_SIZE * 4];
    uint32_t fat_cached_sector;
    int      mounted;
} fat32_state_t;

static fat32_state_t fat32;

// -----------------------------------------------------------------------------
// Static helper functions
// -----------------------------------------------------------------------------

static uint32_t fat32_cluster_to_sector(uint32_t cluster) {
    return fat32.data_start_sector + (cluster - 2) * fat32.sectors_per_cluster;
}

static uint32_t fat32_get_next_cluster(uint32_t cluster) {
    uint32_t fat_offset = cluster * 4;
    uint32_t fat_sector = fat32.fat_start_sector + fat_offset / FAT32_SECTOR_SIZE;
    uint32_t fat_idx = fat_offset % FAT32_SECTOR_SIZE;

    if (fat32.fat_cached_sector != fat_sector) {
        if (ata_read_sectors(fat_sector, 1, fat32.fat_buffer) != 0) {
            return 0x0FFFFFF7;
        }
        fat32.fat_cached_sector = fat_sector;
    }

    uint32_t next = *(uint32_t*)&fat32.fat_buffer[fat_idx];
    return next & 0x0FFFFFFF;
}

static void fat32_entry_to_name(const fat32_dir_entry_t* entry, char* name) {
    int i;
    int j = 0;

    for (i = 0; i < 8; i++) {
        if (entry->dir_name[i] == ' ') break;
        name[j++] = entry->dir_name[i];
    }

    if (entry->dir_name[8] != ' ') {
        name[j++] = '.';
        for (i = 8; i < 11; i++) {
            if (entry->dir_name[i] == ' ') break;
            name[j++] = entry->dir_name[i];
        }
    }

    name[j] = '\0';
}

static int fat32_find_in_dir(uint32_t dir_cluster, const char* filename, fat32_dir_entry_t* out_entry) {
    uint8_t* dir_buffer = malloc(FAT32_SECTOR_SIZE * 32);
    if (!dir_buffer) return FS_ERROR;

    uint32_t cluster = dir_cluster;
    while (cluster < 0x0FFFFFF8) {
        uint32_t sector = fat32_cluster_to_sector(cluster);
        if (ata_read_sectors(sector, fat32.sectors_per_cluster, dir_buffer) != 0) {
            free(dir_buffer);
            return FS_EIO;
        }

        uint32_t max_entries = (fat32.sectors_per_cluster * FAT32_SECTOR_SIZE) / sizeof(fat32_dir_entry_t);
        fat32_dir_entry_t* entry = (fat32_dir_entry_t*)dir_buffer;

        for (uint32_t i = 0; i < max_entries; i++) {
            if (entry[i].dir_name[0] == 0x00) {
                free(dir_buffer);
                return FS_ENOENT;
            }
            if (entry[i].dir_name[0] == 0xE5) continue;
            if (entry[i].dir_attr & 0x08) continue;

            char name[FS_MAX_NAME + 1];
            fat32_entry_to_name(&entry[i], name);

            if (strcmp(name, filename) == 0) {
                memcpy(out_entry, &entry[i], sizeof(fat32_dir_entry_t));
                free(dir_buffer);
                return FS_SUCCESS;
            }
        }

        cluster = fat32_get_next_cluster(cluster);
    }

    free(dir_buffer);
    return FS_ENOENT;
}

static int fat32_read_cluster(uint32_t cluster, uint8_t* buffer) {
    uint32_t sector = fat32_cluster_to_sector(cluster);
    for (uint32_t i = 0; i < fat32.sectors_per_cluster; i++) {
        if (ata_read_sectors(sector + i, 1, buffer + i * FAT32_SECTOR_SIZE) != 0) {
            return FS_EIO;
        }
    }
    return FS_SUCCESS;
}

// -----------------------------------------------------------------------------
// Public API implementation
// -----------------------------------------------------------------------------

void fs_init(void) {
    memset(&fat32, 0, sizeof(fat32));
}

int fs_mount(void) {
    if (fat32.mounted) return FS_SUCCESS;

    uint8_t sector[FAT32_SECTOR_SIZE];
    if (ata_read_sectors(0, 1, sector) != 0) {
        return FS_EIO;
    }

    fat32_bpb_t* bpb = (fat32_bpb_t*)sector;
    fat32.bytes_per_sector = bpb->bpb_bytes_per_sector;
    fat32.sectors_per_cluster = bpb->bpb_sectors_per_cluster;
    fat32.reserved_sectors = bpb->bpb_reserved_sector_count;
    fat32.num_fats = bpb->bpb_num_fats;
    fat32.fat_size = bpb->bpf32_fat_size;
    fat32.root_cluster = bpb->bpf32_root_cluster;
    fat32.total_sectors = bpb->bpb_total_sectors_32;

    fat32.fat_start_sector = fat32.reserved_sectors;
    fat32.data_start_sector = fat32.reserved_sectors + fat32.num_fats * fat32.fat_size;

    fat32.fat_cached_sector = 0xFFFFFFFF;
    fat32.mounted = 1;

    return FS_SUCCESS;
}

int fs_open(const char* path, fs_file_t* file, uint32_t flags) {
    if (!fat32.mounted) return FS_ERROR;

    const char* filename = path;
    if (path[0] == '/') filename++;

    fat32_dir_entry_t entry;
    int ret = fat32_find_in_dir(fat32.root_cluster, filename, &entry);
    if (ret != FS_SUCCESS) return ret;

    if (entry.dir_attr & 0x10) return FS_ERROR;

    memset(file, 0, sizeof(fs_file_t));
    strncpy(file->name, filename, FS_MAX_NAME);
    file->size = entry.dir_file_size;
    file->start_cluster = entry.dir_fst_cluster_lo | ((uint32_t)entry.dir_fst_cluster_hi << 16);
    file->flags = flags;
    file->offset = 0;

    return FS_SUCCESS;
}

int fs_read(fs_file_t* file, void* buffer, uint32_t size) {
    if (!fat32.mounted || !file) return FS_ERROR;
    if (file->offset >= file->size) return 0;

    if (file->offset + size > file->size) {
        size = file->size - file->offset;
    }

    uint8_t* buf = (uint8_t*)buffer;
    uint32_t cluster_size = fat32.sectors_per_cluster * FAT32_SECTOR_SIZE;
    uint32_t cluster_index = file->offset / cluster_size;
    uint32_t cluster_offset = file->offset % cluster_size;

    uint32_t cluster = file->start_cluster;
    for (uint32_t i = 0; i < cluster_index && cluster < 0x0FFFFFF8; i++) {
        cluster = fat32_get_next_cluster(cluster);
    }

    uint32_t bytes_read = 0;
    while (bytes_read < size && cluster < 0x0FFFFFF8) {
        uint8_t cluster_buf[FAT32_MAX_CLUSTER_CHAIN * FAT32_SECTOR_SIZE];
        if (fat32_read_cluster(cluster, cluster_buf) != FS_SUCCESS) {
            if (bytes_read > 0) return bytes_read;
            return FS_EIO;
        }

        uint32_t available = cluster_size - cluster_offset;
        uint32_t to_copy = MIN(size - bytes_read, available);
        memcpy(buf + bytes_read, cluster_buf + cluster_offset, to_copy);

        bytes_read += to_copy;
        cluster_offset = 0;

        if (bytes_read < size) {
            cluster = fat32_get_next_cluster(cluster);
        }
    }

    file->offset += bytes_read;
    return bytes_read;
}

int fs_write(fs_file_t* file, const void* buffer, uint32_t size) {
    (void)file;
    (void)buffer;
    (void)size;
    return FS_ERROR;
}

int fs_close(fs_file_t* file) {
    (void)file;
    return FS_SUCCESS;
}

int fs_list(const char* path, fs_dir_t* dir) {
    if (!fat32.mounted) return FS_ERROR;

    const char* dirname = path;
    if (path[0] == '/') dirname++;

    uint32_t dir_cluster = fat32.root_cluster;
    if (dirname[0] != '\0') {
        fat32_dir_entry_t entry;
        int ret = fat32_find_in_dir(fat32.root_cluster, dirname, &entry);
        if (ret != FS_SUCCESS) return ret;
        if (!(entry.dir_attr & 0x10)) return FS_ERROR;
        dir_cluster = entry.dir_fst_cluster_lo | ((uint32_t)entry.dir_fst_cluster_hi << 16);
    }

    uint8_t* dir_buffer = malloc(FAT32_SECTOR_SIZE * 64);
    if (!dir_buffer) return FS_ERROR;

    uint32_t cluster = dir_cluster;
    uint32_t total_bytes = 0;
    uint32_t max_bytes = FAT32_SECTOR_SIZE * 64;

    while (cluster < 0x0FFFFFF8 && total_bytes < max_bytes) {
        uint32_t sector = fat32_cluster_to_sector(cluster);
        if (ata_read_sectors(sector, fat32.sectors_per_cluster, dir_buffer + total_bytes) != 0) {
            free(dir_buffer);
            return FS_EIO;
        }
        total_bytes += fat32.sectors_per_cluster * FAT32_SECTOR_SIZE;
        cluster = fat32_get_next_cluster(cluster);
    }

    uint32_t count = 0;
    uint32_t max_entries = total_bytes / sizeof(fat32_dir_entry_t);
    fat32_dir_entry_t* entry = (fat32_dir_entry_t*)dir_buffer;

    for (uint32_t i = 0; i < max_entries && count < FAT32_MAX_DIR_ENTRIES; i++) {
        if (entry[i].dir_name[0] == 0x00) break;
        if (entry[i].dir_name[0] == 0xE5) continue;
        if (entry[i].dir_attr & 0x08) continue;
        count++;
    }

    dir->entries = malloc(sizeof(fs_dirent_t) * count);
    if (!dir->entries) {
        free(dir_buffer);
        return FS_ERROR;
    }

    uint32_t idx = 0;
    for (uint32_t i = 0; i < max_entries && idx < count; i++) {
        if (entry[i].dir_name[0] == 0x00) break;
        if (entry[i].dir_name[0] == 0xE5) continue;
        if (entry[i].dir_attr & 0x08) continue;

        fat32_entry_to_name(&entry[i], dir->entries[idx].name);
        dir->entries[idx].size = entry[i].dir_file_size;
        dir->entries[idx].type = (entry[i].dir_attr & 0x10) ? FS_TYPE_DIR : FS_TYPE_FILE;
        idx++;
    }

    dir->count = count;
    dir->position = 0;

    free(dir_buffer);
    return FS_SUCCESS;
}
