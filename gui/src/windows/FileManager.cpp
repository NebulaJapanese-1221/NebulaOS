#include "FileManager.h"
#include "../../../kernel/common/include/fs.h"

FileManager::FileManager(int x, int y, int w, int h) : Window(x, y, w, h), m_files(nullptr), m_file_count(0), m_selected(-1) {
    listFiles();
}

FileManager::~FileManager() {
    if (m_files) {
        for (int i = 0; i < m_file_count; i++) delete[] m_files[i];
        delete[] m_files;
    }
}

void FileManager::listFiles() {
    m_file_count = 1;
    m_files = new char*[m_file_count];
    m_files[0] = new char[8];
    for (int i = 0; i < 7; i++) m_files[0][i] = "/"[i];
    m_files[0][7] = 0;
}

void FileManager::paint() {
    fillRect(m_x, m_y, m_w, m_h, 0xFFCCCCCC);
    drawRect(m_x, m_y, m_w, m_h, 0xFF000000);
    int y = m_y + 20;
    for (int i = 0; i < m_file_count && y < m_y + m_h - 20; i++) {
        if (i == m_selected) fillRect(m_x + 2, y, m_w - 4, 14, 0xFF4444FF);
        y += 16;
    }
}

void FileManager::handleEvent(int event, int param1, int param2) {
    if (event == 1) {
        int idx = (param2 - m_y - 20) / 16;
        if (idx >= 0 && idx < m_file_count) m_selected = idx;
    }
}