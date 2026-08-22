// NebulaOS - VBE BIOS Interface
// ==============================
//
// VBE 2.0+ BIOS interface via INT 0x10

#include "../include/vesa.h"
#include "../include/vbe.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"

// -----------------------------------------------------------------------------
// Manual memory helpers
// -----------------------------------------------------------------------------

static void memcpy8(void* dst, const void* src, uint32_t count) {
    uint8_t* d = (uint8_t*)dst;
    const uint8_t* s = (const uint8_t*)src;
    for (uint32_t i = 0; i < count; i++) {
        d[i] = s[i];
    }
}

static void memset8(void* dst, uint8_t value, uint32_t count) {
    uint8_t* d = (uint8_t*)dst;
    for (uint32_t i = 0; i < count; i++) {
        d[i] = value;
    }
}

// -----------------------------------------------------------------------------
// Get VBE controller information
// -----------------------------------------------------------------------------

bool vbe_get_info(vbe_info_block_t* info) {
    if (!info) {
        return false;
    }

    // Set up VBE signature
    memcpy8(info->signature, "VESA", 4);
    info->version = 0x0300;
    info->oem_string = 0;
    info->capabilities = VBE_CAPABILITY_LFB;
    info->video_modes = 0xFFFF;
    info->total_memory = 0;
    info->oem_software_rev = 0;
    info->oem_vendor_name = 0;
    info->oem_product_name = 0;
    info->oem_product_rev = 0;

    // In a real implementation, this would call INT 0x10, AX=0x4F00
    // For now, return a stub that indicates VBE 3.0 is available
    return true;
}

// -----------------------------------------------------------------------------
// Get VBE mode information
// -----------------------------------------------------------------------------

bool vbe_get_mode_info(uint16_t mode, vbe_mode_info_t* info) {
    if (!info) {
        return false;
    }

    memset8(info, 0, sizeof(*info));

    // In a real implementation, this would call INT 0x10, AX=0x4F01
    // For now, return stub 1024x768x32 mode info
    if (mode == 0x118 || mode == 0x11A || mode == 0x14C) {
        info->mode_attributes = VBE_MODE_ATTR_SUPPORTED | VBE_MODE_ATTR_LINEAR | VBE_MODE_ATTR_EXT_INFO;
        info->x_resolution = 1024;
        info->y_resolution = 768;
        info->bits_per_pixel = 32;
        info->memory_model = VBE_MEMORY_MODEL_DIRECT;
        info->red_mask_size = 8;
        info->red_field_position = 16;
        info->green_mask_size = 8;
        info->green_field_position = 8;
        info->blue_mask_size = 8;
        info->blue_field_position = 0;
        info->reserved_mask_size = 8;
        info->reserved_field_position = 24;
        info->direct_color_mode_info = 0x0F;
        info->phys_base_ptr = 0xE0000000;
        info->bytes_per_scanline = 1024 * 4;
        info->win_granularity = 64;
        info->win_size = 64;
        info->number_of_planes = 1;
        info->number_of_banks = 1;
        info->bank_size = 0;
        info->number_of_image_pages = 1;
        return true;
    }

    return false;
}

// -----------------------------------------------------------------------------
// Set VBE video mode
// -----------------------------------------------------------------------------

bool vbe_set_mode(uint16_t mode) {
    (void)mode;

    // In a real implementation, this would call INT 0x10, AX=0x4F02
    // For now, return success as a stub
    return true;
}

// -----------------------------------------------------------------------------
// Get current VBE mode
// -----------------------------------------------------------------------------

uint16_t vbe_get_mode(void) {
    // In a real implementation, this would call INT 0x10, AX=0x4F03
    return 0x0118;  // Stub: return 1024x768x32
}

// -----------------------------------------------------------------------------
// Set VBE display window
// -----------------------------------------------------------------------------

bool vbe_set_display_window(uint16_t window, uint16_t position) {
    (void)window;
    (void)position;
    return true;
}

// -----------------------------------------------------------------------------
// Get VBE display window
// -----------------------------------------------------------------------------

bool vbe_get_display_window(uint16_t window, uint16_t* position) {
    (void)window;
    if (position) {
        *position = 0;
    }
    return true;
}

// -----------------------------------------------------------------------------
// Set scan line length
// -----------------------------------------------------------------------------

bool vbe_set_scan_line_length(uint16_t pixels, uint16_t* actual) {
    (void)pixels;
    if (actual) {
        *actual = 1024;
    }
    return true;
}

// -----------------------------------------------------------------------------
// Get scan line length
// -----------------------------------------------------------------------------

bool vbe_get_scan_line_length(uint16_t* pixels, uint16_t* actual) {
    if (pixels) {
        *pixels = 1024;
    }
    if (actual) {
        *actual = 1024;
    }
    return true;
}

// -----------------------------------------------------------------------------
// Save/Restore state
// -----------------------------------------------------------------------------

bool vbe_save_state(void* buffer, uint32_t size) {
    (void)buffer;
    (void)size;
    return true;
}

bool vbe_restore_state(void* buffer, uint32_t size) {
    (void)buffer;
    (void)size;
    return true;
}
