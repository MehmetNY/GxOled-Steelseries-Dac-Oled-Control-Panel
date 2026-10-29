//! Drawing context + reusable widgets. Every widget draws itself (reflecting hover / pressed / focus / checked
//! state) and registers its own hit region, so pages only describe *what* is on them.

use crate::{
    engine::Engine,
    model::{Lang, UiModel},
    scene::{HitKind, Rect, Scene, Text},
    theme::{Palette, Tone},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Interaction {
    pub hover: Option<u32>,
    pub pressed: Option<u32>,
    pub focus: Option<u32>,
    pub focus_visible: bool,
}

pub struct Ctx<'a> {
    pub sc: Scene,
    pub p: &'static Palette,
    pub m: &'a UiModel,
    pub eng: &'a Engine,
    pub ix: Interaction,
    /// First visible row of the profile list.
    pub list_first: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
}

impl<'a> Ctx<'a> {
    pub fn list_offset(&self) -> usize {
        self.list_first
    }

    pub fn lang(&self) -> Lang {
        self.m.lang
    }

    pub fn t(&self, en: &'a str, tr: &'a str) -> &'a str {
        self.m.lang.t(en, tr)
    }

    pub fn checked(&self, id: u32) -> bool {
        self.m.is_checked(id)
    }

    pub fn hovered(&self, id: u32) -> bool {
        self.ix.hover == Some(id) && self.ix.pressed.is_none_or(|p| p == id)
    }

    pub fn pressed(&self, id: u32) -> bool {
        self.ix.pressed == Some(id) && self.ix.hover == Some(id)
    }

    /// Shorten `s` with an ellipsis until it fits `max` du.
    pub fn fit(&self, s: &str, size: f32, weight: u16, max: f32) -> String {
        if self.eng.text_width(s, size, weight, 0.0) <= max {
            return s.to_owned();
        }
        let mut chars: Vec<char> = s.chars().collect();
        while chars.len() > 1 {
            chars.pop();
            let candidate: String = chars.iter().collect::<String>().trim_end().to_owned() + "…";
            if self.eng.text_width(&candidate, size, weight, 0.0) <= max {
                return candidate;
            }
        }
        "…".to_owned()
    }

    pub fn focus_ring(&mut self, id: u32, r: Rect, rx: f32) {
        if self.ix.focus_visible && self.ix.focus == Some(id) {
            let o = r.inset(-2.5);
            let accent = self.p.accent;
            self.sc.rect_border(o, rx + 2.5, "none", accent, 2.0);
        }
    }

    // ───────────────────────── cards ─────────────────────────

    pub fn card_frame(&mut self, r: Rect) {
        let (card, border) = (self.p.card, self.p.card_border);
        if !self.p.dark {
            self.sc.shadow(r, 16.0, 6.0, 2.0, self.p.shadow, 0.06);
        }
        self.sc.rect_border(r, 16.0, card, border, 1.0);
    }

    pub fn icon_tile(&mut self, x: f32, y: f32, size: f32, icon: &str, tone: Tone) {
        let (fg, bg) = tone.colors(self.p.dark);
        self.sc.rect(Rect::new(x, y, size, size), size * 0.26, bg);
        let isz = size * 0.56;
        self.sc.icon(icon, x + (size - isz) / 2.0, y + (size - isz) / 2.0, isz, fg, 2.3);
    }

    /// Card with the standard icon-tile + title header. Returns the y where the body may start.
    pub fn card(&mut self, r: Rect, icon: &str, tone: Tone, title: &str) -> f32 {
        self.card_frame(r);
        self.icon_tile(r.x + 24.0, r.y + 24.0, 35.0, icon, tone);
        let t = Text::new(16.5, 600, self.p.text);
        let max = r.w - 72.0 - 20.0;
        let title = self.fit(title, 16.5, 600, max);
        self.sc.text_mid(r.x + 72.0, r.y + 41.5, &title, &t);
        r.y + 24.0 + 35.0 + 20.0
    }

    // ───────────────────────── choices ─────────────────────────

    fn choice_colors(&self, id: u32, checked: bool) -> (&'static str, &'static str, &'static str) {
        let p = self.p;
        if checked {
            (p.sel_bg, p.sel_border, p.text)
        } else if self.pressed(id) {
            (p.sel_bg, p.sel_border, p.text)
        } else if self.hovered(id) {
            (p.row_hover, p.row_border, p.text)
        } else {
            (p.row, p.row_border, p.text_dim)
        }
    }

    fn radio_dot(&mut self, cx: f32, cy: f32, r: f32, id: u32, checked: bool) {
        let accent = self.p.accent;
        if checked && r > 8.0 {
            // Large rows: thin outer ring + accent core.
            let ring = self.p.text;
            self.sc.ring(cx, cy, r - 0.75, ring, 1.6);
            self.sc.circle(cx, cy, r - 4.2, accent);
        } else if checked {
            self.sc.circle(cx, cy, r, accent);
        } else if self.hovered(id) {
            self.sc.ring(cx, cy, r - 0.75, accent, 1.8);
        } else {
            let ring = self.p.ring;
            self.sc.ring(cx, cy, r - 0.75, ring, 1.8);
        }
    }

    /// Full-width option row: radio dot, label and a right-aligned hint.
    pub fn radio_row(&mut self, r: Rect, id: u32, label: &str, hint: Option<&str>) {
        let checked = self.checked(id);
        let (bg, border, fg) = self.choice_colors(id, checked);
        self.sc.rect_border(r, 11.0, bg, border, 1.0);
        let dot_r = if r.h >= 40.0 { 9.0 } else { 7.0 };
        self.radio_dot(r.x + 23.5, r.cy(), dot_r, id, checked);
        let size = if r.h < 40.0 { 15.0 } else { 16.0 };
        let hint_text = hint.map(str::to_owned);
        let hint_w = hint_text.as_ref().map_or(0.0, |h| self.eng.text_width(h, 13.0, 400, 0.0));
        let label = self.fit(label, size, 400, (r.w - 46.0 - 16.0 - hint_w - 12.0).max(40.0));
        self.sc.text_mid(r.x + 45.5, r.cy(), &label, &Text::new(size, 400, fg));
        if let Some(h) = hint_text {
            let color = if checked { self.p.accent } else { self.p.text_muted };
            let h = self.fit(&h, 13.0, 400, (r.w - 46.0 - 24.0).max(40.0) * 0.62);
            self.sc.text_mid(r.right() - 16.0, r.cy(), &h, &Text::new(13.0, 400, color).end());
        }
        self.sc.hit(r, id, HitKind::Click, true);
        self.focus_ring(id, r, 11.0);
    }

    /// Row with a hint that is replaced by "Selected" while checked (mockup behaviour for the opening modes).
    pub fn radio_row_selected_hint(&mut self, r: Rect, id: u32, label: &str, hint: &str, selected_word: &str) {
        let shown = if self.checked(id) { selected_word } else { hint };
        self.radio_row(r, id, label, Some(shown));
    }

    /// Compact chip: radio dot (or icon) + label. Width follows the text unless `min_w` is larger.
    pub fn chip(&mut self, x: f32, y: f32, h: f32, id: u32, label: &str, icon: Option<&str>, min_w: f32) -> Rect {
        let checked = self.checked(id);
        let (bg, border, fg) = self.choice_colors(id, checked);
        let tw = self.eng.text_width(label, 16.0, 400, 0.0);
        let w = (40.0 + tw + 20.0).max(min_w);
        let r = Rect::new(x, y, w, h);
        self.sc.rect_border(r, 11.0, bg, border, 1.0);
        match icon {
            Some(name) => {
                let icon_color = if checked { self.p.accent } else { self.p.text_dim };
                self.sc.icon(name, r.x + 14.0, r.cy() - 8.5, 17.0, icon_color, 2.0);
            }
            None => self.radio_dot(r.x + 22.5, r.cy(), 6.5, id, checked),
        }
        self.sc.text_mid(r.x + 40.0, r.cy(), label, &Text::new(16.0, 400, fg));
        self.sc.hit(r, id, HitKind::Click, true);
        self.focus_ring(id, r, 11.0);
        r
    }

    /// Fixed-width chip (used in grids so columns line up). The optional hint is drawn right-aligned in small type.
    pub fn chip_w(&mut self, r: Rect, id: u32, label: &str, icon: Option<&str>, hint: Option<&str>) {
        let checked = self.checked(id);
        let (bg, border, fg) = self.choice_colors(id, checked);
        self.sc.rect_border(r, 11.0, bg, border, 1.0);
        match icon {
            Some(name) => {
                let icon_color = if checked { self.p.accent } else { self.p.text_dim };
                self.sc.icon(name, r.x + 14.0, r.cy() - 8.5, 17.0, icon_color, 2.0);
            }
            None => self.radio_dot(r.x + 22.5, r.cy(), 6.5, id, checked),
        }
        let hint_w = hint.map_or(0.0, |h| self.eng.text_width(h, 12.5, 400, 0.0) + 10.0);
        let label = self.fit(label, 16.0, 400, r.w - 40.0 - 12.0 - hint_w);
        self.sc.text_mid(r.x + 40.0, r.cy(), &label, &Text::new(16.0, 400, fg));
        if let Some(h) = hint {
            let color = if checked { self.p.accent } else { self.p.text_muted };
            self.sc.text_mid(r.right() - 12.0, r.cy(), h, &Text::new(12.5, 400, color).end());
        }
        self.sc.hit(r, id, HitKind::Click, true);
        self.focus_ring(id, r, 11.0);
    }

    pub fn toggle_switch(&mut self, x: f32, cy: f32, on: bool) {
        let (w, h) = (50.0, 27.0);
        let track = Rect::new(x, cy - h / 2.0, w, h);
        let fill = if on { self.p.accent } else { self.p.toggle_off };
        self.sc.rect(track, h / 2.0, fill);
        let knob_x = if on { x + w - h / 2.0 } else { x + h / 2.0 };
        self.sc.circle(knob_x, cy, 10.5, "#ffffff");
    }

    /// Whole-card toggle: icon tile, title, subtitle and the switch on the right. Returns nothing; `r` is the card.
    pub fn toggle_card(&mut self, r: Rect, id: u32, icon: &str, tone: Tone, title: &str, subtitle: &str) {
        let on = self.checked(id);
        self.card_frame(r);
        if self.hovered(id) {
            let hover = self.p.row_hover;
            self.sc.rect_border(r.inset(1.0), 15.0, "none", hover, 1.0);
        }
        self.icon_tile(r.x + 24.0, r.cy() - 17.5, 35.0, icon, tone);
        let sw_x = r.right() - 24.0 - 50.0;
        let max = sw_x - (r.x + 72.0) - 14.0;
        let title = self.fit(title, 15.0, 500, max);
        let subtitle = self.fit(subtitle, 12.5, 400, max);
        self.sc.text_mid(r.x + 72.0, r.cy() - 9.0, &title, &Text::new(15.0, 500, self.p.text));
        self.sc.text_mid(r.x + 72.0, r.cy() + 11.5, &subtitle, &Text::new(12.5, 400, self.p.text_muted));
        self.toggle_switch(sw_x, r.cy(), on);
        self.sc.hit(r, id, HitKind::Click, true);
        self.focus_ring(id, r, 16.0);
    }

    pub fn button(&mut self, r: Rect, id: u32, label: &str, kind: ButtonKind, icon: Option<&str>) {
        let p = self.p;
        let hover = self.hovered(id);
        let down = self.pressed(id);
        let (bg, border, fg) = match kind {
            ButtonKind::Primary => {
                let bg = if down { "#d95a00" } else if hover { "#ff7d1f" } else { p.accent };
                (bg, bg, p.on_accent)
            }
            ButtonKind::Secondary => {
                let bg = if down { p.sel_bg } else if hover { p.row_hover } else { p.row };
                (bg, if hover { p.sel_border } else { p.row_border }, p.text)
            }
            ButtonKind::Danger => {
                let (fg, tile) = Tone::Red.colors(p.dark);
                let bg = if down || hover { tile } else { p.row };
                (bg, if hover { fg } else { p.row_border }, fg)
            }
        };
        self.sc.rect_border(r, 11.0, bg, border, 1.0);
        let size = 15.0;
        let tw = self.eng.text_width(label, size, 600, 0.0);
        let icon_w = if icon.is_some() { 22.0 } else { 0.0 };
        let total = (tw + icon_w).min(r.w - 16.0);
        let start = r.cx() - total / 2.0;
        if let Some(name) = icon {
            self.sc.icon(name, start, r.cy() - 8.0, 16.0, fg, 2.0);
        }
        let label = self.fit(label, size, 600, r.w - 16.0 - icon_w);
        self.sc.text_mid(start + icon_w, r.cy(), &label, &Text::new(size, 600, fg));
        self.sc.hit(r, id, HitKind::Click, true);
        self.focus_ring(id, r, 11.0);
    }

    // ───────────────────────── small pieces ─────────────────────────

    pub fn badge(&mut self, right: f32, cy: f32, label: &str) {
        let t = Text::new(12.5, 600, self.p.text_dim).spaced(0.5);
        let w = self.eng.text_width(label, 12.5, 600, 0.5) + 30.0;
        let r = Rect::new(right - w, cy - 17.0, w, 34.0);
        let row = self.p.row;
        self.sc.rect(r, 9.0, row);
        self.sc.text_mid(r.cx(), r.cy(), label, &t.mid());
    }

    pub fn pill_green(&mut self, right: f32, cy: f32, label: &str) {
        let w = self.eng.text_width(label, 13.5, 500, 0.0) + 40.0;
        let r = Rect::new(right - w, cy - 15.0, w, 30.0);
        let (bg, fg) = (self.p.green_bg, self.p.green);
        self.sc.rect(r, 9.0, bg);
        self.sc.circle(r.x + 15.0, r.cy(), 3.5, fg);
        self.sc.text_mid(r.x + 26.0, r.cy(), label, &Text::new(13.5, 500, fg));
    }

    pub fn kv_row(&mut self, x: f32, right: f32, y: f32, key: &str, value: &str, divider: bool) {
        let cy = y + 15.5;
        self.sc.text_mid(x, cy, key, &Text::new(13.5, 400, self.p.text_muted));
        let value = self.fit(value, 13.5, 500, (right - x) * 0.72);
        self.sc.text_mid(right, cy, &value, &Text::new(13.5, 500, self.p.text).end());
        if divider {
            let d = self.p.divider;
            self.sc.rect(Rect::new(x, y + 31.0, right - x, 1.0), 0.0, d);
        }
    }

    /// Page heading with subtitle (top of the left column). Returns the y below it.
    pub fn page_heading(&mut self, x: f32, y: f32, title: &str, subtitle: &str) -> f32 {
        self.sc.text(x, y + 27.0, title, &Text::new(22.5, 600, self.p.text));
        self.sc.text(x, y + 50.0, subtitle, &Text::new(15.0, 400, self.p.text_muted));
        y + 50.0
    }

    /// Wrapped paragraph; returns the y below the last line.
    pub fn paragraph(&mut self, x: f32, y: f32, w: f32, text: &str, size: f32, color: &str) -> f32 {
        let lines = self.eng.wrap(text, size, 400, w);
        let lh = (size * 1.45).round();
        let mut cy = y;
        for line in &lines {
            self.sc.text(x, cy + size, line, &Text::new(size, 400, color));
            cy += lh;
        }
        cy
    }

    /// Height `paragraph` will occupy (so cards can size themselves before drawing).
    pub fn paragraph_height(&self, w: f32, text: &str, size: f32) -> f32 {
        self.eng.wrap(text, size, 400, w).len() as f32 * (size * 1.45).round()
    }
}
