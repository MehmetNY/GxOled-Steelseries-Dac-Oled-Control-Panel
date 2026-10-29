//! Data the UI shows (pushed in by the application) and the events it hands back.

use crate::ids;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Tr,
}

impl Lang {
    /// Pick the caption for the active language.
    pub fn t<'a>(self, en: &'a str, tr: &'a str) -> &'a str {
        if self == Lang::Tr { tr } else { en }
    }

    /// Upper-case with the Turkish dotted/dotless i rules ("ayarlar" -> "AYARLAR", "izle" -> "İZLE").
    pub fn upper(self, s: &str) -> String {
        if self == Lang::Tr {
            s.chars()
                .flat_map(|c| match c {
                    'i' => vec!['İ'],
                    'ı' => vec!['I'],
                    c => c.to_uppercase().collect(),
                })
                .collect()
        } else {
            s.to_uppercase()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Page {
    #[default]
    General,
    Clock,
    Animations,
    GifStudio,
    Display,
    Media,
    About,
}

impl Page {
    pub const ALL: [Page; 7] =
        [Page::General, Page::Clock, Page::Animations, Page::GifStudio, Page::Display, Page::Media, Page::About];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    pub fn from_nav_id(id: u32) -> Option<Page> {
        id.checked_sub(ids::NAV_BASE).and_then(|i| Self::ALL.get(i as usize).copied())
    }

    pub fn nav_id(self) -> u32 {
        ids::NAV_BASE + self.index() as u32
    }
}

/// RGBA image (straight alpha) used for the donation QR code.
#[derive(Clone, Debug, Default)]
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct UiModel {
    pub lang: Lang,
    pub dark: bool,
    pub page: Page,
    /// Action ids that are currently "on" (selected radios, enabled toggles).
    pub checked: Vec<u32>,
    /// Screen brightness 1..=10, `None` = device default.
    pub brightness: Option<u8>,
    pub profiles: Vec<String>,
    pub profile_selected: usize,
    /// Row of the profile that is currently playing on the OLED.
    pub profile_running: Option<usize>,
    /// 128×64 mirror of the OLED, row-major, `true` = lit pixel.
    pub oled: Vec<bool>,
    pub connected: bool,
    pub device_name: String,
    /// "v0.5.3 Beta" style label for the footer / info rows.
    pub version: String,
    pub qr: Option<Image>,
    /// Text of the JSON editor window title bar (editor kind only).
    pub editor_title: String,
}

pub const OLED_W: usize = 128;
pub const OLED_H: usize = 64;

impl Default for UiModel {
    fn default() -> Self {
        Self {
            lang: Lang::En,
            dark: true,
            page: Page::General,
            checked: Vec::new(),
            brightness: None,
            profiles: Vec::new(),
            profile_selected: 0,
            profile_running: None,
            oled: vec![false; OLED_W * OLED_H],
            connected: false,
            device_name: "SteelSeries Arctis Nova Pro Wireless".to_owned(),
            version: "v0.0.0".to_owned(),
            qr: None,
            editor_title: String::new(),
        }
    }
}

impl UiModel {
    pub fn is_checked(&self, id: u32) -> bool {
        self.checked.contains(&id)
    }

    pub fn set_oled_bitmap<I: IntoIterator<Item = bool>>(&mut self, pixels: I) {
        for (dst, src) in self.oled.iter_mut().zip(pixels) {
            *dst = src;
        }
    }
}

/// What the window host has to act on.
#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    /// A button / radio / toggle was activated (see `ids`).
    Action(u32),
    /// Brightness slider moved to 1..=10.
    Brightness(u8),
    /// Profile list selection changed to this row.
    ProfileSelected(usize),
    /// Sidebar page changed (already applied in the model; host may want to persist it).
    PageChanged(Page),
    /// Escape was pressed: hide the window.
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Arrow,
    Hand,
}
