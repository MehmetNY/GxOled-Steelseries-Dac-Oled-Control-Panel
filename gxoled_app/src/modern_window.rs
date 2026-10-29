//! Win32 host for the modern (beta) windows: the settings window and the profile (JSON) editor.
//!
//! The interface itself lives in the OS-independent `gxoled_ui` crate, which renders the whole window into an
//! RGBA buffer and does its own hit testing. This module only owns the HWND: it forwards mouse, keyboard and
//! wheel input, blits the finished frame with `StretchDIBits`, follows DPI and size changes, themes the native
//! title bar through DWM and, for the editor, hosts one native multi-line EDIT control.
//!
//! The module knows nothing about GxOled's configuration; `modern_ui` is the bridge to the application.
//!
//! Re-entrancy rules (Win32 delivers many messages synchronously from the calls that cause them):
//! * per-window state lives in a `RefCell` reached through `GWLP_USERDATA` and is only ever borrowed with
//!   `try_borrow_mut` for short sections that call no Win32 function able to dispatch a message;
//! * events produced by input are collected inside the borrow and handed to the application *after* it ends,
//!   so the handler may open modal file dialogs or call back into this module.
#![allow(unsafe_op_in_unsafe_fn)]

use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicIsize, Ordering},
    },
};

use gxoled_ui::{
    Key, Ui,
    model::{Cursor, Surface, UiEvent},
    theme,
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::{Dwm::DwmSetWindowAttribute, Gdi::*},
    System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
    UI::{Controls::SetWindowTheme, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Settings,
    Editor,
}

impl Kind {
    fn slot(self) -> usize {
        match self {
            Kind::Settings => 0,
            Kind::Editor => 1,
        }
    }
    fn surface(self) -> Surface {
        match self {
            Kind::Settings => Surface::Settings,
            Kind::Editor => Surface::Editor,
        }
    }
    /// Smallest client area in logical pixels.
    fn min_client(self) -> (i32, i32) {
        match self {
            Kind::Settings => (960, 620),
            Kind::Editor => (760, 480),
        }
    }
    /// Preferred client area in logical pixels.
    fn default_client(self) -> (i32, i32) {
        match self {
            Kind::Settings => (1450, 974),
            Kind::Editor => (930, 680),
        }
    }
}

/// Callbacks into the application.
pub struct Hooks {
    /// A control was activated (never `Close` / `PageChanged`; those are handled here).
    pub on_event: fn(Kind, UiEvent),
    /// The window painted its first complete frame (used to clear the start-up crash marker).
    pub on_first_paint: fn(Kind),
}

static HOOKS: OnceLock<Hooks> = OnceLock::new();
static FAILED: AtomicBool = AtomicBool::new(false);
static EDIT_PREV: AtomicIsize = AtomicIsize::new(0);

pub fn install(hooks: Hooks) {
    let _ = HOOKS.set(hooks);
}

/// True once a panic was caught inside a window procedure; the application then falls back to the classic UI.
pub fn failed() -> bool {
    FAILED.load(Ordering::Relaxed)
}

/// Record a failure caught by the caller (see `settings_ui::guarded`).
pub fn mark_failed() {
    FAILED.store(true, Ordering::Relaxed);
}

const CLASS_NAME: &str = "GxOledModernWindow2026";
const STYLE: u32 = WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN;
const WM_EDITOR_SAVE: u32 = WM_APP + 1;
const WM_MOUSELEAVE: u32 = 0x02A3; // lives in Win32_UI_Controls, which is only needed for SetWindowTheme
const EDIT_ID: usize = 4001;
const ES_MULTILINE: u32 = 0x0004;
const ES_AUTOVSCROLL: u32 = 0x0040;
const ES_NOHIDESEL: u32 = 0x0100;
const ES_WANTRETURN: u32 = 0x1000;
const EM_SETSEL: u32 = 0x00B1;
const EM_SETLIMITTEXT: u32 = 0x00C5;
const EM_SETMARGINS: u32 = 0x00D3;
const EC_LEFTMARGIN_RIGHTMARGIN: usize = 0x0003;
const WM_SETFONT_MSG: u32 = 0x0030;
const PER_MONITOR_AWARE_V2: isize = -4;

thread_local! {
    static HWNDS: Cell<[isize; 2]> = const { Cell::new([0; 2]) };
}

struct Win {
    kind: Kind,
    hwnd: HWND,
    ui: Ui,
    dark: bool,
    dpi: u32,
    bgra: Vec<u8>,
    frame_w: i32,
    frame_h: i32,
    tracking: bool,
    painted_once: bool,
    // Editor only.
    edit: HWND,
    edit_font: HFONT,
    edit_brush: HBRUSH,
}

// ───────────────────────── small helpers ─────────────────────────

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// "#rrggbb" → COLORREF (0x00BBGGRR).
fn colorref(hex: &str) -> u32 {
    let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    ((v >> 16) & 0xff) | (v & 0xff00) | ((v & 0xff) << 16)
}

fn lo(l: isize) -> i32 {
    (l & 0xffff) as u16 as i16 as i32
}

fn hi(l: isize) -> i32 {
    ((l >> 16) & 0xffff) as u16 as i16 as i32
}

fn hwnd_of(kind: Kind) -> Option<HWND> {
    let h = HWNDS.with(|c| c.get()[kind.slot()]);
    if h == 0 { None } else { Some(h as HWND) }
}

unsafe fn cell(hwnd: HWND) -> Option<&'static RefCell<Win>> {
    let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Win>;
    if p.is_null() { None } else { Some(&*p) }
}

