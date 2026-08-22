// NebulaOS - VESA VBE Driver
// ===========================
//
// VESA BIOS Extensions driver implementation

#include "../include/vesa.h"
#include "../include/vbe.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/vga.h"

// -----------------------------------------------------------------------------
// VESA state
// -----------------------------------------------------------------------------

static vesa_state_t vesa_state;
static vbe_info_block_t vbe_info_storage;
static vbe_mode_info_t vbe_mode_info_storage;

// -----------------------------------------------------------------------------
// Simple helper to print hex digit
// -----------------------------------------------------------------------------

static void vga_put_hex_digit(uint8_t digit) {
    char c = (digit < 10) ? ('0' + digit) : ('A' + digit - 10);
    vga_putchar(c);
}

static void vga_put_hex(uint32_t value, uint8_t digits) {
    for (int i = digits - 1; i >= 0; i--) {
        vga_put_hex_digit((value >> (i * 4)) & 0xF);
    }
}

// -----------------------------------------------------------------------------
// Manual copy and zero helpers
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
// Initialize VESA
// -----------------------------------------------------------------------------

bool vesa_init(void) {
    memset8(&vesa_state, 0, sizeof(vesa_state));
    memset8(&vbe_info_storage, 0, sizeof(vbe_info_storage));
    memset8(&vbe_mode_info_storage, 0, sizeof(vbe_mode_info_storage));

    vesa_state.vbe_info = &vbe_info_storage;
    vesa_state.mode_info = &vbe_mode_info_storage;

    vga_puts("VESA: Initializing...\n");

    // Get VBE info
    if (!vbe_get_info(vesa_state.vbe_info)) {
        vga_puts("VESA: VBE not supported\n");
        return false;
    }

    vga_puts("VESA: VBE Version ");
    vga_put_hex((vesa_state.vbe_info->version >> 8) & 0xFF, 1);
    vga_putchar('.');
    vga_put_hex(vesa_state.vbe_info->version & 0xFF, 1);
    vga_puts("\n");

    // Find 1024x768x32 mode
    uint16_t mode = 0;
    if (!vesa_find_mode(1024, 768, 32, &mode)) {
        vga_puts("VESA: 1024x768x32 mode not found\n");
        return false;
    }

    vga_puts("VESA: Found mode 0x");
    vga_put_hex(mode, 4);
    vga_puts(" (1024x768x32)\n");

    // Set video mode
    if (!vesa_set_mode(mode)) {
        vga_puts("VESA: Failed to set video mode\n");
        return false;
    }

    vesa_state.current_mode = mode;
    vesa_state.initialized = true;

    vga_puts("VESA: Initialized\n");
    return true;
}

// -----------------------------------------------------------------------------
// Set video mode
// -----------------------------------------------------------------------------

bool vesa_set_mode(uint16_t mode) {
    vbe_mode_info_t mode_info;

    if (!vesa_get_mode_info(mode, &mode_info)) {
        return false;
    }

    // Set mode via BIOS
    if (!vbe_set_mode(mode)) {
        return false;
    }

    vesa_state.width = mode_info.x_resolution;
    vesa_state.height = mode_info.y_resolution;
    vesa_state.bpp = mode_info.bits_per_pixel;
    vesa_state.stride = mode_info.bytes_per_scanline;
    vesa_state.framebuffer = (void*)(size_t)mode_info.phys_base_ptr;
    vesa_state.framebuffer_size = vesa_state.stride * vesa_state.height;
    vesa_state.lfb_enabled = (mode_info.mode_attributes & VBE_MODE_ATTR_LINEAR) != 0;
    vesa_state.current_mode = mode;

    memcpy8(vesa_state.mode_info, &mode_info, sizeof(mode_info));

    return true;
}

// -----------------------------------------------------------------------------
// Get mode info
// -----------------------------------------------------------------------------

bool vesa_get_mode_info(uint16_t mode, vbe_mode_info_t* info) {
    if (!info) {
        return false;
    }

    return vbe_get_mode_info(mode, info);
}

// -----------------------------------------------------------------------------
// Find mode matching resolution and bpp
// -----------------------------------------------------------------------------

bool vesa_find_mode(uint32_t width, uint32_t height, uint32_t bpp, uint16_t* mode) {
    if (!mode || !vesa_state.vbe_info) {
        return false;
    }

    uint32_t* mode_list = (uint32_t*)(size_t)vesa_state.vbe_info->video_modes;

    if (!mode_list) {
        return false;
    }

    for (size_t i = 0; mode_list[i] != 0xFFFF; i++) {
        uint16_t candidate = mode_list[i] & 0x7FFF;
        vbe_mode_info_t minfo;

        if (!vesa_get_mode_info(candidate, &minfo)) {
            continue;
        }

        if (minfo.x_resolution == width &&
            minfo.y_resolution == height &&
            minfo.bits_per_pixel == bpp &&
            (minfo.mode_attributes & VBE_MODE_ATTR_LINEAR)) {
            *mode = candidate;
            return true;
        }
    }

    return false;
}

// -----------------------------------------------------------------------------
// Get framebuffer address
// -----------------------------------------------------------------------------

