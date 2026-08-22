#ifndef GUI_FILEMANAGER_H
#define GUI_FILEMANAGER_H

#include "../include/Window.h"

class FileManager : public Window {
public:
    FileManager(int x, int y, int w, int h);
    ~FileManager();
    
    void paint() override;
    void handleEvent(int event, int param1, int param2) override;
    
private:
    char** m_files;
    int m_file_count;
    int m_selected;
    void listFiles();
};

#endif