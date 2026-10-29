//! The seven pages. Each page draws its left column and returns the y of its bottom edge;
//! the right column (live OLED etc.) is shared and drawn by `right_column`.

use crate::{
    chrome::{self, GAP, HEADER_H, PAD_L, PAD_R, SIDEBAR_W},
    ctx::{ButtonKind, Ctx},
    ids,
    model::{Lang, Page},
    scene::{HitKind, OverlayKind, Rect, Text},
    theme::Tone,
};

/// Choices offered for "volume screen time", in milliseconds (same table as the application).
pub const VOLUME_HOLD_MS: [u32; 7] = [1000, 1500, 2000, 2500, 3000, 4000, 5000];
pub const VOLUME_HOLD_DEFAULT_MS: u32 = 2500;

/// "2.5 s" / "2,5 sn"; with `with_default` the default choice gets a "(default)" suffix.
pub fn volume_label(ms: u32, lang: Lang, with_default: bool) -> String {
    let tr = lang == Lang::Tr;
    let secs = if ms % 1000 == 0 { format!("{}", ms / 1000) } else { format!("{}.{}", ms / 1000, (ms % 1000) / 100) };
    let secs = if tr { secs.replace('.', ",") } else { secs };
    let default = if with_default && ms == VOLUME_HOLD_DEFAULT_MS { if tr { " (varsayılan)" } else { " (default)" } } else { "" };
    format!("{secs} {}{default}", if tr { "sn" } else { "s" })
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub w: f32,
    pub h: f32,
    /// Left column.
    pub lx: f32,
    pub lw: f32,
    /// Right column.
    pub rx: f32,
    pub rw: f32,
    /// Top of the content area (below the header strip).
    pub top: f32,
}

impl Layout {
    pub fn new(w: f32, h: f32) -> Self {
        let rw = (w * 0.336).clamp(340.0, 520.0);
        let gap = if w >= 1400.0 { 52.0 } else { 34.0 };
        let rx = w - PAD_R - rw;
        let lx = SIDEBAR_W + PAD_L;
        let lw = (rx - gap - lx).max(300.0);
        Layout { w, h, lx, lw, rx, rw, top: HEADER_H }
    }

    pub fn two_cols(&self) -> bool {
        self.lw >= 560.0
    }

    /// Width of one card when a row holds two.
    pub fn half(&self) -> f32 {
        (self.lw - GAP) / 2.0
    }
}

pub struct Built {
    /// Bottom of the tallest column (content coordinates).
    pub bottom: f32,
}

pub fn build(c: &mut Ctx, l: &Layout) -> Built {
    let page = c.m.page;
    let (title, sub) = heading(c, page);
    let y = l.top + 32.0;
    let after = c.page_heading(l.lx, y, title, sub);
    let y0 = after + 30.0;
    let left_bottom = match page {
        Page::General => general(c, l, y0),
        Page::Clock => clock(c, l, y0),
        Page::Animations => animations(c, l, y0),
        Page::GifStudio => gif_studio(c, l, y0),
        Page::Display => display(c, l, y0),
        Page::Media => media(c, l, y0),
        Page::About => about_left(c, l, y0),
    };
    let right_bottom = right_column(c, l, page);
    Built { bottom: left_bottom.max(right_bottom) }
}

fn heading<'a>(c: &Ctx<'a>, page: Page) -> (&'a str, &'a str) {
    match page {
        Page::General => (
            c.t("General Control", "Genel Kontrol"),
            c.t("Configure application behavior and appearance", "Uygulama davranışını ve görünümünü ayarlayın"),
        ),
        Page::Clock => (
            c.t("Clock Gallery", "Saat Galerisi"),
            c.t("Choose how the time appears on the OLED", "Saatin OLED'de nasıl görüneceğini seçin"),
        ),
        Page::Animations => (
            c.t("Animations", "Animasyonlar"),
            c.t("Run and manage 128×64 animation profiles", "128×64 animasyon profillerini çalıştırın ve yönetin"),
        ),
        Page::GifStudio => (
            c.t("GIF Studio", "GIF Studio"),
            c.t("Play your own GIFs on the OLED", "Kendi GIF'lerinizi OLED'de oynatın"),
        ),
        Page::Display => (
            c.t("Display Control", "Ekran Kontrolü"),
            c.t("Brightness, burn-in protection and status icon", "Parlaklık, yanık izi koruması ve durum simgesi"),
        ),
        Page::Media => (
            c.t("Media", "Medya"),
            c.t("Show what is playing on the OLED", "Çalan müziği OLED'de gösterin"),
        ),
        Page::About => (
            c.t("About", "Hakkında"),
            c.t("GxOled Advanced and its open-source credits", "GxOled Advanced ve açık kaynak katkıları"),
        ),
    }
}

// ───────────────────────── right column ─────────────────────────

fn right_column(c: &mut Ctx, l: &Layout, page: Page) -> f32 {
    let mut y = l.top + 36.0;
    if page == Page::About {
        let r = qr_card(c, l.rx, y, l.rw);
        let f = chrome::footer_chip(c, l.rx, r.bottom() + GAP, l.rw);
        return f.bottom();
    }
    let live = chrome::live_panel(c, l.rx, y, l.rw);
    y = live.bottom() + GAP;
    let card = if page == Page::General { chrome::quick_access(c, l.rx, y, l.rw) } else { setup_card(c, l.rx, y, l.rw) };
    let f = chrome::footer_chip(c, l.rx, card.bottom() + GAP, l.rw);
    f.bottom()
}

