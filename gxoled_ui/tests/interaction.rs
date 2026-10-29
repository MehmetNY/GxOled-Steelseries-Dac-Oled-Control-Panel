//! Behaviour of the modern UI toolkit without any window: rendering, hit testing, hover/press/focus,
//! keyboard navigation, scrolling, slider and profile-list handling.

use gxoled_ui::{
    ids,
    model::{Cursor, Lang, Page, UiEvent, UiModel, OLED_H, OLED_W},
    scene::{Hit, HitKind},
    Key, Ui,
};

const W: u32 = 1450;
const H: u32 = 974;

fn model(page: Page) -> UiModel {
    UiModel {
        page,
        checked: vec![ids::MODE_WINDOW, ids::LANG_EN, ids::THEME_DARK, ids::STARTUP, ids::VOLUME_HOLD_BASE + 3],
        brightness: Some(7),
        profiles: (0..14).map(|i| format!("Profile {i}")).collect(),
        profile_selected: 1,
        profile_running: Some(0),
        connected: true,
        version: "v0.5.3 Beta".into(),
        ..UiModel::default()
    }
}

fn ui(page: Page, w: u32, h: u32, dpi: f32) -> Ui {
    let mut ui = Ui::new();
    ui.set_size(w, h, dpi);
    ui.model = model(page);
    ui.invalidate();
    ui.frame().expect("frame");
    ui
}

/// Centre of a hit rect in physical pixels under the current scroll position.
fn centre(ui: &Ui, hit: &Hit, w: u32, dpi: f32) -> (f32, f32) {
    let s = Ui::scale_for(w, dpi);
    let y = if hit.scrolls { hit.rect.cy() - ui.scroll_offset() } else { hit.rect.cy() };
    (hit.rect.cx() * s, y * s)
}

fn find(ui: &Ui, id: u32) -> Hit {
    ui.hits().iter().find(|h| h.id == id).cloned().unwrap_or_else(|| panic!("no hit with id {id}"))
}

fn click(ui: &mut Ui, id: u32) {
    let hit = find(ui, id);
    let (x, y) = centre(ui, &hit, W, 1.0);
    ui.mouse_move(x, y);
    ui.mouse_down(x, y);
    ui.mouse_up(x, y);
}

#[test]
fn every_page_renders_in_every_variant() {
    for (w, h, dpi) in [(W, H, 1.0), (1240, 840, 1.0), (1000, 700, 1.0), (1860, 1260, 1.5), (2400, 1400, 2.0)] {
        let mut ui = Ui::new();
        ui.set_size(w, h, dpi);
        for lang in [Lang::En, Lang::Tr] {
            for dark in [true, false] {
                for page in Page::ALL {
                    let mut m = model(page);
                    m.lang = lang;
                    m.dark = dark;
                    ui.model = m;
                    ui.invalidate();
                    let f = ui.frame().expect("frame");
                    assert_eq!((f.width, f.height), (w, h));
                    assert_eq!(f.rgba.len(), (w * h * 4) as usize);
                    // Opaque and not blank.
                    assert!(f.rgba.chunks_exact(4).all(|p| p[3] == 255), "{page:?} {lang:?} dark={dark} has transparent pixels");
                    let first = &f.rgba[..4];
                    assert!(f.rgba.chunks_exact(4).any(|p| p != first), "{page:?} rendered flat");
                }
            }
        }
    }
}

#[test]
fn hit_rects_are_finite_and_inside_the_window_width() {
    for (w, h) in [(W, H), (1000, 700), (1240, 840)] {
        let mut ui = Ui::new();
        ui.set_size(w, h, 1.0);
        for page in Page::ALL {
            ui.model = model(page);
            ui.invalidate();
            ui.frame();
            let scale = Ui::scale_for(w, 1.0);
            let width_du = w as f32 / scale;
            for hit in ui.hits() {
                let r = hit.rect;
                assert!(r.x.is_finite() && r.y.is_finite() && r.w > 0.0 && r.h > 0.0, "{page:?} id {} bad rect {r:?}", hit.id);
                assert!(r.x >= 0.0 && r.right() <= width_du + 0.5, "{page:?} id {} outside window: {r:?} (width {width_du})", hit.id);
            }
        }
    }
}

