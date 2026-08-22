// NebulaOS - GUI to Kernel Bridge
// ================================
//
// C linkage bridge so the C kernel can drive the C++ GUI and, in doing so,
// trigger the real-mode VBE display switch and render to the real linear
// framebuffer.

#include "../include/GUI.h"
#include "../include/WindowManager.h"
#include "../include/GraphicsContext.h"
#include "../include/Color.h"

// The video driver is written in C; declare the symbols we need with C
// linkage so the C++ compiler emits unmangled references that match the
// C-compiled objects. Note: `bool` is `int` in the C sources, so the single
// boolean parameter is declared as `int` here to avoid an ABI mismatch.
extern "C" {
    void    vbe_set_real_mode_switch(int enable);
    uint16_t vbe_get_mode(void);
    int     vesa_set_mode(uint16_t mode);
    void*   vesa_get_framebuffer(void);
    uint32_t vesa_get_width(void);
    uint32_t vesa_get_height(void);
    uint32_t vesa_get_bpp(void);
    uint32_t vesa_get_stride(void);
}

// -----------------------------------------------------------------------------
// Enter graphics mode using the real VBE switch and initialize the GUI with
// the real linear framebuffer. Returns 1 on success, 0 if graphics is
// unavailable (the caller should keep text mode).
// -----------------------------------------------------------------------------
extern "C" int nebula_gui_enter_graphics(void) {
    using namespace NebulaOS::GUI;

    // Enable the real display mode switch (performed through the real-mode
    // BIOS interface). The actual INT 0x10 set-mode happens in vesa_set_mode.
    vbe_set_real_mode_switch(1);

    uint16_t mode = vbe_get_mode();
    if (!vesa_set_mode(mode)) {
        return 0;
    }

    void* fb = vesa_get_framebuffer();
    uint32_t w = vesa_get_width();
    uint32_t h = vesa_get_height();
    uint32_t bpp = vesa_get_bpp();
    uint32_t stride = vesa_get_stride();

    if (!fb || w == 0 || h == 0) {
        return 0;
    }

    if (!InitializeGUI(w, h, bpp, fb)) {
        return 0;
    }

    // Paint the desktop directly into the real framebuffer so the switch to
    // graphics mode produces visible output.
    WindowManager* wm = WindowManager::GetInstance();
    if (wm) {
        wm->SetDesktopColor(Color(20, 40, 80));
        GraphicsContext gc(fb, w, h, stride);
        wm->PaintDesktop(gc);
    }

    return 1;
}

// -----------------------------------------------------------------------------
// Run the GUI message loop (does not return under normal operation).
// -----------------------------------------------------------------------------
extern "C" void nebula_gui_run(void) {
    NebulaOS::GUI::RunMessageLoop();
}
