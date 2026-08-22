#ifndef GUI_WIDGET_H
#define GUI_WIDGET_H

#include "../include/Control.h"
#include <stdint.h>

class Widget : public Control {
public:
    Widget(int x, int y, int w, int h);
    virtual ~Widget();
    
    virtual void paint() = 0;
    virtual void handleEvent(int event, int param1, int param2) = 0;
    
    void setGeometry(int x, int y, int w, int h);
    int getX() const { return m_x; }
    int getY() const { return m_y; }
    int getWidth() const { return m_w; }
    int getHeight() const { return m_h; }
    bool isVisible() const { return m_visible; }
    void setVisible(bool v) { m_visible = v; }
    
protected:
    int m_x;
    int m_y;
    int m_w;
    int m_h;
    bool m_visible;
    bool m_enabled;
};

#endif