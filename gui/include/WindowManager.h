// NebulaOS GUI - Window Manager
// ================================
//
// Manages all windows in the GUI system

#ifndef NEBULAOS_GUI_WINDOWMANAGER_H
#define NEBULAOS_GUI_WINDOWMANAGER_H

#include "../include/GuiTypes.h"
#include "../include/Rectangle.h"
#include "../include/Point.h"
#include "../include/Color.h"

namespace NebulaOS {
namespace GUI {

class Window;
class GraphicsContext;

// Window manager class
class WindowManager {
public:
    // Singleton access
    static WindowManager* GetInstance();
    static void Initialize(DisplayInfo info);
    static void Shutdown();
    
    // Constructor/Destructor
    WindowManager();
    ~WindowManager();
    
    // Initialization
    bool Initialize(uint32_t width, uint32_t height, uint32_t bpp, void* frameBuffer);
    
    // Display information
    DisplayInfo GetDisplayInfo() const { return displayInfo; }
    void SetDisplayInfo(const DisplayInfo& info) { displayInfo = info; }
    
    // Window management
    WindowHandle RegisterWindow(Window* window);
    void UnregisterWindow(WindowHandle handle);
    Window* GetWindow(WindowHandle handle);
    Window* GetWindowAt(int32_t x, int32_t y);
    
    WindowHandle GetActiveWindow() const { return activeWindow; }
    void SetActiveWindow(WindowHandle handle);
    
    WindowHandle GetFocusWindow() const { return focusWindow; }
    void SetFocusWindow(WindowHandle handle);
    
    WindowHandle GetCaptureWindow() const { return captureWindow; }
    void SetCaptureWindow(WindowHandle handle);
    
    // Window operations
    void BringToFront(WindowHandle handle);
    void SendToBack(WindowHandle handle);
    
    void ShowWindow(WindowHandle handle);
    void HideWindow(WindowHandle handle);
    
    void ActivateWindow(WindowHandle handle);
    void DeactivateWindow(WindowHandle handle);
    
    // Update and repaint
    void UpdateWindow(WindowHandle handle);
    void InvalidateWindow(WindowHandle handle);
    void InvalidateWindow(WindowHandle handle, const Rectangle& rect);
    void RepaintWindow(WindowHandle handle);
    void RepaintWindow(WindowHandle handle, const Rectangle& rect);
    
    // Message handling
    void PostMessage(WindowHandle hwnd, MessageType msg, uint32_t wParam = 0, uint32_t lParam = 0);
    uint32_t SendMessage(WindowHandle hwnd, MessageType msg, uint32_t wParam = 0, uint32_t lParam = 0);
    void BroadcastMessage(MessageType msg, uint32_t wParam = 0, uint32_t lParam = 0);
    
    // Input handling
    void OnMouseMove(int32_t x, int32_t y, uint8_t buttons);
    void OnMouseDown(int32_t x, int32_t y, MouseButton button);
    void OnMouseUp(int32_t x, int32_t y, MouseButton button);
    void OnMouseWheel(int32_t x, int32_t y, int32_t delta);
    void OnKeyDown(KeyCode key, ModifierKey modifiers);
    void OnKeyUp(KeyCode key, ModifierKey modifiers);
    void OnChar(char character, ModifierKey modifiers);
    
    // Timer management
    uint32_t SetTimer(WindowHandle hwnd, uint32_t elapsed, TimerProc proc, void* userData = nullptr);
    void KillTimer(uint32_t timerId);
    void ProcessTimers();
    
    // Message loop
    void RunMessageLoop();
    bool ProcessMessage();
    
    // Z-order management
    void MoveToFront(WindowHandle handle);
    void MoveToBack(WindowHandle handle);
    
    // Desktop
    void SetDesktopColor(const Color& color) { desktopColor = color; }
    Color GetDesktopColor() const { return desktopColor; }
    
    void PaintDesktop(GraphicsContext& gc);
    
    // Cursor
    void SetCursor(int32_t x, int32_t y);
    void GetCursor(int32_t& x, int32_t& y) const;
    void ShowCursor(bool show);
    bool IsCursorShown() const { return cursorVisible; }
    
    // Serialization
    void ToString(char* buffer, size_t size) const;
    
    // Window count
    uint32_t GetWindowCount() const { return windowCount; }

private:
    static WindowManager* instance;
    
    DisplayInfo displayInfo;
    
    Window* windows[GUI_MAX_WINDOWS];
    uint32_t windowCount;
    
    WindowHandle activeWindow;
    WindowHandle focusWindow;
    WindowHandle captureWindow;
    
    Color desktopColor;
    
    // Mouse state
    int32_t mouseX;
    int32_t mouseY;
    uint8_t mouseButtons;
    WindowHandle mouseOverWindow;
    
    // Cursor
    bool cursorVisible;
    int32_t cursorX;
    int32_t cursorY;
    
    // Timers
    struct Timer {
        WindowHandle hwnd;
        uint32_t interval;
        uint32_t elapsed;
        TimerProc proc;
        void* userData;
        bool active;
    };
    
    Timer timers[GUI_MAX_TIMERS];
    uint32_t timerCount;
    uint32_t nextTimerId;
    
    // Helper methods
    void UpdateMouseOver();
    void ProcessMouseMessages(int32_t x, int32_t y, uint8_t buttons);
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_WINDOWMANAGER_H
