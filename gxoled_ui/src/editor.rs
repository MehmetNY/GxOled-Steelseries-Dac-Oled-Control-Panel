//! Chrome of the profile (JSON) editor window: header, the card that frames the text area, a status line and
//! the action bar. The text itself is a native EDIT control owned by the window host, which places it inside
//! the rectangle returned by [`build`].

use crate::{
    ctx::{ButtonKind, Ctx},
    ids,
    scene::{Rect, Text},
    theme::Tone,
};

pub const MARGIN: f32 = 24.0;
pub const HEADER_H: f32 = 76.0;
/// Space under the text card: status line, gap, button row and bottom margin.
pub const FOOTER_H: f32 = 118.0;
/// Inner padding between the card and the native text control.
pub const TEXT_INSET: f32 = 14.0;
pub const BUTTON_H: f32 = 44.0;
const GAP: f32 = 12.0;

/// Smallest content size (design units) at which the layout still works.
pub const MIN_W: f32 = 760.0;
pub const MIN_H: f32 = 480.0;

/// Draw the editor surface for a `w` × `h` design-unit client area. Returns the text-control rectangle.
pub fn build(c: &mut Ctx, w: f32, h: f32) -> Rect {
    let p = c.p;
    c.sc.rect(Rect::new(0.0, 0.0, w, h), 0.0, p.content);

    // Header
    c.sc.rect(Rect::new(0.0, 0.0, w, HEADER_H), 0.0, p.shell);
    c.sc.rect(Rect::new(0.0, HEADER_H - 1.0, w, 1.0), 0.0, p.divider);
    c.icon_tile(MARGIN, (HEADER_H - 40.0) / 2.0, 40.0, "file-json", Tone::Violet);
    let title = c.t("Animation editor", "Animasyon düzenleyici");
    c.sc.text_mid(MARGIN + 54.0, HEADER_H / 2.0 - 10.0, title, &Text::new(18.0, 600, p.text));
    let sub = c.fit(&c.m.editor_title, 13.5, 400, w - 2.0 * MARGIN - 54.0 - 150.0);
    c.sc.text_mid(MARGIN + 54.0, HEADER_H / 2.0 + 14.0, &sub, &Text::new(13.5, 400, p.text_muted));
    let hint = c.t("Ctrl+S saves and applies", "Ctrl+S kaydeder ve uygular");
    c.sc.text_mid(w - MARGIN, HEADER_H / 2.0, hint, &Text::new(12.5, 400, p.text_muted).end());

    // Text card
    let card = Rect::new(MARGIN, HEADER_H + 16.0, w - 2.0 * MARGIN, (h - HEADER_H - 16.0 - FOOTER_H).max(120.0));
    c.card_frame(card);
    let text = card.inset(TEXT_INSET);

    // Status line
    let sy = card.bottom() + 12.0;
    if !c.m.editor_status.is_empty() {
        let (icon, color) = if c.m.editor_status_ok { ("circle-check", p.green) } else { ("circle-x", "#ef4444") };
        c.sc.icon(icon, MARGIN + 2.0, sy + 2.0, 18.0, color, 2.2);
        let msg = c.fit(&c.m.editor_status, 14.0, 500, w - 2.0 * MARGIN - 32.0);
        c.sc.text_mid(MARGIN + 30.0, sy + 11.0, &msg, &Text::new(14.0, 500, color));
    }

    // Action bar: content-sized buttons, shrunk evenly when the window is narrow.
    let tr = c.lang() == crate::model::Lang::Tr;
    let specs: [(u32, &str, ButtonKind, &str); 5] = [
        (ids::EDITOR_SAVE, if tr { "Kaydet ve uygula" } else { "Save & apply" }, ButtonKind::Primary, "save"),
        (ids::EDITOR_FORMAT, if tr { "Doğrula / Biçimlendir" } else { "Validate / Format" }, ButtonKind::Secondary, "wand-sparkles"),
        (ids::EDITOR_IMPORT, if tr { "JSON içe aktar" } else { "Import JSON" }, ButtonKind::Secondary, "file-up"),
        (ids::EDITOR_EXPORT, if tr { "JSON dışa aktar" } else { "Export JSON" }, ButtonKind::Secondary, "file-down"),
        (ids::EDITOR_RESET, if tr { "Sıfırla" } else { "Reset" }, ButtonKind::Danger, "rotate-ccw"),
    ];
    let natural: Vec<f32> = specs.iter().map(|(_, label, _, _)| c.eng.text_width(label, 15.0, 600, 0.0) + 22.0 + 40.0).collect();
    let avail = w - 2.0 * MARGIN - GAP * (specs.len() as f32 - 1.0);
    let total: f32 = natural.iter().sum();
    let k = if total > avail { avail / total } else { 1.0 };
    let by = card.bottom() + 12.0 + 22.0 + 14.0;
    let mut x = MARGIN;
    for ((id, label, kind, icon), nw) in specs.iter().zip(&natural) {
        let bw = nw * k;
        c.button(Rect::new(x, by, bw, BUTTON_H), *id, label, *kind, Some(icon));
        x += bw + GAP;
    }
    text
}
