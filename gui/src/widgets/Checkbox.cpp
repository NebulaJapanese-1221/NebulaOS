#include "Checkbox.h"

Checkbox::Checkbox(int x, int y, const char* text) : Widget(x, y, 120, 16) {
    int len = 0;
    while (text[len]) len++;
    m_text = new char[len + 1];
    for (int i = 0; i <= len; i++) m_text[i] = text[i];
    m_checked = false;
}

Checkbox::~Checkbox() {
    delete[] m_text;
}

void Checkbox::paint() {
    int bx = m_x;
    int by = m_y;
    for (int i = 0; i < 12; i++) {
        for (int j = 0; j < 12; j++) {
            uint32_t col = 0xFF000000;
            if (i > 0 && i < 11 && j > 0 && j < 11) col = 0xFFFFFFFF;
            if (m_checked && i > 1 && i < 11 && j > 1 && j < 11) col = 0xFF0000FF;
            drawPixel(bx + i, by + j, col);
        }
    }
}

void Checkbox::handleEvent(int event, int param1, int param2) {
    if (event == 1 && param1 >= m_x && param1 < m_x + 12 && param2 >= m_y && param2 < m_y + 12) {
        m_checked = !m_checked;
    }
}