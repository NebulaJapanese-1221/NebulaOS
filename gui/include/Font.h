// NebulaOS GUI - Font Class
// ===========================
//
// Font representation for text rendering

#ifndef NEBULAOS_GUI_FONT_H
#define NEBULAOS_GUI_FONT_H

#include "../include/GuiTypes.h"
#include "../include/Rectangle.h"
#include "../include/Color.h"

namespace NebulaOS {
namespace GUI {

class GraphicsContext;

class GraphicsContext;

// Font metrics
struct FontMetrics {
    int32_t ascent;
    int32_t descent;
    int32_t height;
    int32_t maxWidth;
    int32_t avgWidth;
};

// Character metrics
struct CharMetrics {
    int32_t width;
    int32_t bearingX;
    int32_t bearingY;
    int32_t advance;
};

// Font class
class Font {
public:
    // Constructor/Destructor
    Font();
    Font(const char* name, uint32_t size, FontStyle style = FontStyle::FONT_NORMAL);
    virtual ~Font();
    
    // Font properties
    const char* GetName() const { return name; }
    void SetName(const char* newName);
    
    uint32_t GetSize() const { return size; }
    void SetSize(uint32_t newSize);
    
    FontStyle GetStyle() const { return style; }
    void SetStyle(FontStyle newStyle);
    
    FontMetrics GetMetrics() const { return metrics; }
    
    // Character metrics
    virtual CharMetrics GetCharMetrics(char c) const;
    virtual int32_t GetCharWidth(char c) const;
    virtual int32_t GetCharHeight(char c) const;
    
    // Text measurement
    virtual Size MeasureText(const char* text) const;
    virtual int32_t GetTextWidth(const char* text) const;
    virtual int32_t GetTextHeight(const char* text) const;
    
    // Text rendering
    virtual void DrawCharacter(GraphicsContext& gc, char c, int32_t x, int32_t y, const Color& color) const;
    virtual void DrawText(GraphicsContext& gc, const char* text, int32_t x, int32_t y, const Color& color) const;
    
    // Serialization
    virtual void ToString(char* buffer, size_t size) const;

protected:
    char name[64];
    uint32_t size;
    FontStyle style;
    FontMetrics metrics;
    
    // For bitmap fonts
    const uint8_t* bitmap;
    uint32_t bitmapWidth;
    uint32_t bitmapHeight;
    uint32_t firstChar;
    uint32_t charCount;
};

// Built-in fonts
class BuiltinFont : public Font {
public:
    BuiltinFont();
    BuiltinFont(uint32_t size, FontStyle style = FontStyle::FONT_NORMAL);
    
    CharMetrics GetCharMetrics(char c) const override;
    int32_t GetCharWidth(char c) const override;
    int32_t GetCharHeight(char c) const override;
    
    Size MeasureText(const char* text) const override;
    int32_t GetTextWidth(const char* text) const override;
    int32_t GetTextHeight(const char* text) const override;
    
    void DrawCharacter(GraphicsContext& gc, char c, int32_t x, int32_t y, const Color& color) const override;
    void DrawText(GraphicsContext& gc, const char* text, int32_t x, int32_t y, const Color& color) const override;
};

// TrueType font (uses builtin bitmap fallback until TrueType parser is implemented)
class TrueTypeFont : public BuiltinFont {
public:
    TrueTypeFont();
    TrueTypeFont(const char* name, uint32_t size, FontStyle style = FontStyle::FONT_NORMAL);
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_FONT_H
