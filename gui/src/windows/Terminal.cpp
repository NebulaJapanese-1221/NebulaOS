#include "Terminal.h"

Terminal::Terminal(int x, int y, int w, int h) : Widget(x, y, w, h), m_rows(0), m_cols(0), m_cursor_x(0), m_cursor_y(0), m_buffer_size(0) {
    m_cols = (w - 4) / 8;
    m_rows = (h - 24) / 8;
    m_buffer_size = m_cols * m_rows;
    m_buffer = new char[m_buffer_size + 1];
    for (int i = 0; i < m_buffer_size; i++) m_buffer[i] = ' ';
    m_buffer[m_buffer_size] = 0;
}

Terminal::~Terminal() {
    delete[] m_buffer;
}

void Terminal::paint() {
    fillRect(m_x, m_y, m_w, m_h, 0xFF000000);
    drawRect(m_x, m_y, m_w, m_h, 0xFF00FF00);
    for (int r = 0; r < m_rows; r++) {
        for (int c = 0; c < m_cols; c++) {
            char ch = m_buffer[r * m_cols + c];
            if (ch == ' ') continue;
            drawChar(m_x + 2 + c * 8, m_y + 22 + r * 8, ch, 0xFF00FF00);
        }
    }
    drawChar(m_x + 2 + m_cursor_x * 8, m_y + 22 + m_cursor_y * 8, '_', 0xFF00FF00);
}

void Terminal::handleEvent(int event, int param1, int param2) {
    if (event == 2) {
        char c = (char)(param1 & 0xFF);
        if (c == '\n') {
            m_cursor_x = 0;
            m_cursor_y++;
        } else if (c == '\b') {
            if (m_cursor_x > 0) m_cursor_x--;
        } else if (c >= 32 && c <= 126) {
            int idx = m_cursor_y * m_cols + m_cursor_x;
            if (idx < m_buffer_size) {
                m_buffer[idx] = c;
                m_cursor_x++;
            }
        }
        if (m_cursor_x >= m_cols) {
            m_cursor_x = 0;
            m_cursor_y++;
        }
        if (m_cursor_y >= m_rows) {
            for (int i = 0; i < m_cols * (m_rows - 1); i++) m_buffer[i] = m_buffer[i + m_cols];
            for (int i = 0; i < m_cols; i++) m_buffer[(m_rows - 1) * m_cols + i] = ' ';
            m_cursor_y = m_rows - 1;
        }
    }
}

void Terminal::write(const char* str) {
    while (*str) {
        char c = *str++;
        if (c == '\n') {
            m_cursor_x = 0;
            m_cursor_y++;
        } else {
            int idx = m_cursor_y * m_cols + m_cursor_x;
            if (idx < m_buffer_size) {
                m_buffer[idx] = c;
                m_cursor_x++;
            }
        }
        if (m_cursor_x >= m_cols) {
            m_cursor_x = 0;
            m_cursor_y++;
        }
        if (m_cursor_y >= m_rows) {
            for (int i = 0; i < m_cols * (m_rows - 1); i++) m_buffer[i] = m_buffer[i + m_cols];
            for (int i = 0; i < m_cols; i++) m_buffer[(m_rows - 1) * m_cols + i] = ' ';
            m_cursor_y = m_rows - 1;
        }
    }
}