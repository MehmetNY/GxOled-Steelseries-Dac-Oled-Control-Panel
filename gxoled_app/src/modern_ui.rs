//! Beta: bridge between the modern windows (`modern_window` + the `gxoled_ui` crate) and the application.
//!
//! Everything here runs on the main (UI) thread, so the state lives in thread-locals. It mirrors the classic
//! `settings_window` one for one: the same control ids map to the same `MenuEvent`s, profiles are read from and
//! written to the same files, and nothing in the classic window is touched. If anything here goes wrong the
//! dispatcher (`settings_ui`) simply falls back to the classic window.
//!
//! Re-entrancy: no `RefCell` borrow is ever held across a call into `modern_window` or across a modal dialog.

use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    sync::mpsc::Sender,
};

use gxoled_lib::Bitmap;
use gxoled_ui::{
    Ui, ids,
    model::{Image, Lang, UiEvent},
};

use super::{
    ClockBurnInMode, Config, ConfigClockStyle, ConfigLanguage, ConfigQuitMode, ConfigShiftMode, ConfigTheme, ConfigTimeMode,
    ConfigUiMode, ConfigUiStyle, GoAnimation, MenuEvent, StatusNotifyMode,
    modern_window::{self, Kind},
    settings_window::{self as classic, ProfileEntry},
};

// ───────────────────────── state ─────────────────────────

#[derive(Clone, PartialEq)]
struct Snapshot {
    checked: Vec<u32>,
    brightness: Option<u8>,
    running: Option<(GoAnimation, Option<String>)>,
    language: ConfigLanguage,
    theme: ConfigTheme,
}

struct EditorCtx {
    animation: GoAnimation,
    target: PathBuf,
}

struct Bridge {
    tx: Sender<MenuEvent>,
    language: ConfigLanguage,
    theme: ConfigTheme,
    snapshot: Option<Snapshot>,
    entries: Vec<ProfileEntry>,
    selected: usize,
    qr: Option<Image>,
    connected: bool,
    oled: Vec<bool>,
    editor: Option<EditorCtx>,
}

thread_local! {
    static BRIDGE: RefCell<Option<Bridge>> = const { RefCell::new(None) };
    /// A modal dialog is running (its nested message loop could otherwise deliver a second click).
    static BUSY: Cell<bool> = const { Cell::new(false) };
}

