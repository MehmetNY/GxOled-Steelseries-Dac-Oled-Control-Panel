//! Window furniture shared by every page: sidebar, header strip and the right-hand column
//! (live OLED mirror, context card, footer chip).

use crate::{
    ctx::Ctx,
    ids,
    model::Page,
    scene::{HitKind, OverlayKind, Rect, Text},
    theme::Tone,
};

pub const SIDEBAR_W: f32 = 200.0;
pub const HEADER_H: f32 = 50.0;
pub const PAD_L: f32 = 36.0;
pub const PAD_R: f32 = 22.0;
pub const GAP: f32 = 22.0;

pub fn nav_icon(page: Page) -> &'static str {
    match page {
        Page::General => "settings",
        Page::Clock => "clock",
        Page::Animations => "play",
        Page::GifStudio => "image",
        Page::Display => "monitor",
        Page::Media => "music",
        Page::About => "info",
    }
}

pub fn nav_label(page: Page, tr: bool) -> &'static str {
    match (page, tr) {
        (Page::General, false) => "General",
        (Page::General, true) => "Genel",
        (Page::Clock, false) => "Clock",
        (Page::Clock, true) => "Saat",
        (Page::Animations, false) => "Animations",
        (Page::Animations, true) => "Animasyonlar",
        (Page::GifStudio, _) => "GIF Studio",
        (Page::Display, false) => "Display",
        (Page::Display, true) => "Ekran",
        (Page::Media, false) => "Media",
        (Page::Media, true) => "Medya",
        (Page::About, false) => "About",
        (Page::About, true) => "Hakkında",
    }
}

/// Window background: the shell colour everywhere with the content area inset. Drawn first.
pub fn backdrop(c: &mut Ctx, w: f32, h: f32) {
    let (shell, content) = (c.p.shell, c.p.content);
    c.sc.rect(Rect::new(0.0, 0.0, w, h), 0.0, shell);
    c.sc.rect(Rect::new(SIDEBAR_W, HEADER_H, w - SIDEBAR_W, h - HEADER_H), 0.0, content);
}

/// Sidebar and header strip. Drawn *after* the scrolling content, which slides underneath them: painting an
/// opaque L-shaped frame over it is several times cheaper than rasterising the content through a clip group.
pub fn front(c: &mut Ctx, w: f32, h: f32) {
    let shell = c.p.shell;
    c.sc.rect(Rect::new(0.0, 0.0, SIDEBAR_W, h), 0.0, shell);
    c.sc.rect(Rect::new(SIDEBAR_W, 0.0, (w - SIDEBAR_W).max(0.0), HEADER_H), 0.0, shell);
    sidebar(c, h);
    header(c, w);
}

fn sidebar(c: &mut Ctx, _h: f32) {
    let tr = c.lang() == crate::model::Lang::Tr;
    let p = c.p;
    // Logo block
    c.sc.rect(Rect::new(20.0, 30.0, 35.0, 35.0), 9.0, p.accent);
    c.sc.text_mid(37.5, 47.5, "Gx", &Text::new(13.5, 700, "#ffffff").mid());
    c.sc.text_mid(68.0, 47.5, "GxOled", &Text::new(17.0, 700, p.text));
    c.sc.rect(Rect::new(0.0, 90.5, SIDEBAR_W, 1.0), 0.0, p.divider);

    for (i, page) in Page::ALL.iter().enumerate() {
        let id = page.nav_id();
        let r = Rect::new(10.0, 111.5 + i as f32 * 47.5, 180.0, 45.0);
        let active = c.m.page == *page;
        if active {
            c.sc.rect(r, 11.0, p.accent);
        } else if c.pressed(id) {
            c.sc.rect(r, 11.0, p.row_hover);
        } else if c.hovered(id) {
            c.sc.rect(r, 11.0, p.row);
        }
        let (fg, weight) = if active {
            (p.on_accent, 500)
        } else if c.hovered(id) {
            (p.text, 400)
        } else {
            (p.text_dim, 400)
        };
        c.sc.icon(nav_icon(*page), r.x + 12.5, r.cy() - 9.0, 18.0, fg, 2.2);
        c.sc.text_mid(r.x + 43.0, r.cy(), nav_label(*page, tr), &Text::new(16.0, weight, fg));
        c.sc.hit(r, id, HitKind::Nav, true);
        c.focus_ring(id, r, 11.0);
    }

    // Quit sits right under the page list, separated by a rule (as in the design).
    let rule_y = 111.5 + 7.0 * 47.5 + 12.0;
    c.sc.rect(Rect::new(10.0, rule_y, 180.0, 1.0), 0.0, p.divider);
    let q = Rect::new(10.0, rule_y + 8.0, 180.0, 45.0);
    if c.pressed(ids::QUIT) {
        c.sc.rect(q, 11.0, p.row_hover);
    } else if c.hovered(ids::QUIT) {
        let (fg, bg) = Tone::Red.colors(p.dark);
        c.sc.rect(q, 11.0, bg);
        c.sc.icon("log-out", q.x + 12.5, q.cy() - 9.0, 18.0, fg, 2.2);
        c.sc.text_mid(q.x + 43.0, q.cy(), c.t("Quit GxOled", "GxOled'den çık"), &Text::new(16.0, 400, fg));
        c.sc.hit(q, ids::QUIT, HitKind::Click, true);
        return;
    }
    c.sc.icon("log-out", q.x + 12.5, q.cy() - 9.0, 18.0, p.text_muted, 2.2);
    c.sc.text_mid(q.x + 43.0, q.cy(), c.t("Quit GxOled", "GxOled'den çık"), &Text::new(16.0, 400, p.text_muted));
    c.sc.hit(q, ids::QUIT, HitKind::Click, true);
    c.focus_ring(ids::QUIT, q, 11.0);
}

