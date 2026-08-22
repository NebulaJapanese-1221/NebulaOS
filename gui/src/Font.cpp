// NebulaOS GUI - Font Implementation
// =====================================
//
// Implementation of the Font classes

#include "../include/Font.h"
#include "../include/GraphicsContext.h"
#include "../../lib/include/string.h"

namespace NebulaOS {
namespace GUI {

// -----------------------------------------------------------------------------
// Font::Font - Constructor/Destructor
// -----------------------------------------------------------------------------

Font::Font() {
    name[0] = '\0';
    size = 12;
    style = FontStyle::FONT_NORMAL;
    bitmap = nullptr;
    bitmapWidth = 0;
    bitmapHeight = 0;
    firstChar = 0;
    charCount = 0;
    
    // Default metrics
    metrics.ascent = 10;
    metrics.descent = 4;
    metrics.height = 14;
    metrics.maxWidth = 8;
    metrics.avgWidth = 6;
}

Font::Font(const char* name, uint32_t size, FontStyle style) {
    if (name) {
        strncpy(this->name, name, sizeof(this->name) - 1);
        this->name[sizeof(this->name) - 1] = '\0';
    } else {
        this->name[0] = '\0';
    }
    this->size = size;
    this->style = style;
    bitmap = nullptr;
    bitmapWidth = 0;
    bitmapHeight = 0;
    firstChar = 0;
    charCount = 0;
    
    // Calculate metrics based on size
    metrics.ascent = (int32_t)(size * 0.7f);
    metrics.descent = (int32_t)(size * 0.3f);
    metrics.height = (int32_t)size + 2;
    metrics.maxWidth = (int32_t)(size * 0.6f);
    metrics.avgWidth = (int32_t)(size * 0.5f);
}

Font::~Font() {
    // Nothing to do (bitmap is not owned by this class)
}

// -----------------------------------------------------------------------------
// Font::Font properties
// -----------------------------------------------------------------------------

void Font::SetName(const char* newName) {
    if (newName) {
        strncpy(name, newName, sizeof(name) - 1);
        name[sizeof(name) - 1] = '\0';
    } else {
        name[0] = '\0';
    }
}

void Font::SetSize(uint32_t newSize) {
    size = newSize;
    metrics.ascent = (int32_t)(size * 0.7f);
    metrics.descent = (int32_t)(size * 0.3f);
    metrics.height = (int32_t)size + 2;
    metrics.maxWidth = (int32_t)(size * 0.6f);
    metrics.avgWidth = (int32_t)(size * 0.5f);
}

void Font::SetStyle(FontStyle newStyle) {
    style = newStyle;
}

// -----------------------------------------------------------------------------
// Font::Character metrics
// -----------------------------------------------------------------------------

CharMetrics Font::GetCharMetrics(char c) const {
    CharMetrics metrics;
    metrics.width = this->metrics.avgWidth;
    metrics.bearingX = 0;
    metrics.bearingY = this->metrics.ascent;
    metrics.advance = metrics.width + 1;
    return metrics;
}

int32_t Font::GetCharWidth(char c) const {
    return metrics.avgWidth;
}

int32_t Font::GetCharHeight(char c) const {
    return metrics.height;
}

// -----------------------------------------------------------------------------
// Font::Text measurement
// -----------------------------------------------------------------------------

Size Font::MeasureText(const char* text) const {
    if (!text || text[0] == '\0') {
        return Size(0, metrics.height);
    }
    
    uint32_t width = 0;
    uint32_t height = metrics.height;
    
    for (int i = 0; text[i] != '\0'; i++) {
        width += GetCharWidth(text[i]);
    }
    
    return Size(width, height);
}

int32_t Font::GetTextWidth(const char* text) const {
    if (!text) return 0;
    
    int32_t width = 0;
    for (int i = 0; text[i] != '\0'; i++) {
        width += GetCharWidth(text[i]);
    }
    return width;
}

int32_t Font::GetTextHeight(const char* text) const {
    return metrics.height;
}

// -----------------------------------------------------------------------------
// Font::Text rendering
// -----------------------------------------------------------------------------

void Font::DrawCharacter(GraphicsContext& gc, char c, int32_t x, int32_t y, const Color& color) const {
    // Default implementation: draw a simple rectangle for the character
    // In a real implementation, this would render the actual character
    
    CharMetrics cm = GetCharMetrics(c);
    gc.FillRect(x, y - cm.bearingY, cm.width, metrics.height, color);
}

void Font::DrawText(GraphicsContext& gc, const char* text, int32_t x, int32_t y, const Color& color) const {
    if (!text || text[0] == '\0') return;
    
    int32_t currentX = x;
    
    for (int i = 0; text[i] != '\0'; i++) {
        DrawCharacter(gc, text[i], currentX, y, color);
        currentX += GetCharWidth(text[i]);
    }
}

// -----------------------------------------------------------------------------
// Font::Serialization
// -----------------------------------------------------------------------------

void Font::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    const char* styleStr = "NORMAL";
    switch (style) {
        case FontStyle::FONT_BOLD: styleStr = "BOLD"; break;
        case FontStyle::FONT_ITALIC: styleStr = "ITALIC"; break;
        case FontStyle::FONT_UNDERLINE: styleStr = "UNDERLINE"; break;
        default: break;
    }
    