fn label_of<'a>(c: &Ctx, table: &[(u32, &'a str)]) -> Option<&'a str> {
    table.iter().find(|(id, _)| c.checked(*id)).map(|(_, s)| *s)
}

/// "Current setup" summary shown under the live OLED on every page except General/About.
fn setup_card(c: &mut Ctx, x: f32, y: f32, w: f32) -> Rect {
    let rows = 4.0;
    let r = Rect::new(x, y, w, 24.0 + 35.0 + 12.0 + rows * 31.0 + 18.0);
    let _ = c.card(r, "sliders-horizontal", Tone::Orange, c.t("Current setup", "Mevcut ayarlar"));
    let style = label_of(
        c,
        &[
            (ids::STYLE_DIGITAL, c.t("Digital Standard", "Dijital Standart")),
            (ids::STYLE_DIGITAL_BOLD, c.t("Digital Big", "Dijital Büyük")),
            (ids::STYLE_ANALOG_ROUND, c.t("Analog Round", "Analog Yuvarlak")),
            (ids::STYLE_ANALOG_SQUARE, c.t("Analog Square", "Analog Kare")),
            (ids::STYLE_CASIO, "Casio Digital Dial"),
        ],
    )
    .unwrap_or("—");
    let time = label_of(
        c,
        &[(ids::TIME_OFF, c.t("Off", "Kapalı")), (ids::TIME_24, c.t("24-hour", "24 saat")), (ids::TIME_12, c.t("12-hour", "12 saat"))],
    )
    .unwrap_or("—");
    let anim = c
        .m
        .profile_running
        .and_then(|i| c.m.profiles.get(i).cloned())
        .unwrap_or_else(|| c.t("None (clock)", "Yok (saat)").to_owned());
    let bright = match c.m.brightness {
        Some(v) => format!("{}%", v as u32 * 10),
        None => c.t("Device default", "Cihaz varsayılanı").to_owned(),
    };
    let mut ry = y + 24.0 + 35.0 + 12.0;
    let (x0, x1) = (x + 24.0, r.right() - 24.0);
    c.kv_row(x0, x1, ry, c.t("Clock style", "Saat tarzı"), style, true);
    ry += 31.0;
    c.kv_row(x0, x1, ry, c.t("Time display", "Saat gösterimi"), time, true);
    ry += 31.0;
    c.kv_row(x0, x1, ry, c.t("Animation", "Animasyon"), &anim, true);
    ry += 31.0;
    c.kv_row(x0, x1, ry, c.t("Brightness", "Parlaklık"), &bright, false);
    r
}

fn qr_card(c: &mut Ctx, x: f32, y: f32, w: f32) -> Rect {
    let p = c.p;
    let qr = (w - 96.0).min(260.0);
    let h = 24.0 + 35.0 + 18.0 + qr + 22.0 + 52.0 + 24.0;
    let r = Rect::new(x, y, w, h);
    let body = c.card(r, "coffee", Tone::Orange, c.t("Buy me a coffee", "Bana bir kahve ısmarla"));
    let qx = x + (w - qr) / 2.0;
    let qrect = Rect::new(qx, body - 2.0, qr, qr);
    c.sc.rect(qrect.inset(-8.0), 12.0, "#ffffff");
    if c.m.qr.is_some() {
        c.sc.overlay(qrect, OverlayKind::Qr, 0.0);
    }
    let cy = qrect.bottom() + 30.0;
    c.sc.text_mid(r.cx(), cy, "USDT · TRON (TRC20)", &Text::new(15.0, 600, p.text).mid());
    c.sc.text_mid(r.cx(), cy + 26.0, c.t("Scan the QR code to support the project.", "Projeyi desteklemek için QR kodu tarayın."), &Text::new(13.0, 400, p.text_muted).mid());
    r
}

// ───────────────────────── General ─────────────────────────

