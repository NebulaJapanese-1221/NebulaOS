// NebulaOS GUI - Rendering Subsystem
// =====================================
//
// Rendering system for the GUI

#ifndef NEBULAOS_GUI_RENDERING_H
#define NEBULAOS_GUI_RENDERING_H

#include "../include/GuiTypes.h"
#include "../include/Rectangle.h"
#include "../include/Color.h"

namespace NebulaOS {
namespace GUI {

class GraphicsContext;
class Font;

// Rendering backend type
enum class RenderingBackend {
    RENDER_BACKEND_SOFTWARE = 0,
    RENDER_BACKEND_VESA = 1,
    RENDER_BACKEND_VGA = 2,
    RENDER_BACKEND_OPENGL = 3,
    RENDER_BACKEND_VULKAN = 4
};

// Rendering flags
enum class RenderingFlags {
    RENDER_FLAG_NONE = 0,
    RENDER_FLAG_DOUBLE_BUFFER = 1,
    RENDER_FLAG_VSYNC = 2,
    RENDER_FLAG_HARDWARE_ACCELERATED = 4
};

// Rendering information
struct RenderingInfo {
    RenderingBackend backend;
    RenderingFlags flags;
    uint32_t width;
    uint32_t height;
    uint32_t bitsPerPixel;
    uint32_t stride;
    void* frameBuffer;
    void* backBuffer;
};

// Rendering context class
class RenderingContext {
public:
    RenderingContext();
    virtual ~RenderingContext();
    
    // Initialize rendering
    virtual bool Initialize(uint32_t width, uint32_t height, uint32_t bpp, void* frameBuffer);
    virtual void Shutdown();
    
    // Clear screen
    virtual void Clear(const Color& color = Color::Black());
    virtual void ClearRect(const Rectangle& rect, const Color& color);
    
    // Flush rendering
    virtual void Flush();
    virtual void SwapBuffers();
    
    // Get rendering info
    virtual RenderingInfo GetInfo() const;
    
    // Set rendering flags
    virtual void SetFlags(RenderingFlags flags);
    virtual RenderingFlags GetFlags() const;
};

// Initialize rendering subsystem
bool InitializeRendering(uint32_t width, uint32_t height, uint32_t bpp, void* frameBuffer);

// Shutdown rendering subsystem
void ShutdownRendering();

// Get rendering context
RenderingContext* GetRenderingContext();

// Create rendering context
RenderingContext* CreateRenderingContext(RenderingBackend backend);

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_RENDERING_H