#[test]
fn action_ids_are_unique_per_page_except_mirrored_modes() {
    let mirrored = [ids::MODE_TRAY, ids::MODE_WINDOW, ids::MODE_HYBRID];
    let mut ui = Ui::new();
    ui.set_size(W, H, 1.0);
    for page in Page::ALL {
        ui.model = model(page);
        ui.invalidate();
        ui.frame();
        let mut seen: Vec<u32> = Vec::new();
        for hit in ui.hits() {
            if seen.contains(&hit.id) {
                assert!(mirrored.contains(&hit.id), "duplicate hit id {} on {page:?}", hit.id);
                assert!(!hit.focusable, "the mirrored copy of {} must not take a second Tab stop", hit.id);
            }
            seen.push(hit.id);
        }
    }
}

#[test]
fn clicking_a_radio_reports_its_action() {
    let mut ui = ui(Page::General, W, H, 1.0);
    click(&mut ui, ids::THEME_LIGHT);
    assert_eq!(ui.take_events(), vec![UiEvent::Action(ids::THEME_LIGHT)]);
}

#[test]
fn press_and_release_on_different_targets_does_nothing() {
    let mut ui = ui(Page::General, W, H, 1.0);
    let a = find(&ui, ids::THEME_LIGHT);
    let b = find(&ui, ids::THEME_DARK);
    let (ax, ay) = centre(&ui, &a, W, 1.0);
    let (bx, by) = centre(&ui, &b, W, 1.0);
    ui.mouse_down(ax, ay);
    ui.mouse_up(bx, by);
    assert!(ui.take_events().is_empty());
}

#[test]
fn hover_marks_dirty_and_changes_the_cursor() {
    let mut ui = ui(Page::General, W, H, 1.0);
    let hit = find(&ui, ids::LANG_TR);
    let (x, y) = centre(&ui, &hit, W, 1.0);
    assert_eq!(ui.cursor_at(2.0, 2.0), Cursor::Arrow);
    assert!(!ui.needs_render());
    ui.mouse_move(x, y);
    assert!(ui.needs_render(), "hover must schedule a repaint");
    assert_eq!(ui.cursor_at(x, y), Cursor::Hand);
    ui.frame();
    ui.mouse_leave();
    assert!(ui.needs_render());
}

#[test]
fn sidebar_navigation_switches_pages_and_resets_scroll() {
    let mut ui = ui(Page::Display, W, 600, 1.0);
    ui.scroll_by(200.0);
    assert!(ui.scroll_offset() > 0.0);
    let nav = Page::Media.nav_id();
    let hit = find(&ui, nav);
    let (x, y) = centre(&ui, &hit, W, 1.0);
    ui.mouse_down(x, y);
    ui.mouse_up(x, y);
    assert_eq!(ui.model.page, Page::Media);
    assert_eq!(ui.scroll_offset(), 0.0);
    assert!(ui.take_events().contains(&UiEvent::PageChanged(Page::Media)));
}

#[test]
fn escape_asks_the_host_to_close() {
    let mut ui = ui(Page::General, W, H, 1.0);
    ui.key_down(Key::Escape);
    assert_eq!(ui.take_events(), vec![UiEvent::Close]);
}

#[test]
fn tab_walks_the_focus_ring_and_enter_activates() {
    let mut ui = ui(Page::General, W, H, 1.0);
    assert_eq!(ui.focused(), None);
    ui.key_down(Key::Tab { shift: false });
    let first = ui.focused().expect("focus after Tab");
    ui.key_down(Key::Tab { shift: false });
    let second = ui.focused().unwrap();
    assert_ne!(first, second);
    ui.key_down(Key::Tab { shift: true });
    assert_eq!(ui.focused(), Some(first));
    for _ in 0..80 {
        if ui.focused() == Some(ids::THEME_LIGHT) {
            break;
        }
        ui.key_down(Key::Tab { shift: false });
    }
    assert_eq!(ui.focused(), Some(ids::THEME_LIGHT), "Tab must reach every control");
    ui.take_events();
    ui.key_down(Key::Enter);
    assert_eq!(ui.take_events(), vec![UiEvent::Action(ids::THEME_LIGHT)]);
    ui.key_down(Key::Space);
    assert_eq!(ui.take_events(), vec![UiEvent::Action(ids::THEME_LIGHT)]);
}

