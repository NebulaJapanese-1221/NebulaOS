pub mod ja_jp;

use alloc::sync::Arc;
use spin::Mutex;

// A trait for all localisable strings in the UI
pub trait Localisation: Send + Sync {
    fn start(&self) -> &'static str;
    fn app_settings(&self) -> &'static str;
    fn app_terminal(&self) -> &'static str;
    fn app_text_editor(&self) -> &'static str;
    fn app_calculator(&self) -> &'static str;
    fn app_paint(&self) -> &'static str;
    fn app_system_info(&self) -> &'static str;
    fn ctx_properties(&self) -> &'static str;
    fn ctx_refresh(&self) -> &'static str;
    fn btn_shutdown(&self) -> &'static str;
    fn btn_reboot(&self) -> &'static str;
    fn settings_tab_system(&self) -> &'static str;
    fn settings_tab_a11y(&self) -> &'static str;
    fn settings_tab_theme(&self) -> &'static str;
    fn settings_tab_display(&self) -> &'static str;
    fn settings_tab_mouse(&self) -> &'static str;
    fn label_bg_color(&self) -> &'static str;
    fn label_preview(&self) -> &'static str;
    fn label_presets(&self) -> &'static str;
    fn label_mouse_speed(&self) -> &'static str;
    fn label_brightness(&self) -> &'static str;
    fn btn_apply(&self) -> &'static str;
    fn btn_cancel(&self) -> &'static str;
    fn option_high_contrast(&self) -> &'static str;
    fn option_large_text(&self) -> &'static str;
    fn info_version(&self) -> &'static str;
    fn info_resolution(&self) -> &'static str;
    fn info_memory(&self) -> &'static str;
    fn info_uptime(&self) -> &'static str;
    fn preset_nebula(&self) -> &'static str;
    fn preset_sunset(&self) -> &'static str;
    fn settings_tab_language(&self) -> &'static str;
    fn lang_english(&self) -> &'static str;
    fn lang_japanese(&self) -> &'static str;
}

// English implementation
struct EnUs;
impl Localisation for EnUs {
    fn start(&self) -> &'static str { "Start" }
    fn app_settings(&self) -> &'static str { "Settings" }
    fn app_terminal(&self) -> &'static str { "Terminal" }
    fn app_text_editor(&self) -> &'static str { "Text Editor" }
    fn app_calculator(&self) -> &'static str { "Calculator" }
    fn app_paint(&self) -> &'static str { "Paint" }
    fn app_system_info(&self) -> &'static str { "System Info" }
    fn ctx_properties(&self) -> &'static str { "Properties" }
    fn ctx_refresh(&self) -> &'static str { "Refresh" }
    fn btn_shutdown(&self) -> &'static str { "Shutdown" }
    fn btn_reboot(&self) -> &'static str { "Reboot" }
    fn settings_tab_system(&self) -> &'static str { "System" }
    fn settings_tab_a11y(&self) -> &'static str { "Accessibility" }
    fn settings_tab_theme(&self) -> &'static str { "Theme" }
    fn settings_tab_display(&self) -> &'static str { "Display" }
    fn settings_tab_mouse(&self) -> &'static str { "Mouse" }
    fn label_bg_color(&self) -> &'static str { "Background Color:" }
    fn label_preview(&self) -> &'static str { "Preview:" }
    fn label_presets(&self) -> &'static str { "Presets:" }
    fn label_mouse_speed(&self) -> &'static str { "Mouse Speed:" }
    fn label_brightness(&self) -> &'static str { "Brightness:" }
    fn btn_apply(&self) -> &'static str { "Apply" }
    fn btn_cancel(&self) -> &'static str { "Cancel" }
    fn option_high_contrast(&self) -> &'static str { "High Contrast" }
    fn option_large_text(&self) -> &'static str { "Large Text" }
    fn info_version(&self) -> &'static str { "Version:" }
    fn info_resolution(&self) -> &'static str { "Resolution:" }
    fn info_memory(&self) -> &'static str { "Memory:" }
    fn info_uptime(&self) -> &'static str { "Uptime:" }
    fn preset_nebula(&self) -> &'static str { "Nebula" }
    fn preset_sunset(&self) -> &'static str { "Sunset" }
    fn settings_tab_language(&self) -> &'static str { "Language" }
    fn lang_english(&self) -> &'static str { "English" }
    fn lang_japanese(&self) -> &'static str { "Japanese" }
}

