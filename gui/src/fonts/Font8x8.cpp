#include "Font8x8.h"

const uint8_t* Font8x8::getCharData(char c) {
    if (c < 32) c = '?';
    if (c > 127) c = '?';
    c -= 32;
    return font8x8_data[c];
}