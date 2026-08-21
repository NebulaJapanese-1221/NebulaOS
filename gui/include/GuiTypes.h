// NebulaOS GUI - Type Definitions
// =================================
//
// Common type definitions for the GUI system

#ifndef NEBULAOS_GUI_TYPES_H
#define NEBULAOS_GUI_TYPES_H

#include "../../kernel/common/include/stdint.h"

// Using standard integer types from kernel
using int8_t = ::int8_t;
using uint8_t = ::uint8_t;
using int16_t = ::int16_t;
using uint16_t = ::uint16_t;
using int32_t = ::int32_t;
using uint32_t = ::uint32_t;
using int64_t = ::int64_t;
using uint64_t = ::uint64_t;
using size_t = ::size_t;

namespace NebulaOS {
namespace GUI {

// Forward declarations
class Point;
class Size;
class Rectangle;
class Color;

// Window handle type
using WindowHandle = uint32_t;
const WindowHandle INVALID_WINDOW = 0xFFFFFFFF;

// Control ID type
using ControlID = uint32_t;
const ControlID INVALID_CONTROL = 0xFFFFFFFF;

// Timer ID type
using TimerID = uint32_t;
const TimerID INVALID_TIMER = 0xFFFFFFFF;

// Message types
enum class MessageType {
    MSG_NONE,
    MSG_PAINT,
    MSG_MOUSE_MOVE,
    MSG_MOUSE_DOWN,
    MSG_MOUSE_UP,
    MSG_MOUSE_DOUBLE_CLICK,
    MSG_KEY_DOWN,
    MSG_KEY_UP,
    MSG_CHAR,
    MSG_SIZE,
    MSG_MOVE,
    MSG_CLOSE,
    MSG_ACTIVATE,
    MSG_DEACTIVATE,
    MSG_TIMER,
    MSG_COMMAND,
    MSG_USER
};

// Mouse buttons
enum class MouseButton {
    LEFT = 1,
    RIGHT = 2,
    MIDDLE = 4
};

// Mouse state
struct MouseState {
    int32_t x;
    int32_t y;
    uint8_t buttons;
    int32_t wheel;
};

// Key codes (partial)
enum class KeyCode {
    KEY_NONE = 0,
    KEY_ESCAPE = 1,
    KEY_1 = 2,
    KEY_2 = 3,
    KEY_3 = 4,
    KEY_4 = 5,
    KEY_5 = 6,
    KEY_6 = 7,
    KEY_7 = 8,
    KEY_8 = 9,
    KEY_9 = 10,
    KEY_0 = 11,
    KEY_MINUS = 12,
    KEY_EQUALS = 13,
    KEY_BACKSPACE = 14,
    KEY_TAB = 15,
    KEY_Q = 16,
    KEY_W = 17,
    KEY_E = 18,
    KEY_R = 19,
    KEY_T = 20,
    KEY_Y = 21,
    KEY_U = 22,
    KEY_I = 23,
    KEY_O = 24,
    KEY_P = 25,
    KEY_LEFTBRACKET = 26,
    KEY_RIGHTBRACKET = 27,
    KEY_ENTER = 28,
    KEY_LEFTCTRL = 29,
    KEY_A = 30,
    KEY_S = 31,
    KEY_D = 32,
    KEY_F = 33,
    KEY_G = 34,
    KEY_H = 35,
    KEY_J = 36,
    KEY_K = 37,
    KEY_L = 38,
    KEY_SEMICOLON = 39,
    KEY_APOSTROPHE = 40,
    KEY_GRAVE = 41,
    KEY_LEFTSHIFT = 42,
    KEY_BACKSLASH = 43,
    KEY_Z = 44,
    KEY_X = 45,
    KEY_C = 46,
    KEY_V = 47,
    KEY_B = 48,
    KEY_N = 49,
    KEY_M = 50,
    KEY_COMMA = 51,
    KEY_DOT = 52,
    KEY_SLASH = 53,
    KEY_RIGHTSHIFT = 54,
    KEY_KPASTERISK = 55,
    KEY_LEFTALT = 56,
    KEY_SPACE = 57,
    KEY_CAPSLOCK = 58,
    KEY_F1 = 59,
    KEY_F2 = 60,
    KEY_F3 = 61,
    KEY_F4 = 62,
    KEY_F5 = 63,
    KEY_F6 = 64,
    KEY_F7 = 65,
    KEY_F8 = 66,
    KEY_F9 = 67,
    KEY_F10 = 68,
    KEY_NUMLOCK = 69,
    KEY_SCROLLLOCK = 70,
    KEY_KP7 = 71,
    KEY_KP8 = 72,
    KEY_KP9 = 73,
    KEY_KPMINUS = 74,
    KEY_KP4 = 75,
    KEY_KP5 = 76,
    KEY_KP6 = 77,
    KEY_KPPLUS = 78,
    KEY_KP1 = 79,
    KEY_KP2 = 80,
    KEY_KP3 = 81,
    KEY_KP0 = 82,
    KEY_KPDOT = 83,
    KEY_F11 = 87,
    KEY_F12 = 88,
    KEY_UP = 192,
    KEY_DOWN = 193,
    KEY_LEFT = 194,
    KEY_RIGHT = 195,
    KEY_HOME = 196,
    KEY_END = 197,
    KEY_DELETE = 198
};

// Modifier keys (regular enum for bitwise operations)
enum ModifierKey {
    MOD_NONE = 0,
    MOD_SHIFT = 1,
    MOD_CTRL = 2,
    MOD_ALT = 4,
    MOD_WIN = 8
};

// Event structure
struct Event {
    MessageType type;
    uint32_t timestamp;
    union {
        struct {
            int32_t x;
            int32_t y;
        } mouseMove;
        struct {
            int32_t x;
            int32_t y;
            MouseButton button;
        } mouseButton;
        struct {
            KeyCode key;
            ModifierKey modifiers;
            char character;
        } key;
        struct {
            WindowHandle window;
        } window;
        struct {
            uint32_t id;
            uint32_t param1;
            uint32_t param2;
        } command;
    } data;
};

// Message structure for window procedure
struct Message {
    WindowHandle hwnd;
    MessageType msg;
    uint32_t wParam;
    uint32_t lParam;
    uint32_t timestamp;
};

// Window procedure callback type
using WindowProc = void (*)(WindowHandle hwnd, MessageType msg, uint32_t wParam, uint32_t lParam);

// Timer callback type
using TimerProc = void (*)(uint32_t timerId, uint32_t elapsed, void* userData);

// Draw callback type
using DrawProc = void (*)(void* context, int32_t x, int32_t y, uint32_t width, uint32_t height);

// Font style
enum class FontStyle {
    FONT_NORMAL = 0,
    FONT_BOLD = 1,
    FONT_ITALIC = 2,
    FONT_UNDERLINE = 4,
    FONT_STRIKEOUT = 8
};

// Window styles
enum class WindowStyle : uint32_t {
    WS_NONE = 0,
    WS_BORDER = 1,
    WS_CAPTION = 2,
    WS_SYSMENU = 4,
    WS_MINIMIZEBOX = 8,
    WS_MAXIMIZEBOX = 16,
    WS_RESIZABLE = 32,
    WS_VISIBLE = 64,
    WS_ENABLED = 128,
    WS_CLASSIC = 256,
    WS_TOOLWINDOW = 512,
    WS_POPUP = 1024,
    WS_CHILD = 2048
};

// Bitwise operators for WindowStyle
inline WindowStyle operator|(WindowStyle a, WindowStyle b) { 
    return static_cast<WindowStyle>(static_cast<uint32_t>(a) | static_cast<uint32_t>(b)); 
}
inline WindowStyle operator&(WindowStyle a, WindowStyle b) { 
    return static_cast<WindowStyle>(static_cast<uint32_t>(a) & static_cast<uint32_t>(b)); 
}
inline WindowStyle operator^(WindowStyle a, WindowStyle b) { 
    return static_cast<WindowStyle>(static_cast<uint32_t>(a) ^ static_cast<uint32_t>(b)); 
}
inline bool operator==(WindowStyle a, WindowStyle b) { 
    return static_cast<uint32_t>(a) == static_cast<uint32_t>(b); 
}
inline bool operator!=(WindowStyle a, WindowStyle b) { 
    return static_cast<uint32_t>(a) != static_cast<uint32_t>(b); 
}

// Window extended styles
enum class WindowStyleEx : uint32_t {
    WS_EX_NONE = 0,
    WS_EX_CLIENTEDGE = 1,
    WS_EX_WINDOWEDGE = 2,
    WS_EX_STATICEDGE = 4,
    WS_EX_TOOLWINDOW = 8
};

// Bitwise operators for WindowStyleEx
inline WindowStyleEx operator|(WindowStyleEx a, WindowStyleEx b) { 
    return static_cast<WindowStyleEx>(static_cast<uint32_t>(a) | static_cast<uint32_t>(b)); 
}
inline WindowStyleEx operator&(WindowStyleEx a, WindowStyleEx b) { 
    return static_cast<WindowStyleEx>(static_cast<uint32_t>(a) & static_cast<uint32_t>(b)); 
}

// Default window dimensions
const int32_t DEFAULT_WINDOW_WIDTH = 640;
const int32_t DEFAULT_WINDOW_HEIGHT = 480;

// GUI system constants
const uint32_t GUI_MAX_WINDOWS = 256;
const uint32_t GUI_MAX_CONTROLS = 1024;
const uint32_t GUI_MAX_TIMERS = 64;

// Display information
struct DisplayInfo {
    uint32_t width;
    uint32_t height;
    uint32_t bitsPerPixel;
    uint32_t refreshRate;
    void* frameBuffer;
};

// Graphics context
class GraphicsContext;

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_TYPES_H