fn general(c: &mut Ctx, l: &Layout, y: f32) -> f32 {
    // App opening mode
    let rows_h = 3.0 * 44.0 + 2.0 * 14.0;
    let card = Rect::new(l.lx, y, l.lw, 79.0 + rows_h + 24.0);
    let mut ry = c.card(card, "layout-template", Tone::Orange, c.t("App Opening Mode", "Uygulama Açılış Modu"));
    let selected = c.t("Selected", "Seçili");
    let modes = [
        (ids::MODE_TRAY, c.t("Tray menu", "Tepsi menüsü"), c.t("Minimizes to system tray", "Sistem tepsisine küçülür")),
        (ids::MODE_WINDOW, c.t("Window", "Pencere"), c.t("Tray icon opens settings", "Tepsi simgesi ayarları açar")),
        (ids::MODE_HYBRID, c.t("Hybrid", "Karma"), c.t("Left = window, right = menu", "Sol = pencere, sağ = menü")),
    ];
    for (id, label, hint) in modes {
        c.radio_row_selected_hint(Rect::new(l.lx + 24.0, ry, l.lw - 48.0, 44.0), id, label, hint, selected);
        ry += 58.0;
    }
    let mut y = card.bottom() + GAP;

    let two = l.two_cols();
    let (hw, col2) = if two { (l.half(), l.lx + l.half() + GAP) } else { (l.lw, l.lx) };

    // Language + appearance
    let lang_card = Rect::new(l.lx, y, hw, 141.0);
    let by = c.card(lang_card, "globe", Tone::Indigo, "Language / Dil");
    let r1 = c.chip(l.lx + 24.0, by, 39.0, ids::LANG_EN, "English", None, 0.0);
    c.chip(r1.right() + 14.0, by, 39.0, ids::LANG_TR, "Türkçe", None, 0.0);
    if !two {
        y = lang_card.bottom() + GAP;
    }
    let app_card = Rect::new(col2, if two { y } else { y }, hw, 141.0);
    let by = c.card(app_card, "sun", Tone::Violet, c.t("Appearance", "Görünüm"));
    let r1 = c.chip(col2 + 24.0, by, 39.0, ids::THEME_LIGHT, c.t("Light", "Açık"), Some("sun"), 0.0);
    c.chip(r1.right() + 14.0, by, 39.0, ids::THEME_DARK, c.t("Dark", "Koyu"), Some("moon"), 0.0);
    y = app_card.bottom() + GAP;

    // Start with Windows (+ interface style) | When GxOled closes
    let start = Rect::new(l.lx, y, hw, 73.0);
    c.toggle_card(start, ids::STARTUP, "power", Tone::Green, c.t("Start with Windows", "Windows ile başlat"), c.t("Launch on system startup", "Sistem açılışında başlat"));
    let mut left_bottom = start.bottom();

    let close_y = if two { y } else { start.bottom() + GAP };
    let close = Rect::new(col2, close_y, hw, 79.0 + 33.0 + 8.5 + 33.0 + 20.0);
    let mut ry = c.card(close, "circle-x", Tone::Red, c.t("When GxOled Closes", "GxOled Kapanırken"));
    c.radio_row(Rect::new(col2 + 24.0, ry, hw - 48.0, 33.0), ids::QUIT_SCREENSAVER, c.t("Screensaver", "Ekran koruyucu"), None);
    ry += 41.5;
    c.radio_row(Rect::new(col2 + 24.0, ry, hw - 48.0, 33.0), ids::QUIT_STOCK_UI, c.t("SteelSeries stock UI", "SteelSeries varsayılanı"), None);

    // Interface style (beta) under the toggle card
    let iy = if two { start.bottom() + GAP } else { close.bottom() + GAP };
    let ix = l.lx;
    let iface = Rect::new(ix, iy, hw, 141.0);
    let by = c.card(iface, "app-window", Tone::Blue, c.t("Settings window (Beta)", "Ayar penceresi (Beta)"));
    let r1 = c.chip(ix + 24.0, by, 39.0, ids::UI_MODERN, "Modern", None, 0.0);
    c.chip(r1.right() + 14.0, by, 39.0, ids::UI_CLASSIC, c.t("Classic", "Klasik"), None, 0.0);
    left_bottom = left_bottom.max(iface.bottom()).max(close.bottom());
    left_bottom
}

// ───────────────────────── Clock ─────────────────────────

/// Small vector stand-ins for the five clock faces, drawn on a black "OLED" tile.
fn clock_face(c: &mut Ctx, kind: u32, r: Rect) {
    let white = "#ffffff";
    let (cx, cy) = (r.cx(), r.cy());
    let s = r.h;
    match kind {
        ids::STYLE_DIGITAL => {
            c.sc.text_mid(cx, cy, "10:08", &Text::new(s * 0.34, 500, white).mid().spaced(1.0));
        }
        ids::STYLE_DIGITAL_BOLD => {
            c.sc.text_mid(cx, cy, "10:08", &Text::new(s * 0.56, 700, white).mid());
        }
        ids::STYLE_ANALOG_ROUND => {
            let rad = s * 0.40;
            c.sc.ring(cx, cy, rad, white, 1.6);
            for i in 0..12 {
                let a = (i as f32) * std::f32::consts::TAU / 12.0;
                let (sn, cs) = a.sin_cos();
                c.sc.line(cx + sn * rad * 0.84, cy - cs * rad * 0.84, cx + sn * rad * 0.94, cy - cs * rad * 0.94, white, 1.2);
            }
            c.sc.line(cx, cy, cx + rad * 0.0, cy - rad * 0.62, white, 2.0);
            c.sc.line(cx, cy, cx + rad * 0.5, cy + rad * 0.2, white, 1.6);
        }
        ids::STYLE_ANALOG_SQUARE => {
            let side = s * 0.74;
            c.sc.rect_border(Rect::new(cx - side / 2.0, cy - side / 2.0, side, side), 6.0, "none", white, 1.6);
            for i in 0..4 {
                let a = (i as f32) * std::f32::consts::FRAC_PI_2;
                let (sn, cs) = a.sin_cos();
                let h = side / 2.0;
                c.sc.line(cx + sn * h * 0.74, cy - cs * h * 0.74, cx + sn * h * 0.9, cy - cs * h * 0.9, white, 1.4);
            }
            c.sc.line(cx, cy, cx, cy - side * 0.32, white, 2.0);
            c.sc.line(cx, cy, cx + side * 0.26, cy + side * 0.1, white, 1.6);
        }
        _ => {
            let w = r.w * 0.72;
            let lcd = Rect::new(cx - w / 2.0, cy - s * 0.30, w, s * 0.60);
            c.sc.rect_border(lcd, 4.0, "none", white, 1.2);
            c.sc.text_mid(cx - w * 0.06, cy + s * 0.02, "10:08", &Text::new(s * 0.30, 700, white).mid());
            c.sc.text_mid(lcd.right() - 8.0, cy + s * 0.08, "58", &Text::new(s * 0.17, 600, white).end());
            c.sc.text_mid(lcd.x + 9.0, cy - s * 0.17, "WED", &Text::new(s * 0.11, 600, white));
        }
    }
}