fn with_bridge<R>(f: impl FnOnce(&mut Bridge) -> R) -> Option<R> {
    BRIDGE.with(|b| b.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

fn send(event: MenuEvent) {
    if let Some(tx) = with_bridge(|b| b.tx.clone()) {
        let _ = tx.send(event);
    }
}

fn lang() -> Lang {
    match with_bridge(|b| b.language) {
        Some(ConfigLanguage::Turkish) => Lang::Tr,
        _ => Lang::En,
    }
}

fn t(en: &str, tr: &str) -> String {
    lang().t(en, tr).to_owned()
}

fn is_dark() -> bool {
    with_bridge(|b| b.theme == ConfigTheme::Dark).unwrap_or(true)
}

// ───────────────────────── config → UI state ─────────────────────────

fn snapshot(c: &Config) -> Snapshot {
    let mut checked = vec![
        match c.ui_mode {
            ConfigUiMode::Tray => ids::MODE_TRAY,
            ConfigUiMode::Window => ids::MODE_WINDOW,
            ConfigUiMode::Hybrid => ids::MODE_HYBRID,
        },
        match c.clock_style {
            ConfigClockStyle::DigitalOriginal => ids::STYLE_DIGITAL,
            ConfigClockStyle::DigitalBold => ids::STYLE_DIGITAL_BOLD,
            ConfigClockStyle::AnalogRound => ids::STYLE_ANALOG_ROUND,
            ConfigClockStyle::AnalogSquare => ids::STYLE_ANALOG_SQUARE,
            ConfigClockStyle::CasioF91W => ids::STYLE_CASIO,
        },
        match c.time_mode {
            ConfigTimeMode::Off => ids::TIME_OFF,
            ConfigTimeMode::H24 => ids::TIME_24,
            ConfigTimeMode::H12 => ids::TIME_12,
        },
        match c.oled_shift {
            ConfigShiftMode::Off => ids::SHIFT_OFF,
            ConfigShiftMode::Low => ids::SHIFT_LOW,
            ConfigShiftMode::High => ids::SHIFT_HIGH,
        },
        match c.clock_burn_in_mode() {
            ClockBurnInMode::Off => ids::BURN_OFF,
            ClockBurnInMode::Wide => ids::BURN_WIDE,
            ClockBurnInMode::NoOverlap => ids::BURN_NO_OVERLAP,
        },
        match c.status_notify {
            StatusNotifyMode::Off => ids::NOTIFY_OFF,
            StatusNotifyMode::Notify => ids::NOTIFY_NOTIFY,
            StatusNotifyMode::Always => ids::NOTIFY_ALWAYS,
            StatusNotifyMode::WhenConnected => ids::NOTIFY_CONNECTED,
            StatusNotifyMode::WhenDisconnected => ids::NOTIFY_DISCONNECTED,
        },
        match c.quit_mode {
            ConfigQuitMode::Screensaver => ids::QUIT_SCREENSAVER,
            ConfigQuitMode::StockUi => ids::QUIT_STOCK_UI,
        },
        ids::VOLUME_HOLD_BASE
            + super::VOLUME_OSD_HOLD_CHOICES_MS
                .iter()
                .position(|ms| *ms == c.volume_osd_hold_ms)
                .unwrap_or(3) as u32,
        match c.language {
            ConfigLanguage::English => ids::LANG_EN,
            ConfigLanguage::Turkish => ids::LANG_TR,
        },
        match c.theme {
            ConfigTheme::Light => ids::THEME_LIGHT,
            ConfigTheme::Dark => ids::THEME_DARK,
        },
        match c.ui_style {
            ConfigUiStyle::Modern => ids::UI_MODERN,
            ConfigUiStyle::Classic => ids::UI_CLASSIC,
        },
    ];
    if c.idle_timeout {
        checked.push(ids::IDLE);
    }
    if c.show_media {
        checked.push(ids::MEDIA);
    }
    if c.startup {
        checked.push(ids::STARTUP);
    }
    checked.sort_unstable();
    Snapshot {
        checked,
        brightness: c.screen_brightness,
        running: c.go_animation.map(|a| (a, c.go_profile_filename.clone())),
        language: c.language,
        theme: c.theme,
    }
}

fn running_index(entries: &[ProfileEntry], running: &Option<(GoAnimation, Option<String>)>) -> Option<usize> {
    let (kind, file) = running.as_ref()?;
    entries.iter().position(|e| e.animation == *kind && e.custom == *file)
}

fn lang_of(l: ConfigLanguage) -> Lang {
    if l == ConfigLanguage::Turkish { Lang::Tr } else { Lang::En }
}

/// Push `snap` into a UI model. `follow_running` moves the list selection to the playing profile.
fn apply_snapshot(ui: &mut Ui, snap: &Snapshot, entries: &[ProfileEntry], follow_running: bool) {
    ui.model.checked = snap.checked.clone();
    ui.model.brightness = snap.brightness;
    ui.model.lang = lang_of(snap.language);
    ui.model.dark = snap.theme == ConfigTheme::Dark;
    let running = running_index(entries, &snap.running);
    ui.model.profile_running = running;
    if follow_running {
        if let Some(i) = running {
            ui.model.profile_selected = i;
        }
    }
    ui.invalidate();
}

fn version_label() -> String {
    format!("v{}", super::display_version())
}

fn qr_image() -> Option<Image> {
    let (w, h, px) = classic::load_embedded_qr()?;
    let mut rgba = Vec::with_capacity(px.len() * 4);
    for p in px {
        rgba.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]);
    }
    Some(Image { w, h, rgba })
}

// ───────────────────────── creation / visibility ─────────────────────────

/// Remember the channel and hook the window events. Creates no window.
pub fn init(tx: Sender<MenuEvent>, config: &Config) {
    BRIDGE.with(|b| {
        *b.borrow_mut() = Some(Bridge {
            tx,
            language: config.language,
            theme: config.theme,
            snapshot: Some(snapshot(config)),
            entries: Vec::new(),
            selected: 0,
            qr: None,
            connected: false,
            oled: vec![false; 128 * 64],
            editor: None,
        });
    });
    modern_window::install(modern_window::Hooks { on_event, on_first_paint });
}

fn marker_path() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.config_dir().join("GxOled").join(".modern-ui-pending"))
}

