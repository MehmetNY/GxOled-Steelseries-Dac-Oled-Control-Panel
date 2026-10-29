//! GxOled modern settings UI.
//!
//! A tiny retained-mode toolkit: pages describe themselves as an SVG scene plus hit regions, `resvg` rasterises
//! it in software, and the host (a Win32 window) only forwards input and blits the resulting bitmap. Nothing here
//! depends on the operating system, so the whole interface can be rendered to PNG and unit-tested anywhere.

pub mod chrome;
pub mod ctx;
pub mod editor;
pub mod engine;
pub mod icons_data;
pub mod ids;
pub mod model;
pub mod pages;
pub mod scene;
pub mod theme;

use std::time::{Duration, Instant};

use chrome::{HEADER_H, SIDEBAR_W};
use ctx::{Ctx, Interaction};
use engine::{Engine, Pixmap};
use model::{Cursor, OLED_H, OLED_W, Page, Surface, UiEvent, UiModel};
use scene::{Hit, HitKind, Overlay, OverlayKind, Rect, Scene};

/// Width (design units) the layout was drawn for; the window scales the design proportionally to this.
pub const DESIGN_W: f32 = 1450.0;
pub const SCALE_MIN: f32 = 0.8;
pub const SCALE_MAX: f32 = 1.15;
const CONTENT_BOTTOM_PAD: f32 = 36.0;
const DOUBLE_CLICK: Duration = Duration::from_millis(450);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Tab { shift: bool },
    Enter,
    Space,
    Escape,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
}

/// A finished frame: tightly packed RGBA (opaque), `width * height * 4` bytes.
pub struct Frame<'a> {
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
}

#[derive(Clone, Copy)]
struct SliderDrag {
    track: Rect,
    min: u8,
    max: u8,
}

pub struct Ui {
    engine: Engine,
    pub model: UiModel,
    size_px: (u32, u32),
    dpi: f32,
    hover: Option<u32>,
    pressed: Option<u32>,
    focus: Option<u32>,
    focus_visible: bool,
    scroll: f32,
    max_scroll: f32,
    list_first: usize,
    drag: Option<SliderDrag>,
    last_click: Option<(u32, Instant)>,
    // render cache
    dirty: bool,
    base: Option<Pixmap>,
    hits: Vec<Hit>,
    overlays: Vec<Overlay>,
    list_area: Option<(Rect, usize)>,
    frame: Vec<u8>,
    scale: f32,
    events: Vec<UiEvent>,
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}

