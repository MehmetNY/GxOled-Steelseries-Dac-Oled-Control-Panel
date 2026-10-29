#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod os;
#[cfg(target_os = "windows")]
mod settings_window;
// Beta (v0.5.3): modern settings windows. `settings_window` (classic) stays the default-safe fallback.
#[cfg(target_os = "windows")]
mod modern_window;
#[cfg(target_os = "windows")]
mod modern_ui;
#[cfg(target_os = "windows")]
mod settings_ui;
#[cfg(target_os = "windows")]
mod go_bridge;

use chrono::{DateTime, Local, TimeDelta, Timelike};
use gxoled_draw::{ClockStyle as DrawClockStyle, DrawDevice, DrawEvent, DrawLayer, LayerId, ShiftMode, TextRenderer, bitmap_from_memory, try_decode_gif_frames};
use gxoled_lib::{Bitmap, Device};
use os::{Media, OSFeatures, OSImpl};
use rfd::{MessageDialog, MessageLevel};
use sdl3_sys::everything as sdl;
use serde::{Deserialize, Serialize};
use std::{
    ffi::CStr,
    io::Write,
    os::raw::c_void,
    path::PathBuf,
    sync::{Arc, mpsc},
    thread::sleep,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const NOTIF_DUR: Duration = Duration::from_secs(5);

/// Times (ms) the base station's own volume screen can stay up after the knob stops moving,
/// before GxOled takes the OLED back. The first entry that equals the default is the factory value.
const VOLUME_OSD_HOLD_CHOICES_MS: [u32; 7] = [1000, 1500, 2000, 2500, 3000, 4000, 5000];
const DEFAULT_VOLUME_OSD_HOLD_MS: u32 = 2500;

/// Snap a stored value to the nearest selectable one (a hand-edited config must not produce a
/// time the UI cannot show); the lower choice wins a tie.
fn normalize_volume_osd_hold_ms(ms: u32) -> u32 {
    VOLUME_OSD_HOLD_CHOICES_MS
        .iter()
        .copied()
        .min_by_key(|choice| (choice.abs_diff(ms), *choice))
        .unwrap_or(DEFAULT_VOLUME_OSD_HOLD_MS)
}

/// Menu/window caption of one choice, e.g. `2.5 s (default)` / `2,5 sn (varsayılan)`.
#[cfg(target_os = "windows")]
fn volume_osd_hold_label(ms: u32, turkish: bool) -> String {
    let seconds = if ms % 1000 == 0 { format!("{}", ms / 1000) } else { format!("{}.{}", ms / 1000, (ms % 1000) / 100) };
    let seconds = if turkish { seconds.replace('.', ",") } else { seconds };
    let unit = if turkish { "sn" } else { "s" };
    let default = if ms == DEFAULT_VOLUME_OSD_HOLD_MS { if turkish { " (varsayılan)" } else { " (default)" } } else { "" };
    format!("{seconds} {unit}{default}")
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq)]
enum ConfigTimeMode {
    Off,
    #[default]
    H24,
    H12,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
enum ConfigClockStyle {
    #[default]
    DigitalOriginal,
    DigitalBold,
    AnalogRound,
    AnalogSquare,
    CasioF91W,
}
impl ConfigClockStyle {
    fn to_draw(self) -> DrawClockStyle {
        match self {
            Self::DigitalOriginal => DrawClockStyle::DigitalOriginal,
            Self::DigitalBold => DrawClockStyle::DigitalBold,
            Self::AnalogRound => DrawClockStyle::AnalogRound,
            Self::AnalogSquare => DrawClockStyle::AnalogSquare,
            Self::CasioF91W => DrawClockStyle::CasioF91W,
        }
    }
    fn is_analog(self) -> bool {
        matches!(self, Self::AnalogRound | Self::AnalogSquare)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
enum ConfigLanguage { #[default] English, Turkish }

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
enum ConfigTheme { Light, #[default] Dark }

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
enum MediaMode { ClockAndMedia, #[default] MediaOnly }

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
enum ConfigUiMode {
    #[default]
    Tray,
    Window,
    Hybrid,
}

/// Beta: which settings window the tray / settings button opens. Both always exist in the binary; only this
/// setting (and the automatic fallbacks in `settings_ui`) decide, so going back is one click.
#[cfg(target_os = "windows")]
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
enum ConfigUiStyle {
    #[default]
    Modern,
    Classic,
}

/// "0.5.3-beta" -> "0.5.3 Beta", "0.5.3-beta.2" -> "0.5.3 Beta 2" (for labels); other versions are shown unchanged.
fn display_version_of(version: &str) -> String {
    match version.split_once("-beta") {
        Some((base, "")) => format!("{base} Beta"),
        Some((base, rest)) => format!("{base} Beta {}", rest.trim_start_matches('.')),
        None => version.to_owned(),
    }
}

fn display_version() -> String {
    display_version_of(env!("CARGO_PKG_VERSION"))
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
enum GoAnimation { Matrix, Hyperspace, StarWars, GameOfLife }
impl GoAnimation {
    #[cfg(target_os = "windows")]
    fn arg(self) -> &'static str {
        match self {
            Self::Matrix => "matrix", Self::Hyperspace => "hyperspace",
            Self::StarWars => "starwars", Self::GameOfLife => "life",
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq)]
enum ConfigShiftMode {
    Off,
    Low,
    #[default]
    High,
}
impl ConfigShiftMode {
    fn to_api(self) -> ShiftMode {
        match self {
            ConfigShiftMode::Off => ShiftMode::Off,
            ConfigShiftMode::Low => ShiftMode::Low,
            ConfigShiftMode::High => ShiftMode::High,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClockBurnInMode {
    Off,
    Wide,
    NoOverlap,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq)]
enum ConfigQuitMode {
    #[default]
    Screensaver,
    StockUi,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
enum StatusNotifyMode {
    Off,
    #[default]
    Notify,
    Always,
    WhenConnected,
    WhenDisconnected,
}

#[derive(Serialize, Deserialize, Default)]
struct ConfigFont {
    path: PathBuf,
    size: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Config {
    font: Option<ConfigFont>,
    time_mode: ConfigTimeMode,
    clock_style: ConfigClockStyle,
    ui_mode: ConfigUiMode,
    // Beta: modern or classic settings window (older config files simply lack this key).
    #[cfg(target_os = "windows")]
    ui_style: ConfigUiStyle,
    language: ConfigLanguage,
    theme: ConfigTheme,
    go_animation: Option<GoAnimation>,
    // Filename only: a user-created 128x64 profile in profiles/custom.
    go_profile_filename: Option<String>,
    show_media: bool,
    media_mode: MediaMode,
    idle_timeout: bool,
    oled_shift: ConfigShiftMode,
    status_notify: StatusNotifyMode,
    quit_mode: ConfigQuitMode,
    // `clock_shift` is kept for compatibility with existing config files.
    // When true without `clock_shift_no_overlap`, it selects the wider movement mode.
    clock_shift: bool,
    clock_shift_no_overlap: bool,
    // Last brightness level selected from the UI. `None` means the app has not changed it.
    screen_brightness: Option<u8>,
    #[cfg(target_os = "windows")]
    startup: bool,
    // Remember the last selected animation and whether it was playing.
    animation_path: Option<PathBuf>,
    animation_playing: bool,
    // How long the base station's own volume screen stays up after the knob stops (ms; one of
    // `VOLUME_OSD_HOLD_CHOICES_MS`).
    volume_osd_hold_ms: u32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            font: None,
            time_mode: ConfigTimeMode::default(),
            clock_style: ConfigClockStyle::default(),
            ui_mode: ConfigUiMode::default(),
            #[cfg(target_os = "windows")]
            ui_style: ConfigUiStyle::default(),
            language: ConfigLanguage::default(),
            theme: ConfigTheme::default(),
            go_animation: None,
            go_profile_filename: None,
            show_media: true,
            media_mode: MediaMode::default(),
            idle_timeout: true,
            oled_shift: ConfigShiftMode::default(),
            status_notify: StatusNotifyMode::default(),
            quit_mode: ConfigQuitMode::default(),
            clock_shift: false,
            clock_shift_no_overlap: false,
            screen_brightness: None,
            #[cfg(target_os = "windows")]
            startup: false,
            animation_path: None,
            animation_playing: false,
            volume_osd_hold_ms: DEFAULT_VOLUME_OSD_HOLD_MS,
        }
    }
}
impl Config {
    fn clock_burn_in_mode(&self) -> ClockBurnInMode {
        if self.clock_shift_no_overlap {
            ClockBurnInMode::NoOverlap
        } else if self.clock_shift {
            ClockBurnInMode::Wide
        } else {
            ClockBurnInMode::Off
        }
    }

    fn path() -> anyhow::Result<PathBuf> {
        let dirs = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Could not determine the user configuration directory"))?;
        Ok(dirs.config_dir().join("GxOled").join("config.toml"))
    }

    // One-time migration path for settings created by the upstream-branded desktop app.
    // Keep reading this legacy file so renaming the application doesn't reset user settings.
    fn legacy_path() -> anyhow::Result<PathBuf> {
        let dirs = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Could not determine the user configuration directory"))?;
        Ok(dirs.config_dir().join("ggoled_app.toml"))
    }

    fn parse_config(text: &str) -> Option<Config> {
        let mut conf = toml::from_str::<Config>(text).ok()?;
        // Ignore invalid manual edits rather than issuing unsupported hardware commands.
        if conf.screen_brightness.is_some_and(|value| !(1..=10).contains(&value)) {
            conf.screen_brightness = None;
        }
        conf.volume_osd_hold_ms = normalize_volume_osd_hold_ms(conf.volume_osd_hold_ms);
        // Old versions stored ClockAndMedia. Migrate that broken layout to the
        // exclusive media screen while keeping the legacy enum variant readable.
        conf.media_mode = MediaMode::MediaOnly;
        Some(conf)
    }

    /// Save through a temporary file so an interrupted write does not truncate the user's
    /// existing settings file.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string(self)?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("config.toml");
        let temp_path = path.with_file_name(format!(".{file_name}.{}.{}.tmp", std::process::id(), nonce));

        let result = (|| -> anyhow::Result<()> {
            let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp_path)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temp_path, &path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp_path);
        }
        result
    }

    pub fn load() -> Config {
        if let Ok(path) = Self::path() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Some(config) = Self::parse_config(&text) {
                    return config;
                }
            }
        }

        // If this is the first GxOled launch, migrate the previous configuration file
        // into the new product-specific directory. The old file is left untouched as a
        // backup; future saves go only to the new location.
        if let Ok(legacy_path) = Self::legacy_path() {
            if let Ok(text) = std::fs::read_to_string(&legacy_path) {
                if let Some(config) = Self::parse_config(&text) {
                    if let Err(error) = config.save() {
                        eprintln!("Could not migrate legacy GxOled settings: {error}");
                    }
                    return config;
                }
            }
        }

        Config::default()
    }
}

#[cfg(target_os = "windows")]
const STARTUP_REG_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const STARTUP_REG_NAME: &str = "GxOled";
#[cfg(target_os = "windows")]
const LEGACY_STARTUP_REG_NAME: &str = "ggoled_app";

#[cfg(target_os = "windows")]
fn registry_startup_value_exists(value_label: &str) -> anyhow::Result<bool> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_QUERY_VALUE, REG_SZ,
    };

    let subkey: Vec<u16> = std::ffi::OsStr::new(STARTUP_REG_PATH)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let value_name: Vec<u16> = std::ffi::OsStr::new(value_label)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut key = std::ptr::null_mut();
    let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_QUERY_VALUE, &mut key) };
    if status == 2 {
        return Ok(false);
    }
    if status != 0 {
        anyhow::bail!("Could not read Windows startup setting (error {})", status);
    }

