// NebulaOS GUI - Graphics Context
// =================================
//
// Graphics drawing context for rendering

#ifndef NEBULAOS_GUI_GRAPHICSCONTEXT_H
#define NEBULAOS_GUI_GRAPHICSCONTEXT_H

#include "../include/Color.h"
#include "../include/Rectangle.h"
#include "../include/Point.h"
#include "../include/Size.h"
#include "../include/GuiTypes.h"

namespace NebulaOS {
namespace GUI {

// Font class (forward declaration)
class Font;

// Graphics context class
class GraphicsContext {
public:
    // Constructor
    GraphicsContext();
    GraphicsContext(void* buffer, uint32_t width, uint32_t height, uint32_t stride);
    GraphicsContext(const GraphicsContext& other);
    
    ~GraphicsContext();
    
    // Copy assignment
    GraphicsContext& operator=(const GraphicsContext& other);
    
    // Drawing primitives
    void Clear(const Color& color = Color::Black());
    void ClearRect(const Rectangle& rect, const Color& color);
    
    void DrawPixel(int32_t x, int32_t y, const Color& color);
    void DrawPixel(const Point& point, const Color& color);
    
    void DrawLine(int32_t x1, int32_t y1, int32_t x2, int32_t y2, const Color& color);
    void DrawLine(const Point& p1, const Point& p2, const Color& color);
    
    void DrawRect(const Rectangle& rect, const Color& color);
    void DrawRect(int32_t x, int32_t y, uint32_t width, uint32_t height, const Color& color);
    
    void FillRect(const Rectangle& rect, const Color& color);
    void FillRect(int32_t x, int32_t y, uint32_t width, uint32_t height, const Color& color);
    
    void DrawRoundRect(const Rectangle& rect, uint32_t cornerRadius, const Color& color);
    void FillRoundRect(const Rectangle& rect, uint32_t cornerRadius, const Color& color);
    
    void DrawCircle(int32_t x, int32_t y, uint32_t radius, const Color& color);
    void DrawCircle(const Point& center, uint32_t radius, const Color& color);
    
    void FillCircle(int32_t x, int32_t y, uint32_t radius, const Color& color);
    void FillCircle(const Point& center, uint32_t radius, const Color& color);
    
    void DrawEllipse(const Rectangle& bounds, const Color& color);
    void FillEllipse(const Rectangle& bounds, const Color& color);
    
    // Text drawing
    void DrawCharacter(char c, int32_t x, int32_t y, const Color& color, const Font* font = nullptr);
    void DrawText(const char* text, int32_t x, int32_t y, const Color& color, const Font* font = nullptr);
    void DrawText(const char* text, const Rectangle& rect, const Color& color, 
                  FontStyle style = FontStyle::FONT_NORMAL, const Font* font = nullptr);
    
    // State management
    void SaveState();
    void RestoreState();
    
    void PushClip(const Rectangle& rect);
    void PopClip();
    void SetClip(const Rectangle& rect);
    Rectangle GetClip() const;
    
    void SetColor(const Color& color);
    Color GetColor() const;
    
    void SetBackgroundColor(const Color& color);
    Color GetBackgroundColor() const;
    
    void SetFont(const Font* font);
    const Font* GetFont() const;
    
    int32_t GetTextWidth(const char* text) const;
    int32_t GetTextHeight(const char* text) const;
    int32_t GetCharWidth(char c) const;
    int32_t GetCharHeight(char c) const;
    
    void SetDrawMode(uint32_t mode);  // For ROP2 modes (CopyPen, XorPen, etc.)
    uint32_t GetDrawMode() const;
    
    // Buffer information
    uint32_t GetWidth() const { return width; }
    uint32_t GetHeight() const { return height; }
    uint32_t GetStride() const { return stride; }
    uint32_t GetBitsPerPixel() const { return bpp; }
    void* GetBuffer() const { return buffer; }
    
    // Blitting
    void BitBlt(GraphicsContext* src, int32_t srcX, int32_t srcY, 
                int32_t srcWidth, int32_t srcHeight,
                int32_t destX, int32_t destY, uint32_t rop = 0xCCAA0020 /* SRCCOPY */);
    
    void StretchBlt(GraphicsContext* src, int32_t srcX, int32_t srcY,
                   int32_t srcWidth, int32_t srcHeight,
                   int32_t destX, int32_t destY, int32_t destWidth, int32_t destHeight,
                   uint32_t rop = 0xCCAA0020 /* SRCCOPY */);
    
    // Gradient fills
    void FillGradient(const Rectangle& rect, const Color& color1, const Color& color2, 
                      bool horizontal = true);
    
    // Pattern fills
    void FillPattern(const Rectangle& rect, uint8_t pattern[8]);
    
    // Alpha blending
    void DrawWithAlpha(const GraphicsContext* src, int32_t x, int32_t y, uint8_t alpha);
    
    // Serialization
    void ToString(char* buffer, size_t size) const;

private:
    void* buffer;            // Frame buffer pointer
    uint32_t width;          // Width in pixels
    uint32_t height;         // Height in pixels
    uint32_t stride;         // Stride in bytes
    uint32_t bpp;            // Bits per pixel
    
    Color currentColor;     // Current drawing color
    Color backgroundColor; // Current background color
    const Font* currentFont; // Current font
    
    Rectangle clipRect;     // Current clipping rectangle
    
    // For state saving
    struct GraphicsState {
        Color color;
        Color bgColor;
        const Font* font;
        Rectangle clip;
        uint32_t drawMode;
    };
    
    GraphicsState savedState;
    bool stateSaved;
    
    // Helper functions
    void DrawHorizontalLine(int32_t x1, int32_t x2, int32_t y, const Color& color);
    void DrawVerticalLine(int32_t x, int32_t y1, int32_t y2, const Color& color);
    
    bool IsPointInClip(int32_t x, int32_t y) const;
    bool IsRectInClip(const Rectangle& rect) const;
    Rectangle GetClippedRect(const Rectangle& rect) const;
    
    void SetPixelInternal(int32_t x, int32_t y, const Color& color);
    Color GetPixelInternal(int32_t x, int32_t y) const;
    
    void DrawBresenhamLine(int32_t x1, int32_t y1, int32_t x2, int32_t y2, const Color& color);
    void FillScanLine(int32_t y, int32_t x1, int32_t x2, const Color& color);
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_GRAPHICSCONTEXT_H