fn with_win<R>(hwnd: HWND, f: impl FnOnce(&mut Win) -> R) -> Option<R> {
    let c = unsafe { cell(hwnd) }?;
    let mut w = c.try_borrow_mut().ok()?;
    Some(f(&mut w))
}

fn invalidate(hwnd: HWND) {
    unsafe { InvalidateRect(hwnd, null(), 0) };
}

// ───────────────────────── DPI (loaded dynamically: Windows 10 1607+) ─────────────────────────

struct DpiApi {
    window_dpi: Option<unsafe extern "system" fn(HWND) -> u32>,
    set_thread_context: Option<unsafe extern "system" fn(isize) -> isize>,
    adjust_rect: Option<unsafe extern "system" fn(*mut RECT, u32, i32, u32, u32) -> i32>,
}

fn dpi_api() -> &'static DpiApi {
    static API: OnceLock<DpiApi> = OnceLock::new();
    API.get_or_init(|| unsafe {
        let user32 = LoadLibraryW(wide("user32.dll").as_ptr());
        let get = |name: &'static [u8]| if user32.is_null() { None } else { GetProcAddress(user32, name.as_ptr()) };
        DpiApi {
            window_dpi: std::mem::transmute(get(b"GetDpiForWindow\0")),
            set_thread_context: std::mem::transmute(get(b"SetThreadDpiAwarenessContext\0")),
            adjust_rect: std::mem::transmute(get(b"AdjustWindowRectExForDpi\0")),
        }
    })
}

fn dpi_of(hwnd: HWND) -> u32 {
    match dpi_api().window_dpi {
        Some(f) => match unsafe { f(hwnd) } {
            0 => 96,
            d => d,
        },
        None => 96,
    }
}

/// Makes the calling thread per-monitor-v2 DPI aware while it exists. The classic window was created under
/// whatever awareness the process has; the thread must not stay switched, or its coordinates would be scaled.
struct DpiScope(isize);

impl DpiScope {
    fn enter() -> Self {
        match dpi_api().set_thread_context {
            Some(f) => DpiScope(unsafe { f(PER_MONITOR_AWARE_V2) }),
            None => DpiScope(0),
        }
    }
}

impl Drop for DpiScope {
    fn drop(&mut self) {
        if self.0 != 0 {
            if let Some(f) = dpi_api().set_thread_context {
                unsafe { f(self.0) };
            }
        }
    }
}

