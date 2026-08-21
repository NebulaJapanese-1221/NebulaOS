// NebulaOS - VESA Graphics Driver
// ================================
//
// VESA/VBE graphics mode driver header

#ifndef NEBULAOS_DRIVERS_VESA_H
#define NEBULAOS_DRIVERS_VESA_H

#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/nebula.h"

// VBE function numbers
#define VBE_FUNCTION_INFO           0x4F00
#define VBE_FUNCTION_MODE_INFO      0x4F01
#define VBE_FUNCTION_SET_MODE       0x4F02
#define VBE_FUNCTION_GET_MODE       0x4F03
#define VBE_FUNCTION_SAVE_RESTORE   0x4F04
#define VBE_FUNCTION_DISPLAY_WINDOW 0x4F05
#define VBE_FUNCTION_SCANLINE_LEN   0x4F06
#define VBE_FUNCTION_GET_DCX_INFO   0x4F07

// VBE return status codes
#define VBE_STATUS_OK              0x00
#define VBE_STATUS_FAILED          0x01
#define VBE_STATUS_NOT_SUPPORTED   0x02
#define VBE_STATUS_INVALID         0x03

// VBE capabilities
#define VBE_CAPABILITY_DAC_SWITCH   0x01
#define VBE_CAPABILITY_8BIT_DAC     0x02
#define VBE_CAPABILITY_LFB          0x04
#define VBE_CAPABILITY_RETRACTORY   0x08

// VBE mode attributes
#define VBE_MODE_ATTR_SUPPORTED     0x01
#define VBE_MODE_ATTR_EXT_INFO      0x02
#define VBE_MODE_ATTR_BIOS_TSEG     0x04
#define VBE_MODE_ATTR_A0000         0x08
#define VBE_MODE_ATTR_B0000         0x10
#define VBE_MODE_ATTR_B8000         0x20
#define VBE_MODE_ATTR_LINEAR        0x40
#define VBE_MODE_ATTR_DOUBLESCAN    0x80

// Memory models
#define VBE_MEMORY_MODEL_TEXT       0x00
#define VBE_MEMORY_MODEL_CGA        0x01
#define VBE_MEMORY_MODEL_HERCULES   0x02
#define VBE_MEMORY_MODEL_PLANAR     0x03
#define VBE_MEMORY_MODEL_PACKED     0x04
#define VBE_MEMORY_MODEL_NONCHAIN   0x05
#define VBE_MEMORY_MODEL_DIRECT     0x06
#define VBE_MEMORY_MODEL_YUV        0x07

// Color models
#define VBE_COLOR_MODEL_RGB        0x06

// VBE Info Block structure
typedef struct PACKED {
    char signature[4];       // "VESA"
    uint16_t version;         // VBE version
    uint32_t oem_string;      // Pointer to OEM string
    uint32_t capabilities;    // Capabilities
    uint32_t video_modes;     // Pointer to video modes list
    uint16_t total_memory;    // Total memory in 64KB blocks
    uint16_t oem_software_rev;
    uint32_t oem_vendor_name;
    uint32_t oem_product_name;
    uint32_t oem_product_rev;
    uint8_t reserved[222];
    uint8_t oem_data[256];
} vbe_info_block_t;

// VBE Mode Info Block structure
typedef struct PACKED {
    uint16_t mode_attributes;    // Mode attributes
    uint8_t win_a_attributes;    // Window A attributes
    uint8_t win_b_attributes;    // Window B attributes
    uint16_t win_granularity;    // Window granularity
    uint16_t win_size;          // Window size
    uint16_t win_a_segment;     // Window A segment
    uint16_t win_b_segment;     // Window B segment
    uint32_t win_func;          // Window function pointer
    uint16_t bytes_per_scanline; // Bytes per scan line
    
    // VBE 1.2+
    uint16_t x_resolution;      // Horizontal resolution
    uint16_t y_resolution;      // Vertical resolution
    uint8_t x_char_size;         // Character cell width
    uint8_t y_char_size;         // Character cell height
    uint8_t number_of_planes;   // Number of memory planes
    uint8_t bits_per_pixel;     // Bits per pixel
    uint8_t number_of_banks;    // Number of banks
    uint8_t memory_model;       // Memory model
    uint8_t bank_size;          // Bank size in KB
    uint8_t number_of_image_pages;
    uint8_t reserved1;
    
    // Direct color fields
    uint8_t red_mask_size;      // Size of direct color red mask
    uint8_t red_field_position; // Bit position of LSB of red mask
    uint8_t green_mask_size;    // Size of direct color green mask
    uint8_t green_field_position;
    uint8_t blue_mask_size;     // Size of direct color blue mask
    uint8_t blue_field_position;
    uint8_t reserved_mask_size;
    uint8_t reserved_field_position;
    
    // Color pointer fields
    uint8_t direct_color_mode_info;
    
    // VBE 2.0+
    uint32_t phys_base_ptr;     // Physical address for flat memory framebuffer
    uint32_t reserved2;
    uint16_t reserved3;
    
    // VBE 3.0+
    uint16_t linear_bytes_per_scanline;
    uint8_t bnk_number_of_image_pages;
    uint8_t linear_number_of_image_pages;
    uint8_t linear_red_mask_size;
    uint8_t linear_red_field_position;
    uint8_t linear_green_mask_size;
    uint8_t linear_green_field_position;
    uint8_t linear_blue_mask_size;
    uint8_t linear_blue_field_position;
    uint8_t linear_reserved_mask_size;
    uint8_t linear_reserved_field_position;
    uint32_t max_pixel_clock;
    
    uint8_t reserved4[189];
} vbe_mode_info_t;

// VESA driver state
typedef struct {
    bool initialized;
    bool lfb_enabled;
    uint32_t width;
    uint32_t height;
    uint32_t bpp;
    uint32_t stride;
    void* framebuffer;
    uint32_t framebuffer_size;
    vbe_info_block_t* vbe_info;
    vbe_mode_info_t* mode_info;
    uint16_t current_mode;
} vesa_state_t;

// VESA functions
bool vesa_init();
bool vesa_set_mode(uint16_t mode);
bool vesa_get_mode_info(uint16_t mode, vbe_mode_info_t* info);
bool vesa_find_mode(uint32_t width, uint32_t height, uint32_t bpp, uint16_t* mode);

// Framebuffer access
void* vesa_get_framebuffer();
uint32_t vesa_get_width();
uint32_t vesa_get_height();
uint32_t vesa_get_bpp();
uint32_t vesa_get_stride();

// Clear screen
void vesa_clear(uint32_t color);

// Draw pixel
void vesa_set_pixel(uint32_t x, uint32_t y, uint32_t color);
uint32_t vesa_get_pixel(uint32_t x, uint32_t y);

// Mode information
const vbe_mode_info_t* vesa_get_current_mode_info();
const vbe_info_block_t* vesa_get_vbe_info();

// Check capabilities
bool vesa_is_linear_framebuffer();
bool vesa_is_direct_color();

// Get color masks
void vesa_get_color_masks(uint32_t* red_mask, uint32_t* green_mask, uint32_t* blue_mask);

// Make color from RGB
uint32_t vesa_make_color(uint8_t r, uint8_t g, uint8_t b);

#endif // NEBULAOS_DRIVERS_VESA_H