#[test]
fn shift_tab_from_nothing_goes_to_the_last_control() {
    let mut ui = ui(Page::General, W, H, 1.0);
    ui.key_down(Key::Tab { shift: true });
    let last = ui.hits().iter().rev().find(|h| h.focusable).map(|h| h.id);
    assert_eq!(ui.focused(), last);
}

#[test]
fn focusing_an_offscreen_control_scrolls_it_into_view() {
    let mut ui = ui(Page::Display, W, 560, 1.0);
    assert_eq!(ui.scroll_offset(), 0.0);
    for _ in 0..40 {
        ui.key_down(Key::Tab { shift: false });
    }
    ui.frame();
    assert!(ui.scroll_offset() > 0.0, "tabbing deep into the page must scroll");
}

#[test]
fn wheel_scrolls_the_page_and_clamps() {
    let mut ui = ui(Page::Display, W, 500, 1.0);
    ui.wheel(700.0, 300.0, -3.0);
    let a = ui.scroll_offset();
    assert!(a > 0.0);
    for _ in 0..100 {
        ui.wheel(700.0, 300.0, -3.0);
    }
    ui.frame();
    let end = ui.scroll_offset();
    ui.wheel(700.0, 300.0, -3.0);
    assert_eq!(ui.scroll_offset(), end, "scroll must clamp at the bottom");
    for _ in 0..200 {
        ui.wheel(700.0, 300.0, 3.0);
    }
    assert_eq!(ui.scroll_offset(), 0.0);
}

#[test]
fn page_that_fits_does_not_scroll() {
    let mut ui = ui(Page::Media, W, 2000, 1.0);
    ui.wheel(700.0, 300.0, -3.0);
    assert_eq!(ui.scroll_offset(), 0.0);
}

#[test]
fn brightness_slider_maps_the_track_to_1_through_10() {
    let mut ui = ui(Page::Display, W, 1400, 1.0);
    let hit = find(&ui, ids::BRIGHTNESS);
    let HitKind::Slider { track, min, max } = hit.kind else { panic!("brightness is not a slider") };
    assert_eq!((min, max), (1, 10));
    let s = Ui::scale_for(W, 1.0);
    let y = (track.cy() - ui.scroll_offset()) * s;
    ui.mouse_down((track.x + 1.0) * s, y);
    assert!(matches!(ui.take_events().last(), Some(UiEvent::Brightness(1))));
    // Dragging far to the left of the track stays clamped at the minimum.
    ui.mouse_move(0.0, y);
    assert_eq!(ui.model.brightness, Some(1));
    ui.mouse_move((track.right() + 60.0) * s, y);
    assert!(matches!(ui.take_events().last(), Some(UiEvent::Brightness(10))));
    ui.mouse_move((track.x + track.w * 0.5) * s, y);
    let mid = ui.take_events();
    let Some(UiEvent::Brightness(v)) = mid.last() else { panic!("no brightness event: {mid:?}") };
    assert!((5..=6).contains(v), "midpoint gave {v}");
    ui.mouse_up(0.0, 0.0);
    assert!(ui.take_events().is_empty(), "releasing a drag is not a click");
    assert_eq!(ui.model.brightness, Some(*v));
}

#[test]
fn slider_arrow_keys_step_by_one() {
    let mut ui = ui(Page::Display, W, 1400, 1.0);
    let hit = find(&ui, ids::BRIGHTNESS);
    let (x, y) = centre(&ui, &hit, W, 1.0);
    ui.mouse_down(x, y);
    ui.mouse_up(x, y);
    ui.take_events();
    ui.model.brightness = Some(4);
    ui.key_down(Key::Right);
    assert_eq!(ui.take_events(), vec![UiEvent::Brightness(5)]);
    ui.key_down(Key::Left);
    ui.key_down(Key::Left);
    assert_eq!(ui.model.brightness, Some(3));
}