fn outer_size(client_w: i32, client_h: i32, dpi: u32) -> (i32, i32) {
    let mut r = RECT { left: 0, top: 0, right: client_w, bottom: client_h };
    unsafe {
        match dpi_api().adjust_rect {
            Some(f) => {
                f(&mut r, STYLE, 0, 0, dpi);
            }
            None => {
                AdjustWindowRectEx(&mut r, STYLE, 0, 0);
            }
        }
    }
    (r.right - r.left, r.bottom - r.top)
}

fn scale_px(logical: i32, dpi: u32) -> i32 {
    (logical as i64 * dpi as i64 / 96) as i32
}

// ───────────────────────── pixel conversion ─────────────────────────

fn rgba_to_bgra(src: &[u8], dst: &mut Vec<u8>) {
    dst.resize(src.len(), 0);
    for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        d[0] = s[2];
        d[1] = s[1];
        d[2] = s[0];
        d[3] = 255;
    }
}

impl Win {
    /// Render if needed and refresh the BGRA copy the paint handler blits.
    fn ensure_frame(&mut self) {
        if self.ui.needs_render() || self.bgra.is_empty() {
            if let Some(f) = self.ui.frame() {
                rgba_to_bgra(f.rgba, &mut self.bgra);
                self.frame_w = f.width as i32;
                self.frame_h = f.height as i32;
            }
        }
    }

    fn blit(&self, hdc: HDC) {
        if self.bgra.is_empty() || self.frame_w <= 0 || self.frame_h <= 0 {
            return;
        }
        let mut bmi: BITMAPINFO = unsafe { zeroed() };
        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = self.frame_w;
        bmi.bmiHeader.biHeight = -self.frame_h; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;
        unsafe {
            StretchDIBits(
                hdc,
                0,
                0,
                self.frame_w,
                self.frame_h,
                0,
                0,
                self.frame_w,
                self.frame_h,
                self.bgra.as_ptr().cast(),
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
    }

    /// Copy only the live-OLED rectangle of the composited frame into the BGRA buffer.
    fn refresh_oled_region(&mut self) -> Option<RECT> {
        if self.ui.needs_render() {
            return None; // a full repaint is already pending and will pick the new pixels up
        }
        self.ui.refresh_overlays();
        let (x, y, w, h) = self.ui.oled_rect_px()?;
        let f = self.ui.current_frame()?;
        if f.width as i32 != self.frame_w || f.height as i32 != self.frame_h || self.bgra.len() != f.rgba.len() {
            return None;
        }
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = ((x + w).min(self.frame_w), (y + h).min(self.frame_h));
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        for row in y0..y1 {
            let a = ((row * self.frame_w + x0) * 4) as usize;
            let b = ((row * self.frame_w + x1) * 4) as usize;
            for (d, s) in self.bgra[a..b].chunks_exact_mut(4).zip(f.rgba[a..b].chunks_exact(4)) {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = 255;
            }
        }
        Some(RECT { left: x0, top: y0, right: x1, bottom: y1 })
    }
}

// ───────────────────────── window procedure ─────────────────────────

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe { handle(hwnd, msg, wp, lp) })) {
        Ok(r) => r,
        Err(_) => {
            FAILED.store(true, Ordering::Relaxed);
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
                DefWindowProcW(hwnd, msg, wp, lp)
            }
        }
    }
}

fn dispatch(kind: Kind, hwnd: HWND, event: UiEvent) {
    match event {
        UiEvent::Close => unsafe {
            ShowWindow(hwnd, SW_HIDE);
        },
        UiEvent::PageChanged(_) => {}
        other => {
            if let Some(h) = HOOKS.get() {
                (h.on_event)(kind, other);
            }
        }
    }
}

