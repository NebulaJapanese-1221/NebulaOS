// NebulaOS GUI - Panel Implementation
// =====================================
//
// Implementation of the Panel control class

#include "../include/Panel.h"
#include "../include/GraphicsContext.h"
#include "../include/WindowManager.h"
#include "../../lib/include/string.h"

namespace NebulaOS {
namespace GUI {

// -----------------------------------------------------------------------------
// Constructor
// -----------------------------------------------------------------------------

Panel::Panel() {
    id = INVALID_CONTROL;
    x = 0;
    y = 0;
    width = 100;
    height = 100;
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
    borderStyle = PanelBorderStyle::BORDER_SUNKEN;
    borderColor = Color::Gray();
    borderWidth = 1;
    childCount = 0;
    
    for (uint32_t i = 0; i < 64; i++) {
        children[i] = nullptr;
    }
}

Panel::Panel(ControlID id, int32_t x, int32_t y, uint32_t width, uint32_t height) {
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
    borderStyle = PanelBorderStyle::BORDER_SUNKEN;
    borderColor = Color::Gray();
    borderWidth = 1;
    childCount = 0;
    
    for (uint32_t i = 0; i < 64; i++) {
        children[i] = nullptr;
    }
}

Panel::Panel(ControlID id, const Rectangle& bounds) {
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
    borderStyle = PanelBorderStyle::BORDER_SUNKEN;
    borderColor = Color::Gray();
    borderWidth = 1;
    childCount = 0;
    
    for (uint32_t i = 0; i < 64; i++) {
        children[i] = nullptr;
    }
}

// -----------------------------------------------------------------------------
// Destructor
// -----------------------------------------------------------------------------

Panel::~Panel() {
    // Remove all children (but don't delete them)
    for (uint32_t i = 0; i < childCount; i++) {
        if (children[i]) {
            children[i]->SetParent(INVALID_WINDOW);
            children[i] = nullptr;
        }
    }
    childCount = 0;
}

// -----------------------------------------------------------------------------
// Child management
// -----------------------------------------------------------------------------

void Panel::AddChild(Control* control) {
    if (!control || childCount >= 64) return;
    
    // Check if control is already a child
    for (uint32_t i = 0; i < childCount; i++) {
        if (children[i] == control) {
            return;
        }
    }
    
    children[childCount++] = control;
    control->SetParent(parent);  // Set parent to panel's parent (window)
    UpdateLayout();
    Invalidate();
}

void Panel::RemoveChild(Control* control) {
    if (!control) return;
    
    for (uint32_t i = 0; i < childCount; i++) {
        if (children[i] == control) {
            children[i] = nullptr;
            // Shift remaining children
            for (uint32_t j = i; j < childCount - 1; j++) {
                children[j] = children[j + 1];
            }
            childCount--;
            control->SetParent(INVALID_WINDOW);
            UpdateLayout();
            Invalidate();
            return;
        }
    }
}

void Panel::RemoveChild(ControlID id) {
    for (uint32_t i = 0; i < childCount; i++) {
        if (children[i] && children[i]->GetID() == id) {
            RemoveChild(children[i]);
            return;
        }
    }
}

Control* Panel::GetChild(ControlID id) {
    for (uint32_t i = 0; i < childCount; i++) {
        if (children[i] && children[i]->GetID() == id) {
            return children[i];
        }
    }
    return nullptr;
}

Control* Panel::GetChildAt(int32_t x, int32_t y) {
    for (int32_t i = childCount - 1; i >= 0; i--) {
        if (children[i] && children[i]->HitTest(x, y)) {
            return children[i];
        }
    }
    return nullptr;
}

// -----------------------------------------------------------------------------
// Layout
// -----------------------------------------------------------------------------

void Panel::UpdateLayout() {
    // Default: do nothing
    // In a real implementation, this would handle layout management
}

// -----------------------------------------------------------------------------
// Painting
// -----------------------------------------------------------------------------

void Panel::Paint(GraphicsContext& gc) {
    if (!visible) return;
    
    // Save current state
    gc.SaveState();
    
    // Set up clipping
    gc.PushClip(Rectangle(x, y, width, height));
    
    // Paint background
    PaintBackground(gc);
    
    // Paint border
    PaintBorder(gc);
    
    // Paint children
    PaintChildren(gc);
    
    // Restore state
    gc.PopClip();
    gc.RestoreState();
}

void Panel::PaintBackground(GraphicsContext& gc) {
    if (bgColor.GetAlpha() > 0) {
        gc.FillRect(x, y, width, height, bgColor);
    }
}

void Panel::PaintBorder(GraphicsContext& gc) {
    switch (borderStyle) {
        case PanelBorderStyle::BORDER_SINGLE:
            gc.DrawRect(x, y, width, height, borderColor);
            break;
        
        case PanelBorderStyle::BORDER_RAISED:
            // Draw raised border
            gc.DrawLine(x, y, x + width - 1, y, Color::White());
            gc.DrawLine(x, y, x, y + height - 1, Color::White());
            gc.DrawLine(x, y + height - 1, x + width - 1, y + height - 1, Color::DarkGray());
            gc.DrawLine(x + width - 1, y, x + width - 1, y + height - 1, Color::DarkGray());
            break;
        
        case PanelBorderStyle::BORDER_SUNKEN:
            // Draw sunken border
            gc.DrawLine(x, y, x + width - 1, y, Color::DarkGray());
            gc.DrawLine(x, y, x, y + height - 1, Color::DarkGray());
            gc.DrawLine(x, y + height - 1, x + width - 1, y + height - 1, Color::White());
            gc.DrawLine(x + width - 1, y, x + width - 1, y + height - 1, Color::White());
            break;
        
        case PanelBorderStyle::BORDER_ETCHED:
            // Draw etched border (combined raised and sunken)
            gc.DrawLine(x, y, x + width - 1, y, Color::White());
            gc.DrawLine(x, y, x, y + height - 1, Color::White());
            gc.DrawLine(x + 1, y + 1, x + width - 2, y + 1, Color::DarkGray());
            gc.DrawLine(x + 1, y + 1, x + 1, y + height - 2, Color::DarkGray());
            gc.DrawLine(x, y + height - 1, x + width - 1, y + height - 1, Color::DarkGray());
            gc.DrawLine(x + width - 1, y, x + width - 1, y + height - 1, Color::DarkGray());
            gc.DrawLine(x + 1, y + height - 2, x + width - 2, y + height - 2, Color::White());
            gc.DrawLine(x + width - 2, y + 1, x + width - 2, y + height - 2, Color::White());
            break;
        
        default:
            // No border
            break;
    }
}

void Panel::PaintContent(GraphicsContext& gc) {
    PaintChildren(gc);
}

void Panel::PaintChildren(GraphicsContext& gc) {
    for (uint32_t i = 0; i < childCount; i++) {
        if (children[i] && children[i]->IsVisible()) {
            children[i]->Paint(gc);
        }
    }
}

// -----------------------------------------------------------------------------
// Message handling
// -----------------------------------------------------------------------------

void Panel::OnMessage(MessageType msg, uint32_t wParam, uint32_t lParam) {
    // Forward message to children
    for (uint32_t i = 0; i < childCount; i++) {
        if (children[i]) {
            children[i]->OnMessage(msg, wParam, lParam);
        }
    }
    
    Control::OnMessage(msg, wParam, lParam);
}

void Panel::OnMouseMove(int32_t x, int32_t y, uint8_t buttons) {
    // Forward to children
    Control* child = GetChildAt(x, y);
    if (child) {
        child->OnMouseMove(x - child->GetX(), y - child->GetY(), buttons);
    }
    
    Control::OnMouseMove(x, y, buttons);
}

void Panel::OnMouseDown(int32_t x, int32_t y, MouseButton button) {
    // Forward to children
    Control* child = GetChildAt(x, y);
    if (child) {
        child->OnMouseDown(x - child->GetX(), y - child->GetY(), button);
    }
    
    Control::OnMouseDown(x, y, button);
}

void Panel::OnMouseUp(int32_t x, int32_t y, MouseButton button) {
    // Forward to children
    Control* child = GetChildAt(x, y);
    if (child) {
        child->OnMouseUp(x - child->GetX(), y - child->GetY(), button);
    }
    
    Control::OnMouseUp(x, y, button);
}

// -----------------------------------------------------------------------------
// Serialization
// -----------------------------------------------------------------------------

void Panel::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    const char* styleStr = "NONE";
    switch (borderStyle) {
        case PanelBorderStyle::BORDER_SINGLE: styleStr = "SINGLE"; break;
        case PanelBorderStyle::BORDER_RAISED: styleStr = "RAISED"; break;
        case PanelBorderStyle::BORDER_SUNKEN: styleStr = "SUNKEN"; break;
        case PanelBorderStyle::BORDER_ETCHED: styleStr = "ETCHED"; break;
        default: break;
    }
    
    snprintf(buffer, size, "Panel(id=%u, x=%d, y=%d, w=%u, h=%u, border=%s, children=%u)",
             id, x, y, width, height, styleStr, childCount);
}

} // namespace GUI
} // namespace NebulaOS