// Japanese implementation
impl Localisation for ja_jp::JaJp {
    fn start(&self) -> &'static str { ja_jp::START }
    fn app_settings(&self) -> &'static str { ja_jp::APP_SETTINGS }
    fn app_terminal(&self) -> &'static str { ja_jp::APP_TERMINAL }
    fn app_text_editor(&self) -> &'static str { ja_jp::APP_TEXT_EDITOR }
    fn app_calculator(&self) -> &'static str { ja_jp::APP_CALCULATOR }
    fn app_paint(&self) -> &'static str { ja_jp::APP_PAINT }
    fn app_system_info(&self) -> &'static str { ja_jp::APP_SYSTEM_INFO }
    fn ctx_properties(&self) -> &'static str { ja_jp::CTX_PROPERTIES }
    fn ctx_refresh(&self) -> &'static str { ja_jp::CTX_REFRESH }
    fn btn_shutdown(&self) -> &'static str { ja_jp::BTN_SHUTDOWN }
    fn btn_reboot(&self) -> &'static str { ja_jp::BTN_REBOOT }
    fn settings_tab_system(&self) -> &'static str { ja_jp::SETTINGS_TAB_SYSTEM }
    fn settings_tab_a11y(&self) -> &'static str { ja_jp::SETTINGS_TAB_A11Y }
    fn settings_tab_theme(&self) -> &'static str { ja_jp::SETTINGS_TAB_THEME }
    fn settings_tab_display(&self) -> &'static str { ja_jp::SETTINGS_TAB_DISPLAY }
    fn settings_tab_mouse(&self) -> &'static str { ja_jp::SETTINGS_TAB_MOUSE }
    fn label_bg_color(&self) -> &'static str { ja_jp::LABEL_BG_COLOR }
    fn label_preview(&self) -> &'static str { ja_jp::LABEL_PREVIEW }
    fn label_presets(&self) -> &'static str { ja_jp::LABEL_PRESETS }
    fn label_mouse_speed(&self) -> &'static str { ja_jp::LABEL_MOUSE_SPEED }
    fn label_brightness(&self) -> &'static str { ja_jp::LABEL_BRIGHTNESS }
    fn btn_apply(&self) -> &'static str { ja_jp::BTN_APPLY }
    fn btn_cancel(&self) -> &'static str { ja_jp::BTN_CANCEL }
    fn option_high_contrast(&self) -> &'static str { ja_jp::OPTION_HIGH_CONTRAST }
    fn option_large_text(&self) -> &'static str { ja_jp::OPTION_LARGE_TEXT }
    fn info_version(&self) -> &'static str { ja_jp::INFO_VERSION }
    fn info_resolution(&self) -> &'static str { ja_jp::INFO_RESOLUTION }
    fn info_memory(&self) -> &'static str { ja_jp::INFO_MEMORY }
    fn info_uptime(&self) -> &'static str { ja_jp::INFO_UPTIME }
    fn preset_nebula(&self) -> &'static str { ja_jp::PRESET_NEBULA }
    fn preset_sunset(&self) -> &'static str { ja_jp::PRESET_SUNSET }
    fn settings_tab_language(&self) -> &'static str { ja_jp::SETTINGS_TAB_LANGUAGE }
    fn lang_english(&self) -> &'static str { ja_jp::LANG_ENGLISH }
    fn lang_japanese(&self) -> &'static str { ja_jp::LANG_JAPANESE }
}

pub static CURRENT_LOCALE: Mutex<Option<Arc<dyn Localisation>>> = Mutex::new(None);

#[derive(Clone, Copy, PartialEq)]
pub enum Language {
    English,
    Japanese,
}

pub fn init() {
    // Set default language
    *CURRENT_LOCALE.lock() = Some(Arc::new(EnUs));
}

pub fn set_language(lang: Language) {
    let new_locale: Arc<dyn Localisation> = match lang {
        Language::English => Arc::new(EnUs),
        Language::Japanese => Arc::new(ja_jp::JaJp),
    };
    *CURRENT_LOCALE.lock() = Some(new_locale);
    // Request a full redraw to update all text
    super::gui::FULL_REDRAW_REQUESTED.store(true, core::sync::atomic::Ordering::Relaxed);
}