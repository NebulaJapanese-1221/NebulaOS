#ifndef GUI_MENUBAR_H
#define GUI_MENUBAR_H

#include "Widget.h"
#include <stdint.h>

class MenuItem {
public:
    char* text;
    void (*action)(void);
    MenuItem* submenu;
    int submenu_count;
    
    MenuItem(const char* t);
    ~MenuItem();
};

class MenuBar : public Widget {
public:
    MenuBar(int x, int y, int w);
    ~MenuBar();
    
    void paint() override;
    void handleEvent(int event, int param1, int param2) override;
    void addItem(MenuItem* item);
    
private:
    MenuItem** m_items;
    int m_count;
    int m_selected;
};

#endif