/// Set just before the first modern window is created and cleared after its first successful paint. If it is
/// still there at the next start, the previous start never got that far and the app falls back to classic.
pub fn arm_crash_guard() {
    if let Some(p) = marker_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, b"modern ui start in progress");
    }
}

pub fn clear_crash_guard() {
    if let Some(p) = marker_path() {
        let _ = std::fs::remove_file(p);
    }
}

/// True (and the marker removed) if the previous modern start did not complete.
pub fn take_crash_guard() -> bool {
    match marker_path() {
        Some(p) if p.exists() => {
            let _ = std::fs::remove_file(p);
            true
        }
        _ => false,
    }
}

fn on_first_paint(kind: Kind) {
    if kind == Kind::Settings {
        clear_crash_guard();
    }
}

fn settings_title() -> String {
    t("GxOled — Settings (Beta)", "GxOled — Ayarlar (Beta)")
}

fn labels(entries: &[ProfileEntry]) -> Vec<String> {
    entries.iter().map(|e| e.label.clone()).collect()
}

fn ensure_settings() -> Result<(), String> {
    if modern_window::exists(Kind::Settings) {
        return Ok(());
    }
    arm_crash_guard();
    let built = with_bridge(|b| {
        b.entries = classic::profile_entries();
        if b.qr.is_none() {
            b.qr = qr_image();
        }
        let mut ui = Ui::new();
        ui.model.version = version_label();
        ui.model.qr = b.qr.clone();
        ui.model.connected = b.connected;
        ui.model.profiles = labels(&b.entries);
        ui.model.profile_selected = b.selected.min(b.entries.len().saturating_sub(1));
        ui.model.set_oled_bitmap(b.oled.iter().copied());
        if let Some(s) = b.snapshot.clone() {
            apply_snapshot(&mut ui, &s, &b.entries, true);
        }
        (ui, b.theme == ConfigTheme::Dark)
    });
    let Some((ui, dark)) = built else {
        clear_crash_guard();
        return Err("modern UI bridge is not initialised".to_owned());
    };
    let result = modern_window::create(Kind::Settings, &settings_title(), dark, ui);
    if result.is_err() {
        clear_crash_guard();
    }
    result
}

pub fn open() -> Result<(), String> {
    ensure_settings()?;
    modern_window::show(Kind::Settings);
    Ok(())
}

pub fn hide() {
    modern_window::hide(Kind::Settings);
    modern_window::hide(Kind::Editor);
}

pub fn is_visible() -> bool {
    modern_window::is_visible(Kind::Settings) || modern_window::is_visible(Kind::Editor)
}

pub fn failed() -> bool {
    modern_window::failed()
}

// ───────────────────────── updates from the application ─────────────────────────

pub fn sync(config: &Config) {
    let snap = snapshot(config);
    let changed = with_bridge(|b| {
        if b.snapshot.as_ref() == Some(&snap) {
            None
        } else {
            let running_changed = b.snapshot.as_ref().map(|s| &s.running) != Some(&snap.running);
            b.snapshot = Some(snap.clone());
            Some((b.entries.clone(), running_changed))
        }
    })
    .flatten();
    let Some((entries, running_changed)) = changed else { return };
    modern_window::with_ui(Kind::Settings, |ui| apply_snapshot(ui, &snap, &entries, running_changed));
    modern_window::with_ui(Kind::Editor, |ui| {
        ui.model.lang = lang_of(snap.language);
        ui.model.dark = snap.theme == ConfigTheme::Dark;
        ui.invalidate();
    });
}

pub fn localize(language: ConfigLanguage) {
    with_bridge(|b| b.language = language);
    modern_window::set_title(Kind::Settings, &settings_title());
    let title = editor_title();
    modern_window::set_title(Kind::Editor, &title);
    for kind in [Kind::Settings, Kind::Editor] {
        modern_window::with_ui(kind, |ui| {
            ui.model.lang = lang_of(language);
            ui.invalidate();
        });
    }
}

pub fn set_theme(theme: ConfigTheme) {
    with_bridge(|b| b.theme = theme);
    let dark = theme == ConfigTheme::Dark;
    for kind in [Kind::Settings, Kind::Editor] {
        modern_window::with_ui(kind, |ui| {
            ui.model.dark = dark;
            ui.invalidate();
        });
        modern_window::apply_theme(kind, dark);
    }
}

