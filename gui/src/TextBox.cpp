// NebulaOS GUI - TextBox Implementation
// =======================================
//
// Implementation of the TextBox control class

#include "../include/TextBox.h"
#include "../include/GraphicsContext.h"
#include "../include/WindowManager.h"
#include "../include/Theme.h"
#include "../../lib/include/string.h"
#include "../../kernel/common/include/stdint.h"

namespace NebulaOS {
namespace GUI {

static char textbox_clipboard[1024];
static size_t textbox_clipboard_len = 0;

// -----------------------------------------------------------------------------
// Constructor
// -----------------------------------------------------------------------------

TextBox::TextBox() {
    id = INVALID_CONTROL;
    x = 0;
    y = 0;
    width = 200;
    height = 25;
    parent = INVALID_WINDOW;
    visible = true;
    enabled = true;
    focused = false;
    pressed = false;
    hovered = false;
    zOrder = 0;
    text[0] = '\0';
    fgColor = Color::Black();
    bgColor = Color::White();
    font = nullptr;
    textLength = 0;
    cursorPosition = 0;
    selectionStart = 0;
    selectionEnd = 0;
    readOnly = false;
    multiLine = false;
    passwordMode = false;
    maxLength = sizeof(text) - 1;
}

TextBox::TextBox(ControlID id, int32_t x, int32_t y, uint32_t width, uint32_t height) {
    this->id = id;
    this->x = x;
    this->y = y;
    this->width = width;
    this->height = height;
    parent = INVALID_WINDOW;
    visible = true;
    enabled = true;
    focused = false;
    pressed = false;
    hovered = false;
    zOrder = 0;
    text[0] = '\0';
    fgColor = Color::Black();
    bgColor = Color::White();
    font = nullptr;
    textLength = 0;
    cursorPosition = 0;
    selectionStart = 0;
    selectionEnd = 0;
    readOnly = false;
    multiLine = false;
    passwordMode = false;
    maxLength = sizeof(text) - 1;
}

TextBox::TextBox(ControlID id, const Rectangle& bounds) {
    this->id = id;
    x = bounds.x;
    y = bounds.y;
    width = bounds.width;
    height = bounds.height;
    parent = INVALID_WINDOW;
    visible = true;
    enabled = true;
    focused = false;
    pressed = false;
    hovered = false;
    zOrder = 0;
    text[0] = '\0';
    fgColor = Color::Black();
    bgColor = Color::White();
    font = nullptr;
    textLength = 0;
    cursorPosition = 0;
    selectionStart = 0;
    selectionEnd = 0;
    readOnly = false;
    multiLine = false;
    passwordMode = false;
    maxLength = sizeof(text) - 1;
}

// -----------------------------------------------------------------------------
// Destructor
// -----------------------------------------------------------------------------

TextBox::~TextBox() {
    // Nothing to do
}

// -----------------------------------------------------------------------------
// Text manipulation
// -----------------------------------------------------------------------------

void TextBox::SetText(const char* newText) {
    if (newText) {
        strncpy(text, newText, sizeof(text) - 1);
        text[sizeof(text) - 1] = '\0';
        textLength = strlen(text);
        cursorPosition = textLength;
        selectionStart = textLength;
        selectionEnd = textLength;
    } else {
        text[0] = '\0';
        textLength = 0;
        cursorPosition = 0;
        selectionStart = 0;
        selectionEnd = 0;
    }
    Invalidate();
}

void TextBox::AppendText(const char* newText) {
    if (!newText) return;
    
    size_t newLen = strlen(newText);
    if (textLength + newLen >= maxLength) {
        newLen = maxLength - textLength - 1;
    }
    
    if (newLen > 0) {
        strncat(text, newText, newLen);
        textLength += newLen;
        cursorPosition = textLength;
        selectionStart = textLength;
        selectionEnd = textLength;
        Invalidate();
    }
}

void TextBox::InsertText(size_t position, const char* newText) {
    if (!newText || position > textLength) return;
    
    size_t newLen = strlen(newText);
    if (textLength + newLen >= maxLength) {
        newLen = maxLength - textLength - 1;
    }
    
    if (newLen > 0) {
        // Make room for new text
        size_t remaining = textLength - position;
        if (remaining > 0) {
            memmove(text + position + newLen, text + position, remaining);
        }
        
        // Copy new text
        memcpy(text + position, newText, newLen);
        textLength += newLen;
        text[textLength] = '\0';
        
        cursorPosition = position + newLen;
        selectionStart = cursorPosition;
        selectionEnd = cursorPosition;
        Invalidate();
    }
}

void TextBox::DeleteText(size_t position, size_t count) {
    if (position >= textLength) return;
    
    if (position + count > textLength) {
        count = textLength - position;
    }
    
    if (count > 0) {
        memmove(text + position, text + position + count, textLength - position - count);
        textLength -= count;
        text[textLength] = '\0';
        
        if (cursorPosition > position) {
            cursorPosition = (cursorPosition >= position + count) ? cursorPosition - count : position;
        }
        selectionStart = cursorPosition;
        selectionEnd = cursorPosition;
        Invalidate();
    }
}

void TextBox::Clear() {
    text[0] = '\0';
    textLength = 0;
    cursorPosition = 0;
    selectionStart = 0;
    selectionEnd = 0;
    Invalidate();
}

// -----------------------------------------------------------------------------
// Selection
// -----------------------------------------------------------------------------

void TextBox::SetSelection(size_t start, size_t end) {
    if (start > end) {
        selectionStart = end;
        selectionEnd = start;
    } else {
        selectionStart = start;
        selectionEnd = end;
    }
    
    if (selectionStart > textLength) selectionStart = textLength;
    if (selectionEnd > textLength) selectionEnd = textLength;
    
    cursorPosition = selectionEnd;
    Invalidate();
}

void TextBox::GetSelection(size_t& start, size_t& end) const {
    start = selectionStart;
    end = selectionEnd;
}

void TextBox::SelectAll() {
    SetSelection(0, textLength);
}

void TextBox::SelectNone() {
    selectionStart = cursorPosition;
    selectionEnd = cursorPosition;
    Invalidate();
}

// -----------------------------------------------------------------------------
// Cursor position
// -----------------------------------------------------------------------------

void TextBox::SetCursorPosition(size_t position) {
    if (position <= textLength) {
        cursorPosition = position;
        selectionStart = cursorPosition;
        selectionEnd = cursorPosition;
        ScrollToCursor();
        Invalidate();
    }
}

void TextBox::MoveCursorLeft() {
    if (cursorPosition > 0) {
        cursorPosition--;
        if (!HasSelection()) {
            selectionStart = cursorPosition;
            selectionEnd = cursorPosition;
        } else {
            selectionEnd = cursorPosition;
        }
        ScrollToCursor();
        Invalidate();
    }
}

void TextBox::MoveCursorRight() {
    if (cursorPosition < textLength) {
        cursorPosition++;
        if (!HasSelection()) {
            selectionStart = cursorPosition;
            selectionEnd = cursorPosition;
        } else {
            selectionStart = cursorPosition;
        }
        ScrollToCursor();
        Invalidate();
    }
}

void TextBox::MoveCursorHome() {
    cursorPosition = 0;
    selectionStart = cursorPosition;
    selectionEnd = cursorPosition;
    ScrollToCursor();
    Invalidate();
}

void TextBox::MoveCursorEnd() {
    cursorPosition = textLength;
    selectionStart = cursorPosition;
    selectionEnd = cursorPosition;
    ScrollToCursor();
    Invalidate();
}

// -----------------------------------------------------------------------------
// Max length
// -----------------------------------------------------------------------------

void TextBox::SetMaxLength(size_t max) {
    if (max < sizeof(text)) {
        maxLength = max;
    } else {
        maxLength = sizeof(text) - 1;
    }
    
    // Truncate text if necessary
    if (textLength >= maxLength) {
        text[maxLength] = '\0';
        textLength = maxLength;
        cursorPosition = textLength;
        selectionStart = textLength;
        selectionEnd = textLength;
    }
}

// -----------------------------------------------------------------------------
// Clipboard
// -----------------------------------------------------------------------------

void TextBox::SetClipboard(const char* text) {
    if (!text) return;
    size_t len = strlen(text);
    if (len >= sizeof(textbox_clipboard)) {
        len = sizeof(textbox_clipboard) - 1;
    }
    memcpy(textbox_clipboard, text, len);
    textbox_clipboard[len] = '\0';
    textbox_clipboard_len = len;
}

const char* TextBox::GetClipboard() {
    return textbox_clipboard;
}

bool TextBox::HasClipboard() {
    return textbox_clipboard_len > 0;
}

void TextBox::ClearClipboard() {
    textbox_clipboard[0] = '\0';
    textbox_clipboard_len = 0;
}

// -----------------------------------------------------------------------------
// Painting
// -----------------------------------------------------------------------------

void TextBox::Paint(GraphicsContext& gc) {
    if (!visible) return;
    
    // Save current state
    gc.SaveState();
    
    // Set up clipping
    gc.PushClip(Rectangle(x, y, width, height));
    
    // Paint background
    PaintBackground(gc);
    
    // Paint border
    PaintBorder(gc);
    
    // Paint content
    DrawTextContent(gc);
    DrawSelection(gc);
    
    // Draw cursor if focused
    if (focused) {
        DrawCursor(gc);
    }
    
    // Restore state
    gc.PopClip();
    gc.RestoreState();
}

void TextBox::PaintBackground(GraphicsContext& gc) {
    Color bg = enabled ? bgColor : Color::LightGray();
    gc.FillRect(x, y, width, height, bg);
}

void TextBox::PaintBorder(GraphicsContext& gc) {
    Color borderColor = enabled ? Color::Black() : Color::Gray();
    gc.DrawRect(x, y, width, height, borderColor);
    
    // Draw inset border for 3D effect
    gc.DrawLine(x + 1, y + 1, x + width - 2, y + 1, Color::White());
    gc.DrawLine(x + 1, y + 1, x + 1, y + height - 2, Color::White());
    gc.DrawLine(x + width - 1, y + 1, x + width - 1, y + height - 1, Color::DarkGray());
    gc.DrawLine(x + 1, y + height - 1, x + width - 1, y + height - 1, Color::DarkGray());
}

void TextBox::PaintContent(GraphicsContext& gc) {
    DrawTextContent(gc);
}

void TextBox::DrawTextContent(GraphicsContext& gc) {
    if (text[0] == '\0') return;
    
    int32_t textX = x + 3;
    int32_t textY = y + 3;
    
    // Apply password mode
    char displayText[1024];
    if (passwordMode) {
        memset(displayText, '*', textLength);
        displayText[textLength] = '\0';
    } else {
        strcpy(displayText, text);
    }
    
    // Draw text with selection background
    gc.SetColor(fgColor);
    gc.DrawText(displayText, textX, textY, fgColor);
}

void TextBox::DrawSelection(GraphicsContext& gc) {
    if (!HasSelection()) return;
    
    // Calculate selection rectangle
    int32_t startX = x + 3;
    int32_t endX = x + 3;
    
    // Simple selection drawing (would be more sophisticated with proper text measurement)
    for (size_t i = selectionStart; i < selectionEnd; i++) {
        startX += gc.GetCharWidth(text[i]);
    }
    
    endX = startX;
    startX = x + 3;
    
    // Draw selection rectangle
    gc.FillRect(startX, y + 3, endX - startX, gc.GetCharHeight('A'), Color::Blue());
    
    // Redraw selected text in white
    for (size_t i = selectionStart; i < selectionEnd; i++) {
        char ch = passwordMode ? '*' : text[i];
        gc.DrawCharacter(ch, startX, y + 3, Color::White());
        startX += gc.GetCharWidth(ch);
    }
}

void TextBox::DrawCursor(GraphicsContext& gc) {
    if (!focused) return;
    
    // Calculate cursor position
    int32_t cursorX = x + 3;
    for (size_t i = 0; i < cursorPosition; i++) {
        cursorX += gc.GetCharWidth(text[i]);
    }
    
    int32_t cursorY = y + 3;
    int32_t cursorHeight = gc.GetCharHeight('A');
    
    // Draw blinking cursor
    gc.FillRect(cursorX, cursorY, 2, cursorHeight, Color::Black());
}

// -----------------------------------------------------------------------------
// Helper methods
// -----------------------------------------------------------------------------

void TextBox::ScrollToCursor() {
    // Simplified - in a real implementation, this would scroll the view
    // to ensure the cursor is visible
}

void TextBox::DeleteSelection() {
    if (HasSelection()) {
        DeleteText(selectionStart, selectionEnd - selectionStart);
        SelectNone();
    }
}

size_t TextBox::GetCharacterAtPosition(int32_t x, int32_t y) const {
    // Simplified - would use text measurement in a real implementation
    return 0;
}

// -----------------------------------------------------------------------------
// Message handling
// -----------------------------------------------------------------------------

void TextBox::OnMessage(MessageType msg, uint32_t wParam, uint32_t lParam) {
    Control::OnMessage(msg, wParam, lParam);
}

void TextBox::OnMouseDown(int32_t x, int32_t y, MouseButton button) {
    if (!enabled || !visible || button != MouseButton::LEFT) return;
    
    if (HitTest(x, y)) {
        focused = true;
        pressed = true;
        
        // Set cursor position based on click
        cursorPosition = GetCharacterAtPosition(x - this->x, y - this->y);
        selectionStart = cursorPosition;
        selectionEnd = cursorPosition;
        
        Invalidate();
    }
}

void TextBox::OnMouseDoubleClick(int32_t x, int32_t y, MouseButton button) {
    if (!enabled || !visible) return;
    
    // Select word at cursor position (simplified)
    SelectAll();
}

void TextBox::OnKeyDown(KeyCode key, ModifierKey modifiers) {
    if (!focused || readOnly) return;
    
    switch (key) {
        case KeyCode::KEY_LEFT:
            if (modifiers & MOD_SHIFT) {
                if (cursorPosition > selectionStart) {
                    selectionEnd = cursorPosition;
                    cursorPosition--;
                    selectionStart = cursorPosition;
                } else {
                    selectionStart = cursorPosition;
                    cursorPosition--;
                    selectionEnd = cursorPosition;
                }
            } else {
                MoveCursorLeft();
            }
            break;
        
        case KeyCode::KEY_RIGHT:
            if (modifiers & MOD_SHIFT) {
                if (cursorPosition < selectionEnd) {
                    selectionStart = cursorPosition;
                    cursorPosition++;
                    selectionEnd = cursorPosition;
                } else {
                    selectionEnd = cursorPosition;
                    cursorPosition++;
                    selectionStart = cursorPosition;
                }
            } else {
                MoveCursorRight();
            }
            break;
        
        case KeyCode::KEY_HOME:
            if (modifiers & MOD_SHIFT) {
                selectionStart = 0;
                selectionEnd = cursorPosition;
                cursorPosition = 0;
            } else {
                MoveCursorHome();
            }
            break;
        
        case KeyCode::KEY_END:
            if (modifiers & MOD_SHIFT) {
                selectionStart = cursorPosition;
                selectionEnd = textLength;
                cursorPosition = textLength;
            } else {
                MoveCursorEnd();
            }
            break;
        
        case KeyCode::KEY_DELETE:
            if (HasSelection()) {
                DeleteSelection();
            } else {
                DeleteText(cursorPosition, 1);
            }
            break;
        
        case KeyCode::KEY_BACKSPACE:
            if (HasSelection()) {
                DeleteSelection();
            } else if (cursorPosition > 0) {
                MoveCursorLeft();
                DeleteText(cursorPosition, 1);
            }
            break;
        
        case KeyCode::KEY_A:
            if (modifiers & MOD_CTRL) {
                SelectAll();
            }
            break;
        
        case KeyCode::KEY_C:
            if (modifiers & MOD_CTRL && HasSelection()) {
                size_t start, end;
                GetSelection(start, end);
                SetClipboard(text + start);
            }
            break;
        
        case KeyCode::KEY_X:
            if (modifiers & MOD_CTRL && HasSelection()) {
                size_t start, end;
                GetSelection(start, end);
                SetClipboard(text + start);
                DeleteSelection();
            }
            break;
        
        case KeyCode::KEY_V:
            if (modifiers & MOD_CTRL && HasClipboard()) {
                DeleteSelection();
                InsertText(cursorPosition, GetClipboard());
            }
            break;
        
        default:
            break;
    }
    
    Invalidate();
}

void TextBox::OnChar(char character, ModifierKey modifiers) {
    if (!focused || readOnly || character < 32 || character > 126) return;
    
    // Delete selection
    DeleteSelection();
    
    // Insert character
    if (textLength < maxLength) {
        InsertText(cursorPosition, &character);
        cursorPosition++;
        selectionStart = cursorPosition;
        selectionEnd = cursorPosition;
        Invalidate();
    }
}

// -----------------------------------------------------------------------------
// Focus
// -----------------------------------------------------------------------------

void TextBox::Focus() {
    focused = true;
    Invalidate();
}

void TextBox::Unfocus() {
    focused = false;
    Invalidate();
}

// -----------------------------------------------------------------------------
// Serialization
// -----------------------------------------------------------------------------

void TextBox::ToString(char* buffer, size_t size) const {
    if (!buffer || size == 0) return;
    
    snprintf(buffer, size, "TextBox(id=%u, x=%d, y=%d, w=%u, h=%u, text='%s', cursor=%u)",
             id, x, y, width, height, text, (uint32_t)cursorPosition);
}

} // namespace GUI
} // namespace NebulaOS
