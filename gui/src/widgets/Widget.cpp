#include "Widget.h"

Widget::Widget(int x, int y, int w, int h) : m_x(x), m_y(y), m_w(w), m_h(h), m_visible(true), m_enabled(true) {}

Widget::~Widget() {}

void Widget::setGeometry(int x, int y, int w, int h) {
    m_x = x;
    m_y = y;
    m_w = w;
    m_h = h;
}