fn clock(c: &mut Ctx, l: &Layout, y: f32) -> f32 {
    let styles = [
        (ids::STYLE_DIGITAL, c.t("Digital Standard", "Dijital Standart")),
        (ids::STYLE_DIGITAL_BOLD, c.t("Digital Big", "Dijital Büyük")),
        (ids::STYLE_ANALOG_ROUND, c.t("Analog Round", "Analog Yuvarlak")),
        (ids::STYLE_ANALOG_SQUARE, c.t("Analog Square", "Analog Kare")),
        (ids::STYLE_CASIO, "Casio Digital Dial"),
    ];
    let inner = l.lw - 48.0;
    let cols: usize = if inner >= 560.0 { 3 } else if inner >= 360.0 { 2 } else { 1 };
    let gap = 14.0;
    let tw = (inner - gap * (cols as f32 - 1.0)) / cols as f32;
    let pv_h = (tw - 20.0) / 2.0;
    let th = 10.0 + pv_h + 10.0 + 33.0 + 10.0;
    let rows = styles.len().div_ceil(cols) as f32;
    let card = Rect::new(l.lx, y, l.lw, 79.0 + rows * th + (rows - 1.0) * gap + 24.0);
    let by = c.card(card, "watch", Tone::Orange, c.t("Clock style", "Saat tarzı"));
    for (i, (id, label)) in styles.iter().enumerate() {
        let (col, row) = (i % cols, i / cols);
        let t = Rect::new(l.lx + 24.0 + col as f32 * (tw + gap), by + row as f32 * (th + gap), tw, th);
        let on = c.checked(*id);
        let (bg, border) = if on {
            (c.p.sel_bg, c.p.sel_border)
        } else if c.pressed(*id) {
            (c.p.sel_bg, c.p.sel_border)
        } else if c.hovered(*id) {
            (c.p.row_hover, c.p.row_border)
        } else {
            (c.p.row, c.p.row_border)
        };
        c.sc.rect_border(t, 13.0, bg, border, 1.0);
        let screen = Rect::new(t.x + 10.0, t.y + 10.0, tw - 20.0, pv_h);
        c.sc.rect_border(screen, 8.0, c.p.oled_bg, c.p.oled_border, 1.0);
        clock_face(c, *id, screen);
        let cy = screen.bottom() + 10.0 + 16.5;
        let accent = c.p.accent;
        let ring = c.p.ring;
        if on {
            c.sc.circle(t.x + 24.0, cy, 6.5, accent);
        } else {
            c.sc.ring(t.x + 24.0, cy, 6.5 - 0.75, if c.hovered(*id) { accent } else { ring }, 1.8);
        }
        let fg = if on || c.hovered(*id) { c.p.text } else { c.p.text_dim };
        let label = c.fit(label, 15.0, 400, tw - 24.0 - 22.0);
        c.sc.text_mid(t.x + 40.0, cy, &label, &Text::new(15.0, 400, fg));
        c.sc.hit(t, *id, HitKind::Click, true);
        c.focus_ring(*id, t, 13.0);
    }
    let mut y = card.bottom() + GAP;
    let tcard = Rect::new(l.lx, y, l.lw, 141.0);
    let by = c.card(tcard, "timer", Tone::Blue, c.t("Time display", "Saat gösterimi"));
    let opts = [(ids::TIME_OFF, c.t("Off", "Kapalı")), (ids::TIME_24, c.t("24-hour", "24 saat")), (ids::TIME_12, c.t("12-hour (AM/PM)", "12 saat (ÖÖ/ÖS)"))];
    let mut x = l.lx + 24.0;
    for (id, label) in opts {
        let r = c.chip(x, by, 39.0, id, label, None, 0.0);
        x = r.right() + 14.0;
    }
    y = tcard.bottom();
    y
}

// ───────────────────────── Animations ─────────────────────────

const LIST_ROW_H: f32 = 40.0;
const LIST_VISIBLE: usize = 8;

pub fn list_visible_rows() -> usize {
    LIST_VISIBLE
}