    let mut value_type = 0u32;
    let mut byte_len = 0u32;
    let status = unsafe {
        RegQueryValueExW(
            key,
            value_name.as_ptr(),
            std::ptr::null(),
            &mut value_type,
            std::ptr::null_mut(),
            &mut byte_len,
        )
    };
    if status == 2 {
        unsafe { RegCloseKey(key) };
        return Ok(false);
    }
    if status != 0 || value_type != REG_SZ {
        unsafe { RegCloseKey(key) };
        return Ok(false);
    }

    // Registry data is untrusted input. Bound the allocation and reject malformed UTF-16 sizes.
    if byte_len > 64 * 1024 || byte_len % 2 != 0 {
        unsafe { RegCloseKey(key) };
        return Ok(false);
    }

    let mut buffer = vec![0u8; byte_len as usize];
    let status = unsafe {
        RegQueryValueExW(
            key,
            value_name.as_ptr(),
            std::ptr::null(),
            &mut value_type,
            buffer.as_mut_ptr(),
            &mut byte_len,
        )
    };
    unsafe { RegCloseKey(key) };
    if status != 0 || byte_len as usize > buffer.len() || byte_len % 2 != 0 {
        return Ok(false);
    }

    let value = String::from_utf16_lossy(
        &buffer[..byte_len as usize]
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    );
    Ok(!value.trim_matches('\0').trim().is_empty())
}

#[cfg(target_os = "windows")]
fn windows_startup_enabled() -> anyhow::Result<bool> {
    registry_startup_value_exists(STARTUP_REG_NAME)
}

#[cfg(target_os = "windows")]
fn remove_legacy_startup_value() -> anyhow::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, HKEY_CURRENT_USER, KEY_SET_VALUE,
    };

    let subkey: Vec<u16> = std::ffi::OsStr::new(STARTUP_REG_PATH)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let value_name: Vec<u16> = std::ffi::OsStr::new(LEGACY_STARTUP_REG_NAME)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut key = std::ptr::null_mut();
    let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_SET_VALUE, &mut key) };
    if status == 2 {
        return Ok(());
    }
    if status != 0 {
        anyhow::bail!("Could not open Windows startup key for migration (error {})", status);
    }
    let status = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };
    unsafe { RegCloseKey(key) };
    if status != 0 && status != 2 {
        anyhow::bail!("Could not remove legacy startup entry (error {})", status);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn migrate_legacy_windows_startup() -> anyhow::Result<bool> {
    let current_enabled = windows_startup_enabled()?;
    let legacy_enabled = registry_startup_value_exists(LEGACY_STARTUP_REG_NAME)?;
    if legacy_enabled {
        // Preserve the user's previous startup preference but point it to the new executable.
        if !current_enabled {
            set_windows_startup(true)?;
        }
        remove_legacy_startup_value()?;
    }
    Ok(current_enabled || legacy_enabled)
}

#[cfg(target_os = "windows")]
fn set_windows_startup(enabled: bool) -> anyhow::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    let subkey: Vec<u16> = std::ffi::OsStr::new(STARTUP_REG_PATH)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let value_name: Vec<u16> = std::ffi::OsStr::new(STARTUP_REG_NAME)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let exe = std::env::current_exe()?;
    // Keep the Windows path in native UTF-16; a lossy UTF-8 conversion could corrupt
    // startup registration for an executable in a path containing unusual Unicode.
    let mut command_wide = Vec::new();
    command_wide.push(u16::from(b'"'));
    command_wide.extend(exe.as_os_str().encode_wide());
    command_wide.push(u16::from(b'"'));
    command_wide.push(0);
    // REG_SZ stores UTF-16LE. Build its byte representation without a raw-pointer slice.
    let command_bytes: Vec<u8> = command_wide
        .iter()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();

    let mut key = std::ptr::null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if status != 0 {
        anyhow::bail!("Could not open Windows startup registry key (error {})", status);
    }

    let result = (|| -> anyhow::Result<()> {
        if enabled {
            let status = unsafe {
                RegSetValueExW(
                    key,
                    value_name.as_ptr(),
                    0,
                    REG_SZ,
                    command_bytes.as_ptr(),
                    command_bytes.len() as u32,
                )
            };
            if status != 0 {
                anyhow::bail!("Could not enable Windows startup (error {})", status);
            }
        } else {
            let status = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };
            if status != 0 && status != 2 {
                anyhow::bail!("Could not disable Windows startup (error {})", status);
            }
        }
        Ok(())
    })();

    unsafe { RegCloseKey(key) };
    result
}

fn save_config(config: &Config) {
    if let Err(error) = config.save() {
        MessageDialog::new()
            .set_level(MessageLevel::Error)
            .set_title("GxOled settings")
            .set_description(&format!("Could not save settings: {error}"))
            .show();
    }
}

struct Icon {
    _pixels: Vec<u8>,
    surf: *mut sdl::SDL_Surface,
}
impl Icon {
    fn load(buf: &[u8]) -> Self {
        let pixels = image::load_from_memory(buf)
            .unwrap()
            .resize(32, 32, image::imageops::FilterType::Lanczos3)
            .to_rgba8()
            .into_vec();
        let surf = unsafe {
            sdl::SDL_CreateSurfaceFrom(
                32,
                32,
                sdl::SDL_PixelFormat::RGBA32,
                pixels.as_ptr() as *mut c_void,
                32 * 4,
            )
        };
        assert!(!surf.is_null());
        Self { _pixels: pixels, surf }
    }
}
impl Drop for Icon {
    fn drop(&mut self) {
        unsafe { sdl::SDL_DestroySurface(self.surf) };
    }
}

fn menu_check(menu: *mut sdl::SDL_TrayMenu, title: &'static CStr, checked: bool) -> *mut sdl::SDL_TrayEntry {
    let checked = if checked {
        sdl::SDL_TRAYENTRY_CHECKED
    } else {
        sdl::SDL_TrayEntryFlags(0)
    };
    unsafe { sdl::SDL_InsertTrayEntryAt(menu, -1, title.as_ptr(), sdl::SDL_TRAYENTRY_CHECKBOX | checked) }
}

extern "C" fn c_menu_callback(userdata: *mut c_void, _entry: *mut sdl::SDL_TrayEntry) {
    #[allow(clippy::borrowed_box)] // needs to be boxed
    let f: &Box<dyn Fn()> = unsafe { &*(userdata as *mut Box<dyn Fn()>) };
    f();
}

fn menu_callback(entry: *mut sdl::SDL_TrayEntry, f: impl Fn()) {
    let f: Box<Box<dyn Fn()>> = Box::new(Box::new(f));
    let f = Box::leak(f) as *mut Box<dyn Fn()> as *mut c_void;
    unsafe { sdl::SDL_SetTrayEntryCallback(entry, Some(c_menu_callback), f) };
}

#[derive(Clone, Copy, Debug)]
struct ClockRect {
    x: isize,
    y: isize,
    w: usize,
    h: usize,
}

impl ClockRect {
    fn overlaps(self, other: ClockRect) -> bool {
        self.x < other.x + other.w as isize
            && other.x < self.x + self.w as isize
            && self.y < other.y + other.h as isize
            && other.y < self.y + self.h as isize
    }

    fn separated_from(self, other: ClockRect, margin: isize) -> bool {
        self.x + self.w as isize + margin <= other.x
            || other.x + other.w as isize + margin <= self.x
            || self.y + self.h as isize + margin <= other.y
            || other.y + other.h as isize + margin <= self.y
    }
}

#[derive(Clone, Copy)]
struct ClockPosition {
    x: isize,
    y: isize,
    rect: ClockRect,
}

fn clock_layout(
    dev: &DrawDevice,
    text: &str,
    media_visible: bool,
    style: ConfigClockStyle,
) -> (isize, isize, usize, usize, usize, usize) {
    let (screen_w, screen_h) = dev.dimensions();
    match style {
        ConfigClockStyle::AnalogRound | ConfigClockStyle::AnalogSquare => {
            let size = if media_visible { 32 } else { 54 };
            let base_x = (screen_w as isize - size as isize) / 2;
            let base_y = if media_visible { 4 } else { (screen_h as isize - size as isize) / 2 };
            (base_x, base_y, size, size, screen_w, screen_h)
        }
        ConfigClockStyle::CasioF91W => (0, 0, 128, 64, screen_w, screen_h),
        ConfigClockStyle::DigitalOriginal | ConfigClockStyle::DigitalBold => {
            let size = if style == ConfigClockStyle::DigitalBold {
                dev.text_size() * if media_visible {0.94} else {1.48}
            } else {dev.text_size() * if media_visible {0.80} else {1.0}};
            let bitmap = dev.render_lines_at_size(text, size).into_iter().next().unwrap();
            let bitmap_w = bitmap.w + if style == ConfigClockStyle::DigitalBold { 1 } else { 0 };
            let line_height = dev.font_line_height_at_size(size);
            let base_x = (screen_w as isize - bitmap_w.min(screen_w) as isize) / 2;
            let base_y = if media_visible { 4 } else { (screen_h as isize - line_height as isize) / 2 };
            (base_x, base_y, bitmap_w, bitmap.h, screen_w, screen_h)
        }
    }
}

fn make_clock_position(
    x: isize,
    y: isize,
    bitmap_w: usize,
    bitmap_h: usize,
    screen_w: usize,
) -> ClockPosition {
    let (visible_x, visible_w) = if bitmap_w >= screen_w {
        (0, screen_w)
    } else {
        (x, bitmap_w)
    };
    ClockPosition {
        x,
        y,
        rect: ClockRect {
            x: visible_x,
            y,
            w: visible_w,
            h: bitmap_h,
        },
    }
}

fn grid_values(maximum: isize, step: usize) -> Vec<isize> {
    if maximum <= 0 {
        return vec![0];
    }
    let mut values: Vec<isize> = (0..=maximum as usize)
        .step_by(step.max(1))
        .map(|v| v as isize)
        .collect();
    if values.last().copied() != Some(maximum) {
        values.push(maximum);
    }
    values
}

fn minute_random_index(minute_key: i64, length: usize) -> usize {
    if length <= 1 {
        return 0;
    }
    // SplitMix64: no extra RNG dependency, but the result varies irregularly by minute.
    let mut value = (minute_key as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value as usize) % length
}

