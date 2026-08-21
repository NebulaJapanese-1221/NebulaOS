// NebulaOS GUI - Main Implementation
// ===================================
//
// Implementation of the main GUI system functions

#include "../include/GUI.h"
#include "../include/WindowManager.h"
#include "../include/GraphicsContext.h"
#include "../include/Rendering.h"
#include "../include/Input.h"
#include "../include/Timer.h"
#include "../include/Theme.h"
#include "../include/Font.h"

namespace NebulaOS {
namespace GUI {

// Static variables
static bool guiInitialized = false;
static DisplayInfo displayInfo = {0, 0, 0, 0, nullptr};

// -----------------------------------------------------------------------------
// Initialize the GUI system
// -----------------------------------------------------------------------------

bool InitializeGUI(uint32_t width, uint32_t height, uint32_t bpp, void* frameBuffer) {
    if (guiInitialized) {
        return true;
    }
    
    // Store display information
    displayInfo.width = width;
    displayInfo.height = height;
    displayInfo.bitsPerPixel = bpp;
    displayInfo.refreshRate = 60;
    displayInfo.frameBuffer = frameBuffer;
    
    // Initialize window manager
    WindowManager::Initialize(displayInfo);
    
    // Initialize subsystems
    InitializeRendering(width, height, bpp, frameBuffer);
    InitializeInput();
    InitializeTimers();
    InitializeTheme();
    
    // Create default font
    BuiltinFont* defaultFont = new BuiltinFont(12, FontStyle::FONT_NORMAL);
    
    guiInitialized = true;
    
    return true;
}

// -----------------------------------------------------------------------------
// Shutdown the GUI system
// -----------------------------------------------------------------------------

void ShutdownGUI() {
    if (!guiInitialized) {
        return;
    }
    
    // Shutdown subsystems
    ShutdownTheme();
    ShutdownTimers();
    ShutdownInput();
    ShutdownRendering();
    
    // Shutdown window manager
    WindowManager::Shutdown();
    
    guiInitialized = false;
}

// -----------------------------------------------------------------------------
// Run the GUI message loop
// -----------------------------------------------------------------------------

void RunMessageLoop() {
    WindowManager* wm = WindowManager::GetInstance();
    if (wm) {
        wm->RunMessageLoop();
    }
}

// -----------------------------------------------------------------------------
// Process a single message
// -----------------------------------------------------------------------------

bool ProcessMessage(const Message& msg) {
    WindowManager* wm = WindowManager::GetInstance();
    if (wm) {
        return wm->ProcessMessage();
    }
    return false;
}

// -----------------------------------------------------------------------------
// Post a message to a window
// -----------------------------------------------------------------------------

bool PostMessage(WindowHandle hwnd, MessageType msg, uint32_t wParam, uint32_t lParam) {
    WindowManager* wm = WindowManager::GetInstance();
    if (wm) {
        wm->PostMessage(hwnd, msg, wParam, lParam);
        return true;
    }
    return false;
}

// -----------------------------------------------------------------------------
// Send a message to a window (synchronous)
// -----------------------------------------------------------------------------

uint32_t SendMessage(WindowHandle hwnd, MessageType msg, uint32_t wParam, uint32_t lParam) {
    WindowManager* wm = WindowManager::GetInstance();
    if (wm) {
        return wm->SendMessage(hwnd, msg, wParam, lParam);
    }
    return 0;
}

// -----------------------------------------------------------------------------
// Broadcast a message to all windows
// -----------------------------------------------------------------------------

void BroadcastMessage(MessageType msg, uint32_t wParam, uint32_t lParam) {
    WindowManager* wm = WindowManager::GetInstance();
    if (wm) {
        wm->BroadcastMessage(msg, wParam, lParam);
    }
}

// -----------------------------------------------------------------------------
// Get the window manager
// -----------------------------------------------------------------------------

WindowManager* GetWindowManager() {
    return WindowManager::GetInstance();
}

// -----------------------------------------------------------------------------
// Get the display information
// -----------------------------------------------------------------------------

DisplayInfo GetDisplayInfo() {
    return displayInfo;
}

// -----------------------------------------------------------------------------
// Check if GUI is initialized
// -----------------------------------------------------------------------------

bool IsGUIInitialized() {
    return guiInitialized;
}

} // namespace GUI
} // namespace NebulaOS

// -----------------------------------------------------------------------------
// Stub implementations for GUI subsystems
// -----------------------------------------------------------------------------

namespace NebulaOS {
namespace GUI {

bool InitializeRendering(uint32_t width, uint32_t height, uint32_t bpp, void* frameBuffer) {
    (void)width; (void)height; (void)bpp; (void)frameBuffer;
    return true;
}

void ShutdownRendering() {
}

bool InitializeInput() {
    return true;
}

void ShutdownInput() {
}

bool InitializeTimers() {
    return true;
}

void ShutdownTimers() {
}

bool InitializeTheme() {
    return true;
}

void ShutdownTheme() {
}

} // namespace GUI
} // namespace NebulaOS
