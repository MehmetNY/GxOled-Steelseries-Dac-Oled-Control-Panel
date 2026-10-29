//! Action identifiers. They deliberately reuse the numeric control IDs of the classic Win32 window, so the
//! application maps them to `MenuEvent` with exactly the same table for both interfaces.

pub const MODE_TRAY: u32 = 300;
pub const MODE_WINDOW: u32 = 301;
pub const MODE_HYBRID: u32 = 302;

pub const STYLE_DIGITAL: u32 = 310;
pub const STYLE_DIGITAL_BOLD: u32 = 311;
pub const STYLE_ANALOG_ROUND: u32 = 312;
pub const STYLE_ANALOG_SQUARE: u32 = 313;
pub const STYLE_CASIO: u32 = 314;

pub const TIME_OFF: u32 = 320;
pub const TIME_24: u32 = 321;
pub const TIME_12: u32 = 322;

pub const LANG_EN: u32 = 380;
pub const LANG_TR: u32 = 381;
pub const THEME_LIGHT: u32 = 390;
pub const THEME_DARK: u32 = 391;
/// Settings-window style (beta): modern or classic.
pub const UI_MODERN: u32 = 392;
pub const UI_CLASSIC: u32 = 393;

pub const IDLE: u32 = 354;
pub const MEDIA: u32 = 355;
pub const GITHUB: u32 = 369;
pub const KOFI: u32 = 370;
pub const QUIT: u32 = 371;
pub const PROFILE_EDIT: u32 = 372;
pub const PROFILE_SELECT: u32 = 601;
pub const PROFILE_NEW: u32 = 602;
pub const PROFILE_IMPORT: u32 = 603;
pub const PROFILE_DELETE: u32 = 604;
pub const PROFILE_EXPORT: u32 = 605;
pub const STARTUP: u32 = 606;

pub const SHIFT_OFF: u32 = 620;
pub const SHIFT_LOW: u32 = 621;
pub const SHIFT_HIGH: u32 = 622;
pub const BURN_OFF: u32 = 625;
pub const BURN_WIDE: u32 = 626;
pub const BURN_NO_OVERLAP: u32 = 627;
pub const NOTIFY_OFF: u32 = 651;
pub const NOTIFY_NOTIFY: u32 = 652;
pub const NOTIFY_ALWAYS: u32 = 653;
pub const NOTIFY_CONNECTED: u32 = 654;
pub const NOTIFY_DISCONNECTED: u32 = 655;
pub const QUIT_SCREENSAVER: u32 = 660;
pub const QUIT_STOCK_UI: u32 = 661;
/// 670..=676, one per entry of the volume-screen-time choices.
pub const VOLUME_HOLD_BASE: u32 = 670;
pub const BRIGHTNESS: u32 = 680;

pub const GIF_ADD: u32 = 6060;
pub const GIF_TOGGLE: u32 = 6061;
pub const ANIM_STOP: u32 = 6062;

pub const EDITOR_SAVE: u32 = 401;
pub const EDITOR_RESET: u32 = 402;
pub const EDITOR_IMPORT: u32 = 403;
pub const EDITOR_EXPORT: u32 = 404;
pub const EDITOR_FORMAT: u32 = 405;

/// Sidebar page buttons: `NAV_BASE + Page::index()`.
pub const NAV_BASE: u32 = 200;
/// Profile list rows: `LIST_BASE + row`.
pub const LIST_BASE: u32 = 7000;