    snprintf(buffer, size, "Font(name='%s', size=%u, style=%s, height=%d)",
             name, size, styleStr, metrics.height);
}

// -----------------------------------------------------------------------------
// BuiltinFont implementation
// -----------------------------------------------------------------------------

BuiltinFont::BuiltinFont() : Font("Builtin", 12, FontStyle::FONT_NORMAL) {
    // Built-in font has fixed metrics
    metrics.ascent = 10;
    metrics.descent = 4;
    metrics.height = 14;
    metrics.maxWidth = 8;
    metrics.avgWidth = 6;
}

BuiltinFont::BuiltinFont(uint32_t size, FontStyle style) : Font("Builtin", size, style) {
    // Adjust metrics based on size
    metrics.ascent = (int32_t)(size * 0.7f);
    metrics.descent = (int32_t)(size * 0.3f);
    metrics.height = (int32_t)size + 2;
    metrics.maxWidth = (int32_t)(size * 0.6f);
    metrics.avgWidth = (int32_t)(size * 0.5f);
}

CharMetrics BuiltinFont::GetCharMetrics(char c) const {
    CharMetrics cm;
    cm.width = metrics.avgWidth;
    cm.bearingX = 0;
    cm.bearingY = metrics.ascent;
    cm.advance = cm.width + 1;
    return cm;
}

int32_t BuiltinFont::GetCharWidth(char c) const {
    // Simple fixed-width characters
    return metrics.avgWidth;
}

int32_t BuiltinFont::GetCharHeight(char c) const {
    return metrics.height;
}

Size BuiltinFont::MeasureText(const char* text) const {
    if (!text || text[0] == '\0') {
        return Size(0, metrics.height);
    }
    
    return Size((uint32_t)(strlen(text) * GetCharWidth('A')), (uint32_t)metrics.height);
}

int32_t BuiltinFont::GetTextWidth(const char* text) const {
    if (!text) return 0;
    return (int32_t)(strlen(text) * GetCharWidth('A'));
}

int32_t BuiltinFont::GetTextHeight(const char* text) const {
    return metrics.height;
}

void BuiltinFont::DrawCharacter(GraphicsContext& gc, char c, int32_t x, int32_t y, const Color& color) const {
    // Draw character as a filled rectangle
    gc.FillRect(x, y - metrics.ascent, metrics.avgWidth, metrics.height, color);
}

void BuiltinFont::DrawText(GraphicsContext& gc, const char* text, int32_t x, int32_t y, const Color& color) const {
    if (!text || text[0] == '\0') return;
    
    int32_t currentX = x;
    for (int i = 0; text[i] != '\0'; i++) {
        DrawCharacter(gc, text[i], currentX, y, color);
        currentX += GetCharWidth(text[i]);
    }
}

// -----------------------------------------------------------------------------
// TrueTypeFont implementation (stub)
// -----------------------------------------------------------------------------

TrueTypeFont::TrueTypeFont() : BuiltinFont() {
    name[0] = '\0';
}

TrueTypeFont::TrueTypeFont(const char* name, uint32_t size, FontStyle style) : BuiltinFont(size, style) {
    if (name) {
        strncpy(this->name, name, sizeof(this->name) - 1);
        this->name[sizeof(this->name) - 1] = '\0';
    } else {
        this->name[0] = '\0';
    }
}

} // namespace GUI
} // namespace NebulaOS
