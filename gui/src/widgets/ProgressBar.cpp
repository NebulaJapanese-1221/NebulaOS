#include "ProgressBar.h"

ProgressBar::ProgressBar(int x, int y, int w, int h) : Widget(x, y, w, h), m_min(0), m_max(100), m_value(0) {}

ProgressBar::~ProgressBar() {}

void ProgressBar::paint() {
    drawRect(m_x, m_y, m_w, m_h, 0xFF000000);
    fillRect(m_x + 1, m_y + 1, m_w - 2, m_h - 2, 0xFFCCCCCC);
    int fill = (int)(((float)(m_value - m_min) / (float)(m_max - m_min)) * (m_w - 2));
    if (fill > 0) fillRect(m_x + 1, m_y + 1, fill, m_h - 2, 0xFF0000FF);
}

void ProgressBar::handleEvent(int event, int param1, int param2) {
    (void)event; (void)param1; (void)param2;
}