/// After any input: schedule a repaint if the UI changed, then hand the events to the application.
fn after_input(hwnd: HWND) {
    let Some((events, dirty, kind)) = with_win(hwnd, |w| (w.ui.take_events(), w.ui.needs_render(), w.kind)) else {
        return;
    };
    if dirty {
        invalidate(hwnd);
    }
    for e in events {
        dispatch(kind, hwnd, e);
    }
}

fn key_of(vk: u16, shift: bool) -> Option<Key> {
    Some(match vk {
        VK_TAB => Key::Tab { shift },
        VK_RETURN => Key::Enter,
        VK_SPACE => Key::Space,
        VK_ESCAPE => Key::Escape,
        VK_LEFT => Key::Left,
        VK_RIGHT => Key::Right,
        VK_UP => Key::Up,
        VK_DOWN => Key::Down,
        VK_PRIOR => Key::PageUp,
        VK_NEXT => Key::PageDown,
        VK_HOME => Key::Home,
        VK_END => Key::End,
        _ => return None,
    })
}

unsafe fn handle(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let cs = lp as *const CREATESTRUCTW;
            let params = (*cs).lpCreateParams as *const RefCell<Win>;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, params as isize);
            if let Some(c) = cell(hwnd) {
                if let Ok(mut w) = c.try_borrow_mut() {
                    w.hwnd = hwnd;
                }
            }
        }
        WM_NCDESTROY => {
            let p = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut RefCell<Win>;
            if !p.is_null() {
                drop(Box::from_raw(p));
            }
        }
        WM_ERASEBKGND => return 1,
        WM_GETMINMAXINFO => {
            if let Some(c) = cell(hwnd) {
                if let Ok(w) = c.try_borrow() {
                    let (cw, ch) = w.kind.min_client();
                    let (ow, oh) = outer_size(scale_px(cw, w.dpi), scale_px(ch, w.dpi), w.dpi);
                    let mmi = lp as *mut MINMAXINFO;
                    (*mmi).ptMinTrackSize.x = ow;
                    (*mmi).ptMinTrackSize.y = oh;
                    return 0;
                }
            }
        }
        WM_SIZE => {
            if wp as u32 == SIZE_MINIMIZED {
                return 0;
            }
            let (cw, ch) = (lo(lp), hi(lp));
            if cw <= 0 || ch <= 0 {
                return 0;
            }
            let edit = with_win(hwnd, |w| {
                w.ui.set_size(cw as u32, ch as u32, w.dpi as f32 / 96.0);
                w.bgra.clear();
                (w.edit, w.ui.editor_text_rect_px())
            });
            if let Some((edit, Some((x, y, tw, th)))) = edit {
                if !edit.is_null() {
                    SetWindowPos(edit, null_mut(), x, y, tw, th, SWP_NOZORDER | SWP_NOACTIVATE);
                }
            }
            invalidate(hwnd);
            return 0;
        }
        WM_DPICHANGED => {
            let new_dpi = (wp & 0xffff) as u32;
            let suggested = lp as *const RECT;
            let font_update = with_win(hwnd, |w| {
                w.dpi = new_dpi.max(96);
                (w.edit, w.edit_font)
            });
            if let Some((edit, old_font)) = font_update {
                if !edit.is_null() {
                    let font = make_edit_font(new_dpi);
                    SendMessageW(edit, WM_SETFONT_MSG, font as usize, 1);
                    with_win(hwnd, |w| w.edit_font = font);
                    if !old_font.is_null() {
                        DeleteObject(old_font);
                    }
                }
            }
            if !suggested.is_null() {
                let r = *suggested;
                SetWindowPos(hwnd, null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
            }
            return 0;
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let first = with_win(hwnd, |w| {
                w.ensure_frame();
                w.blit(hdc);
                let first = !w.painted_once && !w.bgra.is_empty();
                if first {
                    w.painted_once = true;
                }
                (first, w.kind)
            });
            EndPaint(hwnd, &ps);
            if let Some((true, kind)) = first {
                if let Some(h) = HOOKS.get() {
                    (h.on_first_paint)(kind);
                }
            }
            return 0;
        }
        WM_SETCURSOR => {
            if (lp & 0xffff) as u32 == HTCLIENT {
                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);
                ScreenToClient(hwnd, &mut pt);
                let hand = with_win(hwnd, |w| w.ui.cursor_at(pt.x as f32, pt.y as f32) == Cursor::Hand).unwrap_or(false);
                SetCursor(LoadCursorW(null_mut(), if hand { IDC_HAND } else { IDC_ARROW }));
                return 1;
            }
        }
        WM_MOUSEMOVE => {
            let (x, y) = (lo(lp) as f32, hi(lp) as f32);
            let start_tracking = with_win(hwnd, |w| {
                if wp & 0x0001 == 0 {
                    w.ui.cancel_press(); // the button went up somewhere we did not see
                }
                w.ui.mouse_move(x, y);
                !std::mem::replace(&mut w.tracking, true)
            })
            .unwrap_or(false);
            if start_tracking {
                let mut tme = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                TrackMouseEvent(&mut tme);
            }
            after_input(hwnd);
            return 0;
        }
        WM_MOUSELEAVE => {
            with_win(hwnd, |w| {
                w.tracking = false;
                w.ui.mouse_leave();
            });
            after_input(hwnd);
            return 0;
        }
        WM_LBUTTONDOWN => {
            SetCapture(hwnd);
            if GetFocus() != hwnd {
                SetFocus(hwnd);
            }
            let (x, y) = (lo(lp) as f32, hi(lp) as f32);
            with_win(hwnd, |w| w.ui.mouse_down(x, y));
            after_input(hwnd);
            return 0;
        }
        WM_LBUTTONUP => {
            let (x, y) = (lo(lp) as f32, hi(lp) as f32);
            with_win(hwnd, |w| w.ui.mouse_up(x, y));
            ReleaseCapture();
            after_input(hwnd);
            return 0;
        }
        WM_CAPTURECHANGED => {
            with_win(hwnd, |w| w.ui.cancel_press());
            after_input(hwnd);
            return 0;
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp >> 16) & 0xffff) as u16 as i16 as f32;
            let mut pt = POINT { x: lo(lp), y: hi(lp) }; // screen coordinates
            ScreenToClient(hwnd, &mut pt);
            with_win(hwnd, |w| w.ui.wheel(pt.x as f32, pt.y as f32, delta / 120.0));
            after_input(hwnd);
            return 0;
        }
        WM_KEYDOWN => {
            let shift = GetKeyState(VK_SHIFT as i32) < 0;
            let ctrl = GetKeyState(VK_CONTROL as i32) < 0;
            if ctrl && wp == b'S' as usize {
                if let Some(h) = HOOKS.get() {
                    if with_win(hwnd, |w| w.kind) == Some(Kind::Editor) {
                        (h.on_event)(Kind::Editor, UiEvent::Action(gxoled_ui::ids::EDITOR_SAVE));
                        return 0;
                    }
                }
            }
            if let Some(key) = key_of(wp as u16, shift) {
                with_win(hwnd, |w| w.ui.key_down(key));
                after_input(hwnd);
                return 0;
            }
        }
        WM_SETFOCUS => {
            // The editor window only exists to host its text control: keep the caret there.
            if let Some(edit) = with_win(hwnd, |w| if w.kind == Kind::Editor { w.edit } else { null_mut() }) {
                if !edit.is_null() {
                    SetFocus(edit);
                    return 0;
                }
            }
        }
        WM_EDITOR_SAVE => {
            if let Some(h) = HOOKS.get() {
                (h.on_event)(Kind::Editor, UiEvent::Action(gxoled_ui::ids::EDITOR_SAVE));
            }
            return 0;
        }
        WM_CTLCOLOREDIT => {
            let colors = with_win(hwnd, |w| (w.dark, w.edit_brush));
            if let Some((dark, brush)) = colors {
                if !brush.is_null() {
                    let p = theme::palette(dark);
                    SetTextColor(wp as HDC, colorref(p.text));
                    SetBkColor(wp as HDC, colorref(p.card));
                    return brush as LRESULT;
                }
            }
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

// The multi-line EDIT does not select all on Ctrl+A and would beep on the control characters; Ctrl+S saves.
unsafe extern "system" fn edit_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let ctrl = GetKeyState(VK_CONTROL as i32) < 0;
    match msg {
        WM_KEYDOWN if ctrl && wp == b'A' as usize => {
            SendMessageW(hwnd, EM_SETSEL, 0, -1);
            return 0;
        }
        WM_KEYDOWN if ctrl && wp == b'S' as usize => {
            PostMessageW(GetParent(hwnd), WM_EDITOR_SAVE, 0, 0);
            return 0;
        }
        WM_CHAR if wp == 1 || wp == 19 => return 0,
        _ => {}
    }
    let prev = EDIT_PREV.load(Ordering::Acquire);
    if prev != 0 {
        let prev: WNDPROC = std::mem::transmute(prev);
        CallWindowProcW(prev, hwnd, msg, wp, lp)
    } else {
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

// ───────────────────────── creation ─────────────────────────

fn register_class() -> Result<(), String> {
    static REGISTERED: OnceLock<bool> = OnceLock::new();
    let ok = *REGISTERED.get_or_init(|| unsafe {
        let inst = GetModuleHandleW(null());
        let name = wide(CLASS_NAME);
        let icon = LoadIconW(inst, 1usize as *const u16); // resource id 1 = the application icon
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst,
            hIcon: icon,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: name.as_ptr(),
            hIconSm: icon,
        };
        RegisterClassExW(&class) != 0
    });
    if ok { Ok(()) } else { Err("RegisterClassExW failed".to_owned()) }
}

fn make_edit_font(dpi: u32) -> HFONT {
    unsafe {
        CreateFontW(-scale_px(15, dpi), 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, wide("Consolas").as_ptr())
    }
}

fn work_area_at_cursor() -> RECT {
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            info.rcWork
        } else {
            RECT { left: 0, top: 0, right: 1280, bottom: 720 }
        }
    }
}