fn header(c: &mut Ctx, w: f32) {
    let p = c.p;
    let cy = 25.5;
    let x = SIDEBAR_W + 30.0;
    c.sc.text_mid(x, cy, "128 X 64", &Text::new(13.5, 600, p.accent).spaced(2.0));
    let bar_x = x + c.eng.text_width("128 X 64", 13.5, 600, 2.0) + 21.0;
    c.sc.rect(Rect::new(bar_x, cy - 12.0, 1.0, 24.0), 0.0, p.divider);
    let title = c.lang().upper(c.t("SteelSeries DAC OLED control", "SteelSeries DAC OLED kontrol"));
    c.sc.text_mid(bar_x + 20.0, cy, &title, &Text::new(13.5, 600, p.text).spaced(2.0));
    let dot = if c.m.connected { p.green } else { p.text_muted };
    let name = if c.m.connected {
        c.m.device_name.clone()
    } else {
        c.t("Waiting for device…", "Cihaz bekleniyor…").to_owned()
    };
    let nw = c.eng.text_width(&name, 13.0, 400, 0.0);
    c.sc.text_mid(w - PAD_R - 6.0, cy, &name, &Text::new(13.0, 400, p.text_dim).end());
    c.sc.circle(w - PAD_R - 6.0 - nw - 13.0, cy, 3.5, dot);
}

// ───────────────────────── right column ─────────────────────────

/// Draws the live OLED panel; returns its rect.
pub fn live_panel(c: &mut Ctx, x: f32, y: f32, w: f32) -> Rect {
    let p = c.p;
    let oled_w = w - 48.0;
    let oled_h = oled_w / 2.0;
    let rows_h = 3.0 * 31.0 + 6.0;
    let h = 70.0 + oled_h + 14.0 + rows_h + 14.0;
    let r = Rect::new(x, y, w, h);
    c.card_frame(r);
    let cy = y + 37.0;
    let live = if c.m.connected { p.green } else { p.text_muted };
    c.sc.circle(x + 30.0, cy, 4.5, live);
    c.sc.text_mid(x + 46.0, cy, &c.lang().upper(c.t("Live OLED", "Canlı OLED")), &Text::new(12.5, 600, p.text_dim).spaced(1.9));
    let mono = c.lang().upper(c.t("Monochrome", "Monokrom"));
    c.badge(r.right() - 24.0, cy, &mono);
    // Black display with a hairline frame; the pixels themselves are composited after rasterisation.
    let screen = Rect::new(x + 24.0, y + 70.0, oled_w, oled_h);
    c.sc.rect_border(screen, 12.0, p.oled_bg, p.oled_border, 1.0);
    c.sc.overlay(screen.inset(1.0), OverlayKind::Oled, 11.0);
    let mut ry = screen.bottom() + 14.0;
    c.kv_row(x + 24.0, r.right() - 24.0, ry, c.t("Device", "Cihaz"), "SteelSeries · 128×64", true);
    ry += 31.0;
    let version = format!("GxOled {}", c.m.version);
    c.kv_row(x + 24.0, r.right() - 24.0, ry, c.t("Version", "Sürüm"), &version, true);
    ry += 31.0;
    c.sc.text_mid(x + 24.0, ry + 21.0, c.t("Mode", "Mod"), &Text::new(13.5, 400, p.text_muted));
    c.pill_green(r.right() - 24.0, ry + 21.0, c.t("Local Settings", "Yerel Ayarlar"));
    r
}

