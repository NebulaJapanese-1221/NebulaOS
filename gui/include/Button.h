// NebulaOS GUI - Button Control
// ================================
//
// Button control class for GUI applications

#ifndef NEBULAOS_GUI_BUTTON_H
#define NEBULAOS_GUI_BUTTON_H

#include "../include/Control.h"
#include "../include/GuiTypes.h"
#include "../include/GraphicsContext.h"

namespace NebulaOS {
namespace GUI {

// Button control class
class Button : public Control {
public:
    // Constructor
    Button();
    Button(ControlID id, int32_t x, int32_t y, uint32_t width, uint32_t height);
    Button(ControlID id, const Rectangle& bounds);
    
    virtual ~Button();
    
    // Button state
    bool IsChecked() const { return checked; }
    void SetChecked(bool checked);
    void Check();
    void Uncheck();
    void Toggle();
    
    bool IsToggleButton() const { return toggleButton; }
    void SetToggleButton(bool toggle) { toggleButton = toggle; }
    
    // Button style
    void SetButtonStyle(uint32_t style) { buttonStyle = style; }
    uint32_t GetButtonStyle() const { return buttonStyle; }
    
    // Painting
    virtual void Paint(GraphicsContext& gc) override;
    virtual void PaintBackground(GraphicsContext& gc) override;
    virtual void PaintContent(GraphicsContext& gc) override;
    virtual void PaintBorder(GraphicsContext& gc) override;
    
    // Message handling
    virtual void OnMessage(MessageType msg, uint32_t wParam, uint32_t lParam) override;
    
    virtual void OnMouseDown(int32_t x, int32_t y, MouseButton button) override;
    virtual void OnMouseUp(int32_t x, int32_t y, MouseButton button) override;
    virtual void OnMouseEnter() override;
    virtual void OnMouseLeave() override;
    
    // Click event
    virtual void OnClick();
    
    // Serialization
    virtual void ToString(char* buffer, size_t size) const override;

protected:
    bool checked;
    bool toggleButton;
    uint32_t buttonStyle;
    
    // Helper methods
    void DrawButtonBackground(GraphicsContext& gc);
    void DrawButtonText(GraphicsContext& gc);
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_BUTTON_H
