// NebulaOS GUI - Window Implementation
// ====================================
//
// Implementation of Window class

#include "../include/Window.h"
#include "../include/GUI.h"
#include "../../lib/include/string.h"

namespace NebulaOS {
namespace GUI {

// Static window manager reference
WindowManager* Window::windowManager = nullptr;

// -----------------------------------------------------------------------------
// Constructors
// -----------------------------------------------------------------------------

Window::Window() {
    handle = INVALID_WINDOW;
    title[0] = '\0';
    className[0] = '\0';
    x = 0;
    y = 0;
    width = 0;
    height = 0;
    style = WindowStyle::WS_NONE;
    exStyle = WindowStyleEx::WS_EX_NONE;
    visible = false;
    enabled = true;
    active = false;
    minimized = false;
    maximized = false;
    parent = INVALID_WINDOW;
    graphicsContext = nullptr;
    windowProc = nullptr;
    userData = nullptr;
    controlCount = 0;
    
    for (uint32_t i = 0; i < 64; i++) {
        controls[i] = nullptr;
    }
}

Window::Window(const char* title, int32_t x, int32_t y, uint32_t width, uint32_t height) {
    handle = INVALID_WINDOW;
    if (title) {
        strncpy(this->title, title, sizeof(this->title) - 1);
        this->title[sizeof(this->title) - 1] = '\0';
    } else {
        this->title[0] = '\0';
    }
    className[0] = '\0';
    this->x = x;
    this->y = y;
    this->width = width;
    this->height = height;
    style = WindowStyle::WS_BORDER | WindowStyle::WS_CAPTION | WindowStyle::WS_VISIBLE;
    exStyle = WindowStyleEx::WS_EX_NONE;
    visible = false;
    enabled = true;
    active = false;
    minimized = false;
    maximized = false;
    parent = INVALID_WINDOW;
    graphicsContext = nullptr;
    windowProc = nullptr;
    userData = nullptr;
    controlCount = 0;
    
    for (uint32_t i = 0; i < 64; i++) {
        controls[i] = nullptr;
    }
}

Window::Window(const char* title, const Rectangle& bounds) {
    handle = INVALID_WINDOW;
    if (title) {
        strncpy(this->title, title, sizeof(this->title) - 1);
        this->title[sizeof(this->title) - 1] = '\0';
    } else {
        this->title[0] = '\0';
    }
    className[0] = '\0';
    x = bounds.x;
    y = bounds.y;
    width = bounds.width;
    height = bounds.height;
    style = WindowStyle::WS_BORDER | WindowStyle::WS_CAPTION | WindowStyle::WS_VISIBLE;
    exStyle = WindowStyleEx::WS_EX_NONE;
    visible = false;
    enabled = true;
    active = false;
    minimized = false;
    maximized = false;
    parent = INVALID_WINDOW;
    graphicsContext = nullptr;
    windowProc = nullptr;
    userData = nullptr;
    controlCount = 0;
    
    for (uint32_t i = 0; i < 64; i++) {
        controls[i] = nullptr;
    }
}

// -----------------------------------------------------------------------------
// Destructor
// -----------------------------------------------------------------------------

Window::~Window() {
    Destroy();
}

// -----------------------------------------------------------------------------
// Create window
// -----------------------------------------------------------------------------

bool Window::Create(const char* title, int32_t x, int32_t y, uint32_t width, uint32_t height,
                    WindowStyle style, WindowStyleEx exStyle) {
    if (title) {
        strncpy(this->title, title, sizeof(this->title) - 1);
        this->title[sizeof(this->title) - 1] = '\0';
    }
    this->x = x;
    this->y = y;
    this->width = width;
    this->height = height;
    this->style = style;
    this->exStyle = exStyle;
    
    // Register with window manager
    if (windowManager) {
        handle = windowManager->RegisterWindow(this);
    } else {
        handle = 1; // First window
    }
    
    // Create graphics context
    // In a real implementation, this would allocate a buffer
    // For now, we'll use the screen framebuffer
    if (windowManager) {
        DisplayInfo info = windowManager->GetDisplayInfo();
        graphicsContext = new GraphicsContext(info.frameBuffer, info.width, info.height, info.width * 4);
    }
    
    visible = (style & WindowStyle::WS_VISIBLE) != WindowStyle::WS_NONE;
    
    // Paint the window
    if (visible) {
        PaintNonClientArea();
        PaintClientArea();
    }
    
    return handle != INVALID_WINDOW;
}

// -----------------------------------------------------------------------------
// Destroy window
// -----------------------------------------------------------------------------

void Window::Destroy() {
    if (handle != INVALID_WINDOW && windowManager) {
        windowManager->UnregisterWindow(handle);
        handle = INVALID_WINDOW;
    }
    
    // Delete graphics context
    if (graphicsContext) {
        delete graphicsContext;
        graphicsContext = nullptr;
    }
    
    // Delete all controls
    for (uint32_t i = 0; i < controlCount; i++) {
        if (controls[i]) {
            delete controls[i];
            controls[i] = nullptr;
        }
    }
    controlCount = 0;
}

// -----------------------------------------------------------------------------
// Set title
// -----------------------------------------------------------------------------

void Window::SetTitle(const char* newTitle) {
    if (newTitle) {
        strncpy(title, newTitle, sizeof(title) - 1);
        title[sizeof(title) - 1] = '\0';
    } else {
        title[0] = '\0';
    }
    
    // Redraw caption if visible
    if (visible && (style & WindowStyle::WS_CAPTION) != WindowStyle::WS_NONE) {
        PaintNonClientArea();
    }
}

// -----------------------------------------------------------------------------
// Set bounds
// -----------------------------------------------------------------------------

void Window::SetBounds(const Rectangle& bounds) {
    SetBounds(bounds.x, bounds.y, bounds.width, bounds.height);
}

void Window::SetBounds(int32_t newX, int32_t newY, uint32_t newWidth, uint32_t newHeight) {
    int32_t oldWidth = width;
    int32_t oldHeight = height;
    
    x = newX;
    y = newY;
    width = newWidth;
    height = newHeight;
    
    if (graphicsContext) {
        // In a real implementation, resize the graphics context
    }
    
    // Notify of size change
    if (oldWidth != (int32_t)newWidth || oldHeight != (int32_t)newHeight) {
        OnSize(newWidth, newHeight);
    }
    
    // Notify of move
    if (oldWidth != newX || oldHeight != newY) {
        OnMove(newX, newY);
    }
}

// -----------------------------------------------------------------------------
// Set position
// -----------------------------------------------------------------------------

void Window::SetPosition(int32_t newX, int32_t newY) {
    int32_t oldX = x;
    int32_t oldY = y;
    x = newX;
    y = newY;
    
    if (oldX != newX || oldY != newY) {
        OnMove(newX, newY);
    }
}

void Window::SetPosition(const Point& position) {
    SetPosition(position.x, position.y);
}

// -----------------------------------------------------------------------------
// Set size
// -----------------------------------------------------------------------------

void Window::SetSize(uint32_t newWidth, uint32_t newHeight) {
    uint32_t oldWidth = width;
    uint32_t oldHeight = height;
    width = newWidth;
    height = newHeight;
    
    if (oldWidth != newWidth || oldHeight != newHeight) {
        OnSize(newWidth, newHeight);
    }
}

void Window::SetSize(const Size& size) {
    SetSize(size.width, size.height);
}

// -----------------------------------------------------------------------------
// Visibility
// -----------------------------------------------------------------------------

void Window::SetVisible(bool visible) {
    if (this->visible == visible) return;
    this->visible = visible;
    
    if (visible) {
        Show();
    } else {
        Hide();
    }
}

void Window::Show() {
    if (visible) return;
    visible = true;
    
    // Paint the window
    PaintNonClientArea();
    PaintClientArea();
    
    // Bring to front
    BringToFront();
}

void Window::Hide() {
    if (!visible) return;
    visible = false;
    
    // In a real implementation, this would redraw the area behind the window
}

// -----------------------------------------------------------------------------
// Enabled state
// -----------------------------------------------------------------------------

void Window::SetEnabled(bool enabled) {
    this->enabled = enabled;
}

void Window::Enable() {
    enabled = true;
}

void Window::Disable() {
    enabled = false;
}

// -----------------------------------------------------------------------------
// Activate
// -----------------------------------------------------------------------------

void Window::Activate() {
    if (active) return;
    active = true;
    OnActivate();
}

// -----------------------------------------------------------------------------
// Window state (minimize/maximize)
// -----------------------------------------------------------------------------

void Window::Minimize() {
    if (minimized) return;
    minimized = true;
    // In a real implementation, hide and save state
}

void Window::Maximize() {
    if (maximized) return;
    maximized = true;
    // In a real implementation, resize to full screen
}

void Window::Restore() {
    minimized = false;
    maximized = false;
    // In a real implementation, restore to previous size
}

// -----------------------------------------------------------------------------
// Style
// -----------------------------------------------------------------------------

void Window::SetStyle(WindowStyle newStyle) {
    style = newStyle;
    // In a real implementation, this would update the window appearance
}

void Window::SetExStyle(WindowStyleEx newExStyle) {
    exStyle = newExStyle;
}

// -----------------------------------------------------------------------------
// Client area
// -----------------------------------------------------------------------------

Rectangle Window::GetClientRect() const {
    // Calculate client area (inside borders and caption)
    int32_t clientX = x;
    int32_t clientY = y;
    uint32_t clientWidth = width;
    uint32_t clientHeight = height;
    
    if ((style & WindowStyle::WS_BORDER) != WindowStyle::WS_NONE) {
        clientX += 1;
        clientY += 1;
        clientWidth -= 2;
        clientHeight -= 2;
    }
    
    if ((style & WindowStyle::WS_CAPTION) != WindowStyle::WS_NONE) {
        clientY += 20;  // Caption height
        clientHeight -= 20;
    }
    
    return Rectangle(clientX, clientY, clientWidth, clientHeight);
}

uint32_t Window::GetClientWidth() const {
    return GetClientRect().width;
}

uint32_t Window::GetClientHeight() const {
    return GetClientRect().height;
}

// -----------------------------------------------------------------------------
// Parent/child
// -----------------------------------------------------------------------------

void Window::SetParent(WindowHandle newParent) {
    parent = newParent;
}

// -----------------------------------------------------------------------------
// Message handling
// -----------------------------------------------------------------------------

void Window::OnMessage(MessageType msg, uint32_t wParam, uint32_t lParam) {
    switch (msg) {
        case MessageType::MSG_PAINT:
            OnPaint();
            break;
        case MessageType::MSG_MOUSE_MOVE:
            OnMouseMove((int16_t)(wParam & 0xFFFF), (int16_t)(wParam >> 16), (uint8_t)(lParam & 0xFF));
            break;
        case MessageType::MSG_MOUSE_DOWN:
            OnMouseDown((int16_t)(wParam & 0xFFFF), (int16_t)(wParam >> 16), (MouseButton)(lParam & 0xFF));
            break;
        case MessageType::MSG_MOUSE_UP:
            OnMouseUp((int16_t)(wParam & 0xFFFF), (int16_t)(wParam >> 16), (MouseButton)(lParam & 0xFF));
            break;
        case MessageType::MSG_MOUSE_DOUBLE_CLICK:
            OnMouseDoubleClick((int16_t)(wParam & 0xFFFF), (int16_t)(wParam >> 16), (MouseButton)(lParam & 0xFF));
            break;
        case MessageType::MSG_KEY_DOWN:
            OnKeyDown((KeyCode)(wParam & 0xFF), (ModifierKey)(lParam & 0xFF));
            break;
        case MessageType::MSG_KEY_UP:
            OnKeyUp((KeyCode)(wParam & 0xFF), (ModifierKey)(lParam & 0xFF));
            break;
        case MessageType::MSG_CHAR:
            OnChar((char)(wParam & 0xFF), (ModifierKey)(lParam & 0xFF));
            break;
        case MessageType::MSG_SIZE:
            OnSize(wParam & 0xFFFF, wParam >> 16);
            break;
        case MessageType::MSG_MOVE:
            OnMove((int16_t)(wParam & 0xFFFF), (int16_t)(wParam >> 16));
            break;
        case MessageType::MSG_CLOSE:
            OnClose();
            break;
        case MessageType::MSG_ACTIVATE:
            OnActivate();
            break;
        case MessageType::MSG_DEACTIVATE:
            OnDeactivate();
            break;
        default:
            if (windowProc) {
                windowProc(handle, msg, wParam, lParam);
            }
            break;
    }
    
    if (windowProc) {
        windowProc(handle, msg, wParam, lParam);
    }
}

// -----------------------------------------------------------------------------
// Default message handlers
// -----------------------------------------------------------------------------

void Window::OnPaint() {
    PaintNonClientArea();
    PaintClientArea();
}

void Window::OnMouseMove(int32_t x, int32_t y, uint8_t buttons) {
    // Convert to client coordinates
    Rectangle client = GetClientRect();
    int32_t clientX = x - client.x;
    int32_t clientY = y - client.y;
    
    // Forward to controls
    for (uint32_t i = 0; i < controlCount; i++) {
        if (controls[i] && controls[i]->HitTest(clientX, clientY)) {
            controls[i]->OnMouseMove(clientX, clientY, buttons);
        }
    }
}

void Window::OnMouseDown(int32_t x, int32_t y, MouseButton button) {
    // Check if in non-client area
    Rectangle nonClient = GetNonClientRect();
    if (x >= nonClient.x && x < nonClient.x + (int32_t)nonClient.width &&
        y >= nonClient.y && y < nonClient.y + (int32_t)nonClient.height) {
        // Non-client area click
        if ((style & WindowStyle::WS_CAPTION) != WindowStyle::WS_NONE) {
            // Start window drag
        }
        return;
    }
    
    // Convert to client coordinates
    Rectangle client = GetClientRect();
    int32_t clientX = x - client.x;
    int32_t clientY = y - client.y;
    
    // Forward to controls
    for (uint32_t i = 0; i < controlCount; i++) {
        if (controls[i] && controls[i]->HitTest(clientX, clientY)) {
            controls[i]->OnMouseDown(clientX, clientY, button);
        }
    }
}

void Window::OnMouseUp(int32_t x, int32_t y, MouseButton button) {
    Rectangle client = GetClientRect();
    int32_t clientX = x - client.x;
    int32_t clientY = y - client.y;
    
    for (uint32_t i = 0; i < controlCount; i++) {
        if (controls[i] && controls[i]->HitTest(clientX, clientY)) {
            controls[i]->OnMouseUp(clientX, clientY, button);
        }
    }
}

void Window::OnMouseDoubleClick(int32_t x, int32_t y, MouseButton button) {
    // Default: maximize on caption double-click
    if ((style & WindowStyle::WS_CAPTION) != WindowStyle::WS_NONE) {
        if (maximized) {
            Restore();
        } else {
            Maximize();
        }
    }
}

void Window::OnKeyDown(KeyCode key, ModifierKey modifiers) {
    // Forward to focused control
}

void Window::OnKeyUp(KeyCode key, ModifierKey modifiers) {
}

void Window::OnChar(char character, ModifierKey modifiers) {
}

void Window::OnSize(uint32_t width, uint32_t height) {
}

void Window::OnMove(int32_t x, int32_t y) {
}

void Window::OnClose() {
    Destroy();
}

void Window::OnActivate() {
    active = true;
}

void Window::OnDeactivate() {
    active = false;
}

void Window::OnMouseEnter() {
}

void Window::OnMouseLeave() {
}

// -----------------------------------------------------------------------------
// Update and repaint
// -----------------------------------------------------------------------------

void Window::Update() {
    if (windowManager) {
        windowManager->UpdateWindow(handle);
    }
}

void Window::Invalidate() {
    Invalidate(Rectangle(0, 0, width, height));
}

void Window::Invalidate(const Rectangle& rect) {
    if (windowManager) {
        windowManager->InvalidateWindow(handle, rect);
    }
}

void Window::Repaint() {
    if (windowManager) {
        windowManager->RepaintWindow(handle);
    }
}

void Window::Repaint(const Rectangle& rect) {
    if (windowManager) {
        windowManager->RepaintWindow(handle, rect);
    }
}

// -----------------------------------------------------------------------------
// Graphics context
// -----------------------------------------------------------------------------

void Window::BeginPaint() {
    if (graphicsContext) {
        graphicsContext->SaveState();
        graphicsContext->SetClip(GetClientRect());
    }
}

void Window::EndPaint() {
    if (graphicsContext) {
        graphicsContext->RestoreState();
    }
}

// -----------------------------------------------------------------------------
// Controls
// -----------------------------------------------------------------------------

Control* Window::GetControl(ControlID id) {
    for (uint32_t i = 0; i < controlCount; i++) {
        if (controls[i] && controls[i]->GetID() == id) {
            return controls[i];
        }
    }
    return nullptr;
}

Control* Window::GetControlAt(int32_t x, int32_t y) {
    for (uint32_t i = controlCount; i > 0; i--) {
        if (controls[i - 1] && controls[i - 1]->HitTest(x, y)) {
            return controls[i - 1];
        }
    }
    return nullptr;
}

void Window::AddControl(Control* control) {
    if (!control || controlCount >= 64) return;
    
    controls[controlCount++] = control;
    control->SetParent(handle);
    
    // In a real implementation, this would redraw the control
}

void Window::RemoveControl(Control* control) {
    if (!control) return;
    
    for (uint32_t i = 0; i < controlCount; i++) {
        if (controls[i] == control) {
            delete controls[i];
            for (uint32_t j = i; j < controlCount - 1; j++) {
                controls[j] = controls[j + 1];
            }
            controls[controlCount - 1] = nullptr;
            controlCount--;
            break;
        }
    }
}

void Window::RemoveControl(ControlID id) {
    Control* control = GetControl(id);
    if (control) {
        RemoveControl(control);
    }
}

// -----------------------------------------------------------------------------
// Hit testing
// -----------------------------------------------------------------------------

bool Window::HitTest(int32_t x, int32_t y) const {
    return x >= this->x && x < this->x + (int32_t)width &&
           y >= this->y && y < this->y + (int32_t)height;
}

bool Window::HitTest(const Point& point) const {
    return HitTest(point.x, point.y);
}

// -----------------------------------------------------------------------------
// Window regions
// -----------------------------------------------------------------------------

Rectangle Window::GetNonClientRect() const {
    return Rectangle(x, y, width, height);
}

Rectangle Window::GetCaptionRect() const {
    if ((style & WindowStyle::WS_CAPTION) != WindowStyle::WS_NONE) {
        return Rectangle(x, y, width, 20);
    }
    return Rectangle();
}

Rectangle Window::GetBorderRect() const {
    if ((style & WindowStyle::WS_BORDER) != WindowStyle::WS_NONE) {
        return Rectangle(x, y, width, height);
    }
    return Rectangle();
}

// -----------------------------------------------------------------------------
// Focus
// -----------------------------------------------------------------------------

void Window::SetFocus() {
    if (windowManager) {
        windowManager->SetFocusWindow(handle);
    }
}

bool Window::HasFocus() const {
    return active;
}

// -----------------------------------------------------------------------------
// Z-order
// -----------------------------------------------------------------------------

void Window::BringToFront() {
    if (windowManager) {
        windowManager->BringToFront(handle);
    }
}

void Window::SendToBack() {
    if (windowManager) {
        windowManager->SendToBack(handle);
    }
}

// -----------------------------------------------------------------------------
// Class name
// -----------------------------------------------------------------------------

void Window::SetClassName(const char* name) {
    if (name) {
        strncpy(className, name, sizeof(className) - 1);
        className[sizeof(className) - 1] = '\0';
    } else {
        className[0] = '\0';
    }
}

// -----------------------------------------------------------------------------
// Serialization
// -----------------------------------------------------------------------------

void Window::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    snprintf(buffer, size, "Window(handle=%u, title='%s', %dx%d)", 
             handle, title, width, height);
}

// -----------------------------------------------------------------------------
// Helper methods
// -----------------------------------------------------------------------------

void Window::UpdateClientSize() {
    // In a real implementation, this would update client size
}

void Window::PaintNonClientArea() {
    if (!graphicsContext) return;
    
    // Draw border
    if ((style & WindowStyle::WS_BORDER) != WindowStyle::WS_NONE) {
        graphicsContext->DrawRect(0, 0, width, height, Color::DarkGray());
    }
    
    // Draw caption
    if ((style & WindowStyle::WS_CAPTION) != WindowStyle::WS_NONE) {
        graphicsContext->FillRect(1, 1, width - 2, 18, Color::LightGray());
        graphicsContext->DrawText(title, 10, 5, Color::Black());
    }
}

void Window::PaintClientArea() {
    if (!graphicsContext) return;
    
    Rectangle client = GetClientRect();
    graphicsContext->FillRect(client, Color::White());
    
    // Paint all controls
    for (uint32_t i = 0; i < controlCount; i++) {
        if (controls[i]) {
            controls[i]->Paint(*graphicsContext);
        }
    }
}

} // namespace GUI
} // namespace NebulaOS
