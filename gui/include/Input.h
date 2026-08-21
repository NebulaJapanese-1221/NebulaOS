// NebulaOS GUI - Input Subsystem
// ================================
//
// Input handling for the GUI system

#ifndef NEBULAOS_GUI_INPUT_H
#define NEBULAOS_GUI_INPUT_H

#include "../include/GuiTypes.h"
#include "../include/Point.h"

namespace NebulaOS {
namespace GUI {

class WindowManager;

// Mouse cursor type
enum class CursorType {
    CURSOR_DEFAULT = 0,
    CURSOR_ARROW = 1,
    CURSOR_IBEAM = 2,
    CURSOR_WAIT = 3,
    CURSOR_CROSS = 4,
    CURSOR_HAND = 5,
    CURSOR_SIZE_NS = 6,
    CURSOR_SIZE_WE = 7,
    CURSOR_SIZE_NWSE = 8,
    CURSOR_SIZE_NESW = 9
};

// Mouse cursor structure
struct Cursor {
    int32_t hotspotX;
    int32_t hotspotY;
    uint32_t width;
    uint32_t height;
    const uint8_t* andMask;
    const uint8_t* xorMask;
};

// Input state structure
struct InputState {
    // Mouse state
    Point mousePosition;
    uint8_t mouseButtons;
    int32_t mouseWheel;
    
    // Keyboard state
    uint8_t modifiers;
    bool keyPressed[256];
    
    // Focus window
    WindowHandle focusWindow;
    
    // Capture window
    WindowHandle captureWindow;
};

// Initialize input subsystem
bool InitializeInput();

// Shutdown input subsystem
void ShutdownInput();

// Update input state
void UpdateInputState();

// Get input state
InputState GetInputState();

// Mouse functions
void SetMousePosition(int32_t x, int32_t y);
void GetMousePosition(int32_t& x, int32_t& y);
Point GetMousePosition();

uint8_t GetMouseButtons();
bool IsMouseButtonDown(MouseButton button);

// Set cursor
void SetCursor(CursorType type);
void SetCursor(const Cursor& cursor);
void ShowCursor(bool show);
bool IsCursorShown();

// Keyboard functions
bool IsKeyDown(KeyCode key);
bool IsKeyUp(KeyCode key);
uint8_t GetModifiers();

// Character input
char GetChar();
bool HasChar();

// Focus management
void SetFocusWindow(WindowHandle hwnd);
WindowHandle GetFocusWindow();

void SetCaptureWindow(WindowHandle hwnd);
WindowHandle GetCaptureWindow();

// Serialization
void InputStateToString(const InputState& state, char* buffer, size_t size);

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_INPUT_H
