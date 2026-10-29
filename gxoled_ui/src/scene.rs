//! Scene description: a growing SVG document plus the clickable regions drawn into it.
//! Everything is authored in design units (du); the renderer scales to physical pixels.

use crate::engine::font_attrs;
use std::fmt::Write;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn cx(&self) -> f32 {
        self.x + self.w / 2.0
    }
    pub fn cy(&self) -> f32 {
        self.y + self.h / 2.0
    }
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }
    pub fn inset(&self, d: f32) -> Rect {
        Rect::new(self.x + d, self.y + d, (self.w - 2.0 * d).max(0.0), (self.h - 2.0 * d).max(0.0))
    }
    pub fn offset(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitKind {
    /// Plain click target (radio, toggle, button, chip, link).
    Click,
    /// Sidebar page switch; `id` is `ids::NAV_BASE + page index`.
    Nav,
    /// Horizontal slider; the track rect maps x to a value in `min..=max`.
    Slider { track: Rect, min: u8, max: u8 },
    /// Row of the profile list (`id` = `ids::LIST_BASE + row`).
    ListRow,
}

#[derive(Clone, Debug)]
pub struct Hit {
    pub rect: Rect,
    pub id: u32,
    pub kind: HitKind,
    /// Lives inside the scrolling content area (rect is in content coordinates).
    pub scrolls: bool,
    pub focusable: bool,
}

/// Raster-only regions composited after the SVG pass so pixel art stays crisp (nearest neighbour).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OverlayKind {
    /// The live 128×64 OLED mirror.
    Oled,
    /// The donation QR image.
    Qr,
}

#[derive(Clone, Copy, Debug)]
pub struct Overlay {
    pub rect: Rect,
    pub kind: OverlayKind,
    pub scrolls: bool,
    /// Rounded-corner radius (du) the pixels are clipped to.
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

#[derive(Clone, Copy, Debug)]
pub struct Text<'a> {
    pub size: f32,
    pub weight: u16,
    pub fill: &'a str,
    pub anchor: Anchor,
    pub spacing: f32,
    pub opacity: f32,
}

