// NebulaOS GUI - Application Class
// =================================
//
// Base application class for GUI applications

#ifndef NEBULAOS_GUI_APPLICATION_H
#define NEBULAOS_GUI_APPLICATION_H

#include "../include/GuiTypes.h"
#include "../include/Window.h"

namespace NebulaOS {
namespace GUI {

class WindowManager;

// Application class
class Application {
public:
    // Constructor/Destructor
    Application();
    Application(const char* name);
    virtual ~Application();
    
    // Application information
    const char* GetName() const { return name; }
    void SetName(const char* newName);
    
    WindowHandle GetMainWindow() const { return mainWindow; }
    void SetMainWindow(WindowHandle hwnd) { mainWindow = hwnd; }
    
    // Initialization
    virtual bool Initialize();
    virtual void Shutdown();
    
    // Run the application
    virtual int Run();
    
    // Message handling
    virtual void OnMessage(WindowHandle hwnd, MessageType msg, uint32_t wParam, uint32_t lParam);
    
    // Event handlers
    virtual void OnCreate();
    virtual void OnDestroy();
    virtual void OnActivate();
    virtual void OnDeactivate();
    
    // Command handling
    virtual void OnCommand(WindowHandle hwnd, uint32_t commandId);
    
    // Window management
    Window* CreateWindow(const char* title, int32_t x, int32_t y, uint32_t width, uint32_t height,
                        WindowStyle style = WindowStyle::WS_BORDER | WindowStyle::WS_CAPTION | WindowStyle::WS_VISIBLE,
                        WindowStyleEx exStyle = WindowStyleEx::WS_EX_NONE);
    
    void CloseWindow(WindowHandle hwnd);
    void CloseAllWindows();
    
    // Dialog functions
    int MessageBox(const char* title, const char* message, uint32_t style = 0);
    
    // Exit the application
    void Exit(int exitCode = 0);
    int GetExitCode() const { return exitCode; }
    
    // Serialization
    virtual void ToString(char* buffer, size_t size) const;

protected:
    char name[128];
    WindowHandle mainWindow;
    int exitCode;
    bool running;
    
    // Helper methods
    static Application* instance;
};

// Get the current application instance
Application* GetApplication();

// Create and run an application
int RunApplication(Application* app);

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_APPLICATION_H
