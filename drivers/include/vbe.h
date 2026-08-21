// NebulaOS - VBE BIOS Interface
// ==============================
//
// VBE 2.0+ BIOS call helpers via INT 0x10

#ifndef NEBULAOS_DRIVERS_VBE_H
#define NEBULAOS_DRIVERS_VBE_H

#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/nebula.h"

// VBE function return codes
#define VBE_RETURN_OK          0x4F
#define VBE_RETURN_FAILED      0x01
#define VBE_RETURN_NOT_SUPPORTED 0x02
#define VBE_RETURN_INVALID     0x03

// VBE BIOS calls
bool vbe_get_info(vbe_info_block_t* info);
bool vbe_get_mode_info(uint16_t mode, vbe_mode_info_t* info);
bool vbe_set_mode(uint16_t mode);
uint16_t vbe_get_mode(void);
bool vbe_set_display_window(uint16_t window, uint16_t position);
bool vbe_get_display_window(uint16_t window, uint16_t* position);
bool vbe_set_scan_line_length(uint16_t pixels, uint16_t* actual);
bool vbe_get_scan_line_length(uint16_t* pixels, uint16_t* actual);
bool vbe_save_state(void* buffer, uint32_t size);
bool vbe_restore_state(void* buffer, uint32_t size);

#endif // NEBULAOS_DRIVERS_VBE_H
