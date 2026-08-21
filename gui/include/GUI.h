// NebulaOS GUI - Main Header
// ==========================
//
// Main include file for the NebulaOS GUI system

#ifndef NEBULAOS_GUI_H
#define NEBULAOS_GUI_H

// Include all GUI headers
#include "../include/GuiTypes.h"
#include "../include/Point.h"
#include "../include/Size.h"
#include "../include/Rectangle.h"
#include "../include/Color.h"
#include "../include/GraphicsContext.h"
#include "../include/Window.h"
#include "../include/WindowManager.h"
#include "../include/Control.h"
#include "../include/Button.h"
#include "../include/Label.h"
#include "../include/TextBox.h"
#include "../include/Panel.h"
#include "../include/Font.h"
#include "../include/Application.h"

// Subsystems
#include "../include/Rendering.h"
#include "../include/Input.h"
#include "../include/Timer.h"
#include "../include/Theme.h"

namespace NebulaOS {
namespace GUI {

// Initialize the GUI system
bool InitializeGUI(uint32_t width, uint32_t height, uint32_t bpp, void* frameBuffer);

// Shutdown the GUI system
void ShutdownGUI();

// Run the GUI message loop
void RunMessageLoop();

// Process a single message
bool ProcessMessage(const Message& msg);

// Post a message to a window
bool PostMessage(WindowHandle hwnd, MessageType msg, uint32_t wParam = 0, uint32_t lParam = 0);

// Send a message to a window (synchronous)
uint32_t SendMessage(WindowHandle hwnd, MessageType msg, uint32_t wParam = 0, uint32_t lParam = 0);

// Broadcast a message to all windows
void BroadcastMessage(MessageType msg, uint32_t wParam = 0, uint32_t lParam = 0);

// Get the window manager
WindowManager* GetWindowManager();

// Get the display information
DisplayInfo GetDisplayInfo();

// Check if GUI is initialized
bool IsGUIInitialized();

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_H
