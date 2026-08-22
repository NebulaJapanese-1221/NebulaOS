#include "RadioButton.h"

RadioButton::RadioButton(int x, int y, const char* text) : Widget(x, y, 120, 16) {
    int len = 0;
    while (text[len]) len++;
    m_text = new char[len + 1];
    for (int i = 0; i <= len; i++) m_text[i] = text[i];
    m_selected = false;
}

RadioButton::~RadioButton() {
    delete[] m_text;
}

void RadioButton::paint() {
    int bx = m_x;
    int by = m_y;
    for (int i = 0; i < 10; i++) {
        for (int j = 0; j < 10; j++) {
            int dx = i - 5;
            int dy = j - 5;
            if (dx * dx + dy * dy <= 25) {
                uint32_t col = m_selected ? 0xFF0000FF : 0xFF888888;
                drawPixel(bx + i, by + j, col);
            }
        }
    }
}

void RadioButton::handleEvent(int event, int param1, int param2) {
    if (event == 1) {
        m_selected = true;
    }
}