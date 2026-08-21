// NebulaOS GUI - GraphicsContext Implementation
// ==============================================
//
// Implementation of graphics drawing context

#include "../include/GraphicsContext.h"
#include "../include/GuiTypes.h"
#include "../include/Font.h"
#include "../../lib/include/string.h"
#include "../../kernel/common/include/nebula.h"

// Helper functions to replace my_min, my_max, my_swap, abs
static inline int32_t my_min(int32_t a, int32_t b) { return a < b ? a : b; }
static inline int32_t my_max(int32_t a, int32_t b) { return a > b ? a : b; }
static inline void my_swap(int32_t& a, int32_t& b) { int32_t t = a; a = b; b = t; }
static inline int32_t my_abs(int32_t a) { return a < 0 ? -a : a; }

namespace NebulaOS {
namespace GUI {

// -----------------------------------------------------------------------------
// Constructors
// -----------------------------------------------------------------------------

GraphicsContext::GraphicsContext() {
    buffer = nullptr;
    width = 0;
    height = 0;
    stride = 0;
    bpp = 32;
    currentColor = Color::White();
    backgroundColor = Color::Black();
    currentFont = nullptr;
    clipRect = Rectangle(0, 0, 0, 0);
    stateSaved = false;
}

GraphicsContext::GraphicsContext(void* buffer, uint32_t width, uint32_t height, uint32_t stride) {
    this->buffer = buffer;
    this->width = width;
    this->height = height;
    this->stride = stride;
    this->bpp = 32;
    currentColor = Color::White();
    backgroundColor = Color::Black();
    currentFont = nullptr;
    clipRect = Rectangle(0, 0, width, height);
    stateSaved = false;
}

GraphicsContext::GraphicsContext(const GraphicsContext& other) {
    buffer = other.buffer;
    width = other.width;
    height = other.height;
    stride = other.stride;
    bpp = other.bpp;
    currentColor = other.currentColor;
    backgroundColor = other.backgroundColor;
    currentFont = other.currentFont;
    clipRect = other.clipRect;
    stateSaved = false;
}

GraphicsContext::~GraphicsContext() {
    // Nothing to free (buffer is owned by caller)
}

// -----------------------------------------------------------------------------
// Assignment operator
// -----------------------------------------------------------------------------

GraphicsContext& GraphicsContext::operator=(const GraphicsContext& other) {
    if (this != &other) {
        buffer = other.buffer;
        width = other.width;
        height = other.height;
        stride = other.stride;
        bpp = other.bpp;
        currentColor = other.currentColor;
        backgroundColor = other.backgroundColor;
        currentFont = other.currentFont;
        clipRect = other.clipRect;
        stateSaved = false;
    }
    return *this;
}

// -----------------------------------------------------------------------------
// Clear the entire context
// -----------------------------------------------------------------------------

void GraphicsContext::Clear(const Color& color) {
    FillRect(Rectangle(0, 0, width, height), color);
}

// -----------------------------------------------------------------------------
// Clear a rectangle
// -----------------------------------------------------------------------------

void GraphicsContext::ClearRect(const Rectangle& rect, const Color& color) {
    FillRect(rect, color);
}

// -----------------------------------------------------------------------------
// Draw a single pixel
// -----------------------------------------------------------------------------

void GraphicsContext::DrawPixel(int32_t x, int32_t y, const Color& color) {
    if (IsPointInClip(x, y)) {
        SetPixelInternal(x, y, color);
    }
}

void GraphicsContext::DrawPixel(const Point& point, const Color& color) {
    DrawPixel(point.x, point.y, color);
}

// -----------------------------------------------------------------------------
// Draw a line
// -----------------------------------------------------------------------------

void GraphicsContext::DrawLine(int32_t x1, int32_t y1, int32_t x2, int32_t y2, const Color& color) {
    DrawBresenhamLine(x1, y1, x2, y2, color);
}

void GraphicsContext::DrawLine(const Point& p1, const Point& p2, const Color& color) {
    DrawLine(p1.x, p1.y, p2.x, p2.y, color);
}

// -----------------------------------------------------------------------------
// Draw rectangle outline
// -----------------------------------------------------------------------------

void GraphicsContext::DrawRect(const Rectangle& rect, const Color& color) {
    DrawRect(rect.x, rect.y, rect.width, rect.height, color);
}

void GraphicsContext::DrawRect(int32_t x, int32_t y, uint32_t width, uint32_t height, const Color& color) {
    if (width == 0 || height == 0) return;
    
    int32_t x2 = x + (int32_t)width - 1;
    int32_t y2 = y + (int32_t)height - 1;
    
    // Top and bottom borders
    DrawHorizontalLine(x, x2, y, color);
    DrawHorizontalLine(x, x2, y2, color);
    
    // Left and right borders
    DrawVerticalLine(x, y + 1, y2 - 1, color);
    DrawVerticalLine(x2, y + 1, y2 - 1, color);
}

// -----------------------------------------------------------------------------
// Fill rectangle
// -----------------------------------------------------------------------------

void GraphicsContext::FillRect(const Rectangle& rect, const Color& color) {
    FillRect(rect.x, rect.y, rect.width, rect.height, color);
}

void GraphicsContext::FillRect(int32_t x, int32_t y, uint32_t width, uint32_t height, const Color& color) {
    if (width == 0 || height == 0) return;
    
    int32_t x2 = x + (int32_t)width - 1;
    int32_t y2 = y + (int32_t)height - 1;
    
    // Clip the rectangle
    Rectangle clip = GetClippedRect(Rectangle(x, y, width, height));
    if (clip.IsEmpty()) return;
    
    for (int32_t row = clip.y; row <= clip.GetBottom() - 1; row++) {
        FillScanLine(row, clip.x, clip.GetRight() - 1, color);
    }
}

// -----------------------------------------------------------------------------
// Draw round rectangle
// -----------------------------------------------------------------------------

void GraphicsContext::DrawRoundRect(const Rectangle& rect, uint32_t cornerRadius, const Color& color) {
    // Simplified implementation - draw rectangle with rounded corners
    // In a real implementation, this would draw proper rounded corners
    DrawRect(rect, color);
}

void GraphicsContext::FillRoundRect(const Rectangle& rect, uint32_t cornerRadius, const Color& color) {
    FillRect(rect, color);
}

// -----------------------------------------------------------------------------
// Draw circle
// -----------------------------------------------------------------------------

void GraphicsContext::DrawCircle(int32_t x, int32_t y, uint32_t radius, const Color& color) {
    DrawCircle(Point(x, y), radius, color);
}

void GraphicsContext::DrawCircle(const Point& center, uint32_t radius, const Color& color) {
    // Midpoint circle algorithm
    int32_t x = radius;
    int32_t y = 0;
    int32_t err = 0;
    
    while (x >= y) {
        DrawPixel(center.x + x, center.y + y, color);
        DrawPixel(center.x + y, center.y + x, color);
        DrawPixel(center.x - y, center.y + x, color);
        DrawPixel(center.x - x, center.y + y, color);
        DrawPixel(center.x - x, center.y - y, color);
        DrawPixel(center.x - y, center.y - x, color);
        DrawPixel(center.x + y, center.y - x, color);
        DrawPixel(center.x + x, center.y - y, color);
        
        y++;
        err += 1 + 2 * y;
        if (2 * (err - x) + 1 > 0) {
            x--;
            err += 1 - 2 * x;
        }
    }
}

// -----------------------------------------------------------------------------
// Fill circle
// -----------------------------------------------------------------------------

void GraphicsContext::FillCircle(int32_t x, int32_t y, uint32_t radius, const Color& color) {
    FillCircle(Point(x, y), radius, color);
}

void GraphicsContext::FillCircle(const Point& center, uint32_t radius, const Color& color) {
    // Midpoint circle algorithm with filling
    int32_t x = radius;
    int32_t y = 0;
    int32_t err = 0;
    
    while (x >= y) {
        DrawHorizontalLine(center.x - x, center.x + x, center.y + y, color);
        DrawHorizontalLine(center.x - x, center.x + x, center.y - y, color);
        DrawHorizontalLine(center.x - y, center.x + y, center.y + x, color);
        DrawHorizontalLine(center.x - y, center.x + y, center.y - x, color);
        
        y++;
        err += 1 + 2 * y;
        if (2 * (err - x) + 1 > 0) {
            x--;
            err += 1 - 2 * x;
        }
    }
}

// -----------------------------------------------------------------------------
// Draw ellipse
// -----------------------------------------------------------------------------

void GraphicsContext::DrawEllipse(const Rectangle& bounds, const Color& color) {
    // Midpoint ellipse algorithm
    int32_t x = 0;
    int32_t y = bounds.height / 2;
    int32_t a2 = (bounds.width / 2) * (bounds.width / 2);
    int32_t b2 = (bounds.height / 2) * (bounds.height / 2);
    int32_t crit1 = -a2 * (bounds.height / 2);
    int32_t crit2 = -b2 * (bounds.width / 2);
    int32_t err = b2 + crit1 + b2;
    
    int32_t cx = bounds.x + bounds.width / 2;
    int32_t cy = bounds.y + bounds.height / 2;
    
    while (y >= 0) {
        DrawPixel(cx + x, cy + y, color);
        DrawPixel(cx - x, cy + y, color);
        DrawPixel(cx + x, cy - y, color);
        DrawPixel(cx - x, cy - y, color);
        
        if (err < crit1) {
            x++;
            err += 2 * b2 * x + b2;
        } else if (err > crit2) {
            y--;
            err -= 2 * a2 * y - a2;
        } else {
            x++;
            y--;
            err += 2 * (b2 * x - a2 * y) + b2;
        }
    }
}

// -----------------------------------------------------------------------------
// Fill ellipse
// -----------------------------------------------------------------------------

void GraphicsContext::FillEllipse(const Rectangle& bounds, const Color& color) {
    // Simplified: draw as filled rectangle for now
    FillRect(bounds, color);
}

// -----------------------------------------------------------------------------
// Text drawing
// -----------------------------------------------------------------------------

void GraphicsContext::DrawCharacter(char c, int32_t x, int32_t y, const Color& color, const Font* font) {
    // For now, just draw a placeholder rectangle
    // In a real implementation, this would render the character from a font
    FillRect(x, y, 8, 16, color);
}

void GraphicsContext::DrawText(const char* text, int32_t x, int32_t y, const Color& color, const Font* font) {
    if (!text) return;
    
    int32_t currX = x;
    int32_t currY = y;
    
    while (*text) {
        DrawCharacter(*text++, currX, currY, color, font);
        currX += 8; // Fixed width for now
        if (*text == '\n') {
            currX = x;
            currY += 16;
        }
    }
}

void GraphicsContext::DrawText(const char* text, const Rectangle& rect, const Color& color, 
                            FontStyle style, const Font* font) {
    // For now, just draw at the rectangle position
    DrawText(text, rect.x, rect.y, color, font);
}

// -----------------------------------------------------------------------------
// State management
// -----------------------------------------------------------------------------

void GraphicsContext::SaveState() {
    if (!stateSaved) {
        savedState.color = currentColor;
        savedState.bgColor = backgroundColor;
        savedState.font = currentFont;
        savedState.clip = clipRect;
        stateSaved = true;
    }
}

void GraphicsContext::RestoreState() {
    if (stateSaved) {
        currentColor = savedState.color;
        backgroundColor = savedState.bgColor;
        currentFont = savedState.font;
        clipRect = savedState.clip;
        stateSaved = false;
    }
}

// -----------------------------------------------------------------------------
// Clip management
// -----------------------------------------------------------------------------

void GraphicsContext::PushClip(const Rectangle& rect) {
    // For now, just set the clip
    SetClip(rect);
}

void GraphicsContext::PopClip() {
    // For now, restore to full context
    clipRect = Rectangle(0, 0, width, height);
}

void GraphicsContext::SetClip(const Rectangle& rect) {
    clipRect = rect;
}

Rectangle GraphicsContext::GetClip() const {
    return clipRect;
}

// -----------------------------------------------------------------------------
// Color management
// -----------------------------------------------------------------------------

void GraphicsContext::SetColor(const Color& color) {
    currentColor = color;
}

Color GraphicsContext::GetColor() const {
    return currentColor;
}

void GraphicsContext::SetBackgroundColor(const Color& color) {
    backgroundColor = color;
}

Color GraphicsContext::GetBackgroundColor() const {
    return backgroundColor;
}

// -----------------------------------------------------------------------------
// Font management
// -----------------------------------------------------------------------------

void GraphicsContext::SetFont(const Font* font) {
    currentFont = font;
}

const Font* GraphicsContext::GetFont() const {
    return currentFont;
}

int32_t GraphicsContext::GetTextWidth(const char* text) const {
    if (currentFont) {
        return currentFont->GetTextWidth(text);
    }
    // Default: assume each character is 8 pixels wide
    return strlen(text) * 8;
}

int32_t GraphicsContext::GetTextHeight(const char* text) const {
    if (currentFont) {
        return currentFont->GetTextHeight(text);
    }
    // Default: 16 pixels
    return 16;
}

int32_t GraphicsContext::GetCharWidth(char c) const {
    if (currentFont) {
        return currentFont->GetCharWidth(c);
    }
    // Default: 8 pixels
    return 8;
}

int32_t GraphicsContext::GetCharHeight(char c) const {
    if (currentFont) {
        return currentFont->GetCharHeight(c);
    }
    // Default: 16 pixels
    return 16;
}

// -----------------------------------------------------------------------------
// Draw mode management
// -----------------------------------------------------------------------------

void GraphicsContext::SetDrawMode(uint32_t mode) {
    // For now, ignore
}

uint32_t GraphicsContext::GetDrawMode() const {
    return 0;
}

// -----------------------------------------------------------------------------
// Serialization
// -----------------------------------------------------------------------------

void GraphicsContext::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    snprintf(buffer, size, "GraphicsContext(%dx%d, bpp=%d)", width, height, bpp);
}

// -----------------------------------------------------------------------------
// BitBlt - Bit block transfer
// -----------------------------------------------------------------------------

void GraphicsContext::BitBlt(GraphicsContext* src, int32_t srcX, int32_t srcY,
                           int32_t srcWidth, int32_t srcHeight,
                           int32_t destX, int32_t destY, uint32_t rop) {
    if (!src || !src->buffer || !buffer) return;
    
    // For now, just copy pixels (SRCCOPY)
    for (int32_t y = 0; y < srcHeight; y++) {
        for (int32_t x = 0; x < srcWidth; x++) {
            if (srcX + x >= 0 && srcX + x < (int32_t)src->width &&
                srcY + y >= 0 && srcY + y < (int32_t)src->height &&
                destX + x >= 0 && destX + x < (int32_t)width &&
                destY + y >= 0 && destY + y < (int32_t)height) {
                
                Color pixel = src->GetPixelInternal(srcX + x, srcY + y);
                SetPixelInternal(destX + x, destY + y, pixel);
            }
        }
    }
}

// -----------------------------------------------------------------------------
// StretchBlt - Stretch bit block transfer
// -----------------------------------------------------------------------------

void GraphicsContext::StretchBlt(GraphicsContext* src, int32_t srcX, int32_t srcY,
                               int32_t srcWidth, int32_t srcHeight,
                               int32_t destX, int32_t destY, int32_t destWidth, int32_t destHeight,
                               uint32_t rop) {
    // For now, same as BitBlt
    BitBlt(src, srcX, srcY, srcWidth, srcHeight, destX, destY, rop);
}

// -----------------------------------------------------------------------------
// Gradient fill
// -----------------------------------------------------------------------------

void GraphicsContext::FillGradient(const Rectangle& rect, const Color& color1, const Color& color2,
                                 bool horizontal) {
    // Simple linear gradient
    int32_t steps = horizontal ? rect.width : rect.height;
    if (steps <= 0) return;
    
    for (int32_t i = 0; i < steps; i++) {
        float ratio = (float)i / (steps - 1);
        Color color = color1.Blend(color2, ratio);
        
        if (horizontal) {
            DrawVerticalLine(rect.x + i, rect.y, rect.y + rect.height - 1, color);
        } else {
            DrawHorizontalLine(rect.x, rect.x + rect.width - 1, rect.y + i, color);
        }
    }
}

// -----------------------------------------------------------------------------
// Pattern fill
// -----------------------------------------------------------------------------

void GraphicsContext::FillPattern(const Rectangle& rect, uint8_t pattern[8]) {
    // For now, just fill with background color
    FillRect(rect, backgroundColor);
}

// -----------------------------------------------------------------------------
// Alpha blending
// -----------------------------------------------------------------------------

void GraphicsContext::DrawWithAlpha(const GraphicsContext* src, int32_t x, int32_t y, uint8_t alpha) {
    // For now, just copy without alpha
    if (src) {
        BitBlt(const_cast<GraphicsContext*>(src), 0, 0, src->width, src->height, x, y);
    }
}

// -----------------------------------------------------------------------------
// Helper functions
// -----------------------------------------------------------------------------

void GraphicsContext::DrawHorizontalLine(int32_t x1, int32_t x2, int32_t y, const Color& color) {
    if (y < 0 || y >= (int32_t)height) return;
    if (x1 > x2) my_swap(x1, x2);
    
    for (int32_t x = x1; x <= x2; x++) {
        if (x >= 0 && x < (int32_t)width) {
            SetPixelInternal(x, y, color);
        }
    }
}

void GraphicsContext::DrawVerticalLine(int32_t x, int32_t y1, int32_t y2, const Color& color) {
    if (x < 0 || x >= (int32_t)width) return;
    if (y1 > y2) my_swap(y1, y2);
    
    for (int32_t y = y1; y <= y2; y++) {
        if (y >= 0 && y < (int32_t)height) {
            SetPixelInternal(x, y, color);
        }
    }
}

bool GraphicsContext::IsPointInClip(int32_t x, int32_t y) const {
    return x >= clipRect.x && x < clipRect.x + (int32_t)clipRect.width &&
           y >= clipRect.y && y < clipRect.y + (int32_t)clipRect.height;
}

bool GraphicsContext::IsRectInClip(const Rectangle& rect) const {
    return !(rect.GetRight() <= clipRect.x ||
             rect.x >= clipRect.GetRight() ||
             rect.GetBottom() <= clipRect.y ||
             rect.y >= clipRect.GetBottom());
}

Rectangle GraphicsContext::GetClippedRect(const Rectangle& rect) const {
    int32_t left = my_max(rect.x, (int32_t)clipRect.x);
    int32_t top = my_max(rect.y, (int32_t)clipRect.y);
    int32_t right = my_min(rect.GetRight(), clipRect.GetRight());
    int32_t bottom = my_min(rect.GetBottom(), clipRect.GetBottom());
    
    if (right < left || bottom < top) {
        return Rectangle();
    }
    return Rectangle(left, top, right - left, bottom - top);
}

void GraphicsContext::SetPixelInternal(int32_t x, int32_t y, const Color& color) {
    if (x < 0 || x >= (int32_t)width || y < 0 || y >= (int32_t)height) return;
    
    // For 32-bit color (RGBA)
    if (bpp == 32) {
        uint32_t* pixels = (uint32_t*)buffer;
        uint32_t offset = y * (stride / 4) + x;
        pixels[offset] = color.ToARGB();
    }
}

Color GraphicsContext::GetPixelInternal(int32_t x, int32_t y) const {
    if (x < 0 || x >= (int32_t)width || y < 0 || y >= (int32_t)height) {
        return Color::Black();
    }
    
    if (bpp == 32) {
        uint32_t* pixels = (uint32_t*)buffer;
        uint32_t offset = y * (stride / 4) + x;
        return Color::FromARGB(pixels[offset]);
    }
    
    return Color::Black();
}

// -----------------------------------------------------------------------------
// Bresenham line algorithm
// -----------------------------------------------------------------------------

void GraphicsContext::DrawBresenhamLine(int32_t x1, int32_t y1, int32_t x2, int32_t y2, const Color& color) {
    int32_t dx = my_abs(x2 - x1);
    int32_t dy = my_abs(y2 - y1);
    int32_t sx = (x1 < x2) ? 1 : -1;
    int32_t sy = (y1 < y2) ? 1 : -1;
    int32_t err = dx - dy;
    
    while (true) {
        DrawPixel(x1, y1, color);
        
        if (x1 == x2 && y1 == y2) break;
        
        int32_t e2 = 2 * err;
        if (e2 > -dy) {
            err -= dy;
            x1 += sx;
        }
        if (e2 < dx) {
            err += dx;
            y1 += sy;
        }
    }
}

void GraphicsContext::FillScanLine(int32_t y, int32_t x1, int32_t x2, const Color& color) {
    if (x1 > x2) my_swap(x1, x2);
    DrawHorizontalLine(x1, x2, y, color);
}

} // namespace GUI
} // namespace NebulaOS