fn clock_obstacles(
    dev: &DrawDevice,
    media: Option<&Media>,
    notification_visible: bool,
    _style: ConfigClockStyle,
) -> Vec<ClockRect> {
    let (screen_w, _) = dev.dimensions();
    let mut obstacles = Vec::new();
    if media.is_some() {
        // Metadata is confined to the bottom of the screen. Treat the ENTIRE
        // lower band as reserved when selecting random clock positions.
        obstacles.push(ClockRect{x:0,y:37,w:screen_w,h:27});
    }
    if notification_visible {
        obstacles.push(ClockRect{x:4,y:4,w:32,h:32});
    }
    obstacles
}

fn choose_clock_offset(
    dev: &DrawDevice,
    text: &str,
    media_visible: bool,
    mode: ClockBurnInMode,
    previous_rect: Option<ClockRect>,
    obstacles: &[ClockRect],
    minute_key: i64,
    style: ConfigClockStyle,
) -> (isize, isize) {
    let (base_x, base_y, bitmap_w, bitmap_h, screen_w, screen_h) =
        clock_layout(dev, text, media_visible, style);
    // OLED screen shifting can displace the whole image by up to 4 px per axis.
    // Keep 4 px clear of the physical screen edges so the clock isn't clipped.
    const EDGE_GUARD: isize = 4;
    let horizontal_span = screen_w as isize - bitmap_w.min(screen_w) as isize;
    let vertical_span = screen_h as isize - bitmap_h.min(screen_h) as isize;
    let min_x = if bitmap_w >= screen_w || horizontal_span < EDGE_GUARD * 2 {
        0
    } else {
        EDGE_GUARD
    };
    let max_x = if bitmap_w >= screen_w {
        0
    } else {
        (horizontal_span - EDGE_GUARD).max(min_x)
    };
    let min_y = if bitmap_h >= screen_h || vertical_span < EDGE_GUARD * 2 {
        0
    } else {
        EDGE_GUARD
    };
    let max_y = if bitmap_h >= screen_h {
        0
    } else {
        (vertical_span - EDGE_GUARD).max(min_y)
    };
    let mut candidates = Vec::<ClockPosition>::new();

    match mode {
        ClockBurnInMode::Off => return (0, 0),
        ClockBurnInMode::Wide => {
            // Medium burn-in protection: noticeably wider than the original ±2x1 px,
            // but a little less aggressive than the previous ±24x12 px revision.
            // The strict NoOverlap mode remains responsible for disjoint clock rectangles.
            const X_DELTAS: &[isize] = &[-20, -14, -8, 0, 8, 14, 20];
            const Y_DELTAS: &[isize] = &[-10, -6, 0, 6, 10];
            for dx in X_DELTAS {
                for dy in Y_DELTAS {
                    let x = if bitmap_w >= screen_w { 0 } else { base_x + *dx };
                    let y = base_y + *dy;
                    if x < min_x || x > max_x || y < min_y || y > max_y {
                        continue;
                    }
                    let candidate = make_clock_position(x, y, bitmap_w, bitmap_h, screen_w);
                    if obstacles.iter().any(|obstacle| candidate.rect.overlaps(*obstacle)) {
                        continue;
                    }
                    if let Some(previous) = previous_rect {
                        let dx = candidate.rect.x - previous.x;
                        let dy = candidate.rect.y - previous.y;
                        // Require a clear but slightly more restrained 12 px move on one axis.
                        // This is deliberately less strict than NoOverlap, which separates the
                        // complete old and new clock rectangles rather than merely moving them.
                        if dx.abs().max(dy.abs()) < 12 {
                            continue;
                        }
                    }
                    if !candidates.iter().any(|p| p.x == candidate.x && p.y == candidate.y) {
                        candidates.push(candidate);
                    }
                }
            }
        }
        ClockBurnInMode::NoOverlap => {
            // Search the available OLED area on a grid. Candidates must not intersect the
            // previous clock's entire bitmap rectangle, not merely its currently lit dots.
            // A 12 px safety gap covers the OLED shifter's full +/-4 px travel and small
            // width changes between time strings, preventing old clock pixels from being reused.
            let xs: Vec<isize> = grid_values(max_x, 4)
                .into_iter()
                .filter(|x| *x >= min_x)
                .collect();
            let y_step = if style.is_analog() && media_visible { 1 } else { 3 };
            let ys: Vec<isize> = grid_values(max_y, y_step)
                .into_iter()
                .filter(|y| *y >= min_y)
                .collect();
            for x in xs {
                for &y in &ys {
                    let candidate = make_clock_position(x, y, bitmap_w, bitmap_h, screen_w);
                    if obstacles.iter().any(|obstacle| candidate.rect.overlaps(*obstacle)) {
                        continue;
                    }
                    // High OLED shifting can change by up to 8 px between positions; the
                    // 12 px gap also leaves a little room for glyph-width changes between seconds.
                    if previous_rect.is_some_and(|previous| {
                        !candidate.rect.separated_from(previous, 12)
                    }) {
                        continue;
                    }
                    candidates.push(candidate);
                }
            }

            // For unusual custom fonts, relax overlay avoidance as a fallback, but keep the
            // previous-area separation guarantee whenever the OLED geometry permits it.
            if candidates.is_empty() {
                let xs = grid_values(max_x, 4);
                let ys = grid_values(max_y, 3);
                for x in xs {
                    for &y in &ys {
                        let candidate = make_clock_position(x, y, bitmap_w, bitmap_h, screen_w);
                        if previous_rect.map_or(true, |previous| {
                            candidate.rect.separated_from(previous, 12)
                        }) {
                            candidates.push(candidate);
                        }
                    }
                }
            }
        }
    }

    if candidates.is_empty() && mode == ClockBurnInMode::Wide {
        // If media/status overlays make the local wide-offset set too restrictive, search the
        // full safe screen area instead. This is still a wide shift and never deliberately moves
        // into an active media/connection overlay.
        let xs = grid_values(max_x, 8);
        let ys = grid_values(max_y, 6);
        for x in xs {
            if x < min_x {
                continue;
            }
            for &y in &ys {
                if y < min_y {
                    continue;
                }
                let candidate = make_clock_position(x, y, bitmap_w, bitmap_h, screen_w);
                if obstacles.iter().any(|obstacle| candidate.rect.overlaps(*obstacle)) {
                    continue;
                }
                if let Some(previous) = previous_rect {
                    let dx = (candidate.rect.x - previous.x).abs();
                    let dy = (candidate.rect.y - previous.y).abs();
                    if dx.max(dy) < 12 {
                        continue;
                    }
                }
                candidates.push(candidate);
            }
        }
    }

    if candidates.is_empty() {
        // A custom font can theoretically be so large that a disjoint legal position does not
        // exist. The caller still gets a centered position rather than panicking or clipping.
        return (0, 0);
    }

    // Candidate order is spatial; minute_random_index gives a repeatable but irregular position
    // per minute without adding a dependency or allowing the same random position every minute.
    let preferred = minute_random_index(minute_key, candidates.len());
    let chosen = candidates[preferred];
    (chosen.x - base_x, chosen.y - base_y)
}

#[derive(Clone)]
enum MenuEvent {
    ToggleCheck,
    SetTimeMode(ConfigTimeMode),
    SetClockStyle(ConfigClockStyle),
    SetGoAnimation(Option<GoAnimation>),
    #[cfg(target_os = "windows")]
    SetGoProfile(GoAnimation, Option<String>),
    #[cfg(target_os = "windows")]
    OpenSettings,
    #[cfg(target_os = "windows")]
    SetUiMode(ConfigUiMode),
    #[cfg(target_os = "windows")]
    SetUiStyle(ConfigUiStyle),
    #[cfg(target_os = "windows")]
    SetLanguage(ConfigLanguage),
    #[cfg(target_os = "windows")]
    SetTheme(ConfigTheme),
    #[cfg(target_os = "windows")]
    ToggleIdleFromWindow,
    #[cfg(target_os = "windows")]
    ToggleMediaFromWindow,
    #[cfg(target_os = "windows")]
    #[cfg(target_os = "windows")]
    OpenGifPickerFromWindow,
    #[cfg(target_os = "windows")]
    ToggleGifFromWindow,
    SetShiftMode(ConfigShiftMode),
    SetStatusNotifyMode(StatusNotifyMode),
    SetQuitMode(ConfigQuitMode),
    SetClockBurnInMode(ClockBurnInMode),
    #[cfg(target_os = "windows")]
    ToggleStartupFromWindow,
    SetBrightness(u8),
    SetVolumeOsdHold(u32),
    #[cfg(target_os = "windows")]
    ToggleStartup,
    GifSelected(PathBuf),
    ToggleAnimation,
    Quit,
}

fn update_clock_style_checked(entries: &[(*mut sdl::SDL_TrayEntry, ConfigClockStyle)], value: ConfigClockStyle) {
    for (entry, style) in entries {
        unsafe { sdl::SDL_SetTrayEntryChecked(*entry, *style == value) };
    }
}

fn bind_menu_event(entry: *mut sdl::SDL_TrayEntry, tx: &mpsc::Sender<MenuEvent>, event: MenuEvent) {
    let tx = tx.clone();
    menu_callback(entry, move || {
        let _ = tx.send(event.clone());
    });
}

struct RadioMenu<T> {
    entries: Vec<(*mut sdl::SDL_TrayEntry, T)>,
    title: *mut sdl::SDL_TrayEntry,
}
impl<T: Copy + PartialEq> RadioMenu<T> {
    fn new(
        menu: *mut sdl::SDL_TrayMenu,
        title: &'static CStr,
        options: &[(&'static CStr, T)],
        initial: T,
        tx: &mpsc::Sender<MenuEvent>,
        into_event: fn(T) -> MenuEvent,
    ) -> Self {
        let submenu_entry = unsafe { sdl::SDL_InsertTrayEntryAt(menu, -1, title.as_ptr(), sdl::SDL_TRAYENTRY_SUBMENU) };
        let submenu = unsafe { sdl::SDL_CreateTraySubmenu(submenu_entry) };
        let entries: Vec<_> = options
            .iter()
            .map(|(label, value)| {
                let entry = menu_check(submenu, label, *value == initial);
                bind_menu_event(entry, tx, into_event(*value));
                (entry, *value)
            })
            .collect();
        Self { entries, title: submenu_entry }
    }
    fn update_checked(&self, value: T) {
        for (entry, v) in &self.entries {
            unsafe { sdl::SDL_SetTrayEntryChecked(*entry, *v == value) };
        }
    }
}

#[cfg(target_os = "windows")]
fn tray_label(entry: *mut sdl::SDL_TrayEntry,en:&str,tr:&str,language:ConfigLanguage) {
    let value=if language==ConfigLanguage::Turkish {tr}else{en};
    if let Ok(value)=std::ffi::CString::new(value) {
        unsafe{sdl::SDL_SetTrayEntryLabel(entry,value.as_ptr());}
    }
}

fn start_gif_file_picker(tx: &mpsc::Sender<MenuEvent>) {
    let tx = tx.clone();
    let result = std::thread::Builder::new()
        .name("gxoled-gif-dialog".to_owned())
        .spawn(move || {
            let path = rfd::FileDialog::new()
                .set_title("Add GIF")
                .add_filter("GIF animation", &["gif"])
                .pick_file();

            if let Some(path) = path {
                let _ = tx.send(MenuEvent::GifSelected(path));
            }
        });

    if let Err(err) = result {
        MessageDialog::new()
            .set_level(MessageLevel::Error)
            .set_title("GxOled")
            .set_description(&format!("Could not start the GIF file dialog: {err}"))
            .show();
    }
}

// The Windows media session can disappear, or a user can disable metadata,
// while old OLED layers are still present. Reconcile both content and layer state.
fn media_layers_need_refresh(
    current: &Option<Media>,
    previous: &Option<Media>,
    layers_present: bool,
    layers_expected: bool,
) -> bool {
    current != previous || layers_present != layers_expected
}

/// Windows: report whether another GxOled is already running. Two copies would both write
/// OLED frames and both react to the dial, which looks like lag or flicker and makes timing
/// measurements meaningless. The mutex handle is deliberately never closed; it lives as long
/// as the process.
#[cfg(target_os = "windows")]
fn another_instance_is_running() -> bool {
    use windows_sys::Win32::{
        Foundation::{ERROR_ALREADY_EXISTS, GetLastError},
        System::Threading::CreateMutexW,
    };
    let name: Vec<u16> = "Local\\GxOled.Advanced.SingleInstance"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        // The guard itself failed: never block startup because of it.
        return false;
    }
    // Must be read immediately after CreateMutexW.
    unsafe { GetLastError() == ERROR_ALREADY_EXISTS }
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        if another_instance_is_running() {
            MessageDialog::new()
                .set_level(MessageLevel::Info)
                .set_title("GxOled")
                .set_description(
                    "GxOled is already running. Look for its icon in the system tray, and use \
                     'Quit GxOled' there before starting another copy.",
                )
                .show();
            return;
        }
    }

