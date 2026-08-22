#include "../include/Color.h"
#include "Renderer.h"

static int iabs(int v) { return v < 0 ? -v : v; }

Renderer::Renderer() : m_fb(nullptr), m_width(0), m_height(0), m_pitch(0), m_valid(false) {}

Renderer::~Renderer() {}

void Renderer::setFramebuffer(void* fb, uint32_t width, uint32_t height, uint32_t pitch) {
    m_fb = fb;
    m_width = width;
    m_height = height;
    m_pitch = pitch;
    m_valid = (fb != nullptr);
}

void Renderer::drawPixel(int x, int y, uint32_t color) {
    if (!m_valid || x < 0 || y < 0 || x >= (int)m_width || y >= (int)m_height) return;
    uint32_t* ptr = (uint32_t*)((uint8_t*)m_fb + y * m_pitch + x * 4);
    *ptr = color;
}

void Renderer::drawLine(int x0, int y0, int x1, int y1, uint32_t color) {
    int dx = x1 - x0;
    int dy = y1 - y0;
    int steps = (iabs(dx) > iabs(dy)) ? iabs(dx) : iabs(dy);
    float xInc = (float)dx / (float)steps;
    float yInc = (float)dy / (float)steps;
    float x = (float)x0;
    float y = (float)y0;
    for (int i = 0; i <= steps; i++) {
        drawPixel((int)x, (int)y, color);
        x += xInc;
        y += yInc;
    }
}

void Renderer::drawRect(int x, int y, int w, int h, uint32_t color) {
    drawLine(x, y, x + w - 1, y, color);
    drawLine(x, y + h - 1, x + w - 1, y + h - 1, color);
    drawLine(x, y, x, y + h - 1, color);
    drawLine(x + w - 1, y, x + w - 1, y + h - 1, color);
}

void Renderer::fillRect(int x, int y, int w, int h, uint32_t color) {
    for (int j = y; j < y + h; j++) {
        for (int i = x; i < x + w; i++) {
            drawPixel(i, j, color);
        }
    }
}

void Renderer::clear(uint32_t color) {
    fillRect(0, 0, m_width, m_height, color);
}