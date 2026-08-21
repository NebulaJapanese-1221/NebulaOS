// NebulaOS GUI - Theme System
// ============================
//
// Theme management for the GUI system

#ifndef NEBULAOS_GUI_THEME_H
#define NEBULAOS_GUI_THEME_H

#include "../include/GuiTypes.h"
#include "../include/Color.h"

namespace NebulaOS {
namespace GUI {

class Font;

// Theme color types
enum class ThemeColor {
    THEME_COLOR_BACKGROUND = 0,
    THEME_COLOR_FOREGROUND = 1,
    THEME_COLOR_WINDOW_BACKGROUND = 2,
    THEME_COLOR_WINDOW_FRAME = 3,
    THEME_COLOR_WINDOW_CAPTION = 4,
    THEME_COLOR_WINDOW_CAPTION_TEXT = 5,
    THEME_COLOR_BUTTON_FACE = 6,
    THEME_COLOR_BUTTON_HIGHLIGHT = 7,
    THEME_COLOR_BUTTON_SHADOW = 8,
    THEME_COLOR_BUTTON_TEXT = 9,
    THEME_COLOR_LABEL_TEXT = 10,
    THEME_COLOR_TEXTBOX_BACKGROUND = 11,
    THEME_COLOR_TEXTBOX_FRAME = 12,
    THEME_COLOR_TEXTBOX_TEXT = 13,
    THEME_COLOR_PANEL_BACKGROUND = 14,
    THEME_COLOR_PANEL_BORDER = 15,
    THEME_COLOR_MENU_BACKGROUND = 16,
    THEME_COLOR_MENU_TEXT = 17,
    THEME_COLOR_MENU_HIGHLIGHT = 18,
    THEME_COLOR_SCROLLBAR = 19,
    THEME_COLOR_SCROLLBAR_THUMB = 20,
    THEME_COLOR_STATUS_TEXT = 21,
    THEME_COLOR_TOOLTIP_BACKGROUND = 22,
    THEME_COLOR_TOOLTIP_TEXT = 23,
    THEME_COLOR_MAX
};

// Theme font types
enum class ThemeFont {
    THEME_FONT_DEFAULT = 0,
    THEME_FONT_WINDOW_CAPTION = 1,
    THEME_FONT_BUTTON = 2,
    THEME_FONT_LABEL = 3,
    THEME_FONT_TEXTBOX = 4,
    THEME_FONT_MENU = 5,
    THEME_FONT_STATUS = 6,
    THEME_FONT_TOOLTIP = 7,
    THEME_FONT_MAX
};

// Theme metric types
enum class ThemeMetric {
    THEME_METRIC_BORDER_WIDTH = 0,
    THEME_METRIC_CAPTION_HEIGHT = 1,
    THEME_METRIC_FRAME_WIDTH = 2,
    THEME_METRIC_BUTTON_MARGIN = 3,
    THEME_METRIC_SCROLLBAR_WIDTH = 4,
    THEME_METRIC_SCROLLBAR_THUMB_MIN = 5,
    THEME_METRIC_MENU_HEIGHT = 6,
    THEME_METRIC_TOOLTIP_DELAY = 7,
    THEME_METRIC_DOUBLE_CLICK_TIME = 8,
    THEME_METRIC_DOUBLE_CLICK_DISTANCE = 9,
    THEME_METRIC_MAX
};

// Theme class
class Theme {
public:
    Theme();
    virtual ~Theme();
    
    // Initialize theme
    virtual bool Initialize();
    virtual void Shutdown();
    
    // Color management
    virtual Color GetColor(ThemeColor colorType) const;
    virtual void SetColor(ThemeColor colorType, const Color& color);
    
    // Font management
    virtual const Font* GetFont(ThemeFont fontType) const;
    virtual void SetFont(ThemeFont fontType, const Font* font);
    
    // Metric management
    virtual int32_t GetMetric(ThemeMetric metricType) const;
    virtual void SetMetric(ThemeMetric metricType, int32_t value);
    
    // Load theme from configuration
    virtual bool Load(const char* themeName);
    virtual bool Save(const char* themeName);
    
    // Apply theme to window
    virtual void ApplyToWindow(WindowHandle hwnd);
    
    // Apply theme to control
    virtual void ApplyToControl(ControlID controlId);
    
    // Serialization
    virtual void ToString(char* buffer, size_t size) const;
};

// Initialize theme subsystem
bool InitializeTheme();

// Shutdown theme subsystem
void ShutdownTheme();

// Get current theme
Theme* GetTheme();

// Set current theme
void SetTheme(Theme* theme);

// Get theme color
Color GetThemeColor(ThemeColor colorType);

// Get theme font
const Font* GetThemeFont(ThemeFont fontType);

// Get theme metric
int32_t GetThemeMetric(ThemeMetric metricType);

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_THEME_H