fn animations(c: &mut Ctx, l: &Layout, y: f32) -> f32 {
    let p = c.p;
    let n = c.m.profiles.len();
    let rows_shown = n.clamp(1, LIST_VISIBLE);
    let list_h = rows_shown as f32 * LIST_ROW_H + (rows_shown as f32 - 1.0) * 6.0;
    let card = Rect::new(l.lx, y, l.lw, 79.0 + list_h + 18.0 + 40.0 + 24.0);
    let by = c.card(card, "film", Tone::Violet, c.t("128×64 animation profiles", "128×64 animasyon profilleri"));
    let area = Rect::new(l.lx + 24.0, by, l.lw - 48.0, list_h);
    c.sc.begin_clip(area, 0.0);
    let first = c.list_offset();
    for (i, name) in c.m.profiles.iter().enumerate().skip(first).take(LIST_VISIBLE) {
        let row_i = i - first;
        let r = Rect::new(area.x, area.y + row_i as f32 * (LIST_ROW_H + 6.0), area.w - if n > LIST_VISIBLE { 14.0 } else { 0.0 }, LIST_ROW_H);
        let id = ids::LIST_BASE + i as u32;
        let sel = c.m.profile_selected == i;
        let running = c.m.profile_running == Some(i);
        let (bg, border, fg) = if sel {
            (p.sel_bg, p.sel_border, p.text)
        } else if c.hovered(id) {
            (p.row_hover, p.row_border, p.text)
        } else {
            (p.row, p.row_border, p.text_dim)
        };
        c.sc.rect_border(r, 10.0, bg, border, 1.0);
        let icon_c = if running { p.green } else if sel { p.accent } else { p.text_muted };
        c.sc.icon(if running { "circle-play" } else { "film" }, r.x + 13.0, r.cy() - 8.5, 17.0, icon_c, 2.0);
        let max = r.w - 44.0 - if running { 100.0 } else { 14.0 };
        let name = c.fit(name, 15.0, 400, max);
        c.sc.text_mid(r.x + 40.0, r.cy(), &name, &Text::new(15.0, 400, fg));
        if running {
            let label = c.t("Running", "Çalışıyor");
            let w = c.eng.text_width(label, 13.0, 600, 0.0) + 22.0;
            let pill = Rect::new(r.right() - 12.0 - w, r.cy() - 12.0, w, 24.0);
            c.sc.rect(pill, 8.0, p.green_bg);
            c.sc.text_mid(pill.cx(), pill.cy(), label, &Text::new(13.0, 600, p.green).mid());
        }
        c.sc.hit(r, id, HitKind::ListRow, true);
        c.focus_ring(id, r, 10.0);
    }
    c.sc.end_group();
    if n > LIST_VISIBLE {
        // Thin scrollbar for the list.
        let track = Rect::new(area.right() - 6.0, area.y, 6.0, area.h);
        c.sc.rect(track, 3.0, p.row);
        let thumb_h = (area.h * LIST_VISIBLE as f32 / n as f32).max(24.0);
        let max_first = (n - LIST_VISIBLE) as f32;
        let ty = area.y + (area.h - thumb_h) * (first as f32 / max_first);
        c.sc.rect(Rect::new(track.x, ty, 6.0, thumb_h), 3.0, p.text_muted);
    }
    c.sc.list_area = Some((area, n));
    let bw = (l.lw - 48.0 - 14.0) / 2.0;
    let by2 = area.bottom() + 18.0;
    c.button(Rect::new(l.lx + 24.0, by2, bw, 40.0), ids::PROFILE_SELECT, c.t("Use selected", "Seçili profili uygula"), ButtonKind::Primary, Some("circle-play"));
    c.button(Rect::new(l.lx + 24.0 + bw + 14.0, by2, bw, 40.0), ids::ANIM_STOP, c.t("Stop animation / Clock", "Animasyonu durdur / Saat"), ButtonKind::Secondary, Some("circle-stop"));

    let y2 = card.bottom() + GAP;
    let note1 = c.t("Create any number of presets or import/export JSON.", "Sınırsız özel profil oluşturun veya JSON içe/dışa aktarın.");
    let note2 = c.t("Custom profiles folder:", "Özel profil klasörü:");
    let folder = "%APPDATA%\\GxOled\\profiles\\custom";
    let note3 = c.t(
        "New Profile opens an empty JSON. Use Reset in the editor for a starter template.",
        "Yeni Profil boş bir JSON ile açılır. Başlangıç şablonu için düzenleyicide Sıfırla'yı kullanın.",
    );
    let text_w = l.lw - 48.0;
    let notes_h = c.paragraph_height(text_w, note1, 13.5) + 8.0 + c.paragraph_height(text_w, note2, 13.5) + c.paragraph_height(text_w, folder, 13.5) + 8.0 + c.paragraph_height(text_w, note3, 13.5);
    let two_rows = l.lw - 48.0 < 520.0;
    let btn_rows = if two_rows { 2.0 } else { 1.0 };
    let card2 = Rect::new(l.lx, y2, l.lw, 79.0 + btn_rows * 40.0 + (btn_rows - 1.0) * 12.0 + 20.0 + notes_h + 20.0);
    let by = c.card(card2, "wand-sparkles", Tone::Orange, c.t("Manage profiles", "Profilleri yönet"));
    let btns: [(u32, &str, &str, ButtonKind); 5] = [
        (ids::PROFILE_EDIT, c.t("Edit…", "Düzenle…"), "square-pen", ButtonKind::Secondary),
        (ids::PROFILE_NEW, c.t("New", "Yeni"), "plus", ButtonKind::Secondary),
        (ids::PROFILE_IMPORT, c.t("Import", "İçe aktar"), "file-up", ButtonKind::Secondary),
        (ids::PROFILE_EXPORT, c.t("Export", "Dışa aktar"), "file-down", ButtonKind::Secondary),
        (ids::PROFILE_DELETE, c.t("Delete", "Sil"), "trash-2", ButtonKind::Danger),
    ];
    let per_row = if two_rows { 3 } else { 5 };
    let bw = (l.lw - 48.0 - 12.0 * (per_row as f32 - 1.0)) / per_row as f32;
    for (i, (id, label, icon, kind)) in btns.iter().enumerate() {
        let (col, row) = (i % per_row, i / per_row);
        let r = Rect::new(l.lx + 24.0 + col as f32 * (bw + 12.0), by + row as f32 * 52.0, bw, 40.0);
        c.button(r, *id, label, *kind, Some(icon));
    }
    let mut ty = by + btn_rows * 40.0 + (btn_rows - 1.0) * 12.0 + 20.0;
    ty = c.paragraph(l.lx + 24.0, ty, text_w, note1, 13.5, p.text_muted) + 8.0;
    ty = c.paragraph(l.lx + 24.0, ty, text_w, note2, 13.5, p.text_muted);
    ty = c.paragraph(l.lx + 24.0, ty, text_w, folder, 13.5, p.text_dim) + 8.0;
    c.paragraph(l.lx + 24.0, ty, text_w, note3, 13.5, p.text_muted);
    card2.bottom()
}

