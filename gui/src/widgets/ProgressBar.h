#ifndef GUI_PROGRESSBAR_H
#define GUI_PROGRESSBAR_H

#include "Widget.h"

class ProgressBar : public Widget {
public:
    ProgressBar(int x, int y, int w, int h);
    ~ProgressBar();
    
    void paint() override;
    void handleEvent(int event, int param1, int param2) override;
    void setValue(int v) { m_value = v; }
    int getValue() const { return m_value; }
    void setMin(int m) { m_min = m; }
    void setMax(int m) { m_max = m; }
    
private:
    int m_min;
    int m_max;
    int m_value;
};

#endif