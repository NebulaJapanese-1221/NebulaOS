#include "fat.hpp"
#include "../../drivers/ata.hpp"
#include <cstring>
namespace {
struct BPB { unsigned char jmp[3]; char oem[8]; unsigned short bytes_per_sector; unsigned char sectors_per_cluster; unsigned short reserved_sectors; unsigned char num_fats; unsigned short root_entries; unsigned short total_sectors_16; unsigned char media; unsigned short fat_size_16; unsigned short sectors_per_track; unsigned short num_heads; unsigned int hidden_sectors; unsigned int total_sectors_32; } __attribute__((packed));
unsigned int start_lba=0; BPB bpb; unsigned int fat_start, root_start, data_start, root_sectors, cluster_size;
bool read_sector(unsigned int lba, void* buf){ return drivers::ata::read_sector(start_lba+lba, buf); }
}
namespace kernel::fs::fat {
bool mount(unsigned int lba) {
    start_lba=lba;
    unsigned char buf[512];
    if (!read_sector(0,buf)) return false;
    memcpy(&bpb,buf,sizeof(BPB));
    fat_start=bpb.reserved_sectors;
    root_sectors=(bpb.root_entries*32 + bpb.bytes_per_sector-1)/bpb.bytes_per_sector;
    root_start=fat_start + bpb.num_fats*bpb.fat_size_16;
    data_start=root_start+root_sectors;
    cluster_size=bpb.sectors_per_cluster*bpb.bytes_per_sector;
    return bpb.bytes_per_sector==512;
}
bool read_file(const char* path, void* buffer, std::size_t* size) {
    // minimal: only root dir, flat lookup
    (void)path;(void)buffer;(void)size;
    return false;
}
}
