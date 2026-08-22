#ifndef GUI_RENDERER_H
#define GUI_RENDERER_H

#include "../include/Color.h"
#include <stdint.h>

class Renderer {
public:
    Renderer();
    ~Renderer();
    
    void setFramebuffer(void* fb, uint32_t width, uint32_t height, uint32_t pitch);
    void drawPixel(int x, int y, uint32_t color);
    void drawLine(int x0, int y0, int x1, int y1, uint32_t color);
    void drawRect(int x, int y, int w, int h, uint32_t color);
    void fillRect(int x, int y, int w, int h, uint32_t color);
    void clear(uint32_t color);
    
    uint32_t getWidth() const { return m_width; }
    uint32_t getHeight() const { return m_height; }
    
private:
    void* m_fb;
    uint32_t m_width;
    uint32_t m_height;
    uint32_t m_pitch;
    bool m_valid;
};

#endif