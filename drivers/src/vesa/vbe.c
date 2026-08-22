// NebulaOS - VBE BIOS Interface
// ==============================
//
// VBE 2.0+ BIOS interface. On x86 the kernel runs in protected mode, so
// real INT 0x10 calls are performed through the real-mode BIOS interface
// (kernel/x86/src/device/realmode.*). When that interface is unavailable
// (e.g. x86_64, or early boot) a static fallback is used.

#include "../include/vesa.h"
#include "../include/vbe.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"

#ifdef NEBULAOS_ARCH_X86
#include "../../kernel/x86/src/device/realmode.h"
#endif

// -----------------------------------------------------------------------------
// Supported mode list (static fallback only)
// -----------------------------------------------------------------------------

static const uint16_t vbe_supported_modes[] = {
    0x0118,
    0x011A,
    0x014C,
    0xFFFF
};

// When false, vbe_set_mode only records the requested mode and does not
// actually switch the display. The GUI enables this once it is ready to
// take over the framebuffer (see VESA framebuffer rendering task).
static bool rm_mode_switch_enabled = false;

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

// Convert a VBE 32-bit segmented pointer (offset:segment) to a flat
// identity-mapped address usable by the kernel.
#ifdef NEBULAOS_ARCH_X86
static uint32_t rm_flat_ptr(uint32_t seg_off) {
    uint16_t off = (uint16_t)(seg_off & 0xFFFF);
    uint16_t seg = (uint16_t)(seg_off >> 16);
    return ((uint32_t)seg << 4) + off;
}
#endif

// -----------------------------------------------------------------------------
// Get VBE controller information (real INT 0x10, AX=0x4F00)
// -----------------------------------------------------------------------------

bool vbe_get_info(vbe_info_block_t* info) {
    if (!info) {
        return false;
    }

#ifdef NEBULAOS_ARCH_X86
    if (realmode_available()) {
        void* buf = realmode_vbe_info_buffer();
        rm_regs_t regs;
        memset8(&regs, 0, sizeof(regs));
        regs.eax = 0x4F00;
        regs.es = (uint16_t)(((uint32_t)buf) >> 4);
        regs.edi = (uint32_t)buf & 0xF;

        if (realmode_call(0x10, &regs) && (regs.eax & 0xFF) == 0x4F) {
            memcpy8(info, buf, sizeof(*info));
            info->video_modes = rm_flat_ptr(info->video_modes);
            return true;
        }
    }
#endif

    // Static fallback
    memcpy8(info->signature, "VESA", 4);
    info->version = 0x0300;
    info->oem_string = 0;
    info->capabilities = VBE_CAPABILITY_LFB;
    info->video_modes = (uint32_t)vbe_supported_modes;
    info->total_memory = 0;
    info->oem_software_rev = 0;
    info->oem_vendor_name = 0;
    info->oem_product_name = 0;
    info->oem_product_rev = 0;

    return true;
}

// -----------------------------------------------------------------------------
// Get VBE mode information (real INT 0x10, AX=0x4F01)
// -----------------------------------------------------------------------------

bool vbe_get_mode_info(uint16_t mode, vbe_mode_info_t* info) {
    if (!info) {
        return false;
    }

#ifdef NEBULAOS_ARCH_X86
    if (realmode_available()) {
        void* buf = realmode_vbe_mode_buffer();
        rm_regs_t regs;
        memset8(&regs, 0, sizeof(regs));
        regs.eax = 0x4F01;
        regs.ecx = mode;
        regs.es = (uint16_t)(((uint32_t)buf) >> 4);
        regs.edi = (uint32_t)buf & 0xF;

        if (realmode_call(0x10, &regs) && (regs.eax & 0xFF) == 0x4F) {
            memcpy8(info, buf, sizeof(*info));
            return true;
        }
    }
#endif

    // Static fallback
    memset8(info, 0, sizeof(*info));

    if (mode == 0x0118 || mode == 0x011A || mode == 0x014C) {
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
// Set VBE video mode (real INT 0x10, AX=0x4F02)
// -----------------------------------------------------------------------------

static uint16_t vbe_current_mode = 0x0118;

bool vbe_set_mode(uint16_t mode) {
    vbe_mode_info_t minfo;
    if (!vbe_get_mode_info(mode, &minfo)) {
        return false;
    }

#ifdef NEBULAOS_ARCH_X86
    if (realmode_available() && rm_mode_switch_enabled) {
        rm_regs_t regs;
        memset8(&regs, 0, sizeof(regs));
        regs.eax = 0x4F02;
        regs.ebx = mode | 0x4000;   // linear framebuffer
        regs.es = 0;
        regs.edi = 0;               // use default CRTC
        if (!(realmode_call(0x10, &regs) && (regs.eax & 0xFF) == 0x4F)) {
            return false;
        }
    }
#endif

    vbe_current_mode = mode;
    return true;
}

// Enable the real display mode switch (used by the GUI).
void vbe_set_real_mode_switch(bool enable) {
    rm_mode_switch_enabled = enable;
}

// True once the real display mode switch has been enabled.
bool vbe_real_mode_switch_active(void) {
    return rm_mode_switch_enabled;
}

// -----------------------------------------------------------------------------
// Get current VBE mode
// -----------------------------------------------------------------------------

uint16_t vbe_get_mode(void) {
    return vbe_current_mode;
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