/// Create the (hidden) window of `kind`. `ui` must already carry its model.
pub fn create(kind: Kind, title: &str, dark: bool, mut ui: Ui) -> Result<(), String> {
    if hwnd_of(kind).is_some() {
        return Ok(());
    }
    register_class()?;
    let _dpi_scope = DpiScope::enter();
    ui.model.surface = kind.surface();
    ui.model.dark = dark;
    ui.invalidate();
    let work = work_area_at_cursor();
    let win = Box::new(RefCell::new(Win {
        kind,
        hwnd: null_mut(),
        ui,
        dark,
        dpi: 96,
        bgra: Vec::new(),
        frame_w: 0,
        frame_h: 0,
        tracking: false,
        painted_once: false,
        edit: null_mut(),
        edit_font: null_mut(),
        edit_brush: null_mut(),
    }));
    let raw = Box::into_raw(win);
    let hwnd = unsafe {
        let inst = GetModuleHandleW(null());
        // Created small, hidden and on the right monitor so it starts out with that monitor's DPI.
        CreateWindowExW(0, wide(CLASS_NAME).as_ptr(), wide(title).as_ptr(), STYLE, work.left + 40, work.top + 40, 400, 300, null_mut(), null_mut(), inst, raw as *const c_void)
    };
    if hwnd.is_null() {
        // Whether WM_NCCREATE ran is unknown, so the allocation is deliberately leaked rather than risk a double free.
        return Err("CreateWindowExW failed".to_owned());
    }
    HWNDS.with(|c| {
        let mut a = c.get();
        a[kind.slot()] = hwnd as isize;
        c.set(a);
    });
    let dpi = dpi_of(hwnd);
    with_win(hwnd, |w| w.dpi = dpi);

    if kind == Kind::Editor {
        create_edit(hwnd, dpi, dark);
    }

    // Final size: the preferred client area, shrunk to fit the monitor, centred in its work area.
    let (dw, dh) = kind.default_client();
    let max_w = (work.right - work.left) - scale_px(40, dpi);
    let max_h = (work.bottom - work.top) - scale_px(40, dpi);
    let (ow, oh) = outer_size(scale_px(dw, dpi), scale_px(dh, dpi), dpi);
    let (ow, oh) = (ow.min(max_w), oh.min(max_h));
    let x = work.left + ((work.right - work.left) - ow) / 2;
    let y = work.top + ((work.bottom - work.top) - oh) / 2;
    unsafe {
        SetWindowPos(hwnd, null_mut(), x, y, ow, oh, SWP_NOZORDER | SWP_NOACTIVATE);
        let mut rc: RECT = zeroed();
        GetClientRect(hwnd, &mut rc);
        let (cw, ch) = (rc.right.max(1), rc.bottom.max(1));
        let placed = with_win(hwnd, |w| {
            w.ui.set_size(cw as u32, ch as u32, dpi as f32 / 96.0);
            w.ui.editor_text_rect_px()
        });
        if let Some(Some((ex, ey, ew, eh))) = placed {
            if let Some(edit) = with_win(hwnd, |w| w.edit) {
                if !edit.is_null() {
                    SetWindowPos(edit, null_mut(), ex, ey, ew, eh, SWP_NOZORDER | SWP_NOACTIVATE);
                }
            }
        }
    }
    apply_theme(kind, dark);
    Ok(())
}

