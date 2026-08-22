// NebulaOS GUI - TextBox Control
// =================================
//
// Text box control for text input

#ifndef NEBULAOS_GUI_TEXTBOX_H
#define NEBULAOS_GUI_TEXTBOX_H

#include "../include/Control.h"
#include "../include/GuiTypes.h"

namespace NebulaOS {
namespace GUI {

class GraphicsContext;

// TextBox control class
class TextBox : public Control {
public:
    // Constructor
    TextBox();
    TextBox(ControlID id, int32_t x, int32_t y, uint32_t width, uint32_t height);
    TextBox(ControlID id, const Rectangle& bounds);
    
    virtual ~TextBox();
    
    // Text content
    const char* GetText() const { return text; }
    void SetText(const char* newText);
    
    // Text manipulation
    void AppendText(const char* newText);
    void InsertText(size_t position, const char* newText);
    void DeleteText(size_t position, size_t count);
    void Clear();
    
    // Selection
    void SetSelection(size_t start, size_t end);
    void GetSelection(size_t& start, size_t& end) const;
    void SelectAll();
    void SelectNone();
    bool HasSelection() const { return selectionStart != selectionEnd; }
    
    // Cursor position
    size_t GetCursorPosition() const { return cursorPosition; }
    void SetCursorPosition(size_t position);
    void MoveCursorLeft();
    void MoveCursorRight();
    void MoveCursorHome();
    void MoveCursorEnd();
    
    // Read-only
    bool IsReadOnly() const { return readOnly; }
    void SetReadOnly(bool ro) { readOnly = ro; }
    
    // Multi-line
    bool IsMultiLine() const { return multiLine; }
    void SetMultiLine(bool ml) { multiLine = ml; }
    
    // Password mode
    bool IsPassword() const { return passwordMode; }
    void SetPassword(bool password) { passwordMode = password; }
    
    // Max length
    size_t GetMaxLength() const { return maxLength; }
    void SetMaxLength(size_t max);
    
    // Clipboard
    static void SetClipboard(const char* text);
    static const char* GetClipboard();
    static bool HasClipboard();
    static void ClearClipboard();
    
    // Painting
    virtual void Paint(GraphicsContext& gc) ;
    virtual void PaintBackground(GraphicsContext& gc) ;
    virtual void PaintContent(GraphicsContext& gc) ;
    virtual void PaintBorder(GraphicsContext& gc) ;
    
    // Message handling
    virtual void OnMessage(MessageType msg, uint32_t wParam, uint32_t lParam) ;
    
    virtual void OnMouseDown(int32_t x, int32_t y, MouseButton button) ;
    virtual void OnMouseDoubleClick(int32_t x, int32_t y, MouseButton button) ;
    virtual void OnKeyDown(KeyCode key, ModifierKey modifiers) ;
    virtual void OnChar(char character, ModifierKey modifiers) ;
    
    // Focus
    virtual void Focus() ;
    virtual void Unfocus() ;
    
    // Serialization
    virtual void ToString(char* buffer, size_t size) const ;

protected:
    char text[1024];  // Text buffer
    size_t textLength;
    size_t cursorPosition;
    size_t selectionStart;
    size_t selectionEnd;
    bool readOnly;
    bool multiLine;
    bool passwordMode;
    size_t maxLength;
    
    // Helper methods
    void DrawTextContent(GraphicsContext& gc);
    void DrawCursor(GraphicsContext& gc);
    void DrawSelection(GraphicsContext& gc);
    void ScrollToCursor();
    void DeleteSelection();
    size_t GetCharacterAtPosition(int32_t x, int32_t y) const;
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_TEXTBOX_H
