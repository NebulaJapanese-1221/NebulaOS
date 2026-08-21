// NebulaOS GUI - Control Class
// ==============================
//
// Base class for all GUI controls

#ifndef NEBULAOS_GUI_CONTROL_H
#define NEBULAOS_GUI_CONTROL_H

#include "../include/GuiTypes.h"
#include "../include/Rectangle.h"
#include "../include/Point.h"
#include "../include/Size.h"
#include "../include/Color.h"
#include "../include/Font.h"

namespace NebulaOS {
namespace GUI {

class GraphicsContext;
class Window;

// Control class - base class for all GUI controls
class Control {
public:
    // Constructor
    Control();
    Control(ControlID id, int32_t x, int32_t y, uint32_t width, uint32_t height);
    Control(ControlID id, const Rectangle& bounds);
    
    virtual ~Control();
    
    // Control ID
    ControlID GetID() const { return id; }
    void SetID(ControlID newId) { id = newId; }
    
    // Bounds
    Rectangle GetBounds() const { return Rectangle(x, y, width, height); }
    void SetBounds(const Rectangle& bounds);
    void SetBounds(int32_t x, int32_t y, uint32_t width, uint32_t height);
    
    // Position
    void SetPosition(int32_t x, int32_t y);
    void SetPosition(const Point& position);
    Point GetPosition() const { return Point(x, y); }
    
    // Size
    void SetSize(uint32_t width, uint32_t height);
    void SetSize(const Size& size);
    Size GetSize() const { return Size(width, height); }
    
    uint32_t GetWidth() const { return width; }
    uint32_t GetHeight() const { return height; }
    int32_t GetX() const { return x; }
    int32_t GetY() const { return y; }
    
    // Parent
    WindowHandle GetParent() const { return parent; }
    void SetParent(WindowHandle newParent) { parent = newParent; }
    
    // Visibility
    bool IsVisible() const { return visible; }
    void SetVisible(bool visible);
    void Show();
    void Hide();
    
    // Enabled state
    bool IsEnabled() const { return enabled; }
    void SetEnabled(bool enabled);
    void Enable();
    void Disable();
    
    // Focus
    bool HasFocus() const { return focused; }
    void SetFocus(bool focused);
    virtual void Focus();
    virtual void Unfocus();
    
    // State
    bool IsPressed() const { return pressed; }
    bool IsHovered() const { return hovered; }
    
    // Z-order
    int32_t GetZOrder() const { return zOrder; }
    void SetZOrder(int32_t order) { zOrder = order; }
    void BringToFront();
    void SendToBack();
    
    // Properties
    const char* GetText() const { return text; }
    void SetText(const char* newText);
    
    Color GetForegroundColor() const { return fgColor; }
    void SetForegroundColor(const Color& color) { fgColor = color; }
    
    Color GetBackgroundColor() const { return bgColor; }
    void SetBackgroundColor(const Color& color) { bgColor = color; }
    
    const Font* GetFont() const { return font; }
    void SetFont(const Font* newFont) { font = newFont; }
    
    // Hit testing
    virtual bool HitTest(int32_t x, int32_t y) const;
    virtual bool HitTest(const Point& point) const;
    
    // Painting
    virtual void Paint(GraphicsContext& gc);
    virtual void PaintBackground(GraphicsContext& gc);
    virtual void PaintBorder(GraphicsContext& gc);
    virtual void PaintContent(GraphicsContext& gc);
    
    // Message handling
    virtual void OnMessage(MessageType msg, uint32_t wParam, uint32_t lParam);
    
    virtual void OnPaint();
    virtual void OnMouseMove(int32_t x, int32_t y, uint8_t buttons);
    virtual void OnMouseDown(int32_t x, int32_t y, MouseButton button);
    virtual void OnMouseUp(int32_t x, int32_t y, MouseButton button);
    virtual void OnMouseEnter();
    virtual void OnMouseLeave();
    virtual void OnKeyDown(KeyCode key, ModifierKey modifiers);
    virtual void OnKeyUp(KeyCode key, ModifierKey modifiers);
    virtual void OnChar(char character, ModifierKey modifiers);
    
    // Update
    virtual void Update();
    
    // Serialization
    virtual void ToString(char* buffer, size_t size) const;

protected:
    ControlID id;
    
    int32_t x;
    int32_t y;
    uint32_t width;
    uint32_t height;
    
    WindowHandle parent;
    
    bool visible;
    bool enabled;
    bool focused;
    bool pressed;
    bool hovered;
    
    int32_t zOrder;
    
    char text[256];
    Color fgColor;
    Color bgColor;
    const Font* font;
    
    // Helper methods
    void Invalidate();
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_CONTROL_H