fn create_edit(parent: HWND, dpi: u32, dark: bool) {
    unsafe {
        let inst = GetModuleHandleW(null());
        let style = WS_CHILD | WS_VISIBLE | WS_VSCROLL | WS_TABSTOP | ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN | ES_NOHIDESEL;
        let edit = CreateWindowExW(0, wide("EDIT").as_ptr(), wide("").as_ptr(), style, 0, 0, 100, 100, parent, EDIT_ID as HMENU, inst, null());
        if edit.is_null() {
            return;
        }
        SendMessageW(edit, EM_SETLIMITTEXT, 65_536, 0);
        SendMessageW(edit, EM_SETMARGINS, EC_LEFTMARGIN_RIGHTMARGIN, (8 | (8 << 16)) as isize);
        let font = make_edit_font(dpi);
        SendMessageW(edit, WM_SETFONT_MSG, font as usize, 1);
        let prev = SetWindowLongPtrW(edit, GWLP_WNDPROC, edit_proc as *const () as usize as isize);
        if prev != 0 {
            EDIT_PREV.store(prev, Ordering::Release);
        }
        let brush = CreateSolidBrush(colorref(theme::palette(dark).card));
        with_win(parent, |w| {
            w.edit = edit;
            w.edit_font = font;
            w.edit_brush = brush;
        });
    }
}