impl<'a> Text<'a> {
    pub fn new(size: f32, weight: u16, fill: &'a str) -> Self {
        Self { size, weight, fill, anchor: Anchor::Start, spacing: 0.0, opacity: 1.0 }
    }
    pub fn mid(mut self) -> Self {
        self.anchor = Anchor::Middle;
        self
    }
    pub fn end(mut self) -> Self {
        self.anchor = Anchor::End;
        self
    }
    pub fn spaced(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
    pub fn alpha(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
}

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Shortest decimal for SVG attributes (two places is far below one device pixel).
pub fn n(v: f32) -> String {
    let s = format!("{:.2}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" { "0".to_owned() } else { s.to_owned() }
}

#[derive(Default)]
pub struct Scene {
    pub body: String,
    defs: String,
    clip_seq: u32,
    pub hits: Vec<Hit>,
    pub overlays: Vec<Overlay>,
    /// Currently open scrolling group (hits/overlays added while true are flagged `scrolls`).
    pub in_scroll: bool,
    /// Visible area (content coordinates) and length of the profile list, for wheel scrolling.
    pub list_area: Option<(Rect, usize)>,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn raw(&mut self, s: &str) {
        self.body.push_str(s);
    }

    pub fn rect(&mut self, r: Rect, rx: f32, fill: &str) {
        let _ = write!(
            self.body,
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}"/>"#,
            n(r.x), n(r.y), n(r.w), n(r.h), n(rx), fill
        );
    }

    /// Rect with a 1-px-style border drawn *inside* its bounds so neighbouring cards align exactly.
    pub fn rect_border(&mut self, r: Rect, rx: f32, fill: &str, stroke: &str, sw: f32) {
        let i = r.inset(sw / 2.0);
        let _ = write!(
            self.body,
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
            n(i.x), n(i.y), n(i.w), n(i.h), n((rx - sw / 2.0).max(0.0)), fill, stroke, n(sw)
        );
    }

    /// Soft drop shadow under a rounded rect (cheap: one blurred rect).
    pub fn shadow(&mut self, r: Rect, rx: f32, blur: f32, dy: f32, color: &str, opacity: f32) {
        self.clip_seq += 1;
        let id = format!("sh{}", self.clip_seq);
        let _ = write!(
            self.defs,
            r#"<filter id="{id}" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="{}"/></filter>"#,
            n(blur)
        );
        let _ = write!(
            self.body,
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}" fill-opacity="{}" filter="url(#{id})"/>"#,
            n(r.x), n(r.y + dy), n(r.w), n(r.h), n(rx), color, n(opacity)
        );
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, fill: &str) {
        let _ = write!(self.body, r#"<circle cx="{}" cy="{}" r="{}" fill="{}"/>"#, n(cx), n(cy), n(r), fill);
    }

    pub fn ring(&mut self, cx: f32, cy: f32, r: f32, stroke: &str, sw: f32) {
        let _ = write!(
            self.body,
            r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{}" stroke-width="{}"/>"#,
            n(cx), n(cy), n(r), stroke, n(sw)
        );
    }

    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, stroke: &str, sw: f32) {
        let _ = write!(
            self.body,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
            n(x1), n(y1), n(x2), n(y2), stroke, n(sw)
        );
    }

    /// Text with its baseline at `y`.
    pub fn text(&mut self, x: f32, y: f32, s: &str, t: &Text) {
        let (family, weight) = font_attrs(t.weight);
        let anchor = match t.anchor {
            Anchor::Start => "start",
            Anchor::Middle => "middle",
            Anchor::End => "end",
        };
        let mut extra = String::new();
        if t.spacing != 0.0 {
            let _ = write!(extra, r#" letter-spacing="{}""#, n(t.spacing));
        }
        if t.opacity < 1.0 {
            let _ = write!(extra, r#" fill-opacity="{}""#, n(t.opacity));
        }
        // With trailing letter-spacing, `end`/`middle` anchors include one extra gap; nudge back so optical edges line up.
        let x = match t.anchor {
            Anchor::End => x + t.spacing,
            Anchor::Middle => x + t.spacing / 2.0,
            Anchor::Start => x,
        };
        let _ = write!(
            self.body,
            r#"<text x="{}" y="{}" font-family="{}" font-weight="{}" font-size="{}" fill="{}" text-anchor="{}"{}>{}</text>"#,
            n(x), n(y), family, weight, n(t.size), t.fill, anchor, extra, escape(s)
        );
    }

    /// Text vertically centred on `cy` (cap-height centring, which looks right for mixed-case UI labels).
    pub fn text_mid(&mut self, x: f32, cy: f32, s: &str, t: &Text) {
        self.text(x, cy + t.size * 0.36, s, t);
    }

    /// Lucide icon (24×24 source) drawn at `size` with its top-left at (x, y).
    pub fn icon(&mut self, name: &str, x: f32, y: f32, size: f32, color: &str, stroke_width: f32) {
        let Some(inner) = crate::icons_data::svg_inner(name) else { return };
        let _ = write!(
            self.body,
            r#"<g transform="translate({} {}) scale({})" fill="none" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round">{}</g>"#,
            n(x), n(y), n(size / 24.0), color, n(stroke_width), inner
        );
    }

    /// Open a clipped group (rounded rect); returns a token for `end_clip`.
    pub fn begin_clip(&mut self, r: Rect, rx: f32) {
        self.clip_seq += 1;
        let _ = write!(
            self.defs,
            r#"<clipPath id="c{}"><rect x="{}" y="{}" width="{}" height="{}" rx="{}"/></clipPath>"#,
            self.clip_seq, n(r.x), n(r.y), n(r.w), n(r.h), n(rx)
        );
        let _ = write!(self.body, r#"<g clip-path="url(#c{})">"#, self.clip_seq);
    }

    pub fn end_group(&mut self) {
        self.body.push_str("</g>");
    }

    pub fn begin_translate(&mut self, dx: f32, dy: f32) {
        let _ = write!(self.body, r#"<g transform="translate({} {})">"#, n(dx), n(dy));
    }

    pub fn hit(&mut self, rect: Rect, id: u32, kind: HitKind, focusable: bool) {
        self.hits.push(Hit { rect, id, kind, scrolls: self.in_scroll, focusable });
    }

    pub fn overlay(&mut self, rect: Rect, kind: OverlayKind, radius: f32) {
        self.overlays.push(Overlay { rect, kind, scrolls: self.in_scroll, radius });
    }

    pub fn finish(self, w: f32, h: f32) -> (String, Vec<Hit>, Vec<Overlay>) {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}"><defs>{}</defs>{}</svg>"#,
            n(w), n(h), n(w), n(h), self.defs, self.body
        );
        (svg, self.hits, self.overlays)
    }
}
