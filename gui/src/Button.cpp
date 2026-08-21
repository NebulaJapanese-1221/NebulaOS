// NebulaOS GUI - Button Implementation
// ======================================
//
// Implementation of the Button control class

#include "../include/Button.h"
#include "../include/GraphicsContext.h"
#include "../include/WindowManager.h"
#include "../include/Theme.h"
#include "../../lib/include/string.h"

namespace NebulaOS {
namespace GUI {

// -----------------------------------------------------------------------------
// Constructor
// -----------------------------------------------------------------------------

Button::Button() {
    id = INVALID_CONTROL;
    x = 0;
    y = 0;
    width = 80;
    height = 25;
    parent = INVALID_WINDOW;
    visible = true;
    enabled = true;
    focused = false;
    pressed = false;
    hovered = false;
    zOrder = 0;
    text[0] = '\0';
    fgColor = Color::Black();
    bgColor = Color::LightGray();
    font = nullptr;
    checked = false;
    toggleButton = false;
    buttonStyle = 0;
}

Button::Button(ControlID id, int32_t x, int32_t y, uint32_t width, uint32_t height) {
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
    bgColor = Color::LightGray();
    font = nullptr;
    checked = false;
    toggleButton = false;
    buttonStyle = 0;
}

Button::Button(ControlID id, const Rectangle& bounds) {
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
    bgColor = Color::LightGray();
    font = nullptr;
    checked = false;
    toggleButton = false;
    buttonStyle = 0;
}

// -----------------------------------------------------------------------------
// Destructor
// -----------------------------------------------------------------------------

Button::~Button() {
    // Nothing to do
}

// -----------------------------------------------------------------------------
// Checked state
// -----------------------------------------------------------------------------

void Button::SetChecked(bool checked) {
    this->checked = checked;
    Invalidate();
}

void Button::Check() {
    SetChecked(true);
}

void Button::Uncheck() {
    SetChecked(false);
}

void Button::Toggle() {
    SetChecked(!checked);
}

// -----------------------------------------------------------------------------
// Painting
// -----------------------------------------------------------------------------

void Button::Paint(GraphicsContext& gc) {
    if (!visible) return;
    
    // Save current state
    gc.SaveState();
    
    // Set up clipping
    gc.PushClip(Rectangle(x, y, width, height));
    
    // Paint background
    DrawButtonBackground(gc);
    
    // Paint border
    PaintBorder(gc);
    
    // Paint content (text)
    DrawButtonText(gc);
    
    // Restore state
    gc.PopClip();
    gc.RestoreState();
}

void Button::PaintBackground(GraphicsContext& gc) {
    Color bg = bgColor;
    
    if (!enabled) {
        // Disabled state - use gray
        bg = Color::LightGray();
    } else if (pressed) {
        // Pressed state - use dark color
        bg = Color::DarkGray();
    } else if (hovered) {
        // Hover state - use light color
        bg = Color::White();
    } else if (checked && toggleButton) {
        // Checked state for toggle button
        bg = Color::DarkGray();
    }
    
    gc.FillRect(x, y, width, height, bg);
}

void Button::PaintBorder(GraphicsContext& gc) {
    if (!enabled) {
        gc.DrawRect(x, y, width, height, Color::Gray());
    } else {
        Color borderColor = hovered ? Color::Blue() : Color::Black();
        gc.DrawRect(x, y, width, height, borderColor);
    }
}

void Button::PaintContent(GraphicsContext& gc) {
    DrawButtonText(gc);
}

void Button::DrawButtonBackground(GraphicsContext& gc) {
    // Use the PaintBackground method
    PaintBackground(gc);
}

void Button::DrawButtonText(GraphicsContext& gc) {
    if (text[0] == '\0') return;
    
    Color textColor = enabled ? fgColor : Color::DarkGray();
    
    // Calculate text position (centered)
    int32_t textWidth = gc.GetTextWidth(text);
    int32_t textHeight = gc.GetTextHeight(text);
    int32_t textX = x + (width - textWidth) / 2;
    int32_t textY = y + (height - textHeight) / 2;
    
    gc.SetColor(textColor);
    gc.DrawText(text, textX, textY, textColor);
}

// -----------------------------------------------------------------------------
// Message handling
// -----------------------------------------------------------------------------

void Button::OnMessage(MessageType msg, uint32_t wParam, uint32_t lParam) {
    Control::OnMessage(msg, wParam, lParam);
}

void Button::OnMouseDown(int32_t x, int32_t y, MouseButton button) {
    if (!enabled || !visible) return;
    
    if (HitTest(x, y)) {
        pressed = true;
        Invalidate();
    }
}

void Button::OnMouseUp(int32_t x, int32_t y, MouseButton button) {
    if (!pressed) return;
    
    pressed = false;
    Invalidate();
    
    // Check if mouse is still over the button
    if (HitTest(x, y)) {
        OnClick();
    }
}

void Button::OnMouseEnter() {
    if (!enabled) return;
    
    hovered = true;
    Invalidate();
}

void Button::OnMouseLeave() {
    hovered = false;
    pressed = false;
    Invalidate();
}

// -----------------------------------------------------------------------------
// Click event
// -----------------------------------------------------------------------------

void Button::OnClick() {
    if (toggleButton) {
        Toggle();
    }
    
    // Post command message to parent
    if (parent != INVALID_WINDOW) {
        WindowManager* wm = WindowManager::GetInstance();
        if (wm) {
            wm->PostMessage(parent, MessageType::MSG_COMMAND, id, 0);
        }
    }
}

// -----------------------------------------------------------------------------
// Serialization
// -----------------------------------------------------------------------------

void Button::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    snprintf(buffer, size, "Button(id=%u, x=%d, y=%d, w=%u, h=%u, text='%s', checked=%d)",
             id, x, y, width, height, text, checked);
}

} // namespace GUI
} // namespace NebulaOS