    // Initial loading
    let mut config = Config::load();
    #[cfg(target_os = "windows")]
    {
        // The Registry is the source of truth for the Windows startup checkbox.
        // We intentionally don't auto-enable it here; we only mirror its current state.
        config.startup = migrate_legacy_windows_startup()
            .unwrap_or_else(|_| windows_startup_enabled().unwrap_or(config.startup));
    }
    #[cfg(target_os = "windows")]
    {
        // Beta safety net. `--classic-ui` / `--modern-ui` force a style (and remember it); a previous modern start
        // that never reached its first paint also falls back to the classic window.
        let args: Vec<String> = std::env::args().skip(1).collect();
        let before = config.ui_style;
        if args.iter().any(|a| a == "--classic-ui") {
            config.ui_style = ConfigUiStyle::Classic;
        } else if args.iter().any(|a| a == "--modern-ui") {
            config.ui_style = ConfigUiStyle::Modern;
        }
        if settings_ui::previous_start_failed() && config.ui_style == ConfigUiStyle::Modern {
            config.ui_style = ConfigUiStyle::Classic;
            let turkish = config.language == ConfigLanguage::Turkish;
            MessageDialog::new()
                .set_level(MessageLevel::Warning)
                .set_title("GxOled")
                .set_description(if turkish {
                    "Modern (Beta) arayüz geçen sefer düzgün açılamadı; GxOled klasik arayüze döndü.\nTepsi menüsündeki 'Ayar penceresi biçimi (Beta)' ile tekrar deneyebilirsiniz."
                } else {
                    "The modern (Beta) interface did not start properly last time, so GxOled went back to the classic one.\nYou can try it again from the tray menu: 'Settings window style (Beta)'."
                })
                .show();
        }
        if config.ui_style != before {
            if let Err(err) = config.save() {
                eprintln!("Could not save the settings window style: {err}");
            }
        }
    }
    // Retain the GIF path but load frames only when playback is active. A stopped
    // animation must not hold thousands of decoded frames in RAM.
    if config.go_animation.is_some() { config.animation_playing = false; }
    let mut animation_frames: Option<Vec<gxoled_draw::Frame>> = if config.animation_playing {
        config.animation_path.as_ref().and_then(|path|
            try_decode_gif_frames(&path.to_string_lossy(), 0x80).ok())
    } else { None };
    if animation_frames.is_none() && config.animation_playing {
        config.animation_playing = false;
        save_config(&config);
    }

    let mut os = OSImpl::new();

    // Create tray icon with menu
    unsafe { sdl::SDL_SetHint(sdl::SDL_HINT_VIDEO_ALLOW_SCREENSAVER, c"1".as_ptr()) };
    assert!(unsafe { sdl::SDL_Init(sdl::SDL_INIT_VIDEO) });
    let icon = Icon::load(include_bytes!("../assets/gxoled.png"));
    let icon_error = Icon::load(include_bytes!("../assets/gxoled_error.png"));
    let tray = unsafe { sdl::SDL_CreateTray(icon_error.surf, c"GxOled".as_ptr()) };
    assert!(!tray.is_null());
    let menu = unsafe { sdl::SDL_CreateTrayMenu(tray) };
    assert!(!menu.is_null());
    let (menu_tx, menu_rx) = mpsc::channel::<MenuEvent>();

    let version_label = format!("GxOled Advanced v{}\0", display_version());
    unsafe {
        let tm_version = sdl::SDL_InsertTrayEntryAt(menu, -1, version_label.as_ptr().cast(), sdl::SDL_TRAYENTRY_BUTTON);
        sdl::SDL_SetTrayEntryEnabled(tm_version, false);
        sdl::SDL_InsertTrayEntryAt(menu, -1, std::ptr::null(), sdl::SDL_TRAYENTRY_BUTTON); // separator
    }

    // Clock style is separated into Digital and Analog groups. The four leaves act as
    // one radio group, so there is always exactly one selected presentation.
    let tm_clock_style_root = unsafe { sdl::SDL_InsertTrayEntryAt(menu, -1, c"Clock style".as_ptr(), sdl::SDL_TRAYENTRY_SUBMENU) };
    let tm_clock_style_menu = unsafe { sdl::SDL_CreateTraySubmenu(tm_clock_style_root) };
    let tm_digital_root = unsafe { sdl::SDL_InsertTrayEntryAt(tm_clock_style_menu, -1, c"Digital".as_ptr(), sdl::SDL_TRAYENTRY_SUBMENU) };
    let tm_digital_menu = unsafe { sdl::SDL_CreateTraySubmenu(tm_digital_root) };
    let tm_analog_root = unsafe { sdl::SDL_InsertTrayEntryAt(tm_clock_style_menu, -1, c"Analog".as_ptr(), sdl::SDL_TRAYENTRY_SUBMENU) };
    let tm_analog_menu = unsafe { sdl::SDL_CreateTraySubmenu(tm_analog_root) };
    let tm_clock_style_entries = vec![
        (menu_check(tm_digital_menu, c"Digital Standard", config.clock_style == ConfigClockStyle::DigitalOriginal), ConfigClockStyle::DigitalOriginal),
        (menu_check(tm_digital_menu, c"Digital Big", config.clock_style == ConfigClockStyle::DigitalBold), ConfigClockStyle::DigitalBold),
        (menu_check(tm_analog_menu, c"Round", config.clock_style == ConfigClockStyle::AnalogRound), ConfigClockStyle::AnalogRound),
        (menu_check(tm_analog_menu, c"Square", config.clock_style == ConfigClockStyle::AnalogSquare), ConfigClockStyle::AnalogSquare),
        (menu_check(tm_digital_menu, c"Casio Digital Dial", config.clock_style == ConfigClockStyle::CasioF91W), ConfigClockStyle::CasioF91W),
    ];
    for (entry, style) in &tm_clock_style_entries {
        bind_menu_event(*entry, &menu_tx, MenuEvent::SetClockStyle(*style));
    }

    let tm_time_radio = RadioMenu::new(
        menu,
        c"Time",
        &[
            (c"Off", ConfigTimeMode::Off),
            (c"24-hour", ConfigTimeMode::H24),
            (c"12-hour (AM/PM)", ConfigTimeMode::H12),
        ],
        config.time_mode,
        &menu_tx,
        MenuEvent::SetTimeMode,
    );

    let tm_media_check = menu_check(menu, c"Show media alongside clock", config.show_media);
    if os.supports_media() {
        bind_menu_event(tm_media_check, &menu_tx, MenuEvent::ToggleCheck);
    } else {
        unsafe {
            config.show_media = false;
            sdl::SDL_SetTrayEntryChecked(tm_media_check, false);
            sdl::SDL_SetTrayEntryEnabled(tm_media_check, false);
        }
    }

    let tm_status_notify = RadioMenu::new(
        menu,
        c"Connection status icon",
        &[
            (c"Off", StatusNotifyMode::Off),
            (c"Notify", StatusNotifyMode::Notify),
            (c"Always (burn-in risk)", StatusNotifyMode::Always),
            (c"When connected (burn-in risk)", StatusNotifyMode::WhenConnected),
            (c"When disconnected (burn-in risk)", StatusNotifyMode::WhenDisconnected),
        ],
        config.status_notify,
        &menu_tx,
        MenuEvent::SetStatusNotifyMode,
    );

    let tm_idle_check = menu_check(menu, c"Screensaver when idle", config.idle_timeout);
    if os.supports_idle() {
        bind_menu_event(tm_idle_check, &menu_tx, MenuEvent::ToggleCheck);
    } else {
        unsafe {
            config.idle_timeout = false;
            sdl::SDL_SetTrayEntryChecked(tm_idle_check, false);
            sdl::SDL_SetTrayEntryEnabled(tm_idle_check, false);
        }
    }

    let tm_shift_radio = RadioMenu::new(
        menu,
        c"OLED screen shift",
        &[
            (c"Off", ConfigShiftMode::Off),
            (c"Low", ConfigShiftMode::Low),
            (c"High", ConfigShiftMode::High),
        ],
        config.oled_shift,
        &menu_tx,
        MenuEvent::SetShiftMode,
    );

    // The firmware command accepts discrete values 1..=10. Level 0 is unsupported,
    // so the UI intentionally starts at the minimum 10% step rather than offering 0%.
    let tm_brightness_radio = RadioMenu::new(
        menu,
        c"Screen brightness",
        &[
            (c"10% (Level 1/10)", 1u8),
            (c"20% (Level 2/10)", 2u8),
            (c"30% (Level 3/10)", 3u8),
            (c"40% (Level 4/10)", 4u8),
            (c"50% (Level 5/10)", 5u8),
            (c"60% (Level 6/10)", 6u8),
            (c"70% (Level 7/10)", 7u8),
            (c"80% (Level 8/10)", 8u8),
            (c"90% (Level 9/10)", 9u8),
            (c"100% (Level 10/10)", 10u8),
        ],
        config.screen_brightness.unwrap_or(0),
        &menu_tx,
        MenuEvent::SetBrightness,
    );

