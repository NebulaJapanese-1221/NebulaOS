// NebulaOS GUI - WindowManager Implementation
// ============================================
//
// Implementation of the WindowManager class

#include "../include/WindowManager.h"
#include "../include/Window.h"
#include "../include/GraphicsContext.h"
#include "../../lib/include/string.h"

namespace NebulaOS {
namespace GUI {

// Static instance
WindowManager* WindowManager::instance = nullptr;

// -----------------------------------------------------------------------------
// Singleton access
// -----------------------------------------------------------------------------

WindowManager* WindowManager::GetInstance() {
    return instance;
}

void WindowManager::Initialize(DisplayInfo info) {
    if (!instance) {
        instance = new WindowManager();
        instance->Initialize(info.width, info.height, info.bitsPerPixel, info.frameBuffer);
    }
}

void WindowManager::Shutdown() {
    if (instance) {
        delete instance;
        instance = nullptr;
    }
}

// -----------------------------------------------------------------------------
// Constructor/Destructor
// -----------------------------------------------------------------------------

WindowManager::WindowManager() {
    displayInfo.width = 0;
    displayInfo.height = 0;
    displayInfo.bitsPerPixel = 0;
    displayInfo.refreshRate = 0;
    displayInfo.frameBuffer = nullptr;
    
    windowCount = 0;
    for (uint32_t i = 0; i < GUI_MAX_WINDOWS; i++) {
        windows[i] = nullptr;
    }
    
    activeWindow = INVALID_WINDOW;
    focusWindow = INVALID_WINDOW;
    captureWindow = INVALID_WINDOW;
    desktopColor = Color::Black();
    
    mouseX = 0;
    mouseY = 0;
    mouseButtons = 0;
    mouseOverWindow = INVALID_WINDOW;
    
    cursorVisible = true;
    cursorX = 0;
    cursorY = 0;
    
    timerCount = 0;
    nextTimerId = 1;
    for (uint32_t i = 0; i < GUI_MAX_TIMERS; i++) {
        timers[i].active = false;
    }
}

WindowManager::~WindowManager() {
    Shutdown();
}

// -----------------------------------------------------------------------------
// Initialize
// -----------------------------------------------------------------------------

bool WindowManager::Initialize(uint32_t width, uint32_t height, uint32_t bpp, void* frameBuffer) {
    displayInfo.width = width;
    displayInfo.height = height;
    displayInfo.bitsPerPixel = bpp;
    displayInfo.refreshRate = 60;
    displayInfo.frameBuffer = frameBuffer;
    
    return true;
}

// -----------------------------------------------------------------------------
// Window management
// -----------------------------------------------------------------------------

WindowHandle WindowManager::RegisterWindow(Window* window) {
    if (!window || windowCount >= GUI_MAX_WINDOWS) {
        return INVALID_WINDOW;
    }
    
    // Find a free slot
    for (uint32_t i = 0; i < GUI_MAX_WINDOWS; i++) {
        if (windows[i] == nullptr) {
            windows[i] = window;
            window->SetHandle(i);
            windowCount++;
            
            // Set as active window if none is active
            if (activeWindow == INVALID_WINDOW) {
                activeWindow = i;
            }
            
            return i;
        }
    }
    
    return INVALID_WINDOW;
}

void WindowManager::UnregisterWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || windows[handle] == nullptr) {
        return;
    }
    
    Window* window = windows[handle];
    
    // Remove from window list
    windows[handle] = nullptr;
    windowCount--;
    
    // Remove from active/focus/capture if needed
    if (activeWindow == handle) {
        activeWindow = INVALID_WINDOW;
    }
    if (focusWindow == handle) {
        focusWindow = INVALID_WINDOW;
    }
    if (captureWindow == handle) {
        captureWindow = INVALID_WINDOW;
    }
    if (mouseOverWindow == handle) {
        mouseOverWindow = INVALID_WINDOW;
    }
    
    // Delete the window
    delete window;
}

Window* WindowManager::GetWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS) {
        return nullptr;
    }
    return windows[handle];
}