pub fn set_connected(connected: bool) {
    with_bridge(|b| b.connected = connected);
    modern_window::with_ui(Kind::Settings, |ui| {
        ui.model.connected = connected;
        ui.invalidate();
    });
}

pub fn preview(bitmap: &Bitmap) {
    if bitmap.w != 128 || bitmap.h != 64 {
        return;
    }
    with_bridge(|b| {
        for (dst, src) in b.oled.iter_mut().zip(bitmap.data.iter()) {
            *dst = src;
        }
    });
    if !modern_window::is_visible(Kind::Settings) {
        return;
    }
    modern_window::with_ui(Kind::Settings, |ui| ui.model.set_oled_bitmap(bitmap.data.iter()));
    modern_window::present_oled(Kind::Settings);
}

// ───────────────────────── events from the windows ─────────────────────────

fn on_event(kind: Kind, event: UiEvent) {
    if BUSY.with(|b| b.get()) {
        return;
    }
    match (kind, event) {
        (Kind::Settings, UiEvent::Brightness(v)) => send(MenuEvent::SetBrightness(v)),
        (Kind::Settings, UiEvent::ProfileSelected(row)) => {
            with_bridge(|b| b.selected = row);
        }
        (Kind::Settings, UiEvent::Action(id)) => settings_action(id),
        (Kind::Editor, UiEvent::Action(id)) => editor_action(id),
        _ => {}
    }
}

/// The classic window's control-id → event table, verbatim (the modern UI reuses the ids).
fn menu_event_for(id: u32) -> Option<MenuEvent> {
    Some(match id {
        ids::MODE_TRAY => MenuEvent::SetUiMode(ConfigUiMode::Tray),
        ids::MODE_WINDOW => MenuEvent::SetUiMode(ConfigUiMode::Window),
        ids::MODE_HYBRID => MenuEvent::SetUiMode(ConfigUiMode::Hybrid),
        ids::STYLE_DIGITAL => MenuEvent::SetClockStyle(ConfigClockStyle::DigitalOriginal),
        ids::STYLE_DIGITAL_BOLD => MenuEvent::SetClockStyle(ConfigClockStyle::DigitalBold),
        ids::STYLE_ANALOG_ROUND => MenuEvent::SetClockStyle(ConfigClockStyle::AnalogRound),
        ids::STYLE_ANALOG_SQUARE => MenuEvent::SetClockStyle(ConfigClockStyle::AnalogSquare),
        ids::STYLE_CASIO => MenuEvent::SetClockStyle(ConfigClockStyle::CasioF91W),
        ids::TIME_OFF => MenuEvent::SetTimeMode(ConfigTimeMode::Off),
        ids::TIME_24 => MenuEvent::SetTimeMode(ConfigTimeMode::H24),
        ids::TIME_12 => MenuEvent::SetTimeMode(ConfigTimeMode::H12),
        ids::LANG_EN => MenuEvent::SetLanguage(ConfigLanguage::English),
        ids::LANG_TR => MenuEvent::SetLanguage(ConfigLanguage::Turkish),
        ids::THEME_LIGHT => MenuEvent::SetTheme(ConfigTheme::Light),
        ids::THEME_DARK => MenuEvent::SetTheme(ConfigTheme::Dark),
        ids::UI_MODERN => MenuEvent::SetUiStyle(ConfigUiStyle::Modern),
        ids::UI_CLASSIC => MenuEvent::SetUiStyle(ConfigUiStyle::Classic),
        ids::QUIT => MenuEvent::Quit,
        ids::STARTUP => MenuEvent::ToggleStartupFromWindow,
        ids::GIF_ADD => MenuEvent::OpenGifPickerFromWindow,
        ids::GIF_TOGGLE => MenuEvent::ToggleGifFromWindow,
        ids::ANIM_STOP => MenuEvent::SetGoAnimation(None),
        ids::SHIFT_OFF => MenuEvent::SetShiftMode(ConfigShiftMode::Off),
        ids::SHIFT_LOW => MenuEvent::SetShiftMode(ConfigShiftMode::Low),
        ids::SHIFT_HIGH => MenuEvent::SetShiftMode(ConfigShiftMode::High),
        ids::BURN_OFF => MenuEvent::SetClockBurnInMode(ClockBurnInMode::Off),
        ids::BURN_WIDE => MenuEvent::SetClockBurnInMode(ClockBurnInMode::Wide),
        ids::BURN_NO_OVERLAP => MenuEvent::SetClockBurnInMode(ClockBurnInMode::NoOverlap),
        ids::NOTIFY_OFF => MenuEvent::SetStatusNotifyMode(StatusNotifyMode::Off),
        ids::NOTIFY_NOTIFY => MenuEvent::SetStatusNotifyMode(StatusNotifyMode::Notify),
        ids::NOTIFY_ALWAYS => MenuEvent::SetStatusNotifyMode(StatusNotifyMode::Always),
        ids::NOTIFY_CONNECTED => MenuEvent::SetStatusNotifyMode(StatusNotifyMode::WhenConnected),
        ids::NOTIFY_DISCONNECTED => MenuEvent::SetStatusNotifyMode(StatusNotifyMode::WhenDisconnected),
        ids::QUIT_SCREENSAVER => MenuEvent::SetQuitMode(ConfigQuitMode::Screensaver),
        ids::QUIT_STOCK_UI => MenuEvent::SetQuitMode(ConfigQuitMode::StockUi),
        ids::IDLE => MenuEvent::ToggleIdleFromWindow,
        ids::MEDIA => MenuEvent::ToggleMediaFromWindow,
        id if (ids::VOLUME_HOLD_BASE..ids::VOLUME_HOLD_BASE + super::VOLUME_OSD_HOLD_CHOICES_MS.len() as u32).contains(&id) => {
            MenuEvent::SetVolumeOsdHold(super::VOLUME_OSD_HOLD_CHOICES_MS[(id - ids::VOLUME_HOLD_BASE) as usize])
        }
        _ => return None,
    })
}

