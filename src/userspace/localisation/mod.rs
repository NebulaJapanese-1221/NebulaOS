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
    fn app_system_info(&self) -> &'static str;
    fn ctx_new_terminal(&self) -> &'static str;
    fn ctx_properties(&self) -> &'static str;
    fn btn_shutdown(&self) -> &'static str;
    fn settings_tab_system(&self) -> &'static str;
    fn settings_tab_a11y(&self) -> &'static str;
    fn settings_tab_theme(&self) -> &'static str;
    fn label_bg_color(&self) -> &'static str;
    fn label_preview(&self) -> &'static str;
    fn label_presets(&self) -> &'static str;
    fn option_high_contrast(&self) -> &'static str;
    fn option_large_text(&self) -> &'static str;
    fn info_version(&self) -> &'static str;
    fn info_kernel(&self) -> &'static str;
    fn info_target(&self) -> &'static str;
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
    fn app_system_info(&self) -> &'static str { "System Info" }
    fn ctx_new_terminal(&self) -> &'static str { "New Terminal" }
    fn ctx_properties(&self) -> &'static str { "Properties" }
    fn btn_shutdown(&self) -> &'static str { "Shutdown" }
    fn settings_tab_system(&self) -> &'static str { "System" }
    fn settings_tab_a11y(&self) -> &'static str { "Accessibility" }
    fn settings_tab_theme(&self) -> &'static str { "Theme" }
    fn label_bg_color(&self) -> &'static str { "Background Color:" }
    fn label_preview(&self) -> &'static str { "Preview:" }
    fn label_presets(&self) -> &'static str { "Presets:" }
    fn option_high_contrast(&self) -> &'static str { "High Contrast" }
    fn option_large_text(&self) -> &'static str { "Large Text" }
    fn info_version(&self) -> &'static str { "Version:" }
    fn info_kernel(&self) -> &'static str { "Kernel:" }
    fn info_target(&self) -> &'static str { "Target:" }
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
    fn app_system_info(&self) -> &'static str { ja_jp::APP_SYSTEM_INFO }
    fn ctx_new_terminal(&self) -> &'static str { ja_jp::CTX_NEW_TERMINAL }
    fn ctx_properties(&self) -> &'static str { ja_jp::CTX_PROPERTIES }
    fn btn_shutdown(&self) -> &'static str { ja_jp::BTN_SHUTDOWN }
    fn settings_tab_system(&self) -> &'static str { ja_jp::SETTINGS_TAB_SYSTEM }
    fn settings_tab_a11y(&self) -> &'static str { ja_jp::SETTINGS_TAB_A11Y }
    fn settings_tab_theme(&self) -> &'static str { ja_jp::SETTINGS_TAB_THEME }
    fn label_bg_color(&self) -> &'static str { ja_jp::LABEL_BG_COLOR }
    fn label_preview(&self) -> &'static str { ja_jp::LABEL_PREVIEW }
    fn label_presets(&self) -> &'static str { ja_jp::LABEL_PRESETS }
    fn option_high_contrast(&self) -> &'static str { ja_jp::OPTION_HIGH_CONTRAST }
    fn option_large_text(&self) -> &'static str { ja_jp::OPTION_LARGE_TEXT }
    fn info_version(&self) -> &'static str { ja_jp::INFO_VERSION }
    fn info_kernel(&self) -> &'static str { ja_jp::INFO_KERNEL }
    fn info_target(&self) -> &'static str { ja_jp::INFO_TARGET }
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