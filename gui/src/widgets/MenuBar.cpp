#include "MenuBar.h"

MenuItem::MenuItem(const char* t) : action(nullptr), submenu(nullptr), submenu_count(0) {
    int len = 0;
    while (t[len]) len++;
    text = new char[len + 1];
    for (int i = 0; i <= len; i++) text[i] = t[i];
}

MenuItem::~MenuItem() {
    delete[] text;
}

MenuBar::MenuBar(int x, int y, int w) : Widget(x, y, w, 16), m_items(nullptr), m_count(0), m_selected(-1) {}

MenuBar::~MenuBar() {
    if (m_items) {
        for (int i = 0; i < m_count; i++) delete m_items[i];
        delete[] m_items;
    }
}

void MenuBar::paint() {
    fillRect(m_x, m_y, m_w, m_h, 0xFF888888);
    int cx = m_x;
    for (int i = 0; i < m_count; i++) {
        uint32_t bg = (i == m_selected) ? 0xFF4444FF : 0xFF888888;
        fillRect(cx, m_y, 60, m_h, bg);
        cx += 60;
    }
}

void MenuBar::handleEvent(int event, int param1, int param2) {
    if (event == 1) {
        int idx = (param1 - m_x) / 60;
        if (idx >= 0 && idx < m_count) {
            m_selected = idx;
            if (m_items[idx] && m_items[idx]->action) {
                m_items[idx]->action();
            }
        }
    }
}

void MenuBar::addItem(MenuItem* item) {
    m_items = new MenuItem*[m_count + 1];
    for (int i = 0; i < m_count; i++) m_items[i] = m_items[i];
    m_items[m_count++] = item;
}