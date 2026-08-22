#ifndef GUI_WIDGET_H
#define GUI_WIDGET_H

#include "Control.h"
#include <stdint.h>

// Free functions used by widget paint routines.
void drawPixel(int x, int y, uint32_t color);
void drawRect(int x, int y, int w, int h, uint32_t color);
void fillRect(int x, int y, int w, int h, uint32_t color);
void drawChar(int x, int y, char c, uint32_t color);

// Set the global framebuffer the widgets draw into (rgbx 32-bit).
void gui_set_framebuffer(void* fb, int width, int height, int pitch);

class Widget : public NebulaOS::GUI::Control {
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