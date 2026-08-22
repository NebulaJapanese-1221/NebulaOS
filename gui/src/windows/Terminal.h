#ifndef GUI_TERMINAL_H
#define GUI_TERMINAL_H

#include "../widgets/Widget.h"

class Terminal : public Widget {
public:
    Terminal(int x, int y, int w, int h);
    ~Terminal();
    
    void paint() override;
    void handleEvent(int event, int param1, int param2) override;
    void write(const char* str);
    
private:
    char* m_buffer;
    int m_rows;
    int m_cols;
    int m_cursor_x;
    int m_cursor_y;
    int m_buffer_size;
};

#endif