Window* WindowManager::GetWindowAt(int32_t x, int32_t y) {
    // Check windows in reverse order (top-most first)
    for (int32_t i = GUI_MAX_WINDOWS - 1; i >= 0; i--) {
        if (windows[i] && windows[i]->IsVisible() && windows[i]->HitTest(x, y)) {
            return windows[i];
        }
    }
    return nullptr;
}

void WindowManager::SetActiveWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle]) {
        return;
    }
    
    if (activeWindow != handle) {
        // Deactivate old window
        if (activeWindow != INVALID_WINDOW && windows[activeWindow]) {
            windows[activeWindow]->OnDeactivate();
        }
        
        activeWindow = handle;
        
        // Activate new window
        if (windows[activeWindow]) {
            windows[activeWindow]->Activate();
        }
    }
}

void WindowManager::SetFocusWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle]) {
        return;
    }
    
    focusWindow = handle;
}

void WindowManager::SetCaptureWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS && handle != INVALID_WINDOW) {
        return;
    }
    
    captureWindow = handle;
}

// -----------------------------------------------------------------------------
// Window operations
// -----------------------------------------------------------------------------

void WindowManager::BringToFront(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle]) {
        return;
    }
    
    // Move window to the end of the list (top-most)
    Window* window = windows[handle];
    
    for (uint32_t i = handle; i < windowCount - 1; i++) {
        windows[i] = windows[i + 1];
        if (windows[i]) {
            windows[i]->SetHandle(i);
        }
    }
    
    windows[windowCount - 1] = window;
    window->SetHandle(windowCount - 1);
    activeWindow = windowCount - 1;
    
    InvalidateWindow(handle, Rectangle(0, 0, displayInfo.width, displayInfo.height));
}

void WindowManager::SendToBack(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle]) {
        return;
    }
    
    // Move window to the front of the list (bottom-most)
    Window* window = windows[handle];
    
    for (int32_t i = handle; i > 0; i--) {
        windows[i] = windows[i - 1];
        if (windows[i]) {
            windows[i]->SetHandle(i);
        }
    }
    
    windows[0] = window;
    window->SetHandle(0);
    
    InvalidateWindow(handle, Rectangle(0, 0, displayInfo.width, displayInfo.height));
}

void WindowManager::ShowWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle]) {
        return;
    }
    
    windows[handle]->SetVisible(true);
    InvalidateWindow(handle);
}

void WindowManager::HideWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle]) {
        return;
    }
    
    windows[handle]->SetVisible(false);
    InvalidateWindow(handle);
}

void WindowManager::ActivateWindow(WindowHandle handle) {
    SetActiveWindow(handle);
}

void WindowManager::DeactivateWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle]) {
        return;
    }
    
    windows[handle]->OnDeactivate();
}

// -----------------------------------------------------------------------------
// Update and repaint
// -----------------------------------------------------------------------------

void WindowManager::UpdateWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle]) {
        return;
    }
    
    windows[handle]->Update();
}

void WindowManager::InvalidateWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle]) {
        return;
    }
    
    InvalidateWindow(handle, Rectangle(0, 0, windows[handle]->GetWidth(), windows[handle]->GetHeight()));
}

void WindowManager::InvalidateWindow(WindowHandle handle, const Rectangle& rect) {
    // In a real implementation, this would schedule a repaint
    // For now, we just mark the window as needing repaint
    if (handle < GUI_MAX_WINDOWS && windows[handle]) {
        windows[handle]->Invalidate(rect);
    }
}

void WindowManager::RepaintWindow(WindowHandle handle) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle] || !windows[handle]->IsVisible()) {
        return;
    }
    
    // Create a graphics context for the window
    // This is a simplified version - in a real implementation, this would
    // use the actual framebuffer
    
    InvalidateWindow(handle);
}

void WindowManager::RepaintWindow(WindowHandle handle, const Rectangle& rect) {
    if (handle >= GUI_MAX_WINDOWS || !windows[handle] || !windows[handle]->IsVisible()) {
        return;
    }
    
    InvalidateWindow(handle, rect);
}