    let tm_volume_osd_radio = RadioMenu::new(
        menu,
        c"Volume screen time",
        &[
            (c"1 s", 1000u32),
            (c"1.5 s", 1500u32),
            (c"2 s", 2000u32),
            (c"2.5 s (default)", 2500u32),
            (c"3 s", 3000u32),
            (c"4 s", 4000u32),
            (c"5 s", 5000u32),
        ],
        config.volume_osd_hold_ms,
        &menu_tx,
        MenuEvent::SetVolumeOsdHold,
    );

    let tm_clock_burn_in_radio = RadioMenu::new(
        menu,
        c"Clock burn-in protection",
        &[
            (c"Off", ClockBurnInMode::Off),
            (c"Random Movements", ClockBurnInMode::Wide),
            (c"No overlapping Pixels", ClockBurnInMode::NoOverlap),
        ],
        config.clock_burn_in_mode(),
        &menu_tx,
        MenuEvent::SetClockBurnInMode,
    );

    #[cfg(target_os = "windows")]
    let tm_startup_check = {
        let entry = menu_check(menu, c"Start GxOled with Windows", config.startup);
        bind_menu_event(entry, &menu_tx, MenuEvent::ToggleStartup);
        entry
    };

    let tm_animation_menu = unsafe { sdl::SDL_InsertTrayEntryAt(menu, -1, c"Animation".as_ptr(), sdl::SDL_TRAYENTRY_SUBMENU) };
    let tm_animation_submenu = unsafe { sdl::SDL_CreateTraySubmenu(tm_animation_menu) };
    let tm_add_gif = unsafe { sdl::SDL_InsertTrayEntryAt(tm_animation_submenu, -1, c"Add GIF".as_ptr(), sdl::SDL_TRAYENTRY_BUTTON) };
    let gif_menu_tx = menu_tx.clone();
    menu_callback(tm_add_gif, move || start_gif_file_picker(&gif_menu_tx));
    let tm_animation_check = menu_check(
        tm_animation_submenu,
        c"Start / Stop animation",
        config.animation_playing && config.animation_path.is_some(),
    );
    unsafe { sdl::SDL_SetTrayEntryEnabled(tm_animation_check, config.animation_path.is_some()) };
    bind_menu_event(tm_animation_check, &menu_tx, MenuEvent::ToggleAnimation);

    // Original Go algorithms, embedded in the application and run only on demand.
    #[cfg(target_os = "windows")]
    let tm_go_anim_radio = RadioMenu::new(
        tm_animation_submenu,
        c"128x64 animations",
        &[
            (c"Off / Clock", None),
            (c"Matrix Rain", Some(GoAnimation::Matrix)),
            (c"Hyperspace", Some(GoAnimation::Hyperspace)),
            (c"Star Wars Intro", Some(GoAnimation::StarWars)),
            (c"Game of Life", Some(GoAnimation::GameOfLife)),
        ],
        config.go_animation,
        &menu_tx,
        MenuEvent::SetGoAnimation,
    );
    #[cfg(target_os = "windows")]
    let tm_open_settings = {
        let entry = unsafe { sdl::SDL_InsertTrayEntryAt(menu, 2, c"Open Settings...".as_ptr(), sdl::SDL_TRAYENTRY_BUTTON) };
        bind_menu_event(entry, &menu_tx, MenuEvent::OpenSettings);
        entry
    };
    #[cfg(target_os = "windows")]
    let tm_ui_radio = RadioMenu::new(
        menu, c"Settings UI mode",
        &[(c"Tray menu",ConfigUiMode::Tray),(c"Window",ConfigUiMode::Window),
          (c"Hybrid",ConfigUiMode::Hybrid)],
        config.ui_mode, &menu_tx, MenuEvent::SetUiMode,
    );
    #[cfg(target_os = "windows")]
    let tm_ui_style_radio = RadioMenu::new(
        menu, c"Settings window style (Beta)",
        &[(c"Modern (Beta)",ConfigUiStyle::Modern),(c"Classic",ConfigUiStyle::Classic)],
        config.ui_style, &menu_tx, MenuEvent::SetUiStyle,
    );
    #[cfg(target_os = "windows")]
    if let Err(err)=settings_ui::create(menu_tx.clone(), &config) {
        eprintln!("GxOled settings window failed: {err}");
    }
    #[cfg(target_os = "windows")]
    {
        settings_ui::sync(&config);
        settings_ui::localize(config.language);
        settings_ui::set_theme(config.theme);
        settings_ui::install_tray_hook(config.ui_mode);
        // Opening mode affects clicking the tray icon, not application startup.
    }

    let tm_quit_mode_radio = RadioMenu::new(
        menu,
        c"On app quit",
        &[
            (c"Screensaver", ConfigQuitMode::Screensaver),
            (c"Stock UI", ConfigQuitMode::StockUi),
        ],
        config.quit_mode,
        &menu_tx,
        MenuEvent::SetQuitMode,
    );

    unsafe { sdl::SDL_InsertTrayEntryAt(menu, -1, std::ptr::null(), sdl::SDL_TRAYENTRY_BUTTON) }; // separator
    let tm_quit = unsafe { sdl::SDL_InsertTrayEntryAt(menu, -1, c"Quit".as_ptr(), sdl::SDL_TRAYENTRY_BUTTON) };
    bind_menu_event(tm_quit, &menu_tx, MenuEvent::Quit);

    #[cfg(target_os = "windows")]
    let translate_tray = |language:ConfigLanguage| {
        let set=|entry,en,tr|tray_label(entry,en,tr,language);
        set(tm_clock_style_root,"Clock style","Saat tarzı");
        set(tm_digital_root,"Digital","Dijital");
        set(tm_analog_root,"Analog","Analog");
        for (e,style) in &tm_clock_style_entries {
            match style {
                ConfigClockStyle::DigitalOriginal=>set(*e,"Digital Standard","Dijital Standart"),
                ConfigClockStyle::DigitalBold=>set(*e,"Digital Big","Dijital Büyük"),
                ConfigClockStyle::AnalogRound=>set(*e,"Round","Yuvarlak"),
                ConfigClockStyle::AnalogSquare=>set(*e,"Square","Kare"),
                ConfigClockStyle::CasioF91W=>{},
            }
        }
        set(tm_time_radio.title,"Time","Saat formatı");
        for (e,value) in &tm_time_radio.entries {match value {
            ConfigTimeMode::Off=>set(*e,"Off","Kapalı"),
            ConfigTimeMode::H24=>set(*e,"24-hour","24 saat"),
            ConfigTimeMode::H12=>set(*e,"12-hour (AM/PM)","12 saat (ÖÖ/ÖS)"),
        }}
        set(tm_media_check,"Show media alongside clock","Saatin yanında medya göster");
        set(tm_status_notify.title,"Connection status icon","Bağlantı durumu simgesi");
        set(tm_idle_check,"Screensaver when idle","Boştayken ekran koruyucu");
        set(tm_shift_radio.title,"OLED screen shift","OLED ekran kaydırma");
        set(tm_brightness_radio.title,"Screen brightness","Ekran parlaklığı");
        set(tm_volume_osd_radio.title,"Volume screen time","Ses ekranı süresi");
        for (e,ms) in &tm_volume_osd_radio.entries {
            // Not `set`: that closure was inferred for 'static literals, these captions are built.
            tray_label(*e,&volume_osd_hold_label(*ms,false),&volume_osd_hold_label(*ms,true),language);
        }
        set(tm_clock_burn_in_radio.title,"Clock burn-in protection","Saat yanık izi koruması");
        set(tm_startup_check,"Start GxOled with Windows","Windows ile GxOled'i başlat");
        set(tm_animation_menu,"Animation","Animasyon");
        set(tm_add_gif,"Add GIF","GIF ekle");
        set(tm_animation_check,"Start / Stop animation","Animasyonu başlat / durdur");
        set(tm_go_anim_radio.title,"SteelClock 128x64","SteelClock 128x64");
        set(tm_open_settings,"Open Settings...","Ayarları aç...");
        set(tm_ui_radio.title,"App opening mode","Uygulama açılış biçimi");
        for (e,value) in &tm_ui_radio.entries {match value {
            ConfigUiMode::Tray=>set(*e,"Tray menu","Tepsi menüsü"),
            ConfigUiMode::Window=>set(*e,"Window","Pencere"),
            ConfigUiMode::Hybrid=>set(*e,"Hybrid","Karma"),
        }}
        set(tm_ui_style_radio.title,"Settings window style (Beta)","Ayar penceresi biçimi (Beta)");
        for (e,value) in &tm_ui_style_radio.entries {match value {
            ConfigUiStyle::Modern=>set(*e,"Modern (Beta)","Modern (Beta)"),
            ConfigUiStyle::Classic=>set(*e,"Classic","Klasik"),
        }}
        set(tm_quit_mode_radio.title,"On app quit","Uygulama kapanırken");
        set(tm_quit,"Quit","Çıkış");
    };
    #[cfg(target_os = "windows")]
    translate_tray(config.language);


    // Load icons
    let icon_hs_connect =
        Arc::new(bitmap_from_memory(include_bytes!("../assets/headset_connected.png"), 0x80).unwrap());
    let icon_hs_disconnect =
        Arc::new(bitmap_from_memory(include_bytes!("../assets/headset_disconnected.png"), 0x80).unwrap());

    let notif_update = |dev: &mut DrawDevice,
                        layer: &mut Option<LayerId>,
                        mode: StatusNotifyMode,
                        connected: bool,
                        expiry: DateTime<Local>| {
        if let Some(id) = layer.take() {
            dev.remove_layer(id);
        }
        let show = match mode {
            StatusNotifyMode::Off => false,
            StatusNotifyMode::Notify => Local::now() < expiry,
            StatusNotifyMode::Always => true,
            StatusNotifyMode::WhenConnected => connected,
            StatusNotifyMode::WhenDisconnected => !connected,
        };
        if show {
            *layer = Some(
                dev.add_layer(gxoled_draw::DrawLayer::Image {
                    bitmap: (if connected {
                        &icon_hs_connect
                    } else {
                        &icon_hs_disconnect
                    })
                    .clone(),
                    x: 4,
                    y: 4,
                }),
            );
        }
    };

    // State
    let mut last_time = Local::now() - TimeDelta::seconds(1);
    let mut last_media: Option<Media> = None;
    let mut time_layers: Vec<LayerId> = vec![];
    let mut media_layers: Vec<LayerId> = vec![];
    let mut notif_layer: Option<LayerId> = None;
    let mut notif_expiry = Local::now();
    let mut is_connected = false;
    #[cfg(target_os = "windows")]
    let mut oled_available = true;
    let mut animation_layer: Option<LayerId> = None;
    #[cfg(target_os = "windows")]
    let mut go_runner: Option<go_bridge::Runner> = None;
    #[cfg(target_os = "windows")]
    let mut go_layer: Option<LayerId> = None;
    #[cfg(target_os = "windows")]
    let mut go_frame: Option<Arc<Bitmap>> = None;
    #[cfg(target_os = "windows")]
    let mut go_last_attempt = Instant::now() - Duration::from_secs(3);
    #[cfg(target_os = "windows")]
    let mut last_preview_second=61u32;
    let mut last_clock_shift_minute: Option<i64> = None;
    let mut clock_offset: (isize, isize) = (0, 0);
    let mut last_clock_rect: Option<ClockRect> = None;