// ───────────────────────── GIF Studio ─────────────────────────

fn gif_studio(c: &mut Ctx, l: &Layout, y: f32) -> f32 {
    let p = c.p;
    let desc = c.t(
        "Add a GIF, then start or stop playback on the 128×64 OLED. Editing tools will live here later.",
        "Bir GIF ekleyin, sonra 128×64 OLED'de oynatmayı başlatın veya durdurun. Düzenleme araçları daha sonra burada olacak.",
    );
    let text_w = l.lw - 48.0;
    let dh = c.paragraph_height(text_w, desc, 15.0);
    let card = Rect::new(l.lx, y, l.lw, 79.0 + dh + 20.0 + 44.0 + 26.0);
    let by = c.card(card, "image-play", Tone::Blue, c.t("GIF Library", "GIF Kitaplığı"));
    let after = c.paragraph(l.lx + 24.0, by - 4.0, text_w, desc, 15.0, p.text_muted);
    let bw = (text_w - 14.0) / 2.0;
    c.button(Rect::new(l.lx + 24.0, after + 20.0, bw, 44.0), ids::GIF_ADD, c.t("Add GIF…", "GIF ekle…"), ButtonKind::Primary, Some("folder-plus"));
    c.button(Rect::new(l.lx + 24.0 + bw + 14.0, after + 20.0, bw, 44.0), ids::GIF_TOGGLE, c.t("Start / Stop GIF", "GIF başlat / durdur"), ButtonKind::Secondary, Some("circle-play"));
    card.bottom()
}

// ───────────────────────── Display ─────────────────────────

/// Column plan for a grid of equal-width chips: the largest column count (≤ `max_cols`) in which every label fits.
struct GridPlan {
    cols: usize,
    cw: f32,
    rows: usize,
}

impl GridPlan {
    const CHIP_H: f32 = 39.0;
    const GAP: f32 = 10.0;

    fn new(c: &Ctx, w: f32, items: &[(u32, String, Option<String>)], max_cols: usize) -> Self {
        let need = items
            .iter()
            .map(|(_, label, hint)| {
                let lw = c.eng.text_width(label, 16.0, 400, 0.0);
                let hw = hint.as_ref().map_or(0.0, |h| c.eng.text_width(h, 12.5, 400, 0.0) + 10.0);
                40.0 + lw + hw + 12.0
            })
            .fold(0.0, f32::max);
        let mut cols = max_cols.min(items.len()).max(1);
        while cols > 1 && (w - Self::GAP * (cols as f32 - 1.0)) / (cols as f32) < need {
            cols -= 1;
        }
        let cw = (w - Self::GAP * (cols as f32 - 1.0)) / cols as f32;
        Self { cols, cw, rows: items.len().div_ceil(cols) }
    }

    fn height(&self) -> f32 {
        self.rows as f32 * Self::CHIP_H + (self.rows as f32 - 1.0) * Self::GAP
    }

    fn draw(&self, c: &mut Ctx, x: f32, y: f32, items: &[(u32, String, Option<String>)]) {
        for (i, (id, label, hint)) in items.iter().enumerate() {
            let (col, row) = (i % self.cols, i / self.cols);
            let r = Rect::new(
                x + col as f32 * (self.cw + Self::GAP),
                y + row as f32 * (Self::CHIP_H + Self::GAP),
                self.cw,
                Self::CHIP_H,
            );
            c.chip_w(r, *id, label, None, hint.as_deref());
        }
    }
}

/// Card with a vertical list of radio rows (same height for every list of three).
fn radio_card(c: &mut Ctx, r: Rect, icon: &str, tone: Tone, title: &str, items: &[(u32, &str)]) {
    let mut by = c.card(r, icon, tone, title);
    for (id, label) in items {
        c.radio_row(Rect::new(r.x + 24.0, by, r.w - 48.0, 33.0), *id, label, None);
        by += 41.5;
    }
}

