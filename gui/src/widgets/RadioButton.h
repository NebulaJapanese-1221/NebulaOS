#ifndef GUI_RADIOBUTTON_H
#define GUI_RADIOBUTTON_H

#include "Widget.h"

class RadioButton : public Widget {
public:
    RadioButton(int x, int y, const char* text);
    ~RadioButton();
    
    void paint() override;
    void handleEvent(int event, int param1, int param2) override;
    bool isSelected() const { return m_selected; }
    void setSelected(bool s) { m_selected = s; }
    
private:
    char* m_text;
    bool m_selected;
};

#endif