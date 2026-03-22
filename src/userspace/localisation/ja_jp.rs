//! Japanese localization (日本語)

// A struct to implement the Localisation trait on.
pub struct JaJp;

// General
pub const START: &str = "スタート";

// Applications
pub const APP_SETTINGS: &str = "設定";
pub const APP_TERMINAL: &str = "ターミナル";
pub const APP_TEXT_EDITOR: &str = "テキストエディタ";
pub const APP_CALCULATOR: &str = "電卓";
pub const APP_PAINT: &str = "ペイント";
pub const APP_SYSTEM_INFO: &str = "システム情報";

// Context Menu
pub const CTX_NEW_TERMINAL: &str = "新しいターミナル";
pub const CTX_PROPERTIES: &str = "属性";
pub const CTX_REFRESH: &str = "更新";

// Start Menu
pub const BTN_SHUTDOWN: &str = "電源を切る";
pub const BTN_REBOOT: &str = "再起動";

// Settings App
pub const SETTINGS_TAB_SYSTEM: &str = "システム";
pub const SETTINGS_TAB_A11Y: &str = "補助機能"; // "Assistive Functions" (fits better than Katakana)
pub const SETTINGS_TAB_THEME: &str = "テーマ";

pub const LABEL_BG_COLOR: &str = "背景色:";
pub const LABEL_PREVIEW: &str = "プレビュー:";
pub const LABEL_PRESETS: &str = "プリセット:";

pub const OPTION_HIGH_CONTRAST: &str = "高コントラスト";
pub const OPTION_LARGE_TEXT: &str = "大きな文字";
pub const INFO_VERSION: &str = "バージョン:";
pub const INFO_KERNEL: &str = "カーネル:";
pub const INFO_TARGET: &str = "ターゲット:";
pub const INFO_RESOLUTION: &str = "解像度:";
pub const INFO_MEMORY: &str = "メモリ:";
pub const INFO_UPTIME: &str = "稼働時間:";

pub const PRESET_NEBULA: &str = "ネビュラ";
pub const PRESET_SUNSET: &str = "夕焼け";

// New for language selection
pub const SETTINGS_TAB_LANGUAGE: &str = "言語";
pub const LANG_ENGLISH: &str = "英語";
pub const LANG_JAPANESE: &str = "日本語";