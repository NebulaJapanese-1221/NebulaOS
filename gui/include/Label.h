// NebulaOS GUI - Label Control
// ===============================
//
// Label control class for displaying text

#ifndef NEBULAOS_GUI_LABEL_H
#define NEBULAOS_GUI_LABEL_H

#include "../include/Control.h"
#include "../include/GuiTypes.h"

namespace NebulaOS {
namespace GUI {

class GraphicsContext;

// Text alignment
enum class TextAlignment {
    ALIGN_LEFT = 0,
    ALIGN_CENTER = 1,
    ALIGN_RIGHT = 2,
    ALIGN_TOP = 0,
    ALIGN_MIDDLE = 4,
    ALIGN_BOTTOM = 8
};

// Label control class
class Label : public Control {
public:
    // Constructor
    Label();
    Label(ControlID id, int32_t x, int32_t y, uint32_t width, uint32_t height);
    Label(ControlID id, const Rectangle& bounds);
    
    virtual ~Label();
    
    // Text alignment
    TextAlignment GetTextAlign() const { return textAlign; }
    void SetTextAlign(TextAlignment align) { textAlign = align; }
    
    // Word wrap
    bool GetWordWrap() const { return wordWrap; }
    void SetWordWrap(bool wrap) { wordWrap = wrap; }
    
    // Multi-line
    bool GetMultiLine() const { return multiLine; }
    void SetMultiLine(bool ml) { multiLine = ml; }
    
    // Painting
    virtual void Paint(GraphicsContext& gc) override;
    virtual void PaintContent(GraphicsContext& gc) override;
    
    // Serialization
    virtual void ToString(char* buffer, size_t size) const override;

protected:
    TextAlignment textAlign;
    bool wordWrap;
    bool multiLine;
    
    // Helper methods
    void DrawLabelText(GraphicsContext& gc);
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_LABEL_H
