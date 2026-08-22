#ifndef GUI_FONTRENDERER_H
#define GUI_FONTRENDERER_H

#include "../include/Color.h"
#include "../include/Font.h"
#include "Renderer.h"

class FontRenderer {
public:
    FontRenderer(Renderer* renderer);
    
    void drawChar(int x, int y, char c, uint32_t color);
    void drawString(int x, int y, const char* str, uint32_t color);
    int getTextWidth(const char* str);
    int getTextHeight() const { return 8; }
    
private:
    Renderer* m_renderer;
};

#endif