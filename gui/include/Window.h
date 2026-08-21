// NebulaOS GUI - Window Class
// ============================
//
// Window class for GUI applications

#ifndef NEBULAOS_GUI_WINDOW_H
#define NEBULAOS_GUI_WINDOW_H

#include "../include/GuiTypes.h"
#include "../include/Rectangle.h"
#include "../include/Point.h"
#include "../include/Size.h"
#include "../include/Color.h"
#include "../include/GraphicsContext.h"

namespace NebulaOS {
namespace GUI {

class Control;
class WindowManager;

// Window class
class Window {
public:
    // Constructor
    Window();
    Window(const char* title, int32_t x, int32_t y, uint32_t width, uint32_t height);
    Window(const char* title, const Rectangle& bounds);
    
    virtual ~Window();
    
    // Window creation and destruction
    bool Create(const char* title, int32_t x, int32_t y, uint32_t width, uint32_t height,
                WindowStyle style = WindowStyle::WS_BORDER | WindowStyle::WS_CAPTION | WindowStyle::WS_VISIBLE,
                WindowStyleEx exStyle = WindowStyleEx::WS_EX_NONE);
    
    void Destroy();
    bool IsValid() const { return handle != INVALID_WINDOW; }
    
    // Window handle
    WindowHandle GetHandle() const { return handle; }
    void SetHandle(WindowHandle h) { handle = h; }
    
    // Window properties
    const char* GetTitle() const { return title; }
    void SetTitle(const char* newTitle);
    
    Rectangle GetBounds() const { return Rectangle(x, y, width, height); }
    void SetBounds(const Rectangle& bounds);
    void SetBounds(int32_t newX, int32_t newY, uint32_t newWidth, uint32_t newHeight);
    
    void SetPosition(int32_t newX, int32_t newY);
    void SetPosition(const Point& position);
    Point GetPosition() const { return Point(x, y); }
    
    void SetSize(uint32_t newWidth, uint32_t newHeight);
    void SetSize(const Size& size);
    Size GetSize() const { return Size(width, height); }
    
    uint32_t GetWidth() const { return width; }
    uint32_t GetHeight() const { return height; }
    int32_t GetX() const { return x; }
    int32_t GetY() const { return y; }
    
    // Window state
    bool IsVisible() const { return visible; }
    void SetVisible(bool visible);
    void Show();
    void Hide();
    
    bool IsEnabled() const { return enabled; }
    void SetEnabled(bool enabled);
    void Enable();
    void Disable();
    
    bool IsActive() const { return active; }
    void Activate();
    
    bool IsMinimized() const { return minimized; }
    bool IsMaximized() const { return maximized; }
    void Minimize();
    void Maximize();
    void Restore();
    
    // Window style
    WindowStyle GetStyle() const { return style; }
    void SetStyle(WindowStyle newStyle);
    
    WindowStyleEx GetExStyle() const { return exStyle; }
    void SetExStyle(WindowStyleEx newExStyle);
    
    // Client area
    Rectangle GetClientRect() const;
    uint32_t GetClientWidth() const;
    uint32_t GetClientHeight() const;
    
    // Parent/child relationships
    WindowHandle GetParent() const { return parent; }
    void SetParent(WindowHandle newParent);
    
    bool IsChild() const { return (style & WindowStyle::WS_CHILD) != WindowStyle::WS_NONE; }
    bool IsPopup() const { return (style & WindowStyle::WS_POPUP) != WindowStyle::WS_NONE; }
    
    // Window procedure
    void SetWindowProc(WindowProc proc) { windowProc = proc; }
    WindowProc GetWindowProc() const { return windowProc; }
    
    // Message handling
    virtual void OnMessage(MessageType msg, uint32_t wParam, uint32_t lParam);
    
    // Default message handlers (can be overridden)
    virtual void OnPaint();
    virtual void OnMouseMove(int32_t x, int32_t y, uint8_t buttons);
    virtual void OnMouseDown(int32_t x, int32_t y, MouseButton button);
    virtual void OnMouseUp(int32_t x, int32_t y, MouseButton button);
    virtual void OnMouseDoubleClick(int32_t x, int32_t y, MouseButton button);
    virtual void OnMouseEnter();
    virtual void OnMouseLeave();
    virtual void OnKeyDown(KeyCode key, ModifierKey modifiers);
    virtual void OnKeyUp(KeyCode key, ModifierKey modifiers);
    virtual void OnChar(char character, ModifierKey modifiers);
    virtual void OnSize(uint32_t width, uint32_t height);
    virtual void OnMove(int32_t x, int32_t y);
    virtual void OnClose();
    virtual void OnActivate();
    virtual void OnDeactivate();
    
    // Update and repaint
    void Update();
    void Invalidate();
    void Invalidate(const Rectangle& rect);
    
    void Repaint();
    void Repaint(const Rectangle& rect);
    
    // Graphics context
    GraphicsContext* GetGraphicsContext() { return graphicsContext; }
    const GraphicsContext* GetGraphicsContext() const { return graphicsContext; }
    
    void BeginPaint();
    void EndPaint();
    
    // Controls
    Control* GetControl(ControlID id);
    Control* GetControlAt(int32_t x, int32_t y);
    void AddControl(Control* control);
    void RemoveControl(Control* control);
    void RemoveControl(ControlID id);
    
    // Hit testing
    bool HitTest(int32_t x, int32_t y) const;
    bool HitTest(const Point& point) const;
    
    // Window regions
    Rectangle GetNonClientRect() const;
    Rectangle GetCaptionRect() const;
    Rectangle GetBorderRect() const;
    
    // Window menu
    void ShowSystemMenu(int32_t x, int32_t y);
    
    // Focus
    void SetFocus();
    bool HasFocus() const;
    
    // Z-order
    void BringToFront();
    void SendToBack();
    
    // Window class name
    const char* GetClassName() const { return className; }
    void SetClassName(const char* name);
    
    // User data
    void SetUserData(void* data) { userData = data; }
    void* GetUserData() const { return userData; }
    
    // Serialization
    void ToString(char* buffer, size_t size) const;

protected:
    WindowHandle handle;
    char title[256];
    char className[64];
    
    int32_t x;
    int32_t y;
    uint32_t width;
    uint32_t height;
    
    WindowStyle style;
    WindowStyleEx exStyle;
    
    bool visible;
    bool enabled;
    bool active;
    bool minimized;
    bool maximized;
    
    WindowHandle parent;
    
    GraphicsContext* graphicsContext;
    WindowProc windowProc;
    
    void* userData;
    
    // Controls
    Control* controls[64];  // Temporary array for controls
    uint32_t controlCount;
    
    // Window manager reference
    static WindowManager* windowManager;
    
    // Helper methods
    void UpdateClientSize();
    void PaintNonClientArea();
    void PaintClientArea();
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_WINDOW_H
