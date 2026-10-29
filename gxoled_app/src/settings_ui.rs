//! Beta: chooses between the classic Win32 settings window and the modern one.
//!
//! The classic window is always created (it owns the tray hook and is the safety net); the modern window is
//! created lazily the first time it is opened. Every function forwards to the classic window and, when it has
//! anything to do, to the modern one, so the application code calls a single module.
//!
//! Falling back to classic is automatic: if the modern window cannot be created, panics inside its window
//! procedure (see `modern_window::failed`), or a previous start never reached its first paint (crash guard), the
//! style is switched back to Classic and saved.

use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc::Sender,
        OnceLock,
    },
};

use gxoled_lib::Bitmap;

use super::{
    modern_ui, settings_window as classic, Config, ConfigLanguage, ConfigTheme, ConfigUiMode, ConfigUiStyle, MenuEvent,
};

static STYLE: AtomicU8 = AtomicU8::new(0);
static FAILURE_HANDLED: AtomicBool = AtomicBool::new(false);
static TX: OnceLock<Sender<MenuEvent>> = OnceLock::new();

const MODERN: u8 = 0;
const CLASSIC: u8 = 1;

fn style() -> ConfigUiStyle {
    if STYLE.load(Ordering::Relaxed) == MODERN { ConfigUiStyle::Modern } else { ConfigUiStyle::Classic }
}

fn store_style(s: ConfigUiStyle) {
    STYLE.store(if s == ConfigUiStyle::Modern { MODERN } else { CLASSIC }, Ordering::Relaxed);
}

/// Run code of the modern interface that is called from the main loop (not from a window procedure, which has its
/// own guard). A panic there must never take the whole application down: it marks the modern UI as failed, after
/// which it is no longer touched and `poll` switches to the classic window. `None` = skipped or panicked.
fn guarded<R>(f: impl FnOnce() -> R) -> Option<R> {
    if modern_ui::failed() {
        return None;
    }
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => Some(r),
        Err(_) => {
            modern_ui::mark_failed();
            None
        }
    }
}

pub fn create(tx: Sender<MenuEvent>, config: &Config) -> Result<(), String> {
    let _ = TX.set(tx.clone());
    store_style(config.ui_style);
    guarded(|| modern_ui::init(tx.clone(), config));
    classic::create(tx)
}

/// True if the previous modern-UI start never reached its first paint; the caller then switches to classic.
pub fn previous_start_failed() -> bool {
    modern_ui::take_crash_guard()
}

pub fn open() {
    if style() == ConfigUiStyle::Classic {
        classic::open();
        return;
    }
    match guarded(modern_ui::open) {
        Some(Ok(())) => {}
        Some(Err(err)) => {
            eprintln!("GxOled modern settings failed: {err}; falling back to the classic window");
            fall_back();
            classic::open();
        }
        None => {
            fall_back();
            classic::open();
        }
    }
}

/// The modern window cannot be used any more: remember Classic (the main loop persists it).
fn fall_back() {
    store_style(ConfigUiStyle::Classic);
    if let Some(tx) = TX.get() {
        let _ = tx.send(MenuEvent::SetUiStyle(ConfigUiStyle::Classic));
    }
}

/// Switch the visible style. The main loop calls this after saving the new `ui_style`; the window of the new
/// style is shown so the effect of the choice is visible at once.
pub fn set_style(new: ConfigUiStyle) {
    let new = if new == ConfigUiStyle::Modern && modern_ui::failed() {
        // A modern window already failed in this session; do not trust it again until the app restarts.
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title("GxOled")
            .set_description(
                "The modern (Beta) interface stopped working in this session. Restart GxOled to try it again.\n\nModern (Beta) arayüz bu oturumda çalışmayı durdurdu. Tekrar denemek için GxOled'i yeniden başlatın.",
            )
            .show();
        fall_back();
        ConfigUiStyle::Classic
    } else {
        new
    };
    store_style(new);
    classic::hide();
    guarded(modern_ui::hide);
    open();
}

pub fn sync(config: &Config) {
    classic::sync(config);
    guarded(|| modern_ui::sync(config));
}

pub fn localize(language: ConfigLanguage) {
    classic::localize(language);
    guarded(|| modern_ui::localize(language));
}

pub fn set_theme(theme: ConfigTheme) {
    classic::set_theme(theme);
    guarded(|| modern_ui::set_theme(theme));
}

pub fn set_tray_mode(mode: ConfigUiMode) {
    classic::set_tray_mode(mode);
}

pub fn install_tray_hook(mode: ConfigUiMode) {
    classic::install_tray_hook(mode);
}

pub fn preview(bitmap: &Bitmap) {
    classic::preview(bitmap);
    guarded(|| modern_ui::preview(bitmap));
}

pub fn set_connected(connected: bool) {
    guarded(|| modern_ui::set_connected(connected));
}

/// Pump window messages for every window of this thread (classic and modern alike) and notice a modern-window failure.
pub fn poll() {
    classic::poll();
    if modern_ui::failed() && !FAILURE_HANDLED.swap(true, Ordering::Relaxed) {
        eprintln!("GxOled modern settings window failed; switching to the classic window");
        fall_back();
    }
}