    // Wait for connect
    let mut last_connect = Instant::now() - Duration::from_secs(1);
    let dev = loop {
        #[cfg(target_os = "windows")]
        settings_ui::poll();
        let mut event = sdl::SDL_Event::default();
        while unsafe { sdl::SDL_PollEvent(&mut event) } {
            if sdl::SDL_EventType(unsafe { event.r#type }) == sdl::SDL_EVENT_QUIT {
                return;
            }
        }
        while let Ok(event) = menu_rx.try_recv() {
            match event {
                MenuEvent::Quit => return,
                #[cfg(target_os = "windows")]
                MenuEvent::OpenSettings => settings_ui::open(),
                #[cfg(target_os = "windows")]
                MenuEvent::SetUiMode(mode) => {
                    config.ui_mode=mode;
                    tm_ui_radio.update_checked(mode);
                    settings_ui::set_tray_mode(mode);
                    save_config(&config);
                }
                #[cfg(target_os = "windows")]
                MenuEvent::SetUiStyle(style) => {
                    config.ui_style=style;
                    tm_ui_style_radio.update_checked(style);
                    save_config(&config);
                    settings_ui::set_style(style);
                }
                #[cfg(target_os = "windows")]
                MenuEvent::SetLanguage(language) => {
                    config.language = language;
                    settings_ui::localize(language);
                    translate_tray(language);
                    save_config(&config);
                }
                #[cfg(target_os = "windows")]
                MenuEvent::SetTheme(theme) => {
                    config.theme = theme;
                    settings_ui::set_theme(theme);
                    save_config(&config);
                }
                MenuEvent::SetClockStyle(style) => {
                    config.clock_style=style;
                    update_clock_style_checked(&tm_clock_style_entries,style);
                    save_config(&config);
                }
                #[cfg(target_os = "windows")]
                MenuEvent::SetGoProfile(kind, filename) => {
                    config.go_profile_filename = filename;
                    config.go_animation = Some(kind);
                    tm_go_anim_radio.update_checked(Some(kind));
                    save_config(&config);
                    // The renderer will start once the SteelSeries device reconnects.
                }
                MenuEvent::SetGoAnimation(anim) => {
                    config.go_animation=anim;
                    config.go_profile_filename=None;
                    #[cfg(target_os = "windows")]
                    tm_go_anim_radio.update_checked(anim);
                    save_config(&config);
                }
                _ => {},
            }
        }
        #[cfg(target_os = "windows")]
        settings_ui::sync(&config);
        if last_connect.elapsed() >= Duration::from_secs(1) {
            last_connect = Instant::now();
            if let Ok(d) = Device::connect() {
                break d;
            }
        }
        sleep(Duration::from_millis(10));
    };
    unsafe { sdl::SDL_SetTrayIcon(tray, icon.surf) };
    #[cfg(target_os = "windows")]
    settings_ui::set_connected(true);
    let mut dev = DrawDevice::new(dev, 30);
    if let Some((font_path, font_size)) = config.font.as_ref().map(|font| (font.path.clone(), font.size)) {
        match TextRenderer::load_from_file(&font_path, font_size) {
            Ok(text_renderer) => dev.set_text_renderer(text_renderer),
            Err(error) => {
                MessageDialog::new()
                    .set_level(MessageLevel::Error)
                    .set_title("GxOled custom font")
                    .set_description(&format!(
                        "Could not load custom font '{}': {error}\nUsing the built-in font instead.",
                        font_path.display()
                    ))
                    .show();
                config.font = None;
                save_config(&config);
            }
        }
    }
    dev.probe();

    // Go!
    dev.set_shift_mode(config.oled_shift.to_api());
    dev.set_volume_osd_hold(Duration::from_millis(config.volume_osd_hold_ms as u64));
    if let Some(brightness) = config.screen_brightness {
        // The selection was persisted; re-apply it to hardware after reconnect/restart.
        dev.set_brightness(brightness);
    }
    if config.animation_playing {
        if let Some(frames) = &animation_frames {
            let first = frames[0].bitmap.clone();
            let (x, y) = dev.center_bitmap(&first);
            animation_layer = Some(dev.add_layer(DrawLayer::Animation {
                frames: frames.clone(),
                x,
                y,
                follow_fps: false,
            }));
        } else {
            config.animation_playing = false;
            save_config(&config);
            unsafe { sdl::SDL_SetTrayEntryChecked(tm_animation_check, false) };
        }
    }
    dev.play();
    'main: loop {
        #[cfg(target_os = "windows")]
        settings_ui::poll();
        // Window event loop
        // TODO: handle system going to sleep
        let mut event = sdl::SDL_Event::default();
        while unsafe { sdl::SDL_PollEvent(&mut event) } {
            let event_type = sdl::SDL_EventType(unsafe { event.r#type });
            if event_type == sdl::SDL_EVENT_QUIT {
                break 'main;
            }
        }

        // Handle tray menu events
        let mut config_updated = false;
        let mut force_redraw = false;


        while let Ok(event) = menu_rx.try_recv() {
            match event {
                MenuEvent::ToggleCheck => {
                    config_updated = true;
                    config.show_media = unsafe { sdl::SDL_GetTrayEntryChecked(tm_media_check) };
                    config.idle_timeout = unsafe { sdl::SDL_GetTrayEntryChecked(tm_idle_check) };
                    if !config.show_media {
                        // Clear immediately, even when last_media was already None.
                        dev.remove_layers(&media_layers);
                        media_layers.clear();
                    }
                    last_media = None;
                    last_clock_shift_minute = None;
                }
                MenuEvent::SetTimeMode(mode) => {
                    config_updated = true;
                    config.time_mode = mode;
                    tm_time_radio.update_checked(mode);
                    last_clock_shift_minute = None;
                    dev.remove_layers(&media_layers);
                    media_layers.clear();
                    last_media = None;
                }
                MenuEvent::SetStatusNotifyMode(mode) => {
                    config_updated = true;
                    config.status_notify = mode;
                    tm_status_notify.update_checked(mode);
                    if animation_layer.is_some() {
                        if let Some(id) = notif_layer.take() {
                            dev.remove_layer(id);
                        }
                    } else {
                        notif_update(&mut dev, &mut notif_layer, mode, is_connected, notif_expiry);
                    }
                    last_clock_shift_minute = None;
                }
                #[cfg(target_os = "windows")]
                MenuEvent::OpenSettings => settings_ui::open(),
                #[cfg(target_os = "windows")]
                MenuEvent::SetUiMode(mode) => {
                    config.ui_mode = mode;
                    tm_ui_radio.update_checked(mode);
                    settings_ui::set_tray_mode(mode);
                    config_updated = true;
                }
                #[cfg(target_os = "windows")]
                MenuEvent::SetUiStyle(style) => {
                    config.ui_style = style;
                    tm_ui_style_radio.update_checked(style);
                    config_updated = true;
                    settings_ui::set_style(style);
                }
                #[cfg(target_os = "windows")]
                MenuEvent::ToggleIdleFromWindow => {
                    config.idle_timeout = !config.idle_timeout;
                    unsafe { sdl::SDL_SetTrayEntryChecked(tm_idle_check,config.idle_timeout); }
                    config_updated = true;
                }
                #[cfg(target_os = "windows")]
                MenuEvent::OpenGifPickerFromWindow => start_gif_file_picker(&menu_tx),
                #[cfg(target_os = "windows")]
                MenuEvent::ToggleGifFromWindow => {
                    let enabled=unsafe{sdl::SDL_GetTrayEntryChecked(tm_animation_check)};
                    unsafe{sdl::SDL_SetTrayEntryChecked(tm_animation_check,!enabled);}
                    let _=menu_tx.send(MenuEvent::ToggleAnimation);
                }
                #[cfg(target_os = "windows")]
                MenuEvent::ToggleStartupFromWindow => {
                    let next=!config.startup;
                    if set_windows_startup(next).is_ok() {
                        config.startup=next;
                        unsafe{sdl::SDL_SetTrayEntryChecked(tm_startup_check,next);}
                        config_updated=true;
                    }
                }
                #[cfg(target_os = "windows")]
                MenuEvent::ToggleMediaFromWindow => {
                    if os.supports_media() {
                        config.show_media = !config.show_media;
                        unsafe { sdl::SDL_SetTrayEntryChecked(tm_media_check,config.show_media); }
                        if !config.show_media {
                            dev.remove_layers(&media_layers);
                            media_layers.clear();
                        }
                        config_updated = true;
                        last_media = None;
                    }
                }
                #[cfg(target_os = "windows")]
                MenuEvent::SetGoProfile(kind, filename) => {
                    config.go_profile_filename = filename;
                    config.go_animation = Some(kind);
                    tm_go_anim_radio.update_checked(Some(kind));
                    save_config(&config);
                    // Replace the current renderer, even when the underlying animation type is unchanged.
                    // (The running-loop handler below also releases the OLED frame.)
                    go_runner=None;
                    if let Some(id)=go_layer.take(){dev.remove_layer(id);}
                    go_frame=None;
                    if let Some(id)=animation_layer.take(){dev.remove_layer(id);}
                    config.animation_playing=false;
                    dev.remove_layers(&time_layers);time_layers.clear();
                    dev.remove_layers(&media_layers);media_layers.clear();
                    last_media=None;
                    config_updated=true;
                    last_time=Local::now()-TimeDelta::seconds(1);
                }
                MenuEvent::SetGoAnimation(anim) => {
                    #[cfg(target_os = "windows")]
                    {
                        go_runner = None;  // Drop kills Go child; no background worker remains.
                        if let Some(id) = go_layer.take() {dev.remove_layer(id);}
                        go_frame = None;
                        config.go_animation = anim;
                        config.go_profile_filename = None;
                        tm_go_anim_radio.update_checked(anim);
                        if anim.is_some() {
                            if let Some(id)=animation_layer.take() {dev.remove_layer(id);}
                            config.animation_playing=false;
                            unsafe {sdl::SDL_SetTrayEntryChecked(tm_animation_check,false);}
                            dev.remove_layers(&time_layers);time_layers.clear();
                            dev.remove_layers(&media_layers);media_layers.clear();
                            if let Some(id)=notif_layer.take(){dev.remove_layer(id);}
                        }
                        config_updated = true;
                        last_time=Local::now()-TimeDelta::seconds(1);
                    }
                    #[cfg(not(target_os = "windows"))]
                    { let _=anim; }
                }
                #[cfg(target_os = "windows")]
                MenuEvent::SetLanguage(language) => {
                    config.language = language;
                    settings_ui::localize(language);
                    translate_tray(language);
                    config_updated = true;
                }
                #[cfg(target_os = "windows")]
                MenuEvent::SetTheme(theme) => {
                    config.theme = theme;
                    settings_ui::set_theme(theme);
                    config_updated = true;
                }
                MenuEvent::SetClockStyle(style) => {
                    #[cfg(target_os = "windows")]
                    {
                        go_runner=None;
                        if let Some(id)=go_layer.take(){dev.remove_layer(id);}
                        go_frame=None;
                        config.go_animation=None;
                        tm_go_anim_radio.update_checked(None);
                    }
                    config_updated = true;
                    config.clock_style = style;
                    update_clock_style_checked(&tm_clock_style_entries, style);
                    last_clock_shift_minute = None;
                    dev.remove_layers(&media_layers);
                    media_layers.clear();
                    last_media = None;
                }
                MenuEvent::SetShiftMode(mode) => {
                    config_updated = true;
                    config.oled_shift = mode;
                    tm_shift_radio.update_checked(mode);
                    dev.set_shift_mode(config.oled_shift.to_api());
                }
                MenuEvent::SetVolumeOsdHold(ms) => {
                    let ms = normalize_volume_osd_hold_ms(ms);
                    config_updated = true;
                    config.volume_osd_hold_ms = ms;
                    tm_volume_osd_radio.update_checked(ms);
                    dev.set_volume_osd_hold(Duration::from_millis(ms as u64));
                }
                MenuEvent::SetClockBurnInMode(mode) => {
                    config_updated = true;
                    config.clock_shift = mode == ClockBurnInMode::Wide;
                    config.clock_shift_no_overlap = mode == ClockBurnInMode::NoOverlap;
                    tm_clock_burn_in_radio.update_checked(mode);
                    // Re-select immediately, even if the user changes modes mid-minute.
                    last_clock_shift_minute = None;
                }
                MenuEvent::SetBrightness(value) => {
                    // The Device is owned by DrawDevice's render thread; request the change
                    // there and update the checked menu item only after hardware confirms it.
                    dev.set_brightness(value);
                }
                #[cfg(target_os = "windows")]
                MenuEvent::ToggleStartup => {
                    let enabled = unsafe { sdl::SDL_GetTrayEntryChecked(tm_startup_check) };
                    if let Err(err) = set_windows_startup(enabled) {
                        unsafe { sdl::SDL_SetTrayEntryChecked(tm_startup_check, config.startup) };
                        MessageDialog::new()
                            .set_level(MessageLevel::Error)
                            .set_title("GxOled")
                            .set_description(&format!("Could not change Windows startup setting: {err}"))
                            .show();
                    } else {
                        config_updated = true;
                        config.startup = enabled;
                    }
                }
                MenuEvent::GifSelected(path) => {
                    let path_string = path.to_string_lossy().into_owned();
                    let decoded = try_decode_gif_frames(&path_string, 0x80);
                    match decoded {
                        Ok(frames) if !frames.is_empty() => {
                            // Replacing the GIF stops the currently running animation.
                            if let Some(id) = animation_layer.take() {
                                dev.remove_layer(id);
                            }
                            animation_frames = None; // Validated, but loaded lazily on Start.
                            let _ = frames;
                            config.animation_path = Some(path.clone());
                            config.animation_playing = false;
                            config_updated = true;
                            unsafe {
                                sdl::SDL_SetTrayEntryChecked(tm_animation_check, false);
                                sdl::SDL_SetTrayEntryEnabled(tm_animation_check, true);
                            }
                            last_time = Local::now() - TimeDelta::seconds(1);
                            last_media = None;
                            force_redraw = true;
                        }
                        Ok(_) => {
                            MessageDialog::new()
                                .set_level(MessageLevel::Error)
                                .set_title("GxOled")
                                .set_description("The selected GIF contains no frames.")
                                .show();
                        }
                        Err(error) => {
                            MessageDialog::new()
                                .set_level(MessageLevel::Error)
                                .set_title("GxOled")
                                .set_description(&format!("Could not load the selected GIF: {error}"))
                                .show();
                        }
                    }
                }
                MenuEvent::ToggleAnimation => {
                    // When returning from a GIF, choose a fresh clock location immediately.
                    last_clock_shift_minute = None;
                    let enabled = unsafe { sdl::SDL_GetTrayEntryChecked(tm_animation_check) };
                    config_updated = true;
                    if enabled {
                        #[cfg(target_os = "windows")]
                        {
                            go_runner = None;
                            if let Some(id)=go_layer.take(){dev.remove_layer(id);}
                            go_frame = None;
                            config.go_animation=None;
                            tm_go_anim_radio.update_checked(None);
                        }
                        if animation_frames.is_none() {
                            animation_frames=config.animation_path.as_ref().and_then(|path|
                                try_decode_gif_frames(&path.to_string_lossy(),0x80).ok());
                        }
                        if let Some(frames) = &animation_frames {
                            let first = frames[0].bitmap.clone();
                            let (x, y) = dev.center_bitmap(&first);
                            dev.remove_layers(&time_layers);
                            dev.remove_layers(&media_layers);
                            if let Some(id) = notif_layer.take() {
                                dev.remove_layer(id);
                            }
                            time_layers.clear();
                            media_layers.clear();
                            last_media = None;
                            animation_layer = Some(dev.add_layer(DrawLayer::Animation {
                                frames: frames.clone(),
                                x,
                                y,
                                follow_fps: false,
                            }));
                            dev.play();
                            config.animation_playing = true;
                            animation_frames=None; // Render thread owns its cloned GIF frames.
                        } else {
                            config.animation_playing = false;
                            unsafe { sdl::SDL_SetTrayEntryChecked(tm_animation_check, false) };
                            MessageDialog::new()
                                .set_level(MessageLevel::Error)
                                .set_title("GxOled")
                                .set_description("Add a GIF first.")
                                .show();
                        }
                    } else {
                        if let Some(id) = animation_layer.take() {
                            dev.remove_layer(id);
                        }
                        config.animation_playing = false;
                        animation_frames=None;
                        last_time = Local::now() - TimeDelta::seconds(1);
                        last_media = None;
                        force_redraw = true;
                    }
                }
                MenuEvent::SetQuitMode(mode) => {
                    config_updated = true;
                    config.quit_mode = mode;
                    tm_quit_mode_radio.update_checked(mode);
                }
                MenuEvent::Quit => break 'main,
            }
        }
        if config_updated {
            save_config(&config);
        }

        force_redraw |= config_updated;
        #[cfg(target_os = "windows")]
        settings_ui::sync(&config);

        #[cfg(target_os = "windows")]
        {
            let idle = config.idle_timeout && os.is_idle();
            if !idle && oled_available && go_runner.is_none() && config.go_animation.is_some()
                && go_last_attempt.elapsed()>=Duration::from_secs(2) {
                go_last_attempt=Instant::now();
                match go_bridge::Runner::start(config.go_animation.unwrap().arg(), config.go_profile_filename.as_deref()) {
                    Ok(runner) => go_runner=Some(runner),
                    Err(err) => {
                        eprintln!("Go animation start failed: {err}");
                        MessageDialog::new().set_level(MessageLevel::Error)
                            .set_title("GxOled Animation")
                            .set_description(&format!("Could not start embedded Go renderer: {err}"))
                            .show();
                        config.go_animation=None;
                        tm_go_anim_radio.update_checked(None);
                        save_config(&config);
                        last_time=Local::now()-TimeDelta::seconds(1);
                    }
                }
            }
            if !idle {
                if let Some(runner)=&go_runner {
                    let mut newest=None;
                    while let Ok(bitmap)=runner.frames.try_recv(){newest=Some(bitmap);}
                    if let Some(bitmap)=newest {
                        let layer=DrawLayer::Image{bitmap:bitmap.clone(),x:0,y:0};
                        if let Some(id)=go_layer {
                            if let Err(layer)=dev.replace_layer(id,layer){go_layer=Some(dev.add_layer(layer));}
                        } else {
                            dev.remove_layers(&time_layers);time_layers.clear();
                            dev.remove_layers(&media_layers);media_layers.clear();
                            if let Some(id)=notif_layer.take(){dev.remove_layer(id);}
                            go_layer=Some(dev.add_layer(layer));
                        }
                        settings_ui::preview(&bitmap);
                        go_frame=Some(bitmap);
                    }
                }
            }
        }

        // Handle events
        while let Some(event) = dev.try_event() {
            #[cfg(debug_assertions)]
            println!("event: {:?}", event);
            match event {
                DrawEvent::DeviceDisconnected => {
                    unsafe { sdl::SDL_SetTrayIcon(tray, icon_error.surf) };
                    #[cfg(target_os = "windows")]
                    {
                        oled_available=false;
                        go_runner=None;
                        if let Some(id)=go_layer.take(){dev.remove_layer(id);}
                        go_frame=None;
                    }
                },
                DrawEvent::DeviceReconnected => {
                    #[cfg(target_os = "windows")]
                    { oled_available=true; }
                    unsafe { sdl::SDL_SetTrayIcon(tray, icon.surf) }
                    dev.probe(); // re-probe when base station reappears
                    // USB reconnection may reset the device-side brightness level.
                    if let Some(brightness) = config.screen_brightness {
                        dev.set_brightness(brightness);
                    }
                }
                DrawEvent::BrightnessSet { value, error } => {
                    if let Some(error) = error {
                        tm_brightness_radio.update_checked(config.screen_brightness.unwrap_or(0));
                        MessageDialog::new()
                            .set_level(MessageLevel::Error)
                            .set_title("GxOled")
                            .set_description(&format!("Could not set screen brightness: {error}"))
                            .show();
                    } else {
                        config.screen_brightness = Some(value);
                        tm_brightness_radio.update_checked(value);
                        save_config(&config);
                    }
                }
                DrawEvent::DeviceEvent(gxoled_lib::DeviceEvent::HeadsetConnection { wireless, .. }) => {
                    if wireless != is_connected {
                        is_connected = wireless;
                        notif_expiry = Local::now() + NOTIF_DUR;
                        if animation_layer.is_some() {
                            if let Some(id) = notif_layer.take() {
                                dev.remove_layer(id);
                            }
                        } else {
                            notif_update(
                                &mut dev,
                                &mut notif_layer,
                                if config.clock_style==ConfigClockStyle::CasioF91W {
                                    StatusNotifyMode::Off
                                } else {config.status_notify},
                                is_connected,
                                notif_expiry,
                            );
                        }
                        last_clock_shift_minute = None;
                        force_redraw = true;
                    }
                }
                DrawEvent::DeviceEvent(_) => {}
            }
        }

        // Update layers every second
        let time = Local::now();
        if time.second() != last_time.second() || force_redraw {
            last_time = time;

            // Remove expired notifications in Notify mode
            if config.status_notify == StatusNotifyMode::Notify
                && time >= notif_expiry
                && let Some(layer) = notif_layer.take()
            {
                dev.remove_layer(layer);
            }

            // Check if idle
            if config.idle_timeout && os.is_idle() {
                #[cfg(target_os = "windows")]
                {
                    go_runner=None;
                    go_layer=None;
                    go_frame=None;
                }
                dev.clear_layers(); // clear screen when idle
                notif_layer = None;
                last_media = None; // reset media so we check again when not idle
                if animation_layer.take().is_some() {
                    config.animation_playing = false;
                    save_config(&config);
                }
                unsafe { sdl::SDL_SetTrayEntryChecked(tm_animation_check, false) };
            } else if animation_layer.is_some() || (cfg!(target_os = "windows") && config.go_animation.is_some()) {
                // Animation owns the OLED while it is active.
                // Keep tray/status processing alive, but don't redraw the clock/media over it.
            } else {
                // Update notifications
                if notif_layer.is_none() {
                    notif_update(
                        &mut dev,
                        &mut notif_layer,
                        if config.clock_style==ConfigClockStyle::CasioF91W {
                            StatusNotifyMode::Off
                        } else {config.status_notify},
                        is_connected,
                        notif_expiry,
                    );
                }

                // Fetch media once a second (before pausing screen)
                let media = if config.show_media && !config.animation_playing && config.go_animation.is_none() {
                    os.get_media().map(Media::sanitize)
                } else {
                    None
                };

                // Time. Update the existing clock layer atomically rather than pausing the
                // render thread; a native tray menu can block this UI loop for several seconds.
                // Media and clock own the full screen exclusively. The old
                // 128x24 overlay caused corrupt-looking combinations on OLED.
                let media_visible = false;
                let clock_hidden_for_media = media.is_some() && config.show_media;
                if config.time_mode != ConfigTimeMode::Off && !clock_hidden_for_media {
                    let time_str = match config.time_mode {
                        ConfigTimeMode::Off => unreachable!(),
                        ConfigTimeMode::H24 => time.format("%H:%M:%S").to_string(),
                        ConfigTimeMode::H12 => {
                            let hour=(time.hour()+11)%12+1;
                            let suffix=if config.language==ConfigLanguage::Turkish {
                                if time.hour()<12 {"ÖÖ"}else{"ÖS"}
                            }else if time.hour()<12 {"AM"}else{"PM"};
                            format!("{hour:02}:{:02}:{:02} {suffix}",time.minute(),time.second())
                        },
                    };
                    // Full-canvas Casio cannot safely use random clock movements.
                    let burn_in_mode = if config.clock_style == ConfigClockStyle::CasioF91W {
                        ClockBurnInMode::Off
                    } else { config.clock_burn_in_mode() };
                    if media != last_media {
                        // Media geometry changes the safe clock area; choose a legal position now.
                        last_clock_shift_minute = None;
                    }
                    if burn_in_mode == ClockBurnInMode::Off {
                        clock_offset = (0, 0);
                        last_clock_shift_minute = None;
                    } else {
                        let minute_key = time.timestamp().div_euclid(60);
                        if last_clock_shift_minute != Some(minute_key) {
                            let obstacles = clock_obstacles(
                                &dev,
                                media.as_ref(),
                                notif_layer.is_some(),
                                config.clock_style,
                            );
                            clock_offset = choose_clock_offset(
                                &dev,
                                &time_str,
                                media_visible,
                                burn_in_mode,
                                last_clock_rect,
                                &obstacles,
                                minute_key,
                                config.clock_style,
                            );
                            last_clock_shift_minute = Some(minute_key);
                        }
                    }
                    let clock_layer = DrawLayer::Clock {
                        style: config.clock_style.to_draw(),
                        twelve_hour: config.time_mode == ConfigTimeMode::H12,
                        x_offset: clock_offset.0,
                        y_offset: clock_offset.1,
                        media_visible,
                        turkish: config.language == ConfigLanguage::Turkish,
                    };
                    if let Some(existing_id) = time_layers.first().copied() {
                        match dev.replace_layer(existing_id, clock_layer) {
                            Ok(()) => {}
                            Err(layer) => time_layers = vec![dev.add_layer(layer)],
                        }
                    } else {
                        time_layers = vec![dev.add_layer(clock_layer)];
                    }

                    // Remember the exact bitmap rectangle actually used this second. The strict
                    // mode compares the next minute's placement against this prior position.
                    let (base_x, base_y, bitmap_w, bitmap_h, screen_w, _) =
                        clock_layout(&dev, &time_str, media_visible, config.clock_style);
                    if bitmap_w > 0 && bitmap_h > 0 {
                        let actual_x = if bitmap_w >= screen_w { 0 } else { base_x + clock_offset.0 };
                        let actual_y = base_y + clock_offset.1;
                        last_clock_rect = Some(ClockRect {
                            x: actual_x,
                            y: actual_y,
                            w: bitmap_w.min(screen_w),
                            h: bitmap_h,
                        });
                    }
                } else {
                    // Keep the last occupied clock rectangle while the clock is disabled,
                    // so re-enabling it in strict mode can still avoid that recent pixel area.
                    dev.remove_layers(&time_layers);
                    time_layers.clear();
                    last_clock_shift_minute = None;
                }

                // Media
                // The visible layer state can outlive last_media (for example when
                // media is turned off in Settings). Never retain such stale pixels.
                let media_layers_expected = media.is_some()
                    && (config.time_mode == ConfigTimeMode::Off || clock_hidden_for_media);
                if media_layers_need_refresh(
                    &media, &last_media, !media_layers.is_empty(), media_layers_expected,
                ) {
                    dev.remove_layers(&media_layers);
                    if let Some(m) = &media {
                        let media_text = format!("{}\n{}", m.title, m.artist);
                        if media_visible {
                            // Two independently sized regions: small clock at the top, metadata
                            // only in the lower band. Text is clipped/truncated in advance so
                            // scrolling and overpainting never collide with the clock.
                            let title: String=m.title.chars().take(40).collect();
                            let artist: String=m.artist.chars().take(40).collect();
                            let text=format!("{}\n{}", title, artist);
                            let mut panel=Bitmap::new(128,20,false);
                            for (index,line) in dev.render_lines_at_size(&text,10.0).iter().take(2).enumerate(){
                                let width=line.w.min(124);
                                let height=line.h.min(10);
                                if width>0 && height>0 {
                                    let clipped=line.crop(0,0,width,height);
                                    panel.blit(&clipped,2,(index*10) as isize,false);
                                }
                            }
                            media_layers=vec![dev.add_layer(DrawLayer::Image{
                                bitmap:Arc::new(panel),x:0,y:39,
                            })];
                        } else if config.time_mode == ConfigTimeMode::Off || clock_hidden_for_media {
                            media_layers = dev.add_text(&media_text, None, Some(12));
                        }
                    }
                    last_media = media;
                }

            }
        }

        #[cfg(target_os = "windows")]
        if time.second() != last_preview_second || force_redraw {
            last_preview_second=time.second();
            if let Some(frame)=&go_frame {settings_ui::preview(frame);}
            else if config.clock_style == ConfigClockStyle::CasioF91W {
                settings_ui::preview(&gxoled_draw::casio_clock_preview(config.time_mode==ConfigTimeMode::H12,config.language==ConfigLanguage::Turkish));
            } else if config.time_mode!=ConfigTimeMode::Off {
                let ts=Local::now().format("%H:%M:%S").to_string();
                let bm=dev.render_lines_at_size(&ts,dev.text_size()).into_iter().next();
                if let Some(bm)=bm {
                    let mut preview=Bitmap::new(128,64,false);
                    let(x,y)=dev.center_bitmap(&bm);
                    preview.blit(&bm,x,y,false);
                    settings_ui::preview(&preview);
                }
            }
        }
        sleep(Duration::from_millis(10));
    }
    #[cfg(target_os = "windows")]
    drop(go_runner);
    let dev = dev.stop();
    match config.quit_mode {
        ConfigQuitMode::Screensaver => dev.draw(&Bitmap::new(dev.width, dev.height, false), 0, 0),
        ConfigQuitMode::StockUi => dev.return_to_ui(),
    }
    .unwrap();
}

#[cfg(test)]
mod media_layer_state_tests {
    use super::{Media, media_layers_need_refresh};

    #[test]
    fn disable_media_clears_stale_layers_even_when_last_media_is_none() {
        assert!(media_layers_need_refresh(&None, &None, true, false));
        assert!(!media_layers_need_refresh(&None, &None, false, false));
    }

    #[test]
    fn stopped_video_clears_previous_media() {
        let previous = Some(Media { title: "Video".into(), artist: "Channel".into() });
        assert!(media_layers_need_refresh(&None, &previous, true, false));
    }

    #[test]
    fn active_media_is_rebuilt_when_layers_are_missing() {
        let media = Some(Media { title: "Playing".into(), artist: "Artist".into() });
        assert!(media_layers_need_refresh(&media, &media, false, true));
        assert!(!media_layers_need_refresh(&media, &media, true, true));
    }
}

#[cfg(all(test, target_os = "windows"))]
mod single_instance_tests {
    use super::another_instance_is_running;

    #[test]
    fn second_call_in_the_same_process_sees_the_mutex() {
        assert!(!another_instance_is_running());
        assert!(another_instance_is_running());
    }
}

#[cfg(test)]
mod display_version_tests {
    use super::display_version_of;

    #[test]
    fn beta_versions_read_as_words_and_stable_ones_are_unchanged() {
        assert_eq!(display_version_of("0.5.2"), "0.5.2");
        assert_eq!(display_version_of("0.5.3-beta"), "0.5.3 Beta");
        assert_eq!(display_version_of("0.5.3-beta.2"), "0.5.3 Beta 2");
    }
}

#[cfg(test)]
mod volume_osd_hold_setting_tests {
    use super::{normalize_volume_osd_hold_ms, Config, DEFAULT_VOLUME_OSD_HOLD_MS, VOLUME_OSD_HOLD_CHOICES_MS};

    #[test]
    fn default_is_two_and_a_half_seconds_and_selectable() {
        assert_eq!(DEFAULT_VOLUME_OSD_HOLD_MS, 2500);
        assert_eq!(Config::default().volume_osd_hold_ms, 2500);
        assert!(VOLUME_OSD_HOLD_CHOICES_MS.contains(&DEFAULT_VOLUME_OSD_HOLD_MS));
        assert_eq!(VOLUME_OSD_HOLD_CHOICES_MS, [1000, 1500, 2000, 2500, 3000, 4000, 5000]);
    }

    #[test]
    fn every_choice_survives_normalisation_and_odd_values_snap_to_the_nearest() {
        for choice in VOLUME_OSD_HOLD_CHOICES_MS {
            assert_eq!(normalize_volume_osd_hold_ms(choice), choice);
        }
        assert_eq!(normalize_volume_osd_hold_ms(0), 1000);
        assert_eq!(normalize_volume_osd_hold_ms(2200), 2000);
        assert_eq!(normalize_volume_osd_hold_ms(2300), 2500);
        assert_eq!(normalize_volume_osd_hold_ms(3500), 3000); // tie: the lower choice wins
        assert_eq!(normalize_volume_osd_hold_ms(60_000), 5000);
    }

    #[test]
    fn a_config_without_the_field_gets_the_default_and_a_stored_value_round_trips() {
        let old = Config::parse_config("show_media = true\n").expect("old config parses");
        assert_eq!(old.volume_osd_hold_ms, 2500);
        let new = Config::parse_config("volume_osd_hold_ms = 4000\n").expect("new config parses");
        assert_eq!(new.volume_osd_hold_ms, 4000);
        let odd = Config::parse_config("volume_osd_hold_ms = 99999\n").expect("odd config parses");
        assert_eq!(odd.volume_osd_hold_ms, 5000);
    }
}