#[test]
fn profile_list_selection_and_double_click() {
    let mut ui = ui(Page::Animations, W, 1100, 1.0);
    let row = ids::LIST_BASE + 3;
    click(&mut ui, row);
    assert_eq!(ui.model.profile_selected, 3);
    assert_eq!(ui.take_events(), vec![UiEvent::ProfileSelected(3)]);
    // Second click within the double-click window selects-and-runs.
    click(&mut ui, row);
    assert_eq!(ui.take_events(), vec![UiEvent::Action(ids::PROFILE_SELECT)]);
}

#[test]
fn profile_list_wheel_scrolls_the_list_not_the_page() {
    let mut ui = ui(Page::Animations, W, 600, 1.0);
    assert!(ui.model.profiles.len() > gxoled_ui::pages::list_visible_rows());
    let first = find(&ui, ids::LIST_BASE);
    let (x, y) = centre(&ui, &first, W, 1.0);
    let before = ui.hits().iter().filter(|h| h.id >= ids::LIST_BASE && h.id < ids::LIST_BASE + 100).count();
    ui.wheel(x, y, -1.0);
    ui.frame();
    assert_eq!(ui.scroll_offset(), 0.0, "wheel over the list must not scroll the page");
    let after = ui.hits().iter().filter(|h| h.id >= ids::LIST_BASE && h.id < ids::LIST_BASE + 100).count();
    assert_eq!(before, after, "the list keeps showing a fixed number of rows");
    // The first visible row is now row 1.
    assert!(ui.hits().iter().any(|h| h.id == ids::LIST_BASE + 1));
    assert!(!ui.hits().iter().any(|h| h.id == ids::LIST_BASE));
}

#[test]
fn arrow_keys_move_the_profile_selection() {
    let mut ui = ui(Page::Animations, W, 1100, 1.0);
    click(&mut ui, ids::LIST_BASE + 1);
    ui.take_events();
    ui.key_down(Key::Down);
    ui.key_down(Key::Down);
    assert_eq!(ui.model.profile_selected, 3);
    ui.key_down(Key::Up);
    assert_eq!(ui.model.profile_selected, 2);
    assert_eq!(ui.take_events().last(), Some(&UiEvent::ProfileSelected(2)));
    for _ in 0..30 {
        ui.key_down(Key::Down);
    }
    assert_eq!(ui.model.profile_selected, ui.model.profiles.len() - 1);
}

#[test]
fn scroll_position_survives_a_model_refresh() {
    let mut ui = ui(Page::Display, W, 500, 1.0);
    ui.scroll_by(150.0);
    ui.frame();
    let before = ui.scroll_offset();
    ui.model.checked.push(ids::SHIFT_HIGH);
    ui.invalidate();
    ui.frame();
    assert_eq!(ui.scroll_offset(), before);
}

#[test]
fn shrinking_the_window_clamps_the_scroll_offset() {
    let mut ui = ui(Page::Display, W, 500, 1.0);
    ui.scroll_by(10_000.0);
    ui.frame();
    assert!(ui.scroll_offset() > 0.0);
    ui.set_size(W, 2400, 1.0);
    ui.frame();
    assert_eq!(ui.scroll_offset(), 0.0, "a taller window leaves nothing to scroll");
}

#[test]
fn oled_mirror_pixels_reach_the_frame_without_a_full_rerender() {
    let mut ui = ui(Page::General, W, H, 1.0);
    let (x, y, w, h) = ui.oled_rect_px().expect("General page shows the live OLED");
    assert!(w > 100 && h > 50);
    let probe = |ui: &mut Ui| {
        let f = ui.frame().unwrap();
        let i = (((y + h / 2) as u32 * f.width + (x + w / 2) as u32) * 4) as usize;
        f.rgba[i]
    };
    assert_eq!(probe(&mut ui), 0, "unlit OLED is black");
    ui.model.oled = vec![true; OLED_W * OLED_H];
    ui.refresh_overlays();
    assert!(!ui.needs_render(), "overlay refresh must not trigger a full render");
    assert_eq!(probe(&mut ui), 255, "lit OLED is white");
}