fn settings_action(id: u32) {
    if let Some(event) = menu_event_for(id) {
        send(event);
        return;
    }
    match id {
        ids::GITHUB => classic::open_github(),
        ids::KOFI => classic::open_kofi(),
        ids::PROFILE_SELECT => profile_use(),
        ids::PROFILE_EDIT => profile_edit(),
        ids::PROFILE_NEW => profile_new(),
        ids::PROFILE_IMPORT => profile_import(),
        ids::PROFILE_EXPORT => profile_export(),
        ids::PROFILE_DELETE => profile_delete(),
        _ => {}
    }
}

// ───────────────────────── modal dialogs ─────────────────────────

/// Run a blocking dialog with the owning window disabled and further clicks ignored.
fn modal<R>(kind: Kind, f: impl FnOnce() -> R) -> R {
    BUSY.with(|b| b.set(true));
    modern_window::set_enabled(kind, false);
    let r = f();
    modern_window::set_enabled(kind, true);
    BUSY.with(|b| b.set(false));
    modern_window::show(kind);
    r
}

fn pick_json(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).add_filter("JSON profile", &["json"]).pick_file()
}

fn save_json(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("JSON profile", &["json"])
        .set_file_name("gxoled-profile.json")
        .save_file()
}

// ───────────────────────── profile list ─────────────────────────

fn selected_entry() -> Option<ProfileEntry> {
    let row = modern_window::with_ui(Kind::Settings, |ui| ui.model.profile_selected)?;
    with_bridge(|b| b.entries.get(row).cloned()).flatten()
}

/// Reload the profile files and select `select` (a custom file name) or keep the current row.
fn refresh_profiles(select: Option<String>) {
    let entries = classic::profile_entries();
    let current = modern_window::with_ui(Kind::Settings, |ui| ui.model.profile_selected);
    let (running, remembered) = with_bridge(|b| {
        b.entries = entries.clone();
        (b.snapshot.as_ref().and_then(|s| running_index(&entries, &s.running)), b.selected)
    })
    .unwrap_or((None, 0));
    let wanted = select
        .and_then(|name| entries.iter().position(|e| e.custom.as_deref() == Some(name.as_str())))
        .unwrap_or_else(|| current.unwrap_or(remembered).min(entries.len().saturating_sub(1)));
    with_bridge(|b| b.selected = wanted);
    modern_window::with_ui(Kind::Settings, |ui| {
        ui.model.profiles = labels(&entries);
        ui.model.profile_selected = wanted;
        ui.model.profile_running = running;
        ui.invalidate();
    });
}

fn profile_use() {
    if let Some(entry) = selected_entry() {
        send(MenuEvent::SetGoProfile(entry.animation, entry.custom));
    }
}

