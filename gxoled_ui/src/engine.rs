//! Software renderer: SVG text in, premultiplied RGBA pixels out. Fonts are embedded (Inter, SIL OFL 1.1),
//! so the result is identical on every machine and no system fonts are ever touched.

use resvg::{tiny_skia, usvg};
use std::{cell::RefCell, collections::HashMap, sync::Arc};

pub use tiny_skia::Pixmap;

const FONT_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.otf");
const FONT_MEDIUM: &[u8] = include_bytes!("../fonts/Inter-Medium.otf");
const FONT_SEMIBOLD: &[u8] = include_bytes!("../fonts/Inter-SemiBold.otf");
const FONT_BOLD: &[u8] = include_bytes!("../fonts/Inter-Bold.otf");

/// Family name + weight to request for a numeric CSS weight. The Medium and SemiBold files register as their own
/// families (name ID 1), so asking "Inter" for 600 would silently fall back to Bold.
pub fn font_attrs(weight: u16) -> (&'static str, u16) {
    match weight {
        0..=450 => ("Inter", 400),
        451..=550 => ("Inter Medium", 500),
        551..=650 => ("Inter SemiBold", 600),
        _ => ("Inter", 700),
    }
}

pub struct Engine {
    opt: usvg::Options<'static>,
    measured: RefCell<HashMap<(String, u32, u16, i32), f32>>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        let mut db = usvg::fontdb::Database::new();
        for font in [FONT_REGULAR, FONT_MEDIUM, FONT_SEMIBOLD, FONT_BOLD] {
            db.load_font_data(font.to_vec());
        }
        let mut opt = usvg::Options::default();
        opt.fontdb = Arc::new(db);
        opt.font_family = "Inter".to_owned();
        Self { opt, measured: RefCell::new(HashMap::new()) }
    }

    /// Rasterise `svg` (authored in design units, `width`×`height`) at `scale` pixels per unit.
    pub fn render(&self, svg: &str, width_du: f32, height_du: f32, scale: f32) -> Option<Pixmap> {
        let tree = usvg::Tree::from_str(svg, &self.opt).ok()?;
        let w = (width_du * scale).round().max(1.0) as u32;
        let h = (height_du * scale).round().max(1.0) as u32;
        let mut pixmap = Pixmap::new(w, h)?;
        resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
        Some(pixmap)
    }

    /// Width of `text` in design units with real shaping and kerning (cached).
    pub fn text_width(&self, text: &str, size: f32, weight: u16, letter_spacing: f32) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let key = (text.to_owned(), (size * 100.0) as u32, weight, (letter_spacing * 100.0) as i32);
        if let Some(width) = self.measured.borrow().get(&key) {
            return *width;
        }
        let (family, w) = font_attrs(weight);
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="4000" height="200"><text x="0" y="100" font-family="{family}" font-weight="{w}" font-size="{size}" letter-spacing="{letter_spacing}">{}</text></svg>"#,
            crate::scene::escape(text)
        );
        let width = usvg::Tree::from_str(&svg, &self.opt)
            .ok()
            .map(|tree| tree.root().abs_bounding_box().right().max(0.0))
            .unwrap_or(text.chars().count() as f32 * size * 0.55);
        self.measured.borrow_mut().insert(key, width);
        width
    }

    /// Greedy word wrap to `max_width` (design units). Explicit '\n' breaks are honoured.
    pub fn wrap(&self, text: &str, size: f32, weight: u16, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
                if !line.is_empty() && self.text_width(&candidate, size, weight, 0.0) > max_width {
                    lines.push(std::mem::take(&mut line));
                    line = word.to_owned();
                } else {
                    line = candidate;
                }
            }
            lines.push(line);
        }
        lines
    }
}
