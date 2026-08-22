#ifndef GUI_FONT8X8_H
#define GUI_FONT8X8_H

#include <stdint.h>

class Font8x8 {
public:
    static const uint8_t* getCharData(char c);
    static int getWidth() { return 8; }
    static int getHeight() { return 8; }
};

#endif