fn display(c: &mut Ctx, l: &Layout, y: f32) -> f32 {
    let two = l.two_cols();
    let hw = l.half();
    let text_w = l.lw - 48.0;

    // Screensaver toggle: full width so its title never needs truncating.
    let idle = Rect::new(l.lx, y, l.lw, 73.0);
    c.toggle_card(
        idle,
        ids::IDLE,
        "moon",
        Tone::Indigo,
        c.t("Screensaver when idle", "Boştayken ekran koruyucu"),
        c.t("Dim and animate when idle", "Boştayken karart ve hareketlendir"),
    );
    let y = idle.bottom() + GAP;

    // OLED screen shift | Clock burn-in protection: two equal radio cards (stacked on narrow windows).
    let radio_h = 79.0 + 3.0 * 33.0 + 2.0 * 8.5 + 20.0;
    let (ra, rb) = if two {
        (Rect::new(l.lx, y, hw, radio_h), Rect::new(l.lx + hw + GAP, y, hw, radio_h))
    } else {
        (Rect::new(l.lx, y, l.lw, radio_h), Rect::new(l.lx, y + radio_h + GAP, l.lw, radio_h))
    };
    radio_card(
        c,
        ra,
        "move",
        Tone::Violet,
        c.t("OLED screen shift", "OLED ekran kaydırma"),
        &[(ids::SHIFT_OFF, c.t("Off", "Kapalı")), (ids::SHIFT_LOW, c.t("Low", "Düşük")), (ids::SHIFT_HIGH, c.t("High", "Yüksek"))],
    );
    radio_card(
        c,
        rb,
        "shield",
        Tone::Green,
        c.t("Clock burn-in protection", "Saat yanık izi koruması"),
        &[
            (ids::BURN_OFF, c.t("Off", "Kapalı")),
            (ids::BURN_WIDE, c.t("Random Movements", "Rastgele hareket")),
            (ids::BURN_NO_OVERLAP, c.t("No overlapping Pixels", "Çakışmayan pikseller")),
        ],
    );
    let y = (if two { ra.bottom() } else { rb.bottom() }) + GAP;

    // Connection status icon: chip grid, full width.
    let notify: Vec<(u32, String, Option<String>)> = [
        (ids::NOTIFY_OFF, c.t("Off", "Kapalı")),
        (ids::NOTIFY_NOTIFY, c.t("Notify", "Bildir")),
        (ids::NOTIFY_ALWAYS, c.t("Always", "Her zaman")),
        (ids::NOTIFY_CONNECTED, c.t("When connected", "Bağlıyken")),
        (ids::NOTIFY_DISCONNECTED, c.t("When disconnected", "Bağlı değilken")),
    ]
    .into_iter()
    .map(|(id, label)| (id, label.to_owned(), None))
    .collect();
    let plan = GridPlan::new(c, text_w, &notify, 3);
    let card = Rect::new(l.lx, y, l.lw, 79.0 + plan.height() + 24.0);
    let by = c.card(card, "wifi", Tone::Blue, c.t("Connection status icon", "Bağlantı durumu simgesi"));
    plan.draw(c, l.lx + 24.0, by, &notify);
    let y = card.bottom() + GAP;

    // Brightness
    let bright = Rect::new(l.lx, y, l.lw, 79.0 + 34.0 + 24.0);
    let by = c.card(bright, "sun-medium", Tone::Orange, c.t("Screen brightness", "Ekran parlaklığı"));
    slider(c, Rect::new(l.lx + 24.0, by + 4.0, text_w, 28.0));
    let y = bright.bottom() + GAP;

    // Volume screen time
    let note = c.t(
        "After you turn the DAC knob its own volume screen stays this long, then GxOled returns.",
        "DAC düğmesini çevirince cihazın kendi ses ekranı bu kadar kalır, sonra GxOled geri döner.",
    );
    let nh = c.paragraph_height(text_w, note, 13.5);
    let lang = c.lang();
    let default_word = c.t("default", "varsayılan");
    let volume: Vec<(u32, String, Option<String>)> = VOLUME_HOLD_MS
        .iter()
        .enumerate()
        .map(|(i, ms)| {
            let hint = (*ms == VOLUME_HOLD_DEFAULT_MS).then(|| default_word.to_owned());
            (ids::VOLUME_HOLD_BASE + i as u32, volume_label(*ms, lang, false), hint)
        })
        .collect();
    let plan = GridPlan::new(c, text_w, &volume, 4);
    let vol = Rect::new(l.lx, y, l.lw, 79.0 + nh + 14.0 + plan.height() + 24.0);
    let by = c.card(vol, "volume-2", Tone::Orange, c.t("Volume screen time", "Ses ekranı süresi"));
    let after = c.paragraph(l.lx + 24.0, by - 6.0, text_w, note, 13.5, c.p.text_muted);
    plan.draw(c, l.lx + 24.0, after + 14.0, &volume);
    vol.bottom()
}