fn profile_edit() {
    let Some(entry) = selected_entry() else { return };
    let Some(target) = classic::selected_profile_path(&entry) else { return };
    let content = std::fs::read_to_string(&target).unwrap_or_else(|_| classic::profile_default(entry.animation).to_owned());
    open_editor(entry, target, classic::pretty_json(&content));
}

fn profile_new() {
    let Some(entry) = selected_entry() else { return };
    let Some((name, target)) = classic::unique_profile_name(entry.animation) else {
        classic::profile_error("New Profile", &t("Unable to allocate a custom profile filename", "Özel profil dosya adı ayrılamadı"));
        return;
    };
    // Only a proposed file name: nothing is created until the user saves a valid 128×64 profile.
    let fresh = ProfileEntry { label: format!("{name} — New Profile"), animation: entry.animation, custom: Some(name) };
    open_editor(fresh, target, "{\n\n}".to_owned());
}

fn profile_import() {
    let Some(path) = modal(Kind::Settings, || pick_json(&t("Import profile", "Profil içe aktar"))) else { return };
    match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|text| classic::create_profile_from_text(&text)) {
        Ok((_, name)) => refresh_profiles(Some(name)),
        Err(err) => classic::profile_error(&t("Import failed", "İçe aktarma başarısız"), &err),
    }
}

fn profile_export() {
    let Some(entry) = selected_entry() else { return };
    let text = classic::selected_profile_path(&entry)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_else(|| classic::profile_default(entry.animation).to_owned());
    let Some(path) = modal(Kind::Settings, || save_json(&t("Export profile", "Profili dışa aktar"))) else { return };
    if let Err(err) = std::fs::write(path, classic::pretty_json(&text)) {
        classic::profile_error(&t("Export failed", "Dışa aktarma başarısız"), &err.to_string());
    }
}

fn profile_delete() {
    let Some(entry) = selected_entry() else { return };
    if entry.custom.is_none() {
        classic::profile_error("GxOled", &t("Built-in profiles cannot be deleted", "Yerleşik profiller silinemez"));
        return;
    }
    if let Some(path) = classic::selected_profile_path(&entry) {
        if let Err(err) = std::fs::remove_file(path) {
            classic::profile_error("GxOled", &err.to_string());
            return;
        }
    }
    let was_running = with_bridge(|b| {
        b.snapshot
            .as_ref()
            .and_then(|s| s.running.as_ref())
            .is_some_and(|(kind, file)| *kind == entry.animation && *file == entry.custom)
    })
    .unwrap_or(false);
    if was_running {
        send(MenuEvent::SetGoAnimation(None));
    }
    refresh_profiles(None);
}

// ───────────────────────── editor ─────────────────────────

fn editor_title() -> String {
    "GxOled — Advanced Animation Editor".to_owned()
}

fn set_status(message: String, ok: bool) {
    modern_window::with_ui(Kind::Editor, |ui| {
        ui.model.editor_status = message;
        ui.model.editor_status_ok = ok;
        ui.invalidate();
    });
}

fn open_editor(entry: ProfileEntry, target: PathBuf, content: String) {
    let (dark, snap) = with_bridge(|b| (b.theme == ConfigTheme::Dark, b.snapshot.clone())).unwrap_or((true, None));
    if !modern_window::exists(Kind::Editor) {
        let mut ui = Ui::new();
        if let Some(s) = &snap {
            ui.model.lang = lang_of(s.language);
        }
        ui.model.dark = dark;
        if let Err(err) = modern_window::create(Kind::Editor, &editor_title(), dark, ui) {
            eprintln!("GxOled modern editor failed: {err}; using the classic editor");
            classic::open_profile_editor_with(entry, target, content);
            return;
        }
    }
    with_bridge(|b| b.editor = Some(EditorCtx { animation: entry.animation, target }));
    modern_window::with_ui(Kind::Editor, |ui| {
        ui.model.editor_title = entry.label.clone();
        ui.model.editor_status.clear();
        ui.model.editor_status_ok = true;
        ui.invalidate();
    });
    modern_window::set_title(Kind::Editor, &format!("GxOled — {}", entry.label));
    modern_window::editor_set_text(&content);
    modern_window::show(Kind::Editor);
    modern_window::editor_focus();
}