/// Footer chip at the bottom of the right column.
pub fn footer_chip(c: &mut Ctx, x: f32, y: f32, w: f32) -> Rect {
    let p = c.p;
    let r = Rect::new(x, y, w, 61.0);
    c.card_frame(r);
    c.sc.rect(Rect::new(x + 18.0, r.cy() - 13.5, 27.0, 27.0), 7.0, p.accent);
    c.sc.text_mid(x + 31.5, r.cy(), "Gx", &Text::new(11.0, 700, "#ffffff").mid());
    c.sc.text_mid(x + 59.0, r.cy(), "GxOled Advanced · MehmetNY", &Text::new(13.8, 400, p.text_muted));
    let v = c.m.version.clone();
    c.sc.text_mid(r.right() - 20.0, r.cy(), &v, &Text::new(13.8, 400, p.text_muted).end());
    r
}

/// "Quick access" card of the General page: the three opening modes with descriptions.
pub fn quick_access(c: &mut Ctx, x: f32, y: f32, w: f32) -> Rect {
    let p = c.p;
    let desc = c.t(
        "Tray, Window or Hybrid opening mode controls how GxOled appears when launched or restored.",
        "Tepsi, Pencere veya Karma açılış modu, GxOled'in başlatıldığında ya da geri yüklendiğinde nasıl görüneceğini belirler.",
    );
    // The description wraps to two or three lines depending on language and width, so the card height follows it.
    let h = 88.0 + c.paragraph_height(w - 48.0, desc, 15.0) + 10.0 + 3.0 * 62.0 + 2.0 * 11.0 + 12.0;
    let r = Rect::new(x, y, w, h);
    c.card_frame(r);
    c.sc.text_mid(x + 24.0, y + 36.0, &c.lang().upper(c.t("App mode", "Uygulama modu")), &Text::new(12.5, 600, p.accent).spaced(1.9));
    c.sc.text_mid(x + 24.0, y + 63.0, c.t("Quick Access", "Hızlı Erişim"), &Text::new(16.0, 600, p.text));
    c.sc.icon("zap", r.right() - 24.0 - 22.0, y + 28.0, 22.0, p.accent, 2.3);
    let after = c.paragraph(x + 24.0, y + 88.0, w - 48.0, desc, 15.0, p.text_muted);
    let modes = [
        (ids::MODE_TRAY, "panel-left", Tone::Blue, c.t("Tray Mode", "Tepsi Modu"), c.t("Icon in system tray only", "Yalnızca sistem tepsisinde simge")),
        (ids::MODE_WINDOW, "scan", Tone::Orange, c.t("Window Mode", "Pencere Modu"), c.t("Tray icon opens this window", "Tepsi simgesi bu pencereyi açar")),
        (ids::MODE_HYBRID, "layers", Tone::Violet, c.t("Hybrid Mode", "Karma Mod"), c.t("Left = window, right = tray", "Sol = pencere, sağ = menü")),
    ];
    let mut ry = after + 10.0;
    for (id, icon, tone, title, sub) in modes {
        let row = Rect::new(x + 24.0, ry, w - 48.0, 62.0);
        let on = c.checked(id);
        let (bg, border) = if on {
            (p.sel_bg, p.sel_border)
        } else if c.pressed(id) {
            (p.sel_bg, p.sel_border)
        } else if c.hovered(id) {
            (p.row_hover, p.row_border)
        } else {
            (p.row, p.row_border)
        };
        c.sc.rect_border(row, 13.0, bg, border, 1.0);
        c.icon_tile(row.x + 18.0, row.cy() - 17.5, 35.0, icon, tone);
        c.sc.text_mid(row.x + 69.0, row.cy() - 10.0, title, &Text::new(14.5, 500, p.text));
        let (sub_text, sub_color) = if on { (c.t("Currently active", "Şu anda etkin"), p.accent) } else { (sub, p.text_muted) };
        c.sc.text_mid(row.x + 69.0, row.cy() + 11.5, sub_text, &Text::new(13.7, 400, sub_color));
        if on {
            c.sc.icon("check", row.right() - 38.0, row.cy() - 10.0, 20.0, p.accent, 2.6);
        }
        // Mirrors the opening-mode card on the General page, which owns keyboard focus for these ids.
        c.sc.hit(row, id, HitKind::Click, false);
        ry += 62.0 + 11.0;
    }
    r
}
