#ifndef GUI_CHECKBOX_H
#define GUI_CHECKBOX_H

#include "Widget.h"

class Checkbox : public Widget {
public:
    Checkbox(int x, int y, const char* text);
    ~Checkbox();
    
    void paint() override;
    void handleEvent(int event, int param1, int param2) override;
    bool isChecked() const { return m_checked; }
    void setChecked(bool c) { m_checked = c; }
    
private:
    char* m_text;
    bool m_checked;
};

#endif