// ───────────────────────── public API ─────────────────────────

pub fn exists(kind: Kind) -> bool {
    hwnd_of(kind).is_some()
}

pub fn is_visible(kind: Kind) -> bool {
    hwnd_of(kind).is_some_and(|h| unsafe { IsWindowVisible(h) } != 0)
}

pub fn show(kind: Kind) {
    let Some(hwnd) = hwnd_of(kind) else { return };
    let _scope = DpiScope::enter();
    // Render before the window appears so the first thing the user sees is the finished page.
    with_win(hwnd, |w| w.ensure_frame());
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        } else {
            ShowWindow(hwnd, SW_SHOW);
        }
        SetForegroundWindow(hwnd);
    }
}

pub fn hide(kind: Kind) {
    if let Some(hwnd) = hwnd_of(kind) {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
    }
}

pub fn set_title(kind: Kind, title: &str) {
    if let Some(hwnd) = hwnd_of(kind) {
        unsafe { SetWindowTextW(hwnd, wide(title).as_ptr()) };
    }
}

/// Disable / enable input to the window (used while a modal file dialog is open).
pub fn set_enabled(kind: Kind, enabled: bool) {
    if let Some(hwnd) = hwnd_of(kind) {
        unsafe { EnableWindow(hwnd, enabled as i32) };
    }
}

