// NebulaOS GUI - Panel Control
// ==============================
//
// Panel control - container for other controls

#ifndef NEBULAOS_GUI_PANEL_H
#define NEBULAOS_GUI_PANEL_H

#include "../include/Control.h"
#include "../include/GuiTypes.h"
#include "../include/Color.h"

namespace NebulaOS {
namespace GUI {

class GraphicsContext;

// Panel border style
enum class PanelBorderStyle {
    BORDER_NONE = 0,
    BORDER_SINGLE = 1,
    BORDER_RAISED = 2,
    BORDER_SUNKEN = 3,
    BORDER_ETCHED = 4
};

// Panel control class
class Panel : public Control {
public:
    // Constructor
    Panel();
    Panel(ControlID id, int32_t x, int32_t y, uint32_t width, uint32_t height);
    Panel(ControlID id, const Rectangle& bounds);
    
    virtual ~Panel();
    
    // Border style
    PanelBorderStyle GetBorderStyle() const { return borderStyle; }
    void SetBorderStyle(PanelBorderStyle style) { borderStyle = style; }
    
    // Border color
    Color GetBorderColor() const { return borderColor; }
    void SetBorderColor(const Color& color) { borderColor = color; }
    
    // Border width
    uint32_t GetBorderWidth() const { return borderWidth; }
    void SetBorderWidth(uint32_t width) { borderWidth = width; }
    
    // Child controls management
    void AddChild(Control* control);
    void RemoveChild(Control* control);
    void RemoveChild(ControlID id);
    Control* GetChild(ControlID id);
    Control* GetChildAt(int32_t x, int32_t y);
    uint32_t GetChildCount() const { return childCount; }
    
    // Layout
    void UpdateLayout();
    
    // Painting
    virtual void Paint(GraphicsContext& gc) override;
    virtual void PaintBackground(GraphicsContext& gc) override;
    virtual void PaintContent(GraphicsContext& gc) override;
    virtual void PaintBorder(GraphicsContext& gc) override;
    
    // Message handling
    virtual void OnMessage(MessageType msg, uint32_t wParam, uint32_t lParam) override;
    
    virtual void OnMouseMove(int32_t x, int32_t y, uint8_t buttons) override;
    virtual void OnMouseDown(int32_t x, int32_t y, MouseButton button) override;
    virtual void OnMouseUp(int32_t x, int32_t y, MouseButton button) override;
    
    // Serialization
    virtual void ToString(char* buffer, size_t size) const override;

protected:
    PanelBorderStyle borderStyle;
    Color borderColor;
    uint32_t borderWidth;
    
    // Child controls
    Control* children[64];
    uint32_t childCount;
    
    // Helper methods
    void PaintChildren(GraphicsContext& gc);
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_PANEL_H