fn editor_action(id: u32) {
    match id {
        ids::EDITOR_SAVE => editor_apply(false),
        ids::EDITOR_RESET => editor_apply(true),
        ids::EDITOR_FORMAT => editor_format(),
        ids::EDITOR_IMPORT => editor_import(),
        ids::EDITOR_EXPORT => editor_export(),
        _ => {}
    }
}

/// Current text, validated as a profile for the editor's animation.
fn editor_valid_text() -> Result<(String, GoAnimation), String> {
    let text = modern_window::editor_text().ok_or_else(|| t("The editor is not available", "Düzenleyici kullanılamıyor"))?;
    if text.len() > 65_536 {
        return Err(t("Profile must be smaller than 64 KiB", "Profil 64 KiB'den küçük olmalı"));
    }
    let kind = classic::valid_profile_text(&text)?;
    Ok((text, kind))
}

fn editor_animation() -> Option<GoAnimation> {
    with_bridge(|b| b.editor.as_ref().map(|e| e.animation)).flatten()
}

fn editor_apply(reset: bool) {
    let Some((animation, target)) = with_bridge(|b| b.editor.as_ref().map(|e| (e.animation, e.target.clone()))).flatten() else {
        return;
    };
    let content = if reset {
        classic::profile_default(animation).to_owned()
    } else {
        match editor_valid_text() {
            Ok((text, kind)) if kind == animation => text,
            Ok(_) => {
                set_status(
                    t(
                        "This editor's widget type cannot be changed. Import a new preset instead.",
                        "Bu düzenleyicinin widget türü değiştirilemez. Bunun yerine yeni bir ön ayar içe aktarın.",
                    ),
                    false,
                );
                return;
            }
            Err(err) => {
                set_status(err, false);
                return;
            }
        }
    };
    let in_custom = classic::custom_directory().is_some_and(|d| target.starts_with(d));
    let written = if reset && !in_custom {
        // A built-in profile resets by removing the user's override.
        match std::fs::remove_file(&target) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    } else {
        target
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(&target, content.as_bytes()))
    };
    if let Err(err) = written {
        set_status(err.to_string(), false);
        return;
    }
    if reset {
        modern_window::editor_set_text(&classic::pretty_json(&content));
    }
    let name = if in_custom { target.file_name().and_then(|x| x.to_str()).map(str::to_owned) } else { None };
    send(MenuEvent::SetGoProfile(animation, name.clone()));
    refresh_profiles(name);
    set_status(
        if reset { t("Reset to the starter template and applied.", "Başlangıç şablonuna döndürüldü ve uygulandı.") } else { t("Saved and applied.", "Kaydedildi ve uygulandı.") },
        true,
    );
}

fn editor_format() {
    match editor_valid_text() {
        Ok((text, _)) => match serde_json::from_str::<serde_json::Value>(&text).and_then(|v| serde_json::to_string_pretty(&v)) {
            Ok(pretty) => {
                modern_window::editor_set_text(&pretty);
                set_status(t("Valid profile — formatted.", "Geçerli profil — biçimlendirildi."), true);
            }
            Err(err) => set_status(err.to_string(), false),
        },
        Err(err) => set_status(err, false),
    }
}

fn editor_import() {
    let Some(animation) = editor_animation() else { return };
    let Some(path) = modal(Kind::Editor, || pick_json(&t("Import JSON", "JSON içe aktar"))) else { return };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => return set_status(err.to_string(), false),
    };
    match classic::valid_profile_text(&text) {
        Ok(kind) if kind == animation => {
            modern_window::editor_set_text(&classic::pretty_json(&text));
            set_status(t("Imported — press Save & Apply to keep it.", "İçe aktarıldı — saklamak için Kaydet ve Uygula'ya basın."), true);
        }
        Ok(_) => set_status(
            t(
                "This JSON contains a different animation. Use Import on the Animations page instead.",
                "Bu JSON farklı bir animasyon içeriyor. Bunun yerine Animasyonlar sayfasındaki İçe aktar'ı kullanın.",
            ),
            false,
        ),
        Err(err) => set_status(err, false),
    }
}

fn editor_export() {
    let (text, _) = match editor_valid_text() {
        Ok(v) => v,
        Err(err) => return set_status(err, false),
    };
    let Some(dest) = modal(Kind::Editor, || save_json(&t("Export JSON", "JSON dışa aktar"))) else { return };
    match std::fs::write(dest, classic::pretty_json(&text)) {
        Ok(()) => set_status(t("Exported.", "Dışa aktarıldı."), true),
        Err(err) => set_status(err.to_string(), false),
    }
}

