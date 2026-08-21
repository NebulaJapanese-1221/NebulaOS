// NebulaOS GUI - Label Implementation
// ======================================
//
// Implementation of the Label control class

#include "../include/Label.h"
#include "../include/GraphicsContext.h"
#include "../include/Theme.h"
#include "../../lib/include/string.h"

namespace NebulaOS {
namespace GUI {

// -----------------------------------------------------------------------------
// Constructor
// -----------------------------------------------------------------------------

Label::Label() {
    id = INVALID_CONTROL;
    x = 0;
    y = 0;
    width = 100;
    height = 20;
    parent = INVALID_WINDOW;
    visible = true;
    enabled = true;
    focused = false;
    pressed = false;
    hovered = false;
    zOrder = 0;
    text[0] = '\0';
    fgColor = Color::Black();
    bgColor = Color::Transparent();
    font = nullptr;
    textAlign = TextAlignment::ALIGN_LEFT;
    wordWrap = false;
    multiLine = false;
}

Label::Label(ControlID id, int32_t x, int32_t y, uint32_t width, uint32_t height) {
    this->id = id;
    this->x = x;
    this->y = y;
    this->width = width;
    this->height = height;
    parent = INVALID_WINDOW;
    visible = true;
    enabled = true;
    focused = false;
    pressed = false;
    hovered = false;
    zOrder = 0;
    text[0] = '\0';
    fgColor = Color::Black();
    bgColor = Color::Transparent();
    font = nullptr;
    textAlign = TextAlignment::ALIGN_LEFT;
    wordWrap = false;
    multiLine = false;
}

Label::Label(ControlID id, const Rectangle& bounds) {
    this->id = id;
    x = bounds.x;
    y = bounds.y;
    width = bounds.width;
    height = bounds.height;
    parent = INVALID_WINDOW;
    visible = true;
    enabled = true;
    focused = false;
    pressed = false;
    hovered = false;
    zOrder = 0;
    text[0] = '\0';
    fgColor = Color::Black();
    bgColor = Color::Transparent();
    font = nullptr;
    textAlign = TextAlignment::ALIGN_LEFT;
    wordWrap = false;
    multiLine = false;
}

// -----------------------------------------------------------------------------
// Destructor
// -----------------------------------------------------------------------------

Label::~Label() {
    // Nothing to do
}

// -----------------------------------------------------------------------------
// Painting
// -----------------------------------------------------------------------------

void Label::Paint(GraphicsContext& gc) {
    if (!visible) return;
    
    // Save current state
    gc.SaveState();
    
    // Set up clipping
    gc.PushClip(Rectangle(x, y, width, height));
    
    // Paint background
    PaintBackground(gc);
    
    // Paint content
    DrawLabelText(gc);
    
    // Restore state
    gc.PopClip();
    gc.RestoreState();
}

void Label::PaintContent(GraphicsContext& gc) {
    DrawLabelText(gc);
}

void Label::DrawLabelText(GraphicsContext& gc) {
    if (text[0] == '\0') return;
    
    Color textColor = enabled ? fgColor : Color::DarkGray();
    gc.SetColor(textColor);
    
    if (font) {
        gc.SetFont(font);
    }
    
    int32_t textWidth = gc.GetTextWidth(text);
    int32_t textHeight = gc.GetTextHeight(text);
    int32_t textX = x;
    int32_t textY = y;
    
    // Calculate text position based on alignment
    switch (textAlign) {
        case TextAlignment::ALIGN_CENTER:
            textX = x + (width - textWidth) / 2;
            break;
        case TextAlignment::ALIGN_RIGHT:
            textX = x + width - textWidth;
            break;
        default: // ALIGN_LEFT
            textX = x;
            break;
    }
    
    // Vertical alignment (simplified - always top for now)
    textY = y + (height - textHeight) / 2;
    
    gc.DrawText(text, textX, textY, textColor);
}

// -----------------------------------------------------------------------------
// Serialization
// -----------------------------------------------------------------------------

void Label::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    const char* alignStr = "LEFT";
    switch (textAlign) {
        case TextAlignment::ALIGN_CENTER: alignStr = "CENTER"; break;
        case TextAlignment::ALIGN_RIGHT: alignStr = "RIGHT"; break;
        default: break;
    }
    
    snprintf(buffer, size, "Label(id=%u, x=%d, y=%d, w=%u, h=%u, text='%s', align=%s)",
             id, x, y, width, height, text, alignStr);
}

} // namespace GUI
} // namespace NebulaOS
