// NebulaOS GUI - Control Implementation
// ========================================
//
// Implementation of the Control base class

#include "../include/Control.h"
#include "../include/GraphicsContext.h"
#include "../include/WindowManager.h"
#include "../../lib/include/string.h"

namespace NebulaOS {
namespace GUI {

// -----------------------------------------------------------------------------
// Constructor
// -----------------------------------------------------------------------------

Control::Control() {
    id = INVALID_CONTROL;
    x = 0;
    y = 0;
    width = 0;
    height = 0;
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
}

Control::Control(ControlID id, int32_t x, int32_t y, uint32_t width, uint32_t height) {
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
}

Control::Control(ControlID id, const Rectangle& bounds) {
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
}

// -----------------------------------------------------------------------------
// Destructor
// -----------------------------------------------------------------------------

Control::~Control() {
    // Nothing to do
}

// -----------------------------------------------------------------------------
// SetBounds
// -----------------------------------------------------------------------------

void Control::SetBounds(const Rectangle& bounds) {
    x = bounds.x;
    y = bounds.y;
    width = bounds.width;
    height = bounds.height;
    Invalidate();
}

void Control::SetBounds(int32_t x, int32_t y, uint32_t width, uint32_t height) {
    this->x = x;
    this->y = y;
    this->width = width;
    this->height = height;
    Invalidate();
}

// -----------------------------------------------------------------------------
// Position
// -----------------------------------------------------------------------------

void Control::SetPosition(int32_t x, int32_t y) {
    this->x = x;
    this->y = y;
    Invalidate();
}

void Control::SetPosition(const Point& position) {
    x = position.x;
    y = position.y;
    Invalidate();
}

// -----------------------------------------------------------------------------
// Size
// -----------------------------------------------------------------------------

void Control::SetSize(uint32_t width, uint32_t height) {
    this->width = width;
    this->height = height;
    Invalidate();
}

void Control::SetSize(const Size& size) {
    width = size.width;
    height = size.height;
    Invalidate();
}

// -----------------------------------------------------------------------------
// Visibility
// -----------------------------------------------------------------------------

void Control::SetVisible(bool visible) {
    if (this->visible != visible) {
        this->visible = visible;
        Invalidate();
    }
}

void Control::Show() {
    SetVisible(true);
}

void Control::Hide() {
    SetVisible(false);
}

// -----------------------------------------------------------------------------
// Enabled state
// -----------------------------------------------------------------------------

void Control::SetEnabled(bool enabled) {
    this->enabled = enabled;
    Invalidate();
}

void Control::Enable() {
    SetEnabled(true);
}

void Control::Disable() {
    SetEnabled(false);
}

// -----------------------------------------------------------------------------
// Focus
// -----------------------------------------------------------------------------

void Control::SetFocus(bool focused) {
    this->focused = focused;
    Invalidate();
}

void Control::Focus() {
    SetFocus(true);
}

void Control::Unfocus() {
    SetFocus(false);
}

// -----------------------------------------------------------------------------
// Text
// -----------------------------------------------------------------------------

void Control::SetText(const char* newText) {
    if (newText) {
        strncpy(text, newText, sizeof(text) - 1);
        text[sizeof(text) - 1] = '\0';
    } else {
        text[0] = '\0';
    }
    Invalidate();
}

// -----------------------------------------------------------------------------
// Hit testing
// -----------------------------------------------------------------------------

bool Control::HitTest(int32_t x, int32_t y) const {
    return x >= this->x && x < this->x + (int32_t)width &&
           y >= this->y && y < this->y + (int32_t)height &&
           visible && enabled;
}

bool Control::HitTest(const Point& point) const {
    return HitTest(point.x, point.y);
}

// -----------------------------------------------------------------------------
// Painting
// -----------------------------------------------------------------------------

void Control::Paint(GraphicsContext& gc) {
    if (!visible) return;
    
    // Save current state
    gc.SaveState();
    
    // Set up clipping
    gc.PushClip(Rectangle(x, y, width, height));
    
    // Paint background
    PaintBackground(gc);
    
    // Paint border
    PaintBorder(gc);
    
    // Paint content
    PaintContent(gc);
    
    // Restore state
    gc.PopClip();
    gc.RestoreState();
}

void Control::PaintBackground(GraphicsContext& gc) {
    if (bgColor.GetAlpha() > 0) {
        gc.FillRect(x, y, width, height, bgColor);
    }
}

void Control::PaintBorder(GraphicsContext& gc) {
    // Default: no border
}

void Control::PaintContent(GraphicsContext& gc) {
    // Default: draw text
    if (text[0] != '\0') {
        gc.SetColor(fgColor);
        if (font) {
            gc.SetFont(font);
        }
        gc.DrawText(text, x + 2, y + 2, fgColor);
    }
}

// -----------------------------------------------------------------------------
// Message handling
// -----------------------------------------------------------------------------

void Control::OnMessage(MessageType msg, uint32_t wParam, uint32_t lParam) {
    switch (msg) {
        case MessageType::MSG_PAINT:
            OnPaint();
            break;
        case MessageType::MSG_MOUSE_MOVE:
            OnMouseMove((int32_t)(wParam >> 16), (int32_t)(wParam & 0xFFFF), (uint8_t)lParam);
            break;
        case MessageType::MSG_MOUSE_DOWN:
            OnMouseDown((int32_t)(wParam >> 16), (int32_t)(wParam & 0xFFFF), (MouseButton)lParam);
            break;
        case MessageType::MSG_MOUSE_UP:
            OnMouseUp((int32_t)(wParam >> 16), (int32_t)(wParam & 0xFFFF), (MouseButton)lParam);
            break;
        case MessageType::MSG_KEY_DOWN:
            OnKeyDown((KeyCode)wParam, (ModifierKey)lParam);
            break;
        case MessageType::MSG_KEY_UP:
            OnKeyUp((KeyCode)wParam, (ModifierKey)lParam);
            break;
        case MessageType::MSG_CHAR:
            OnChar((char)wParam, (ModifierKey)lParam);
            break;
        default:
            break;
    }
}

void Control::OnPaint() {
    // Default: do nothing
}

void Control::OnMouseMove(int32_t x, int32_t y, uint8_t buttons) {
    // Update hover state
    bool newHovered = HitTest(x, y);
    if (newHovered != hovered) {
        hovered = newHovered;
        Invalidate();
    }
}

void Control::OnMouseDown(int32_t x, int32_t y, MouseButton button) {
    if (enabled && HitTest(x, y)) {
        pressed = true;
        Invalidate();
    }
}

void Control::OnMouseUp(int32_t x, int32_t y, MouseButton button) {
    pressed = false;
    Invalidate();
}

void Control::OnMouseEnter() {
    hovered = true;
    Invalidate();
}

void Control::OnMouseLeave() {
    hovered = false;
    Invalidate();
}

void Control::OnKeyDown(KeyCode key, ModifierKey modifiers) {
    // Default: do nothing
}

void Control::OnKeyUp(KeyCode key, ModifierKey modifiers) {
    // Default: do nothing
}

void Control::OnChar(char character, ModifierKey modifiers) {
    // Default: do nothing
}

// -----------------------------------------------------------------------------
// Update
// -----------------------------------------------------------------------------

void Control::Update() {
    // Default: do nothing
}

// -----------------------------------------------------------------------------
// Z-order
// -----------------------------------------------------------------------------

void Control::BringToFront() {
    zOrder = 1000;  // High z-order
    Invalidate();
}

void Control::SendToBack() {
    zOrder = 0;  // Low z-order
    Invalidate();
}

// -----------------------------------------------------------------------------
// Helper methods
// -----------------------------------------------------------------------------

void Control::Invalidate() {
    // Notify parent to repaint
    if (parent != INVALID_WINDOW) {
        WindowManager* wm = WindowManager::GetInstance();
        if (wm) {
            wm->InvalidateWindow(parent, Rectangle(x, y, width, height));
        }
    }
}

// -----------------------------------------------------------------------------
// Serialization
// -----------------------------------------------------------------------------

void Control::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    snprintf(buffer, size, "Control(id=%u, x=%d, y=%d, w=%u, h=%u, text='%s')",
             id, x, y, width, height, text);
}

} // namespace GUI
} // namespace NebulaOS
