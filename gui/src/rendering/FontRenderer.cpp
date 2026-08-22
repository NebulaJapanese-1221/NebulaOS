#include "FontRenderer.h"
#include "../fonts/Font8x8.h"

FontRenderer::FontRenderer(Renderer* renderer) : m_renderer(renderer) {}

void FontRenderer::drawChar(int x, int y, char c, uint32_t color) {
    if (c < 32 || c > 127) c = '?';
    const uint8_t* data = Font8x8::getCharData(c);
    for (int row = 0; row < 8; row++) {
        for (int col = 0; col < 8; col++) {
            if (data[row] & (1 << col)) {
                m_renderer->drawPixel(x + col, y + row, color);
            }
        }
    }
}

void FontRenderer::drawString(int x, int y, const char* str, uint32_t color) {
    while (*str) {
        drawChar(x, y, *str, color);
        x += 8;
        str++;
    }
}

int FontRenderer::getTextWidth(const char* str) {
    int width = 0;
    while (*str) {
        width += 8;
        str++;
    }
    return width;
}