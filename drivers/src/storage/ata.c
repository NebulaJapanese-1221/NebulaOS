// NebulaOS - ATA PIO Driver
// ===========================
//
// ATA/ATAPI PIO mode driver for storage devices

#include "../../drivers/include/storage.h"
#include "../../kernel/common/include/io.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/vga.h"

static int ata_devices = 0;

static uint8_t ata_inb(uint16_t port) {
    return inb(port);
}

static void ata_outb(uint16_t port, uint8_t value) {
    outb(port, value);
}

static void ata_wait(void) {
    for (int i = 0; i < 4; i++) {
        ata_inb(ATA_PRIMARY + 0x0C);
    }
}

static int ata_wait_ready(uint16_t base) {
    for (int i = 0; i < 1000000; i++) {
        uint8_t status = ata_inb(base + 0x07);
        if (!(status & 0x80)) {
            if (status & 0x08 || status & 0x01) {
                return (status & 0x01) ? -1 : 0;
            }
            return 0;
        }
    }
    return -1;
}

int ata_init(void) {
    ata_devices = 0;
    
    for (int bus = 0; bus < 2; bus++) {
        uint16_t base = (bus == 0) ? ATA_PRIMARY : ATA_SECONDARY;
        
        for (int slave = 0; slave < 2; slave++) {
            uint8_t select = (slave == 0) ? ATA_MASTER : ATA_SLAVE;
            
            ata_outb(base + 0x06, select | 0xA0);
            for (int i = 0; i < 4; i++) {
                ata_inb(base + 0x0C);
            }
            
            uint8_t status = ata_inb(base + 0x07);
            if (status == 0xFF) continue;
            
            uint8_t cl = ata_inb(base + 0x04);
            uint8_t ch = ata_inb(base + 0x05);
            
            if (cl == 0x00 && ch == 0x00) continue;
            if (cl == 0x00 && ch == 0xFF) continue;
            
            ata_devices++;
            char buf[64];
            int len = 0;
            buf[len++] = 'A';
            buf[len++] = 'T';
            buf[len++] = 'A';
            buf[len++] = ' ';
            buf[len++] = (bus == 0) ? 'P' : 'S';
            buf[len++] = (slave == 0) ? 'M' : 'S';
            buf[len++] = ':';
            buf[len++] = ' ';
            buf[len++] = '0' + ata_devices;
            buf[len++] = '\n';
            buf[len++] = 0;
            vga_puts(buf);
        }
    }
    
    return ata_devices;
}

int ata_read_sectors(uint32_t lba, uint8_t count, void* buffer) {
    if (!buffer || count == 0) return -1;
    if (count > 256) return -1;
    
    uint16_t base = ATA_PRIMARY;
    uint8_t select = ATA_MASTER;
    
    if (ata_wait_ready(base) != 0) return -1;
    
    ata_outb(base + 0x06, select | 0xE0 | ((lba >> 24) & 0x0F));
    ata_outb(base + 0x01, 0x00);
    ata_outb(base + 0x02, count);
    ata_outb(base + 0x03, (uint8_t)(lba & 0xFF));
    ata_outb(base + 0x04, (uint8_t)((lba >> 8) & 0xFF));
    ata_outb(base + 0x05, (uint8_t)((lba >> 16) & 0xFF));
    ata_outb(base + 0x07, ATA_CMD_READ_PIO);
    
    uint16_t* dst = (uint16_t*)buffer;
    
    for (int s = 0; s < count; s++) {
        if (ata_wait_ready(base) != 0) return -1;
        
        for (int i = 0; i < 256; i++) {
            dst[s * 256 + i] = *(volatile uint16_t*)(base);
        }
    }
    
    return 0;
}

int ata_write_sectors(uint32_t lba, uint8_t count, const void* buffer) {
    if (!buffer || count == 0) return -1;
    if (count > 256) return -1;
    
    uint16_t base = ATA_PRIMARY;
    uint8_t select = ATA_MASTER;
    
    if (ata_wait_ready(base) != 0) return -1;
    
    ata_outb(base + 0x06, select | 0xE0 | ((lba >> 24) & 0x0F));
    ata_outb(base + 0x01, 0x00);
    ata_outb(base + 0x02, count);
    ata_outb(base + 0x03, (uint8_t)(lba & 0xFF));
    ata_outb(base + 0x04, (uint8_t)((lba >> 8) & 0xFF));
    ata_outb(base + 0x05, (uint8_t)((lba >> 16) & 0xFF));
    ata_outb(base + 0x07, ATA_CMD_WRITE_PIO);
    
    const uint16_t* src = (const uint16_t*)buffer;
    
    for (int s = 0; s < count; s++) {
        if (ata_wait_ready(base) != 0) return -1;
        
        for (int i = 0; i < 256; i++) {
            *(volatile uint16_t*)(base) = src[s * 256 + i];
        }
    }
    
    return 0;
}

int ata_get_device_count(void) {
    return ata_devices;
}