// ───────────────────────── tests ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_id_of_the_ui_is_mapped_or_handled_locally() {
        // Ids the dispatcher in `settings_action` / `editor_action` handles itself.
        let local = [
            ids::GITHUB,
            ids::KOFI,
            ids::PROFILE_SELECT,
            ids::PROFILE_EDIT,
            ids::PROFILE_NEW,
            ids::PROFILE_IMPORT,
            ids::PROFILE_EXPORT,
            ids::PROFILE_DELETE,
            ids::EDITOR_SAVE,
            ids::EDITOR_RESET,
            ids::EDITOR_IMPORT,
            ids::EDITOR_EXPORT,
            ids::EDITOR_FORMAT,
        ];
        let mapped = [
            ids::MODE_TRAY, ids::MODE_WINDOW, ids::MODE_HYBRID, ids::STYLE_DIGITAL, ids::STYLE_DIGITAL_BOLD,
            ids::STYLE_ANALOG_ROUND, ids::STYLE_ANALOG_SQUARE, ids::STYLE_CASIO, ids::TIME_OFF, ids::TIME_24, ids::TIME_12,
            ids::LANG_EN, ids::LANG_TR, ids::THEME_LIGHT, ids::THEME_DARK, ids::UI_MODERN, ids::UI_CLASSIC, ids::QUIT,
            ids::STARTUP, ids::GIF_ADD, ids::GIF_TOGGLE, ids::ANIM_STOP, ids::SHIFT_OFF, ids::SHIFT_LOW, ids::SHIFT_HIGH,
            ids::BURN_OFF, ids::BURN_WIDE, ids::BURN_NO_OVERLAP, ids::NOTIFY_OFF, ids::NOTIFY_NOTIFY, ids::NOTIFY_ALWAYS,
            ids::NOTIFY_CONNECTED, ids::NOTIFY_DISCONNECTED, ids::QUIT_SCREENSAVER, ids::QUIT_STOCK_UI, ids::IDLE, ids::MEDIA,
        ];
        for id in mapped {
            assert!(menu_event_for(id).is_some(), "id {id} has no MenuEvent");
            assert!(!local.contains(&id), "id {id} listed twice");
        }
        for i in 0..super::super::VOLUME_OSD_HOLD_CHOICES_MS.len() as u32 {
            assert!(menu_event_for(ids::VOLUME_HOLD_BASE + i).is_some());
        }
        assert!(menu_event_for(ids::VOLUME_HOLD_BASE + super::super::VOLUME_OSD_HOLD_CHOICES_MS.len() as u32).is_none());
        for id in local {
            assert!(menu_event_for(id).is_none(), "id {id} must be handled locally");
        }
    }

    #[test]
    fn volume_hold_choices_match_the_ui() {
        assert_eq!(gxoled_ui::pages::VOLUME_HOLD_MS, super::super::VOLUME_OSD_HOLD_CHOICES_MS);
        assert_eq!(gxoled_ui::pages::VOLUME_HOLD_DEFAULT_MS, super::super::DEFAULT_VOLUME_OSD_HOLD_MS);
    }

    #[test]
    fn snapshot_reflects_config() {
        let mut c = Config::default();
        c.ui_style = ConfigUiStyle::Classic;
        c.screen_brightness = Some(7);
        let s = snapshot(&c);
        assert!(s.checked.contains(&ids::UI_CLASSIC));
        assert!(!s.checked.contains(&ids::UI_MODERN));
        assert!(s.checked.contains(&(ids::VOLUME_HOLD_BASE + 3)), "2.5 s is the default");
        assert_eq!(s.brightness, Some(7));
        let mut sorted = s.checked.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, s.checked);
    }

    #[test]
    fn version_label_matches_the_package_version() {
        let label = version_label();
        assert!(label.starts_with('v'));
        if env!("CARGO_PKG_VERSION").ends_with("-beta") {
            assert!(label.ends_with(" Beta"), "{label}");
            assert!(!label.contains('-'));
        } else {
            assert_eq!(label, format!("v{}", env!("CARGO_PKG_VERSION")));
        }
    }
}
