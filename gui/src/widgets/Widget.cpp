#include "Widget.h"
#include "../fonts/Font8x8.h"

static uint32_t* g_fb = nullptr;
static int g_fb_width = 0;
static int g_fb_height = 0;
static int g_fb_pitch = 0;

void gui_set_framebuffer(void* fb, int width, int height, int pitch) {
    g_fb = (uint32_t*)fb;
    g_fb_width = width;
    g_fb_height = height;
    g_fb_pitch = pitch;
}

void drawPixel(int x, int y, uint32_t color) {
    if (!g_fb) return;
    if (x < 0 || y < 0 || x >= g_fb_width || y >= g_fb_height) return;
    uint32_t* ptr = (uint32_t*)((uint8_t*)g_fb + y * g_fb_pitch + x * 4);
    *ptr = color;
}

void drawRect(int x, int y, int w, int h, uint32_t color) {
    for (int i = 0; i < w; i++) {
        drawPixel(x + i, y, color);
        drawPixel(x + i, y + h - 1, color);
    }
    for (int j = 0; j < h; j++) {
        drawPixel(x, y + j, color);
        drawPixel(x + w - 1, y + j, color);
    }
}

void fillRect(int x, int y, int w, int h, uint32_t color) {
    for (int j = 0; j < h; j++) {
        for (int i = 0; i < w; i++) {
            drawPixel(x + i, y + j, color);
        }
    }
}

void drawChar(int x, int y, char c, uint32_t color) {
    if (!g_fb) return;
    const uint8_t* glyph = Font8x8::getCharData(c);
    for (int row = 0; row < 8; row++) {
        uint8_t bits = glyph[row];
        for (int col = 0; col < 8; col++) {
            if (bits & (0x80 >> col)) {
                drawPixel(x + col, y + row, color);
            }
        }
    }
}

Widget::Widget(int x, int y, int w, int h) : m_x(x), m_y(y), m_w(w), m_h(h), m_visible(true), m_enabled(true) {}

Widget::~Widget() {}

void Widget::setGeometry(int x, int y, int w, int h) {
    m_x = x;
    m_y = y;
    m_w = w;
    m_h = h;
}