#[test]
fn volume_chips_cover_the_seven_choices() {
    let ui = ui(Page::Display, W, 3000, 1.0);
    for i in 0..7 {
        let _ = find(&ui, ids::VOLUME_HOLD_BASE + i);
    }
}

#[test]
fn dpi_scaling_keeps_hit_testing_aligned() {
    let (w, h, dpi) = (1860, 1260, 1.5);
    let mut ui = ui(Page::General, w, h, dpi);
    let hit = find(&ui, ids::LANG_TR);
    let (x, y) = centre(&ui, &hit, w, dpi);
    ui.mouse_down(x, y);
    ui.mouse_up(x, y);
    assert_eq!(ui.take_events(), vec![UiEvent::Action(ids::LANG_TR)]);
}

// ───────────────────────── profile editor surface ─────────────────────────

fn editor(w: u32, h: u32, dpi: f32) -> Ui {
    let mut ui = Ui::new();
    ui.set_size(w, h, dpi);
    ui.model = UiModel {
        surface: gxoled_ui::model::Surface::Editor,
        editor_title: "matrix-001.json — matrix".into(),
        editor_status: "JSON is valid".into(),
        ..UiModel::default()
    };
    ui.invalidate();
    ui.frame().expect("frame");
    ui
}

#[test]
fn editor_reserves_a_text_area_and_five_actions() {
    for (w, h, dpi) in [(930, 680, 1.0), (860, 540, 1.0), (1395, 1020, 1.5), (1860, 1360, 2.0)] {
        let ui = editor(w, h, dpi);
        let (x, y, tw, th) = ui.editor_text_rect_px().expect("text rect");
        assert!(x > 0 && y > 0 && tw > 300 && th > 120, "text rect {x},{y} {tw}x{th} at {w}x{h}");
        assert!(x + tw < w as i32 && y + th < h as i32);
        for id in [ids::EDITOR_SAVE, ids::EDITOR_FORMAT, ids::EDITOR_IMPORT, ids::EDITOR_EXPORT, ids::EDITOR_RESET] {
            let hit = find(&ui, id);
            let s = dpi;
            assert!(hit.rect.right() * s <= w as f32 + 0.5, "button {id} sticks out at {w}x{h}");
            assert!(hit.rect.y * s >= (y + th) as f32, "button {id} overlaps the text area");
        }
    }
}

#[test]
fn editor_buttons_report_actions_and_escape_closes() {
    let mut ui = editor(930, 680, 1.0);
    for id in [ids::EDITOR_SAVE, ids::EDITOR_RESET, ids::EDITOR_FORMAT] {
        // The editor is not width-proportional: one design unit is one pixel at 100 % DPI.
        let hit = find(&ui, id);
        let (x, y) = (hit.rect.cx(), hit.rect.cy());
        ui.mouse_down(x, y);
        ui.mouse_up(x, y);
        assert_eq!(ui.take_events(), vec![UiEvent::Action(id)]);
    }
    ui.key_down(Key::Escape);
    assert_eq!(ui.take_events(), vec![UiEvent::Close]);
}

#[test]
fn editor_renders_in_both_languages_and_themes_with_long_status() {
    let mut ui = Ui::new();
    ui.set_size(930, 680, 1.0);
    for lang in [Lang::En, Lang::Tr] {
        for dark in [true, false] {
            ui.model = UiModel {
                surface: gxoled_ui::model::Surface::Editor,
                lang,
                dark,
                editor_title: "a-very-long-profile-name-that-needs-to-be-shortened-with-an-ellipsis-because-it-is-long.json".into(),
                editor_status: "Invalid JSON: ".to_owned() + &"x".repeat(400),
                editor_status_ok: false,
                ..UiModel::default()
            };
            ui.invalidate();
            let f = ui.frame().expect("frame");
            assert_eq!((f.width, f.height), (930, 680));
        }
    }
}