// -----------------------------------------------------------------------------
// Message handling
// -----------------------------------------------------------------------------

void WindowManager::PostMessage(WindowHandle hwnd, MessageType msg, uint32_t wParam, uint32_t lParam) {
    if (hwnd >= GUI_MAX_WINDOWS || !windows[hwnd]) {
        return;
    }
    
    Message message;
    message.hwnd = hwnd;
    message.msg = msg;
    message.wParam = wParam;
    message.lParam = lParam;
    message.timestamp = 0; // Would use actual timestamp
    
    // In a real implementation, this would add to a message queue
    // For now, just call the window procedure directly
    windows[hwnd]->OnMessage(msg, wParam, lParam);
}

uint32_t WindowManager::SendMessage(WindowHandle hwnd, MessageType msg, uint32_t wParam, uint32_t lParam) {
    if (hwnd >= GUI_MAX_WINDOWS || !windows[hwnd]) {
        return 0;
    }
    
    windows[hwnd]->OnMessage(msg, wParam, lParam);
    return 1; // Return success
}

void WindowManager::BroadcastMessage(MessageType msg, uint32_t wParam, uint32_t lParam) {
    for (uint32_t i = 0; i < GUI_MAX_WINDOWS; i++) {
        if (windows[i] && windows[i]->IsVisible()) {
            PostMessage(i, msg, wParam, lParam);
        }
    }
}

// -----------------------------------------------------------------------------
// Input handling
// -----------------------------------------------------------------------------

void WindowManager::OnMouseMove(int32_t x, int32_t y, uint8_t buttons) {
    mouseX = x;
    mouseY = y;
    mouseButtons = buttons;
    
    // Update mouse over window
    WindowHandle newMouseOver = INVALID_WINDOW;
    Window* window = GetWindowAt(x, y);
    if (window) {
        newMouseOver = window->GetHandle();
    }
    
    if (mouseOverWindow != newMouseOver) {
        // Send mouse leave to old window
        if (mouseOverWindow != INVALID_WINDOW && windows[mouseOverWindow]) {
            windows[mouseOverWindow]->OnMouseLeave();
        }
        
        // Send mouse enter to new window
        mouseOverWindow = newMouseOver;
        if (mouseOverWindow != INVALID_WINDOW && windows[mouseOverWindow]) {
            windows[mouseOverWindow]->OnMouseEnter();
        }
    }
    
    // Process mouse messages
    ProcessMouseMessages(x, y, buttons);
}

void WindowManager::OnMouseDown(int32_t x, int32_t y, MouseButton button) {
    mouseButtons |= (uint8_t)button;
    
    // Set focus to the window under the mouse
    Window* window = GetWindowAt(x, y);
    if (window) {
        SetFocusWindow(window->GetHandle());
        SetCaptureWindow(window->GetHandle());
        SetActiveWindow(window->GetHandle());
    }
    
    ProcessMouseMessages(x, y, mouseButtons);
}

void WindowManager::OnMouseUp(int32_t x, int32_t y, MouseButton button) {
    mouseButtons &= ~(uint8_t)button;
    
    if (captureWindow != INVALID_WINDOW) {
        SetCaptureWindow(INVALID_WINDOW);
    }
    
    ProcessMouseMessages(x, y, mouseButtons);
}

void WindowManager::OnMouseWheel(int32_t x, int32_t y, int32_t delta) {
    // Not implemented
}

void WindowManager::OnKeyDown(KeyCode key, ModifierKey modifiers) {
    if (focusWindow != INVALID_WINDOW && windows[focusWindow]) {
        windows[focusWindow]->OnKeyDown(key, modifiers);
    }
}

void WindowManager::OnKeyUp(KeyCode key, ModifierKey modifiers) {
    if (focusWindow != INVALID_WINDOW && windows[focusWindow]) {
        windows[focusWindow]->OnKeyUp(key, modifiers);
    }
}

void WindowManager::OnChar(char character, ModifierKey modifiers) {
    if (focusWindow != INVALID_WINDOW && windows[focusWindow]) {
        windows[focusWindow]->OnChar(character, modifiers);
    }
}