impl Ui {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            model: UiModel::default(),
            size_px: (1240, 840),
            dpi: 1.0,
            hover: None,
            pressed: None,
            focus: None,
            focus_visible: false,
            scroll: 0.0,
            max_scroll: 0.0,
            list_first: 0,
            drag: None,
            last_click: None,
            dirty: true,
            base: None,
            hits: Vec::new(),
            overlays: Vec::new(),
            list_area: None,
            frame: Vec::new(),
            scale: 1.0,
            events: Vec::new(),
        }
    }

    // ───────────────────────── geometry ─────────────────────────

    /// Client size in physical pixels and the DPI factor (1.0 = 96 dpi).
    pub fn set_size(&mut self, width_px: u32, height_px: u32, dpi: f32) {
        let size = (width_px.max(1), height_px.max(1));
        if size != self.size_px || (dpi - self.dpi).abs() > f32::EPSILON {
            self.size_px = size;
            self.dpi = dpi.max(0.5);
            self.dirty = true;
        }
    }

    /// Pixels per design unit for the current window size.
    pub fn scale_for(width_px: u32, dpi: f32) -> f32 {
        let logical_w = width_px as f32 / dpi.max(0.5);
        dpi.max(0.5) * (logical_w / DESIGN_W).clamp(SCALE_MIN, SCALE_MAX)
    }

    /// Pixels per design unit. The settings window scales proportionally with its width; the editor is a
    /// plain dialog and only follows the DPI.
    fn current_scale(&self) -> f32 {
        match self.model.surface {
            Surface::Settings => Self::scale_for(self.size_px.0, self.dpi),
            Surface::Editor => self.dpi,
        }
    }

    fn du(&self) -> (f32, f32, f32) {
        let scale = self.current_scale();
        (self.size_px.0 as f32 / scale, self.size_px.1 as f32 / scale, scale)
    }

    /// Call after changing `model`; schedules a re-render.
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    /// Drain events produced by input since the last call.
    pub fn take_events(&mut self) -> Vec<UiEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn needs_render(&self) -> bool {
        self.dirty || self.base.is_none()
    }

    // ───────────────────────── rendering ─────────────────────────

    fn rebuild_editor(&mut self) {
        let (w, h, scale) = self.du();
        let mut c = Ctx {
            sc: Scene::new(),
            p: theme::palette(self.model.dark),
            m: &self.model,
            eng: &self.engine,
            ix: Interaction { hover: self.hover, pressed: self.pressed, focus: self.focus, focus_visible: self.focus_visible },
            list_first: 0,
        };
        let text = editor::build(&mut c, w, h);
        let (svg, hits, overlays) = c.sc.finish(w, h);
        self.hits = hits;
        self.overlays = overlays;
        self.list_area = None;
        let _ = text;
        self.scroll = 0.0;
        self.max_scroll = 0.0;
        self.base = self.engine.render(&svg, w, h, scale);
        self.scale = scale;
        self.dirty = false;
        self.compose();
    }

    fn rebuild(&mut self) {
        if self.model.surface == Surface::Editor {
            return self.rebuild_editor();
        }
        let (w, h, scale) = self.du();
        let layout = pages::Layout::new(w, h);
        let mut pass = 0;
        loop {
            pass += 1;
            let mut c = Ctx {
                sc: Scene::new(),
                p: theme::palette(self.model.dark),
                m: &self.model,
                eng: &self.engine,
                ix: Interaction { hover: self.hover, pressed: self.pressed, focus: self.focus, focus_visible: self.focus_visible },
                list_first: self.list_first,
            };
            let viewport = Rect::new(SIDEBAR_W, HEADER_H, w - SIDEBAR_W, h - HEADER_H);
            chrome::backdrop(&mut c, w, h);
            c.sc.begin_translate(0.0, -self.scroll);
            c.sc.in_scroll = true;
            let built = pages::build(&mut c, &layout);
            c.sc.in_scroll = false;
            c.sc.end_group();
            chrome::front(&mut c, w, h);
            let content_h = built.bottom + CONTENT_BOTTOM_PAD - HEADER_H;
            let max_scroll = (content_h - viewport.h).max(0.0);
            if self.scroll > max_scroll + 0.5 && pass < 3 {
                self.scroll = max_scroll;
                self.max_scroll = max_scroll;
                continue;
            }
            self.max_scroll = max_scroll;
            if max_scroll > 0.0 {
                let track = Rect::new(w - 8.0, HEADER_H + 6.0, 4.0, viewport.h - 12.0);
                let thumb_h = (track.h * viewport.h / content_h).max(32.0);
                let ty = track.y + (track.h - thumb_h) * (self.scroll / max_scroll);
                let thumb = c.p.text_muted;
                c.sc.rect(Rect::new(track.x, ty, track.w, thumb_h), 2.0, thumb);
            }
            let list_area = c.sc.list_area;
            let (svg, hits, overlays) = c.sc.finish(w, h);
            self.hits = hits;
            self.overlays = overlays;
            self.list_area = list_area;
            self.base = self.engine.render(&svg, w, h, scale);
            self.scale = scale;
            break;
        }
        self.dirty = false;
        self.compose();
    }

    /// Re-composite the raster overlays (OLED mirror, QR) onto the cached SVG pass.
    fn compose(&mut self) {
        let Some(base) = &self.base else { return };
        self.frame.clear();
        self.frame.extend_from_slice(base.data());
        self.blit_overlays();
    }

    /// Draw the overlays into the current frame. Every overlay pixel is overwritten opaquely and pixels outside
    /// its rounded corners are never touched, so repeating this without restoring the base is exact.
    fn blit_overlays(&mut self) {
        let Some(base) = &self.base else { return };
        if self.frame.len() != base.data().len() {
            return;
        }
        let (bw, bh) = (base.width() as i32, base.height() as i32);
        let scale = self.scale;
        let viewport_top = HEADER_H * scale;
        for ov in self.overlays.clone() {
            let ry = if ov.scrolls { ov.rect.y - self.scroll } else { ov.rect.y };
            let dst = (
                (ov.rect.x * scale).round() as i32,
                (ry * scale).round() as i32,
                (ov.rect.w * scale).round().max(1.0) as i32,
                (ov.rect.h * scale).round().max(1.0) as i32,
            );
            let clip_top = if ov.scrolls { viewport_top.ceil() as i32 } else { 0 };
            let radius = ov.radius * scale;
            match ov.kind {
                OverlayKind::Oled => blit_oled(&mut self.frame, bw, bh, dst, clip_top, radius, &self.model.oled),
                OverlayKind::Qr => {
                    if let Some(img) = &self.model.qr {
                        blit_image(&mut self.frame, bw, bh, dst, clip_top, img);
                    }
                }
            }
        }
    }

    /// Render if anything changed and return the pixels.
    pub fn frame(&mut self) -> Option<Frame<'_>> {
        if self.needs_render() {
            self.rebuild();
        }
        let base = self.base.as_ref()?;
        Some(Frame { width: base.width(), height: base.height(), rgba: &self.frame })
    }

    /// Re-composite only (use after `model.oled` changed; far cheaper than a full render).
    pub fn refresh_overlays(&mut self) {
        if self.base.is_some() && !self.dirty {
            self.blit_overlays();
        }
    }

    /// The last composited frame without triggering a render (`None` before the first one).
    pub fn current_frame(&self) -> Option<Frame<'_>> {
        let base = self.base.as_ref()?;
        Some(Frame { width: base.width(), height: base.height(), rgba: &self.frame })
    }

    /// Forget a press that will never see its button-up (capture lost, window deactivated).
    pub fn cancel_press(&mut self) {
        if self.pressed.is_some() || self.drag.is_some() {
            self.pressed = None;
            self.drag = None;
            self.dirty = true;
        }
    }

    /// Pixel rectangle `(x, y, w, h)` of the live OLED mirror, if visible (for partial invalidation).
    pub fn oled_rect_px(&self) -> Option<(i32, i32, i32, i32)> {
        let ov = self.overlays.iter().find(|o| o.kind == OverlayKind::Oled)?;
        let y = if ov.scrolls { ov.rect.y - self.scroll } else { ov.rect.y };
        let s = self.scale;
        Some(((ov.rect.x * s) as i32, (y * s) as i32, (ov.rect.w * s).ceil() as i32 + 1, (ov.rect.h * s).ceil() as i32 + 1))
    }

    /// Pixels per design unit of the last rendered frame.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Pixel rectangle `(x, y, w, h)` the editor's native text control should occupy (editor surface only).
    /// Available before the first render, so the host can place its control while handling `WM_SIZE`.
    pub fn editor_text_rect_px(&self) -> Option<(i32, i32, i32, i32)> {
        if self.model.surface != Surface::Editor {
            return None;
        }
        let (w, h, s) = self.du();
        let r = editor::text_rect(w, h);
        Some(((r.x * s).round() as i32, (r.y * s).round() as i32, (r.w * s).round() as i32, (r.h * s).round() as i32))
    }

    // ───────────────────────── input ─────────────────────────

    fn to_du(&self, x: f32, y: f32) -> (f32, f32) {
        let s = self.current_scale();
        (x / s, y / s)
    }

    fn hit_at(&self, x: f32, y: f32) -> Option<&Hit> {
        let in_content = x >= SIDEBAR_W && y >= HEADER_H;
        self.hits.iter().rev().find(|h| {
            if h.scrolls {
                in_content && h.rect.contains(x, y + self.scroll)
            } else {
                h.rect.contains(x, y)
            }
        })
    }

    pub fn cursor_at(&self, x_px: f32, y_px: f32) -> Cursor {
        let (x, y) = self.to_du(x_px, y_px);
        if self.drag.is_some() || self.hit_at(x, y).is_some() { Cursor::Hand } else { Cursor::Arrow }
    }

    fn set_hover(&mut self, hover: Option<u32>) {
        if hover != self.hover {
            self.hover = hover;
            self.dirty = true;
        }
    }

    pub fn mouse_move(&mut self, x_px: f32, y_px: f32) {
        let (x, y) = self.to_du(x_px, y_px);
        if let Some(drag) = self.drag {
            self.slide_to(drag, x);
            return;
        }
        let id = self.hit_at(x, y).map(|h| h.id);
        self.set_hover(id);
    }

    pub fn mouse_leave(&mut self) {
        if self.drag.is_none() {
            self.set_hover(None);
        }
    }

    pub fn mouse_down(&mut self, x_px: f32, y_px: f32) {
        let (x, y) = self.to_du(x_px, y_px);
        self.focus_visible = false;
        let Some(hit) = self.hit_at(x, y).cloned() else {
            self.pressed = None;
            self.dirty = true;
            return;
        };
        self.pressed = Some(hit.id);
        self.hover = Some(hit.id);
        if hit.focusable {
            self.focus = Some(hit.id);
        }
        if let HitKind::Slider { track, min, max } = hit.kind {
            let drag = SliderDrag { track, min, max };
            self.drag = Some(drag);
            self.slide_to(drag, x);
        }
        self.dirty = true;
    }

    pub fn mouse_up(&mut self, x_px: f32, y_px: f32) {
        let (x, y) = self.to_du(x_px, y_px);
        let pressed = self.pressed.take();
        let was_dragging = self.drag.take().is_some();
        self.dirty = true;
        let current = self.hit_at(x, y).cloned();
        self.hover = current.as_ref().map(|h| h.id);
        if was_dragging {
            return;
        }
        if let (Some(p), Some(hit)) = (pressed, current) {
            if p == hit.id {
                self.activate(&hit);
            }
        }
    }

    fn activate(&mut self, hit: &Hit) {
        match hit.kind {
            HitKind::Click => self.events.push(UiEvent::Action(hit.id)),
            HitKind::Nav => {
                if let Some(page) = Page::from_nav_id(hit.id) {
                    self.set_page(page);
                }
            }
            HitKind::ListRow => {
                let row = (hit.id - ids::LIST_BASE) as usize;
                let again = self.last_click.is_some_and(|(id, t)| id == hit.id && t.elapsed() < DOUBLE_CLICK);
                if self.model.profile_selected != row {
                    self.model.profile_selected = row;
                    self.events.push(UiEvent::ProfileSelected(row));
                }
                if again {
                    self.events.push(UiEvent::Action(ids::PROFILE_SELECT));
                    self.last_click = None;
                } else {
                    self.last_click = Some((hit.id, Instant::now()));
                }
            }
            HitKind::Slider { .. } => {}
        }
    }

    pub fn set_page(&mut self, page: Page) {
        if self.model.page != page {
            self.model.page = page;
            self.scroll = 0.0;
            self.list_first = 0;
            self.dirty = true;
            self.events.push(UiEvent::PageChanged(page));
        }
    }

    fn slide_to(&mut self, drag: SliderDrag, x_du: f32) {
        let frac = ((x_du - drag.track.x) / drag.track.w).clamp(0.0, 1.0);
        let span = (drag.max - drag.min) as f32;
        let value = drag.min + (frac * span).round() as u8;
        if self.model.brightness != Some(value) {
            self.model.brightness = Some(value);
            self.events.push(UiEvent::Brightness(value));
            self.dirty = true;
        }
    }

    /// Mouse wheel; `lines` > 0 scrolls up. Position decides between the profile list and the page.
    pub fn wheel(&mut self, x_px: f32, y_px: f32, lines: f32) {
        let (x, y) = self.to_du(x_px, y_px);
        if let Some((area, n)) = self.list_area {
            let visible = pages::list_visible_rows();
            if n > visible && x >= SIDEBAR_W && y >= HEADER_H && area.contains(x, y + self.scroll) {
                let max_first = n - visible;
                let next = if lines > 0.0 { self.list_first.saturating_sub(1) } else { (self.list_first + 1).min(max_first) };
                if next != self.list_first {
                    self.list_first = next;
                    self.dirty = true;
                }
                return;
            }
        }
        self.scroll_by(-lines * 64.0);
    }

    pub fn scroll_by(&mut self, delta: f32) {
        let next = (self.scroll + delta).clamp(0.0, self.max_scroll.max(0.0));
        if (next - self.scroll).abs() > 0.01 {
            self.scroll = next;
            self.dirty = true;
        }
    }

    pub fn key_down(&mut self, key: Key) {
        match key {
            Key::Escape => self.events.push(UiEvent::Close),
            Key::Tab { shift } => self.move_focus(if shift { -1 } else { 1 }),
            Key::Enter | Key::Space => {
                if let Some(hit) = self.focus.and_then(|id| self.hits.iter().find(|h| h.id == id)).cloned() {
                    self.focus_visible = true;
                    self.activate(&hit);
                    self.dirty = true;
                }
            }
            Key::Left | Key::Right | Key::Up | Key::Down => self.arrow(key),
            Key::PageUp => self.scroll_by(-(self.du().1 - HEADER_H) * 0.85),
            Key::PageDown => self.scroll_by((self.du().1 - HEADER_H) * 0.85),
            Key::Home => self.scroll_by(-self.scroll),
            Key::End => self.scroll_by(self.max_scroll),
        }
    }

    fn arrow(&mut self, key: Key) {
        let Some(id) = self.focus else { return };
        // A list row may have scrolled out of view (and so out of `hits`) since it was focused; it is still a row.
        let kind = match self.hits.iter().find(|h| h.id == id) {
            Some(h) => h.kind,
            None if (ids::LIST_BASE..ids::LIST_BASE + 10_000).contains(&id) => HitKind::ListRow,
            None => return,
        };
        self.focus_visible = true;
        self.dirty = true;
        match kind {
            HitKind::Slider { min, max, .. } => {
                let cur = self.model.brightness.unwrap_or((min + max) / 2);
                let next = match key {
                    Key::Left | Key::Down => cur.saturating_sub(1).max(min),
                    _ => (cur + 1).min(max),
                };
                if self.model.brightness != Some(next) {
                    self.model.brightness = Some(next);
                    self.events.push(UiEvent::Brightness(next));
                }
            }
            HitKind::ListRow => {
                let n = self.model.profiles.len();
                if n == 0 {
                    return;
                }
                let row = (id - ids::LIST_BASE) as usize;
                let next = match key {
                    Key::Up | Key::Left => row.saturating_sub(1),
                    _ => (row + 1).min(n - 1),
                };
                self.model.profile_selected = next;
                self.focus = Some(ids::LIST_BASE + next as u32);
                let visible = pages::list_visible_rows();
                if next < self.list_first {
                    self.list_first = next;
                } else if next >= self.list_first + visible {
                    self.list_first = next + 1 - visible;
                }
                self.events.push(UiEvent::ProfileSelected(next));
            }
            _ => self.move_focus(if matches!(key, Key::Up | Key::Left) { -1 } else { 1 }),
        }
    }

    fn move_focus(&mut self, dir: i32) {
        let order: Vec<(u32, bool, Rect)> = self.hits.iter().filter(|h| h.focusable).map(|h| (h.id, h.scrolls, h.rect)).collect();
        if order.is_empty() {
            return;
        }
        let cur = self.focus.and_then(|id| order.iter().position(|(i, _, _)| *i == id));
        let next = match cur {
            Some(i) => (i as i32 + dir).rem_euclid(order.len() as i32) as usize,
            None => if dir > 0 { 0 } else { order.len() - 1 },
        };
        let (id, scrolls, rect) = order[next];
        self.focus = Some(id);
        self.focus_visible = true;
        if scrolls {
            let (_, h, _) = self.du();
            let top = HEADER_H + 8.0;
            let bottom = h - 8.0;
            if rect.y - self.scroll < top {
                self.scroll = (rect.y - top).max(0.0);
            } else if rect.bottom() - self.scroll > bottom {
                self.scroll = (rect.bottom() - bottom).min(self.max_scroll);
            }
        }
        self.dirty = true;
    }

    /// Page the pointer-free "focus" currently sits on (used by tests).
    pub fn focused(&self) -> Option<u32> {
        self.focus
    }

    pub fn hits(&self) -> &[Hit] {
        &self.hits
    }

    /// Scroll offset in design units (tests / host).
    pub fn scroll_offset(&self) -> f32 {
        self.scroll
    }
}