void* vesa_get_framebuffer(void) {
    return vesa_state.framebuffer;
}

// -----------------------------------------------------------------------------
// Get current mode info
// -----------------------------------------------------------------------------

const vbe_mode_info_t* vesa_get_current_mode_info(void) {
    return vesa_state.mode_info;
}

// -----------------------------------------------------------------------------
// Get VBE info
// -----------------------------------------------------------------------------

const vbe_info_block_t* vesa_get_vbe_info(void) {
    return vesa_state.vbe_info;
}

// -----------------------------------------------------------------------------
// Get width
// -----------------------------------------------------------------------------

uint32_t vesa_get_width(void) {
    return vesa_state.width;
}

// -----------------------------------------------------------------------------
// Get height
// -----------------------------------------------------------------------------

uint32_t vesa_get_height(void) {
    return vesa_state.height;
}

// -----------------------------------------------------------------------------
// Get bpp
// -----------------------------------------------------------------------------

uint32_t vesa_get_bpp(void) {
    return vesa_state.bpp;
}

// -----------------------------------------------------------------------------
// Get stride
// -----------------------------------------------------------------------------

uint32_t vesa_get_stride(void) {
    return vesa_state.stride;
}

// -----------------------------------------------------------------------------
// Clear screen
// -----------------------------------------------------------------------------

void vesa_clear(uint32_t color) {
    if (!vesa_state.framebuffer) {
        return;
    }

    uint32_t* fb = (uint32_t*)vesa_state.framebuffer;
    uint32_t pixels = vesa_state.stride * vesa_state.height / 4;

    for (uint32_t i = 0; i < pixels; i++) {
        fb[i] = color;
    }
}

// -----------------------------------------------------------------------------
// Set pixel
// -----------------------------------------------------------------------------

void vesa_set_pixel(uint32_t x, uint32_t y, uint32_t color) {
    if (!vesa_state.framebuffer || x >= vesa_state.width || y >= vesa_state.height) {
        return;
    }

    uint32_t* fb = (uint32_t*)vesa_state.framebuffer;
    fb[y * vesa_state.stride / 4 + x] = color;
}

// -----------------------------------------------------------------------------
// Get pixel
// -----------------------------------------------------------------------------

uint32_t vesa_get_pixel(uint32_t x, uint32_t y) {
    if (!vesa_state.framebuffer || x >= vesa_state.width || y >= vesa_state.height) {
        return 0;
    }

    uint32_t* fb = (uint32_t*)vesa_state.framebuffer;
    return fb[y * vesa_state.stride / 4 + x];
}

// -----------------------------------------------------------------------------
// Check if linear framebuffer is enabled
// -----------------------------------------------------------------------------

bool vesa_is_linear_framebuffer(void) {
    return vesa_state.lfb_enabled;
}

// -----------------------------------------------------------------------------
// Check if direct color
// -----------------------------------------------------------------------------

bool vesa_is_direct_color(void) {
    if (!vesa_state.mode_info) {
        return false;
    }
    return vesa_state.mode_info->memory_model == VBE_MEMORY_MODEL_DIRECT;
}

// -----------------------------------------------------------------------------
// Get color masks
// -----------------------------------------------------------------------------

void vesa_get_color_masks(uint32_t* red_mask, uint32_t* green_mask, uint32_t* blue_mask) {
    if (!vesa_state.mode_info) {
        if (red_mask) *red_mask = 0;
        if (green_mask) *green_mask = 0;
        if (blue_mask) *blue_mask = 0;
        return;
    }

    if (red_mask) {
        *red_mask = BIT(vesa_state.mode_info->red_mask_size) - 1;
        *red_mask <<= vesa_state.mode_info->red_field_position;
    }
    if (green_mask) {
        *green_mask = BIT(vesa_state.mode_info->green_mask_size) - 1;
        *green_mask <<= vesa_state.mode_info->green_field_position;
    }
    if (blue_mask) {
        *blue_mask = BIT(vesa_state.mode_info->blue_mask_size) - 1;
        *blue_mask <<= vesa_state.mode_info->blue_field_position;
    }
}

// -----------------------------------------------------------------------------
// Make color from RGB
// -----------------------------------------------------------------------------

uint32_t vesa_make_color(uint8_t r, uint8_t g, uint8_t b) {
    if (!vesa_state.mode_info) {
        return 0;
    }

    uint32_t color = 0;
    uint32_t red_mask = BIT(vesa_state.mode_info->red_mask_size) - 1;
    uint32_t green_mask = BIT(vesa_state.mode_info->green_mask_size) - 1;
    uint32_t blue_mask = BIT(vesa_state.mode_info->blue_mask_size) - 1;

    color |= ((uint32_t)(r >> (8 - vesa_state.mode_info->red_mask_size)) & red_mask) << vesa_state.mode_info->red_field_position;
    color |= ((uint32_t)(g >> (8 - vesa_state.mode_info->green_mask_size)) & green_mask) << vesa_state.mode_info->green_field_position;
    color |= ((uint32_t)(b >> (8 - vesa_state.mode_info->blue_mask_size)) & blue_mask) << vesa_state.mode_info->blue_field_position;

    return color;
}