/// Run `f` on the window's UI, then schedule a repaint if it changed anything.
pub fn with_ui<R>(kind: Kind, f: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    let hwnd = hwnd_of(kind)?;
    let (r, dirty) = with_win(hwnd, |w| {
        let r = f(&mut w.ui);
        (r, w.ui.needs_render())
    })?;
    if dirty && is_visible(kind) {
        invalidate(hwnd);
    }
    Some(r)
}

/// The OLED mirror in `ui.model.oled` changed: repaint just that rectangle.
pub fn present_oled(kind: Kind) {
    let Some(hwnd) = hwnd_of(kind) else { return };
    if !is_visible(kind) {
        return;
    }
    if let Some(Some(rect)) = with_win(hwnd, |w| w.refresh_oled_region()) {
        unsafe { InvalidateRect(hwnd, &rect, 0) };
    }
}

/// DWM caption colours (Windows 11), dark/light frame, and the editor's native control theme.
pub fn apply_theme(kind: Kind, dark: bool) {
    let Some(hwnd) = hwnd_of(kind) else { return };
    let p = theme::palette(dark);
    unsafe {
        let flag: i32 = dark as i32;
        DwmSetWindowAttribute(hwnd, 20, &flag as *const i32 as *const c_void, 4); // DWMWA_USE_IMMERSIVE_DARK_MODE
        let caption = colorref(p.shell);
        let text = colorref(p.text);
        let border = colorref(p.card_border);
        DwmSetWindowAttribute(hwnd, 35, &caption as *const u32 as *const c_void, 4); // DWMWA_CAPTION_COLOR
        DwmSetWindowAttribute(hwnd, 36, &text as *const u32 as *const c_void, 4); // DWMWA_TEXT_COLOR
        DwmSetWindowAttribute(hwnd, 34, &border as *const u32 as *const c_void, 4); // DWMWA_BORDER_COLOR
    }
    if let Some((edit, old_brush)) = with_win(hwnd, |w| {
        w.dark = dark;
        let old = w.edit_brush;
        if !w.edit.is_null() {
            w.edit_brush = unsafe { CreateSolidBrush(colorref(p.card)) };
        }
        (w.edit, old)
    }) {
        if !edit.is_null() {
            unsafe {
                let class = if dark { wide("DarkMode_Explorer") } else { wide("Explorer") };
                SetWindowTheme(edit, class.as_ptr(), null());
                InvalidateRect(edit, null(), 1);
            }
            if !old_brush.is_null() {
                unsafe { DeleteObject(old_brush) };
            }
        }
    }
    invalidate(hwnd);
}

// ───────────────────────── editor text control ─────────────────────────

fn edit_handle() -> Option<HWND> {
    let hwnd = hwnd_of(Kind::Editor)?;
    let edit = with_win(hwnd, |w| w.edit)?;
    if edit.is_null() { None } else { Some(edit) }
}

/// Current editor text with `\n` line endings.
pub fn editor_text() -> Option<String> {
    let edit = edit_handle()?;
    unsafe {
        let len = GetWindowTextLengthW(edit);
        if len < 0 {
            return None;
        }
        let mut buf = vec![0u16; len as usize + 1];
        let read = GetWindowTextW(edit, buf.as_mut_ptr(), len + 1);
        Some(String::from_utf16_lossy(&buf[..read.max(0) as usize]).replace("\r\n", "\n"))
    }
}

/// Replace the editor text (any line-ending style is accepted).
pub fn editor_set_text(text: &str) {
    if let Some(edit) = edit_handle() {
        let crlf = text.replace("\r\n", "\n").replace('\n', "\r\n");
        unsafe { SetWindowTextW(edit, wide(&crlf).as_ptr()) };
    }
}

pub fn editor_focus() {
    if let Some(edit) = edit_handle() {
        unsafe { SetFocus(edit) };
    }
}