// ───────────────────────── raster overlays ─────────────────────────

fn inside_round(px: f32, py: f32, w: f32, h: f32, r: f32) -> bool {
    if r <= 0.5 {
        return true;
    }
    let cx = if px < r { r } else if px > w - r { w - r } else { return true };
    let cy = if py < r { r } else if py > h - r { h - r } else { return true };
    let (dx, dy) = (px - cx, py - cy);
    dx * dx + dy * dy <= r * r
}

fn put(frame: &mut [u8], bw: i32, bh: i32, x: i32, y: i32, rgb: [u8; 3]) {
    if x < 0 || y < 0 || x >= bw || y >= bh {
        return;
    }
    let i = ((y * bw + x) * 4) as usize;
    frame[i] = rgb[0];
    frame[i + 1] = rgb[1];
    frame[i + 2] = rgb[2];
    frame[i + 3] = 255;
}

fn blit_oled(frame: &mut [u8], bw: i32, bh: i32, dst: (i32, i32, i32, i32), clip_top: i32, radius: f32, oled: &[bool]) {
    let (dx, dy, dw, dh) = dst;
    for y in 0..dh {
        let sy = ((y as i64 * OLED_H as i64) / dh as i64) as usize;
        for x in 0..dw {
            if !inside_round(x as f32 + 0.5, y as f32 + 0.5, dw as f32, dh as f32, radius) || dy + y < clip_top {
                continue;
            }
            let sx = ((x as i64 * OLED_W as i64) / dw as i64) as usize;
            let on = oled.get(sy * OLED_W + sx).copied().unwrap_or(false);
            put(frame, bw, bh, dx + x, dy + y, if on { [255, 255, 255] } else { [0, 0, 0] });
        }
    }
}

fn blit_image(frame: &mut [u8], bw: i32, bh: i32, dst: (i32, i32, i32, i32), clip_top: i32, img: &model::Image) {
    let (dx, dy, dw, dh) = dst;
    if img.w == 0 || img.h == 0 || img.rgba.len() < (img.w * img.h * 4) as usize {
        return;
    }
    for y in 0..dh {
        if dy + y < clip_top {
            continue;
        }
        let sy = ((y as i64 * img.h as i64) / dh as i64) as u32;
        for x in 0..dw {
            let sx = ((x as i64 * img.w as i64) / dw as i64) as u32;
            let i = ((sy * img.w + sx) * 4) as usize;
            let a = img.rgba[i + 3] as u32;
            // Straight alpha over white (the QR card is white).
            let mix = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
            put(frame, bw, bh, dx + x, dy + y, [mix(img.rgba[i]), mix(img.rgba[i + 1]), mix(img.rgba[i + 2])]);
        }
    }
}