// -----------------------------------------------------------------------------
// Helper methods
// -----------------------------------------------------------------------------

void WindowManager::ProcessMouseMessages(int32_t x, int32_t y, uint8_t buttons) {
    // Send mouse move to all windows (for hover effects)
    for (uint32_t i = 0; i < GUI_MAX_WINDOWS; i++) {
        if (windows[i] && windows[i]->IsVisible()) {
            windows[i]->OnMouseMove(x, y, buttons);
        }
    }
}

void WindowManager::UpdateMouseOver() {
    mouseOverWindow = INVALID_WINDOW;
    Window* window = GetWindowAt(mouseX, mouseY);
    if (window) {
        mouseOverWindow = window->GetHandle();
    }
}

// -----------------------------------------------------------------------------
// Timer management
// -----------------------------------------------------------------------------

uint32_t WindowManager::SetTimer(WindowHandle hwnd, uint32_t elapsed, TimerProc proc, void* userData) {
    if (timerCount >= GUI_MAX_TIMERS) {
        return 0;
    }
    
    for (uint32_t i = 0; i < GUI_MAX_TIMERS; i++) {
        if (!timers[i].active) {
            timers[i].hwnd = hwnd;
            timers[i].interval = elapsed;
            timers[i].elapsed = 0;
            timers[i].proc = proc;
            timers[i].userData = userData;
            timers[i].active = true;
            timerCount++;
            return nextTimerId++;
        }
    }
    
    return 0;
}

void WindowManager::KillTimer(uint32_t timerId) {
    // Find timer by ID and deactivate it
    // This is a simplified version - in a real implementation, we would
    // track timer IDs properly
    for (uint32_t i = 0; i < GUI_MAX_TIMERS; i++) {
        if (timers[i].active) {
            timers[i].active = false;
            timerCount--;
            return;
        }
    }
}

void WindowManager::ProcessTimers() {
    // Process all active timers
    for (uint32_t i = 0; i < GUI_MAX_TIMERS; i++) {
        if (timers[i].active) {
            timers[i].elapsed++;
            if (timers[i].elapsed >= timers[i].interval) {
                timers[i].elapsed = 0;
                if (timers[i].proc) {
                    timers[i].proc(timers[i].hwnd, timers[i].interval, timers[i].userData);
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Message loop
// -----------------------------------------------------------------------------

void WindowManager::RunMessageLoop() {
    // In a real implementation, this would process messages from a queue
    // For now, this is a placeholder
}

bool WindowManager::ProcessMessage() {
    // Process timers and update
    ProcessTimers();
    
    // In a real implementation, this would process a message from the queue
    return false;
}

// -----------------------------------------------------------------------------
// Z-order management
// -----------------------------------------------------------------------------

void WindowManager::MoveToFront(WindowHandle handle) {
    BringToFront(handle);
}

void WindowManager::MoveToBack(WindowHandle handle) {
    SendToBack(handle);
}

// -----------------------------------------------------------------------------
// Desktop
// -----------------------------------------------------------------------------

void WindowManager::PaintDesktop(GraphicsContext& gc) {
    // Fill desktop with background color
    gc.FillRect(0, 0, displayInfo.width, displayInfo.height, desktopColor);
}

// -----------------------------------------------------------------------------
// Cursor
// -----------------------------------------------------------------------------

void WindowManager::SetCursor(int32_t x, int32_t y) {
    cursorX = x;
    cursorY = y;
}

void WindowManager::GetCursor(int32_t& x, int32_t& y) const {
    x = cursorX;
    y = cursorY;
}

void WindowManager::ShowCursor(bool show) {
    cursorVisible = show;
}

// -----------------------------------------------------------------------------
// Serialization
// -----------------------------------------------------------------------------

void WindowManager::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    snprintf(buffer, size, "WindowManager(windows=%u, active=%u, focus=%u, size=%ux%u)",
             windowCount, activeWindow, focusWindow, displayInfo.width, displayInfo.height);
}

} // namespace GUI
} // namespace NebulaOS