/// Brightness slider with value read-out. Registers a `Slider` hit (value 1..=10).
fn slider(c: &mut Ctx, r: Rect) {
    let p = c.p;
    let id = ids::BRIGHTNESS;
    let label_w = 112.0;
    let track = Rect::new(r.x + 8.0, r.cy() - 3.0, r.w - label_w - 24.0, 6.0);
    let value = c.m.brightness;
    let frac = value.map(|v| (v.clamp(1, 10) as f32 - 1.0) / 9.0).unwrap_or(0.5);
    c.sc.rect(track, 3.0, p.toggle_off);
    if value.is_some() {
        c.sc.rect(Rect::new(track.x, track.y, track.w * frac, track.h), 3.0, p.accent);
    }
    let kx = track.x + track.w * frac;
    let active = c.hovered(id) || c.ix.pressed == Some(id);
    if active {
        c.sc.raw(&format!(r##"<circle cx="{}" cy="{}" r="16" fill="#ff6b00" fill-opacity="0.2"/>"##, crate::scene::n(kx), crate::scene::n(track.cy())));
    }
    c.sc.circle(kx, track.cy(), 10.5, "#ffffff");
    // On the light theme a white knob needs an outline to stand out from the card.
    if !p.dark {
        c.sc.ring(kx, track.cy(), 10.0, "#c4c8cf", 1.5);
    } else if value.is_none() {
        c.sc.ring(kx, track.cy(), 10.0, p.ring, 1.5);
    }
    let text = match value {
        Some(v) => format!("{}%", v as u32 * 10),
        None => c.t("Default", "Varsayılan").to_owned(),
    };
    c.sc.text_mid(r.right() - 4.0, r.cy(), &text, &Text::new(15.0, 600, p.text).end());
    let hit = Rect::new(r.x, r.y - 6.0, r.w - label_w, r.h + 12.0);
    c.sc.hit(hit, id, HitKind::Slider { track, min: 1, max: 10 }, true);
    c.focus_ring(id, hit, 10.0);
}

// ───────────────────────── Media ─────────────────────────

fn media(c: &mut Ctx, l: &Layout, y: f32) -> f32 {
    let p = c.p;
    let t = Rect::new(l.lx, y, l.lw, 73.0);
    c.toggle_card(t, ids::MEDIA, "music", Tone::Violet, c.t("Show media metadata", "Medya bilgilerini göster"), c.t("Title and artist while music plays", "Müzik çalarken başlık ve sanatçı"));
    let y2 = t.bottom() + GAP;
    let items = [
        c.t("Media replaces the clock while music is playing.", "Müzik çalarken medya ekranı saatin yerine geçer."),
        c.t("The clock returns when playback stops.", "Oynatma bittiğinde saat geri döner."),
        c.t("Animations and GIFs stay exclusive and are never interrupted.", "Animasyonlar ve GIF'ler ayrı çalışır, kesilmez."),
    ];
    let text_w = l.lw - 48.0 - 34.0;
    let heights: Vec<f32> = items.iter().map(|s| c.paragraph_height(text_w, s, 15.0).max(24.0)).collect();
    let total: f32 = heights.iter().sum::<f32>() + 14.0 * (items.len() as f32 - 1.0);
    let card = Rect::new(l.lx, y2, l.lw, 79.0 + total + 24.0);
    let mut by = c.card(card, "list-music", Tone::Blue, c.t("How it works", "Nasıl çalışır"));
    for (s, h) in items.iter().zip(&heights) {
        c.sc.icon("circle-check", l.lx + 24.0, by + 1.0, 20.0, p.accent, 2.0);
        c.paragraph(l.lx + 24.0 + 34.0, by, text_w, s, 15.0, p.text_dim);
        by += h + 14.0;
    }
    card.bottom()
}

// ───────────────────────── About ─────────────────────────

fn about_left(c: &mut Ctx, l: &Layout, y: f32) -> f32 {
    let p = c.p;
    let text_w = l.lw - 48.0;
    let hw = (text_w - 14.0) / 2.0;
    let stack = hw < 230.0;
    let card = Rect::new(l.lx, y, l.lw, 79.0 + 24.0 + if stack { 44.0 * 2.0 + 12.0 } else { 44.0 } + 24.0);
    let by = c.card(card, "info", Tone::Orange, "GxOled Advanced");
    c.sc.text_mid(l.lx + 24.0, by + 2.0, &format!("{} · MehmetNY", c.m.version), &Text::new(15.0, 500, p.text_dim));
    let bt = by + 24.0;
    let (b1, b2) = if stack {
        (Rect::new(l.lx + 24.0, bt, text_w, 44.0), Rect::new(l.lx + 24.0, bt + 56.0, text_w, 44.0))
    } else {
        (Rect::new(l.lx + 24.0, bt, hw, 44.0), Rect::new(l.lx + 24.0 + hw + 14.0, bt, hw, 44.0))
    };
    c.button(b1, ids::GITHUB, "GitHub · MehmetNY/GxOled", ButtonKind::Secondary, Some("git-branch"));
    c.button(b2, ids::KOFI, "Ko-fi · Support GxOled", ButtonKind::Primary, Some("heart"));

    let credits = c.t(
        "GxOled builds on JerwuQu/ggoled; animations come from pozitronik/steelclock-go. Their original developers deserve the biggest thanks.",
        "GxOled, JerwuQu/ggoled projesini temel alır; animasyonlar pozitronik/steelclock-go kaynaklarından gelir. En büyük teşekkür her iki projenin geliştiricilerine aittir.",
    );
    let h1 = c.paragraph_height(text_w, credits, 15.0);
    let y2 = card.bottom() + GAP;
    let card2 = Rect::new(l.lx, y2, l.lw, 79.0 + h1 + 24.0);
    let by = c.card(card2, "heart-handshake", Tone::Red, c.t("Credits", "Teşekkürler"));
    c.paragraph(l.lx + 24.0, by - 6.0, text_w, credits, 15.0, p.text_dim);

    let lic = c.t(
        "GxOled is free software under the GNU GPL v3.\nInterface font: Inter (SIL OFL 1.1). Icons: Lucide (ISC). Rendering: resvg (MPL-2.0 / Apache-2.0 / MIT).",
        "GxOled, GNU GPL v3 altında özgür yazılımdır.\nArayüz yazı tipi: Inter (SIL OFL 1.1). Simgeler: Lucide (ISC). Çizim: resvg (MPL-2.0 / Apache-2.0 / MIT).",
    );
    let h2 = c.paragraph_height(text_w, lic, 13.5);
    let y3 = card2.bottom() + GAP;
    let card3 = Rect::new(l.lx, y3, l.lw, 79.0 + h2 + 24.0);
    let by = c.card(card3, "badge-check", Tone::Green, c.t("License & third-party", "Lisans ve üçüncü taraf"));
    c.paragraph(l.lx + 24.0, by - 6.0, text_w, lic, 13.5, p.text_muted);
    card3.bottom()
}
