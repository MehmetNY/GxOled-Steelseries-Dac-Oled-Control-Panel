//! Colour tokens. Values for the dark palette were sampled from the design reference;
//! the light palette keeps the same roles so every widget works in both.

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    /// Window shell behind sidebar + header.
    pub shell: &'static str,
    /// Page background (the content area).
    pub content: &'static str,
    pub card: &'static str,
    pub card_border: &'static str,
    /// Rows, chips and inputs sitting on a card.
    pub row: &'static str,
    pub row_hover: &'static str,
    pub row_border: &'static str,
    pub divider: &'static str,
    pub text: &'static str,
    pub text_dim: &'static str,
    pub text_muted: &'static str,
    pub accent: &'static str,
    pub on_accent: &'static str,
    /// Selected row background / border (accent tinted).
    pub sel_bg: &'static str,
    pub sel_border: &'static str,
    pub accent_tile: &'static str,
    pub ring: &'static str,
    pub toggle_off: &'static str,
    pub green: &'static str,
    pub green_bg: &'static str,
    pub oled_bg: &'static str,
    pub oled_border: &'static str,
    pub shadow: &'static str,
}

pub const DARK: Palette = Palette {
    dark: true,
    shell: "#141414",
    content: "#0f0f0f",
    card: "#1a1a1a",
    card_border: "#262626",
    row: "#222222",
    row_hover: "#292929",
    row_border: "#2b2b2b",
    divider: "#2a2a2a",
    text: "#ffffff",
    text_dim: "#9ca3af",
    text_muted: "#6b7280",
    accent: "#ff6b00",
    on_accent: "#ffffff",
    sel_bg: "#2d2118",
    sel_border: "#5a3414",
    accent_tile: "#372417",
    ring: "#4b4b4b",
    toggle_off: "#3a3a3a",
    green: "#22c55e",
    green_bg: "#14301f",
    oled_bg: "#000000",
    oled_border: "#2a2a2a",
    shadow: "#000000",
};

pub const LIGHT: Palette = Palette {
    dark: false,
    shell: "#ffffff",
    content: "#f4f4f5",
    card: "#ffffff",
    card_border: "#e4e4e7",
    row: "#f4f4f5",
    row_hover: "#ececee",
    row_border: "#e4e4e7",
    divider: "#e4e4e7",
    text: "#111827",
    text_dim: "#4b5563",
    text_muted: "#6b7280",
    accent: "#ff6b00",
    on_accent: "#ffffff",
    sel_bg: "#fff1e6",
    sel_border: "#ffb780",
    accent_tile: "#ffe9d9",
    ring: "#b8bcc4",
    toggle_off: "#c9ccd2",
    green: "#16a34a",
    green_bg: "#dcf5e5",
    oled_bg: "#000000",
    oled_border: "#d4d4d8",
    shadow: "#000000",
};

pub fn palette(dark: bool) -> &'static Palette {
    if dark { &DARK } else { &LIGHT }
}

/// Tinted icon tiles used in card headers (icon colour, tile background) per theme.
#[derive(Clone, Copy, Debug)]
pub enum Tone {
    Orange,
    Indigo,
    Violet,
    Green,
    Red,
    Blue,
    Gray,
}

impl Tone {
    pub fn colors(self, dark: bool) -> (&'static str, &'static str) {
        match (self, dark) {
            (Tone::Orange, true) => ("#ff6b00", "#372417"),
            (Tone::Orange, false) => ("#ea580c", "#ffe9d9"),
            (Tone::Indigo, true) => ("#6366f1", "#1d1f3d"),
            (Tone::Indigo, false) => ("#4f46e5", "#e5e7ff"),
            (Tone::Violet, true) => ("#8b5cf6", "#2a1f45"),
            (Tone::Violet, false) => ("#7c3aed", "#ede5ff"),
            (Tone::Green, true) => ("#22c55e", "#17301f"),
            (Tone::Green, false) => ("#16a34a", "#dcf5e5"),
            (Tone::Red, true) => ("#ef4444", "#3a1d1f"),
            (Tone::Red, false) => ("#dc2626", "#fde4e4"),
            (Tone::Blue, true) => ("#3b82f6", "#1b2640"),
            (Tone::Blue, false) => ("#2563eb", "#dfeaff"),
            (Tone::Gray, true) => ("#9ca3af", "#262626"),
            (Tone::Gray, false) => ("#6b7280", "#e4e4e7"),
        }
    }
}
