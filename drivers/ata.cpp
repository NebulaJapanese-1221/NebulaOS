#include "ata.hpp"
#include <cstring>
namespace {
unsigned int sectors = 0;
unsigned char read_status() {
    unsigned char v; asm volatile("inb %1,%0":"=a"(v):"Nd"((unsigned short)0x1F7)); return v;
}
void write_reg(unsigned short port, unsigned char v) { asm volatile("outb %0,%1"::"a"(v),"Nd"(port)); }
unsigned char read_reg(unsigned short port) { unsigned char v; asm volatile("inb %1,%0":"=a"(v):"Nd"(port)); return v; }
bool wait_ready() { for (int i=0;i<100000;i++) if (!(read_status()&0x80)) return true; return false; }
bool wait_drq() { for (int i=0;i<100000;i++) { unsigned char s=read_status(); if (s&0x08) return true; if (s&0x21) return false; } return false; }
}
namespace drivers::ata {
bool initialize() {
    // Select master drive
    write_reg(0x1F6, 0xA0);
    if (!wait_ready()) return false;
    write_reg(0x1F2, 1);
    write_reg(0x1F3, 0);
    write_reg(0x1F4, 0);
    write_reg(0x1F5, 0);
    write_reg(0x1F7, 0xEC); // IDENTIFY
    if (read_status()==0) return false;
    if (!wait_drq()) return false;
    unsigned short data[256];
    for (int i=0;i<256;i++) data[i]=read_reg(0x1F0) | (read_reg(0x1F0)<<8);
    // total sectors LBA28
    sectors = data[60] | (data[61]<<16);
    return sectors>0;
}
bool read_sector(unsigned int lba, void* buffer) {
    if (!wait_ready()) return false;
    write_reg(0x1F6, 0xE0 | ((lba>>24)&0x0F));
    write_reg(0x1F2, 1);
    write_reg(0x1F3, lba & 0xFF);
    write_reg(0x1F4, (lba>>8)&0xFF);
    write_reg(0x1F5, (lba>>16)&0xFF);
    write_reg(0x1F7, 0x20);
    if (!wait_drq()) return false;
    unsigned short* b=(unsigned short*)buffer;
    for (int i=0;i<256;i++) b[i]=read_reg(0x1F0) | (read_reg(0x1F0)<<8);
    return true;
}
bool write_sector(unsigned int lba, const void* buffer) {
    if (!wait_ready()) return false;
    write_reg(0x1F6, 0xE0 | ((lba>>24)&0x0F));
    write_reg(0x1F2, 1);
    write_reg(0x1F3, lba & 0xFF);
    write_reg(0x1F4, (lba>>8)&0xFF);
    write_reg(0x1F5, (lba>>16)&0xFF);
    write_reg(0x1F7, 0x30);
    if (!wait_drq()) return false;
    const unsigned short* b=(const unsigned short*)buffer;
    for (int i=0;i<256;i++) { write_reg(0x1F0, b[i]&0xFF); write_reg(0x1F0, (b[i]>>8)&0xFF); }
    // flush
    write_reg(0x1F7, 0xE7);
    wait_ready();
    return true;
}
unsigned int sector_count() { return sectors; }
}
