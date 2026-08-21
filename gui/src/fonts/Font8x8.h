// NebulaOS GUI - 8x8 Bitmap Font
// =================================
//
// 8x8 bitmap font for ASCII characters 32-126

#ifndef NEBULAOS_GUI_FONT8X8_H
#define NEBULAOS_GUI_FONT8X8_H

#include "../../include/GuiTypes.h"
#include "../../include/Color.h"

namespace NebulaOS {
namespace GUI {

class Font8x8 {
public:
    static constexpr uint32_t CHAR_WIDTH = 8;
    static constexpr uint32_t CHAR_HEIGHT = 8;
    static constexpr char FIRST_CHAR = 32;
    static constexpr char LAST_CHAR = 126;
    static constexpr uint32_t CHAR_COUNT = LAST_CHAR - FIRST_CHAR + 1;

    Font8x8();
    ~Font8x8();

    const uint8_t* GetCharData(char c) const;
    uint32_t GetWidth() const { return CHAR_WIDTH; }
    uint32_t GetHeight() const { return CHAR_HEIGHT; }
    uint32_t GetCharCount() const { return CHAR_COUNT; }
    char GetFirstChar() const { return FIRST_CHAR; }
    char GetLastChar() const { return LAST_CHAR; }

    static bool IsCharSupported(char c);

private:
    static const uint8_t fontData[CHAR_COUNT][CHAR_HEIGHT];
};

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_FONT8X8_H
