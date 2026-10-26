//! Native Win32 settings window (no browser/server, no additional UI runtime).
//! SDL's existing tray continues to be owned by the main thread.
#![cfg(target_os = "windows")]

use std::{ffi::c_void, ptr, sync::{mpsc::Sender, Mutex, OnceLock, atomic::{AtomicBool, AtomicU8, AtomicIsize, Ordering}}};
use gxoled_draw::gif_studio::{GifEngine, FitMode, DitherMode, StudioSettings};
use super::{ConfigUiSkin, MenuEvent, ConfigClockStyle, ConfigUiMode, ConfigLanguage, ConfigTheme, ConfigTimeMode, ConfigShiftMode, ClockBurnInMode, StatusNotifyMode, ConfigQuitMode, Config, GoAnimation};

type Handle = *mut c_void;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_DRAWITEM: u32 = 0x002b;
const WM_GETMINMAXINFO: u32 = 0x0024;
const WM_GETFONT: u32 = 0x0031;
const BS_OWNERDRAW: u32 = 0x000b;
const ODS_SELECTED: u32 = 0x0001;
const DT_CENTER: u32 = 0x0001;
const DT_VCENTER: u32 = 0x0004;
const DT_SINGLELINE: u32 = 0x0020;
const DT_END_ELLIPSIS: u32 = 0x00008000;
const TRANSPARENT: i32 = 1;

const WM_CTLCOLORSTATIC: u32 = 0x0138;
const WM_CTLCOLORBTN: u32 = 0x0135;
const WM_CTLCOLOREDIT: u32 = 0x0133;
const WM_CTLCOLORLISTBOX: u32 = 0x0134;
const WM_SIZE: u32 = 0x0005;
const WM_VSCROLL:u32=0x0115;
const WM_HSCROLL:u32=0x0114;
const TBM_GETPOS:u32=0x0400;
const TBM_SETPOS:u32=0x0405;
const TBM_SETRANGE:u32=0x0406;
const ICC_BAR_CLASSES:u32=0x0004;
const ID_BRIGHT_SLIDER:u32=680;
const ID_BRIGHT_VALUE:u32=681;
const ID_ABOUT_CREDITS:u32=723;
const ID_ABOUT_COFFEE:u32=724;
const ID_QR_METHOD:u32=725;
const ID_QR_HINT:u32=726;
const SB_VERT:i32=1;
const SIF_RANGE:u32=0x0001;
const SIF_PAGE:u32=0x0002;
const SIF_POS:u32=0x0004;
const WM_MOUSEWHEEL:u32=0x020a;
const WM_KEYDOWN:u32=0x0100;
const WM_DPICHANGED:u32=0x02e0;
const LBS_NOTIFY:u32=0x0001;
const LB_ADDSTRING:u32=0x0180;
const LB_RESETCONTENT:u32=0x0184;
const LB_GETCURSEL:u32=0x0188;
const LB_SETCURSEL:u32=0x0186;
const SWP_NOZORDER:u32=0x0004;
const SWP_NOACTIVATE:u32=0x0010;

const WM_TRAYICON: u32 = 0x0401; // SDL 3.4 Windows tray callback: WM_USER + 1
const WM_LBUTTONUP: isize = 0x0202;
const WM_LBUTTONDBLCLK: isize = 0x0203;
const WM_CONTEXTMENU: isize = 0x007b;
const GWLP_WNDPROC: i32 = -4;
const ID_EDITOR_SAVE:u32=401;
const ID_EDITOR_RESET:u32=402;
const ID_EDITOR_IMPORT:u32=403;
const ID_EDITOR_EXPORT:u32=404;
const ID_EDITOR_FORMAT:u32=405;
const ID_PROFILE_LIST:u32=600;
const ID_PROFILE_SELECT:u32=601;
const ID_PROFILE_NEW:u32=602;
const ID_PROFILE_IMPORT:u32=603;
const ID_PROFILE_DELETE:u32=604;
const ID_PROFILE_EXPORT:u32=605;
const ID_GIF_ADD:u32=6060;
const ID_GIF_TOGGLE:u32=6061;
const ID_ENGINE_CLASSIC:u32=6070;
const ID_ENGINE_STUDIO:u32=6071;
const ID_STUDIO_PRESET_BASE:u32=6080;
const ID_STYLE_BASE:u32=6090;
const ID_STUDIO_FIT_BASE:u32=6100;
const ID_STUDIO_DITHER_BASE:u32=6110;
const ID_STUDIO_INVERT:u32=6120;
const ID_STUDIO_THRESHOLD:u32=6121;
const ID_STUDIO_BRIGHTNESS:u32=6122;
const ID_STUDIO_IMPORT:u32=6123;
const ID_STUDIO_EXPORT:u32=6124;

const ID_KOFI:u32=370;
const ID_ANIM_STOP:u32=6062;
const ID_STARTUP:u32=606;
const ID_SHIFT_BASE:u32=620;
const ID_BURN_BASE:u32=625;
const ID_NOTIFY_BASE:u32=651;
const ID_QUIT_BASE:u32=660;

const ID_EDIT_PROFILE: u32 = 372;
// Main settings is intentionally fixed-size. The earlier proportional layout
// damaged STATIC labels when the parent was resized, so no thick frame or
// maximize box is offered here. The separate JSON editor stays resizable.
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const WS_CAPTION: u32 = 0x00C0_0000;
const WS_SYSMENU: u32 = 0x0008_0000;
const WS_MINIMIZEBOX: u32 = 0x0002_0000;
const SETTINGS_WIDTH: i32 = 950;
const SETTINGS_HEIGHT: i32 = 670;
const SETTINGS_LEFT: i32 = 186;
const SETTINGS_FORM: i32 = 340;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_CHILD: u32 = 0x4000_0000;
const WS_CLIPSIBLINGS:u32=0x0400_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WM_CLOSE: u32 = 0x0010;
const WM_COMMAND: u32 = 0x0111;
const WM_PAINT: u32 = 0x000f;
const WM_SETFONT: u32 = 0x0030;
const WM_CHAR:u32=0x0102;
const EM_SETSEL:u32=0x00B1;
const VK_CONTROL:i32=0x11;
const RDW_INVALIDATE:u32=0x0001;
const RDW_ERASE:u32=0x0004;
const RDW_ALLCHILDREN:u32=0x0080;
const RDW_UPDATENOW:u32=0x0100;
const SWP_NOREDRAW:u32=0x0008;
const WS_VSCROLL:u32=0x00200000;
const WS_CLIPCHILDREN:u32=0x02000000;
const ES_MULTILINE:u32=0x0004;
const ES_AUTOVSCROLL:u32=0x0040;
const ES_WANTRETURN:u32=0x1000;
const EM_SETLIMITTEXT:u32=0x00c5;
const BM_SETCHECK: u32 = 0x00f1;
const BST_CHECKED: usize = 1;
const SW_HIDE: i32 = 0;
// Win32 COLORREF is stored as BGR, not RGB.
const fn rgb(r:u8,g:u8,b:u8)->u32 {(b as u32)<<16 | (g as u32)<<8 | (r as u32)}
const COLOR_DARK_BG: u32 = rgb(14,14,15);
const COLOR_CARD_BG: u32 = rgb(27,27,29);
const COLOR_SIDEBAR: u32 = rgb(19,19,20);
const COLOR_EDITOR_BG:u32 = rgb(18,18,20);
const COLOR_DARK_TEXT: u32 = rgb(247,247,248);
const COLOR_MUTED: u32 = rgb(154,154,166);
const COLOR_CYAN: u32 = rgb(255,106,0);
const COLOR_PURPLE: u32 = rgb(255,136,46);
static SKIN: AtomicU8 = AtomicU8::new(0);
fn accent()->u32 {match SKIN.load(Ordering::Relaxed) {1=>rgb(37,217,207),2=>rgb(191,91,246),_=>COLOR_CYAN}}
fn accent_secondary()->u32 {match SKIN.load(Ordering::Relaxed) {1=>rgb(154,245,228),2=>rgb(230,162,255),_=>COLOR_PURPLE}}
fn accent_surface()->u32 {match SKIN.load(Ordering::Relaxed) {1=>rgb(19,56,58),2=>rgb(47,31,60),_=>COLOR_CYAN_SURFACE}}
pub fn set_skin(skin:ConfigUiSkin) {
    SKIN.store(match skin {ConfigUiSkin::StudioOrange=>0,ConfigUiSkin::StudioCyan=>1,ConfigUiSkin::StudioViolet=>2},Ordering::Relaxed);
    if let Some(hwnd)=STATE.get().and_then(|state|state.lock().ok().map(|s|s.hwnd)) {
        unsafe{InvalidateRect(hwnd as Handle,ptr::null(),1);}
    }
}

const COLOR_BORDER: u32 = rgb(58,58,62);
const COLOR_CYAN_SURFACE: u32 = rgb(61,39,24);
const SW_SHOW: i32 = 5;
const PM_REMOVE: u32 = 1;
const COLOR_WINDOW: usize = 5;
const SRCCOPY: u32 = 0x00cc0020;
const ID_IDLE: u32 = 354;
const ID_MEDIA: u32 = 355;

#[repr(C)]
struct WndClass {
    style: u32,
    proc: Option<unsafe extern "system" fn(Handle,u32,usize,isize)->isize>,
    cls_extra:i32, wnd_extra:i32, instance:Handle, icon:Handle, cursor:Handle,
    background:Handle, menu_name:*const u16, class_name:*const u16,
}
#[repr(C)] #[derive(Clone, Copy, Default)]
struct Rect {left:i32, top:i32, right:i32, bottom:i32}
#[repr(C)] struct Point {x:i32,y:i32}
#[repr(C)] struct MinMaxInfo {reserved:Point,max_size:Point,max_position:Point,min_track_size:Point,max_track_size:Point}
#[repr(C)] struct DrawItem {
    ctl_type:u32,ctl_id:u32,item_id:u32,item_action:u32,item_state:u32,
    hwnd_item:Handle,dc:Handle,rect:Rect,item_data:usize,
}
#[repr(C)]
struct Paint { dc: Handle, erase:i32, rect:Rect, restore:i32, inc_update:i32, reserved:[u8;32] }
#[repr(C)]
struct BitmapInfoHeader {
    size:u32, width:i32, height:i32, planes:u16, bit_count:u16, compression:u32,
    image_size:u32, x_pixels_per_meter:i32, y_pixels_per_meter:i32, used:u32, important:u32,
}
#[repr(C)]
struct BitmapInfo { header: BitmapInfoHeader, colors:[u32;1] }
#[repr(C)]
struct ScrollInfo { size:u32, mask:u32, min:i32, max:i32, page:u32, pos:i32, track_pos:i32 }
#[repr(C)]
struct InitCommonControls { size:u32, classes:u32 }
#[link(name="comctl32")]
unsafe extern "system" { fn InitCommonControlsEx(init:*const InitCommonControls)->i32; }

#[link(name="user32")]
unsafe extern "system" {
    fn RegisterClassW(cls: *const WndClass) -> u16;
    fn CreateWindowExW(ex:u32, cls:*const u16, title:*const u16, style:u32,
        x:i32,y:i32,w:i32,h:i32,parent:Handle,menu:Handle,instance:Handle,param:*const c_void)->Handle;
    fn DefWindowProcW(hwnd:Handle, message:u32, w:usize, l:isize)->isize;
    fn ShowWindow(hwnd:Handle, cmd:i32)->i32;
    fn IsWindowVisible(hwnd:Handle)->i32;
    fn SetForegroundWindow(hwnd:Handle)->i32;
    fn PeekMessageW(msg:*mut Message, hwnd:Handle, min:u32,max:u32,remove:u32)->i32;
    fn TranslateMessage(msg:*const Message)->i32;
    fn DispatchMessageW(msg:*const Message)->isize;
    fn SendMessageW(hwnd:Handle,msg:u32,w:usize,l:isize)->isize;
    fn GetSysColorBrush(n:i32)->Handle;
    fn FindWindowExW(parent:Handle,after:Handle,cls:*const u16,name:*const u16)->Handle;
    fn GetWindowThreadProcessId(hwnd:Handle,pid:*mut u32)->u32;
    fn SetWindowLongPtrW(hwnd:Handle,index:i32,value:isize)->isize;
    fn CallWindowProcW(proc:isize,hwnd:Handle,message:u32,w:usize,l:isize)->isize;
    fn SetWindowTextW(hwnd:Handle,text:*const u16)->i32;
    fn GetWindowTextLengthW(hwnd:Handle)->i32;
    fn GetDlgCtrlID(hwnd:Handle)->i32;
    fn GetWindowTextW(hwnd:Handle,text:*mut u16,max:i32)->i32;
    fn MessageBoxW(hwnd:Handle,text:*const u16,caption:*const u16,kind:u32)->i32;
    fn GetClientRect(hwnd:Handle,rect:*mut Rect)->i32;
    fn GetSystemMetrics(index:i32)->i32;
    fn SetWindowPos(hwnd:Handle, insert:Handle,x:i32,y:i32,w:i32,h:i32,flags:u32)->i32;
    fn SetFocus(hwnd:Handle)->Handle;
    fn SetScrollInfo(hwnd:Handle,bar:i32,info:*const ScrollInfo,redraw:i32)->i32;
    fn SetBkColor(dc:Handle,color:u32)->u32;
    fn LoadCursorW(instance:Handle, id:*const u16)->Handle;
    fn BeginPaint(hwnd:Handle,paint:*mut Paint)->Handle;
    fn EndPaint(hwnd:Handle,paint:*const Paint)->i32;
    fn InvalidateRect(hwnd:Handle,rect:*const Rect,erase:i32)->i32;
    fn RedrawWindow(hwnd:Handle,rect:*const Rect,region:Handle,flags:u32)->i32;
    fn GetKeyState(key:i32)->i16;
    fn FillRect(dc:Handle,rect:*const Rect,brush:Handle)->i32;
    fn SetBkMode(dc:Handle,mode:i32)->i32;
    fn SetTextColor(dc:Handle,color:u32)->u32;
    fn TextOutW(dc:Handle,x:i32,y:i32,text:*const u16,count:i32)->i32;
    fn DrawTextW(dc:Handle,text:*const u16,count:i32,rect:*mut Rect,format:u32)->i32;
}
#[link(name="kernel32")]
unsafe extern "system" { fn GetModuleHandleW(name:*const u16)->Handle; fn GetCurrentProcessId()->u32; }
#[link(name="shell32")]
unsafe extern "system" {
    fn ShellExecuteW(hwnd:Handle, operation:*const u16,file:*const u16,parameters:*const u16,dir:*const u16,show:i32)->Handle;
}
#[link(name="dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(hwnd:Handle,attribute:u32,value:*const u32,size:u32)->i32;
}
#[link(name="uxtheme")]
unsafe extern "system" {
    fn SetWindowTheme(hwnd:Handle,app:*const u16,id:*const u16)->i32;
}
#[link(name="gdi32")]
unsafe extern "system" {
    fn CreateSolidBrush(color:u32)->Handle;
    fn CreatePen(style:i32,width:i32,color:u32)->Handle;
    fn SelectObject(dc:Handle,object:Handle)->Handle;
    fn RoundRect(dc:Handle,left:i32,top:i32,right:i32,bottom:i32,width:i32,height:i32)->i32;
    fn Ellipse(dc:Handle,left:i32,top:i32,right:i32,bottom:i32)->i32;
    fn MoveToEx(dc:Handle,x:i32,y:i32,prev:*mut Point)->i32;
    fn LineTo(dc:Handle,x:i32,y:i32)->i32;
    fn SetStretchBltMode(dc:Handle,mode:i32)->i32;
    fn DeleteObject(handle:Handle)->i32;
    fn StretchDIBits(dc:Handle,dx:i32,dy:i32,dw:i32,dh:i32,sx:i32,sy:i32,sw:i32,sh:i32,
        bytes:*const c_void,info:*const BitmapInfo,color_mode:u32,raster_op:u32)->i32;
    fn CreateFontW(height:i32,width:i32,escapement:i32,orientation:i32,weight:i32,italic:u32,
        underline:u32,strike:u32,charset:u32,out_prec:u32,clip:u32,quality:u32,pitch:u32,name:*const u16)->Handle;
}
#[repr(C)]
struct Message { hwnd:Handle, message:u32, wparam:usize, lparam:isize, time:u32, point_x:i32, point_y:i32, private:u32 }

#[derive(Clone)]
struct ControlPlacement { hwnd:usize, id:u32, page:usize, x:i32,y:i32,w:i32,h:i32, class:String }
#[derive(Clone, PartialEq)]
struct UiSnapshot {
    mode:ConfigUiMode,style:ConfigClockStyle,time:ConfigTimeMode,
    animation:Option<GoAnimation>,profile:Option<String>,
    language:ConfigLanguage,theme:ConfigTheme,skin:ConfigUiSkin,
    gif_engine:GifEngine,gif_studio:StudioSettings,shift:ConfigShiftMode,
    brightness:Option<u8>,burn_wide:bool,burn_no_overlap:bool,
    media:bool,idle:bool,startup:bool,
    status:StatusNotifyMode,quit:ConfigQuitMode,
}
impl UiSnapshot {
    fn capture(c:&Config)->Self {Self{
        mode:c.ui_mode,style:c.clock_style,time:c.time_mode,
        animation:c.go_animation,profile:c.go_profile_filename.clone(),
        language:c.language,theme:c.theme,skin:c.ui_skin,
        gif_engine:c.gif_engine,gif_studio:c.gif_studio,shift:c.oled_shift,
        brightness:c.screen_brightness,burn_wide:c.clock_shift,
        burn_no_overlap:c.clock_shift_no_overlap,
        media:c.show_media,idle:c.idle_timeout,
        startup:c.startup,status:c.status_notify,quit:c.quit_mode,
    }}
}
#[derive(Clone)]
struct ProfileEntry { label:String, animation:GoAnimation, custom:Option<String> }
struct State {
    hwnd: usize,
    tabs: Vec<Vec<usize>>,
    buttons: Vec<(u32, usize)>,
    placements:Vec<ControlPlacement>,
    profile_entries:Vec<ProfileEntry>,
    profile_list:usize,
    scroll:[i32;7],
    running_profile_key: Option<(GoAnimation,Option<String>)>,
    last_sync:Option<UiSnapshot>,
    animation: Option<super::GoAnimation>,
    page: usize,
    framebuffer: Vec<u32>,
    language: ConfigLanguage,
    theme: ConfigTheme,
    qr: Option<(u32,u32,Vec<u32>)>,
}
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
// HWND operations can synchronously dispatch WM_SIZE / WM_PAINT back to wnd_proc.
// Prevent recursive layout and NEVER hold STATE while calling ShowWindow/SetWindowPos.
static LAYOUT_ACTIVE: AtomicBool = AtomicBool::new(false);
struct LayoutGuard;
impl Drop for LayoutGuard {
    fn drop(&mut self) { LAYOUT_ACTIVE.store(false, Ordering::Release); }
}
static TX: OnceLock<Sender<MenuEvent>> = OnceLock::new();
static TRAY_MODE: AtomicU8 = AtomicU8::new(0);
static PREV_TRAY_PROC: AtomicIsize = AtomicIsize::new(0);
static PREV_BRIGHTNESS_PROC: AtomicIsize = AtomicIsize::new(0);
static PREV_EDITOR_PROC: AtomicIsize = AtomicIsize::new(0);
static THEME: AtomicU8 = AtomicU8::new(1);
static LANG: AtomicU8 = AtomicU8::new(0);
static DARK_BRUSH: OnceLock<usize> = OnceLock::new();
static CARD_BRUSH: OnceLock<usize> = OnceLock::new();
static ACTIVE_PAGE: AtomicU8 = AtomicU8::new(0);
static EDIT_BRUSH: OnceLock<usize> = OnceLock::new();
struct Editor { hwnd:usize, edit:usize, animation:super::GoAnimation,
    target:std::path::PathBuf, title:String, controls:Vec<(u32,usize)> }
static EDITOR: OnceLock<Mutex<Editor>> = OnceLock::new();
const PROFILE_MATRIX: &str=include_str!("../../go_animations/cmd/renderer/profiles/matrix.json");
const PROFILE_HYPERSPACE: &str=include_str!("../../go_animations/cmd/renderer/profiles/hyperspace.json");
const PROFILE_STARWARS: &str=include_str!("../../go_animations/cmd/renderer/profiles/starwars_intro.json");
const PROFILE_LIFE: &str=include_str!("../../go_animations/cmd/renderer/profiles/game_of_life.json");
fn profile_name(a:super::GoAnimation)->&'static str {match a {
    super::GoAnimation::Matrix=>"matrix", super::GoAnimation::Hyperspace=>"hyperspace",
    super::GoAnimation::StarWars=>"starwars",super::GoAnimation::GameOfLife=>"life"
}}
fn profile_default(a:super::GoAnimation)->&'static str {match a {
    super::GoAnimation::Matrix=>PROFILE_MATRIX, super::GoAnimation::Hyperspace=>PROFILE_HYPERSPACE,
    super::GoAnimation::StarWars=>PROFILE_STARWARS, super::GoAnimation::GameOfLife=>PROFILE_LIFE
}}
fn profile_path(a:super::GoAnimation)->Option<std::path::PathBuf> {
    directories::BaseDirs::new().map(|base|base.config_dir().join("GxOled").join("profiles").join(format!("{}.json",profile_name(a))))
}



fn custom_directory()->Option<std::path::PathBuf> {
    directories::BaseDirs::new().map(|base|base.config_dir().join("GxOled").join("profiles").join("custom"))
}
fn profile_kind_for_json(value:&serde_json::Value)->Option<GoAnimation> {
    let ty=value.get("widgets")?.as_array()?.first()?.get("type")?.as_str()?;
    match ty {"matrix"=>Some(GoAnimation::Matrix),"hyperspace"=>Some(GoAnimation::Hyperspace),
        "starwars_intro"=>Some(GoAnimation::StarWars),"game_of_life"=>Some(GoAnimation::GameOfLife),_=>None}
}
fn valid_profile_text(text:&str)->Result<GoAnimation,String> {
    if text.len()>65_536 {return Err("Profile exceeds 64 KiB".into());}
    let value:serde_json::Value=serde_json::from_str(text).map_err(|e|format!("Invalid JSON: {e}"))?;
    let kind=profile_kind_for_json(&value).ok_or("Unsupported animation widget")?;
    if value["display"]["width"]!=128 || value["display"]["height"]!=64 ||
        value["widgets"].as_array().is_none_or(|v|v.len()!=1) ||
        !(10..=1000).contains(&value["refresh_rate_ms"].as_u64().unwrap_or(0)) {
        return Err("Only one 128x64 widget and refresh_rate_ms 10–1000 are supported".into());
    }
    Ok(kind)
}
fn profile_entries()->Vec<ProfileEntry>{
    let mut entries=vec![
        ProfileEntry{label:"Matrix Rain — Default".into(),animation:GoAnimation::Matrix,custom:None},
        ProfileEntry{label:"Hyperspace — Default".into(),animation:GoAnimation::Hyperspace,custom:None},
        ProfileEntry{label:"Star Wars Intro — Default".into(),animation:GoAnimation::StarWars,custom:None},
        ProfileEntry{label:"Game of Life — Default".into(),animation:GoAnimation::GameOfLife,custom:None},
    ];
    if let Some(dir)=custom_directory(){if let Ok(paths)=std::fs::read_dir(dir){
        for item in paths.flatten() {
            let path=item.path();
            if path.extension().and_then(|ext|ext.to_str())!=Some("json") {continue;}
            let Some(name)=path.file_name().and_then(|v|v.to_str()).map(str::to_owned) else {continue};
            if name.len()>120 || !name.bytes().all(|c|c.is_ascii_alphanumeric() || b"._-".contains(&c)) || name.contains("..") {continue;}
            if let Ok(text)=std::fs::read_to_string(&path) {
                if let Ok(kind)=valid_profile_text(&text) {
                    entries.push(ProfileEntry{label:format!("{} — {}",name,profile_name(kind)), animation:kind, custom:Some(name)});
                }
            }
        }
    }}
    entries
}
fn chosen_profile()->Option<ProfileEntry> {
    STATE.get().and_then(|state|state.lock().ok().and_then(|s|{
        let idx=unsafe{SendMessageW(s.profile_list as Handle,LB_GETCURSEL,0,0)};
        if idx<0 {None} else {s.profile_entries.get(idx as usize).cloned()}
    }))
}
fn selected_profile_path(entry:&ProfileEntry)->Option<std::path::PathBuf>{
    if let Some(name)=&entry.custom {custom_directory().map(|p|p.join(name))}
    else {profile_path(entry.animation)}
}
fn unique_profile_name(kind:GoAnimation)->Option<(String,std::path::PathBuf)> {
    let dir=custom_directory()?;
    std::fs::create_dir_all(&dir).ok()?;
    for n in 1..1_000_000u32 {
        let name=format!("{}-{:03}.json",profile_name(kind),n);
        let path=dir.join(&name);
        if !path.exists(){return Some((name,path));}
    }
    None
}
fn refresh_profile_list(selected:Option<String>){
    let Some(state)=STATE.get() else{return};
    let Ok(mut s)=state.lock() else{return};
    s.profile_entries=profile_entries();
    let list=s.profile_list as Handle;
    unsafe {SendMessageW(list,LB_RESETCONTENT,0,0);}
    let mut idx=0;
    for (i,p) in s.profile_entries.iter().enumerate(){
        let name=wide(&p.label);
        unsafe {SendMessageW(list,LB_ADDSTRING,0,name.as_ptr() as isize);}
        if selected.is_some() && p.custom==selected {idx=i;}
    }
    unsafe {SendMessageW(list,LB_SETCURSEL,idx,0);}
}
fn create_profile_from_text(text:&str)->Result<(GoAnimation,String),String>{
    let kind=valid_profile_text(text)?;
    let (name,path)=unique_profile_name(kind).ok_or("Unable to allocate profile filename")?;
    std::fs::write(&path,pretty_json(text)).map_err(|e|e.to_string())?;
    Ok((kind,name))
}
fn profile_import(){
    let Some(path)=rfd::FileDialog::new().add_filter("JSON profile", &["json"]).pick_file() else{return};
    match std::fs::read_to_string(&path).map_err(|e|e.to_string()).and_then(|t|create_profile_from_text(&t)) {
        Ok((_,name))=>refresh_profile_list(Some(name)),
        Err(err)=>profile_error("Import failed",&err),
    }
}
fn profile_new_blank(){
    // Allocate a proposed filename only: do not create an invalid JSON file
    // in the profiles directory until the user saves a valid 128×64 profile.
    let Some(entry)=chosen_profile() else {return;};
    let Some((name,target))=unique_profile_name(entry.animation) else {
        profile_error("New Profile","Unable to allocate a custom profile filename");
        return;
    };
    let fresh=ProfileEntry{
        label:format!("{} — New Profile",name),
        animation:entry.animation,
        custom:Some(name),
    };
    open_profile_editor_with(fresh,target,"{\n\n}".to_owned());
}
fn profile_delete(){
    let Some(entry)=chosen_profile() else{return};
    if entry.custom.is_none(){profile_error("GxOled","Built-in profiles cannot be deleted");return;}
    if let Some(path)=selected_profile_path(&entry){
        if let Err(err)=std::fs::remove_file(path){profile_error("GxOled",&err.to_string());return;}
    }
    let was_running=STATE.get().and_then(|v|v.lock().ok().map(|s|
        s.running_profile_key.as_ref().is_some_and(|key|key.0==entry.animation && key.1==entry.custom))).unwrap_or(false);
    if was_running {if let Some(tx)=TX.get(){let _=tx.send(MenuEvent::SetGoAnimation(None));}}
    refresh_profile_list(None);
}
fn profile_export(){
    let Some(entry)=chosen_profile() else{return};
    let text=selected_profile_path(&entry).and_then(|p|std::fs::read_to_string(p).ok())
        .unwrap_or_else(||profile_default(entry.animation).to_owned());
    if let Some(path)=rfd::FileDialog::new().add_filter("JSON profile", &["json"]).set_file_name("gxoled-profile.json").save_file(){
        if let Err(err)=std::fs::write(path,pretty_json(&text)){profile_error("Export failed",&err.to_string());}
    }
}
fn select_profile(){
    if let Some(entry)=chosen_profile(){
        if let Some(tx)=TX.get(){let _=tx.send(MenuEvent::SetGoProfile(entry.animation,entry.custom));}
    }
}
/// SDL 3.4 has no left-click hook. On Windows its tray is an `SDL_TRAY`
/// hidden HWND and WM_USER+1 is its shell notification. Subclass *only* this
/// HWND in the current process; always forward commands and shell lifecycle
/// messages to SDL. This is intentionally version-pinned to SDL 3.4.
unsafe extern "system" fn tray_proc(hwnd:Handle, msg:u32,w:usize,l:isize)->isize {
    if msg == WM_TRAYICON {
        let click = l & 0xffff;
        let mode = TRAY_MODE.load(Ordering::Relaxed);
        let window_click = (click == WM_LBUTTONUP && mode != 0)
            || (click == WM_CONTEXTMENU && mode == 1)
            || (click == WM_LBUTTONDBLCLK && mode == 2);
        if window_click {
            if let Some(tx) = TX.get() { let _ = tx.send(MenuEvent::OpenSettings); }
            return 0;
        }
    }
    let old = PREV_TRAY_PROC.load(Ordering::Acquire);
    if old != 0 { unsafe { CallWindowProcW(old,hwnd,msg,w,l) } }
    else { unsafe { DefWindowProcW(hwnd,msg,w,l) } }
}

pub fn set_tray_mode(mode: ConfigUiMode) {
    TRAY_MODE.store(match mode {ConfigUiMode::Tray=>0,ConfigUiMode::Window=>1,ConfigUiMode::Hybrid=>2},Ordering::Relaxed);
}

pub fn install_tray_hook(mode: ConfigUiMode) {
    if STATE.get().is_none() {return;}
    set_tray_mode(mode);
    if PREV_TRAY_PROC.load(Ordering::Acquire) != 0 { return; }
    let cls=wide("SDL_TRAY");
    let mut after=ptr::null_mut();
    loop {
        let hwnd=unsafe {FindWindowExW(ptr::null_mut(),after,cls.as_ptr(),ptr::null())};
        if hwnd.is_null(){break;}
        after=hwnd;
        let mut pid=0u32;
        unsafe {GetWindowThreadProcessId(hwnd,&mut pid);}
        if pid==unsafe{GetCurrentProcessId()} {
            let old=unsafe {SetWindowLongPtrW(hwnd,GWLP_WNDPROC,tray_proc as *const () as usize as isize)};
            if old!=0 { PREV_TRAY_PROC.store(old,Ordering::Release); }
            break;
        }
    }
}

pub fn localize(language: ConfigLanguage) {
    LANG.store(if language == ConfigLanguage::Turkish {1} else {0}, Ordering::Relaxed);
    let handles = STATE.get().and_then(|state| state.lock().ok().map(|mut s| {
        s.language = language;
        (s.hwnd, s.buttons.clone())
    }));
    if let Some((hwnd, buttons)) = handles {
        let title = wide(if language == ConfigLanguage::Turkish {"GxOled — Ayarlar"} else {"GxOled — Settings"});
        unsafe { SetWindowTextW(hwnd as Handle, title.as_ptr()); }
        update_labels(&buttons, language);
    }
}
pub fn set_theme(theme: ConfigTheme) {
    THEME.store(if theme == ConfigTheme::Dark {1} else {0}, Ordering::Relaxed);
    let handles = STATE.get().and_then(|state| state.lock().ok().map(|mut s| {
        s.theme = theme;
        (s.hwnd, s.buttons.clone())
    }));
    if let Some((hwnd, buttons)) = handles {
        let dark = if theme == ConfigTheme::Dark {1u32} else {0u32};
        let style = if dark == 1 {wide("DarkMode_Explorer")} else {wide("Explorer")};
        unsafe {
            DwmSetWindowAttribute(hwnd as Handle, 20, &dark, 4);
            for (_, handle) in buttons {
                SetWindowTheme(handle as Handle, style.as_ptr(), ptr::null());
                InvalidateRect(handle as Handle, ptr::null(), 1);
            }
            InvalidateRect(hwnd as Handle, ptr::null(), 1);
        }
        let editor_handles = EDITOR.get().and_then(|editor| editor.lock().ok()
            .map(|ed| (ed.hwnd, ed.edit)));
        if let Some((editor_hwnd, edit)) = editor_handles {
            unsafe {
                DwmSetWindowAttribute(editor_hwnd as Handle, 20, &dark, 4);
                SetWindowTheme(edit as Handle, style.as_ptr(), ptr::null());
                InvalidateRect(edit as Handle, ptr::null(), 1);
                InvalidateRect(editor_hwnd as Handle, ptr::null(), 1);
            }
        }
    }
}

fn update_labels(buttons:&[(u32,usize)], language:ConfigLanguage) {
    let tr=language==ConfigLanguage::Turkish;
    for (id,h) in buttons {
        let caption = match *id {
            200 => if tr {"Genel"}else{"General"},
            201 => if tr {"Saat"}else{"Clock"},
            202 => if tr {"Animasyonlar"}else{"Animations"},
            203 => if tr {"Ekran"}else{"Display"},
            204 => if tr {"Medya"}else{"Media"},
            205 => if tr {"Hakkında"}else{"About"},
            206 => "GIF Studio",
            570 => if tr {"Tasarım stili"} else {"Design theme"},
            ID_ENGINE_CLASSIC => "Classic GIF",
            ID_ENGINE_STUDIO => "Studio GIF",
            750 => if tr {"GIF işleme motoru"}else{"GIF processing engine"},
            751 => if tr {"Stüdyo ayarları"}else{"Studio options"},
            752 => if tr {"Kaynak yerleşimi"}else{"Source fit"},
            753 => if tr {"Dithering"}else{"Dithering"},
            754 => if tr {"Eşik değeri"}else{"Threshold"},
            755 => if tr {"Parlaklık düzeltmesi"}else{"Brightness adjustment"},
            756 => if tr {"Değişiklikler GIF tekrar başlatıldığında uygulanır."}else{"Changes take effect when GIF is restarted."},
            ID_STUDIO_INVERT => if tr {"Çıkış renklerini tersine çevir"}else{"Invert output"},
            ID_STUDIO_IMPORT => if tr {"Stüdyo ayarlarını içe aktar"}else{"Import Studio settings"},
            ID_STUDIO_EXPORT => if tr {"Stüdyo ayarlarını dışa aktar"}else{"Export Studio settings"},

            300 => if tr {"Tepsi menüsü"}else{"Tray menu"},
            301 => if tr {"Pencere"}else{"Window"},
            302 => if tr {"Karma"}else{"Hybrid"},
            310 => "Digital Standard",
            311 => "Digital Big",
            314 => "Casio Digital Dial",
            500 => if tr {"Uygulama açılış biçimi"}else{"App opening mode"},
            501 => if tr {"Pencere: simge ayarları açar.\r\nKarma: sol = pencere, sağ = menü."}else{"Window: tray icon opens settings.\r\nHybrid: left = window, right = menu."},
            503 => if tr {"Görünüm"}else{"Appearance"},
            510 => if tr {"SAAT TARZI"}else{"CLOCK STYLE"},
            511 => if tr {"Saat gösterimi"}else{"Time display"},
            520 => if tr {"128×64 animasyon profilleri"}else{"128×64 animation profiles"},
            521 => if tr {"Sınırsız özel profil oluşturun veya JSON içe/dışa aktarın."}else{"Create any number of presets or import/export JSON."},
            522 => if tr {"Özel profil klasörü:\n%APPDATA%\\GxOled\\\nprofiles\\custom"}else{"Custom profiles folder:\n%APPDATA%\\GxOled\\\nprofiles\\custom"},
            530 => if tr {"Ekran kontrolleri"}else{"Display controls"},
            531 => if tr {"Ekran parlaklığı"}else{"Screen brightness"},
            540 => if tr {"Medya kontrolleri"}else{"Media controls"},
            716 => if tr {"Müzik oynarken medya ekranı saatin yerine geçer. Oynatma bittiğinde saat geri döner. Animasyonlar ayrı çalışır."}else{"Media replaces the clock while music plays. The clock returns when playback stops. Animations remain exclusive."},
            ID_ABOUT_CREDITS => if tr {
                "GxOled, JerwuQu/ggoled projesini temel alır; animasyonlar pozitronik/steelclock-go kaynaklarından gelir. En büyük teşekkür her iki projenin geliştiricilerine aittir."
            } else {
                "GxOled builds on JerwuQu/ggoled; animations come from pozitronik/steelclock-go. Their original developers deserve the biggest thanks."
            },
            ID_ABOUT_COFFEE => if tr {
                "Yine de bana bir kahve ısmarlamak isterseniz hayır demem!"
            } else {
                "Still want to buy me a coffee? I wouldn't say no!"
            },
            ID_QR_METHOD => "USDT · TRON (TRC20)",
            ID_QR_HINT => if tr {"Bağış için QR kodunu tarayın."} else {"Scan the QR to buy me a coffee."},
            320 => if tr {"Kapalı"}else{"Off"},
            321 => if tr {"24 saat"}else{"24-hour"},
            322 => if tr {"12 saat"}else{"12-hour"},
            354 => if tr {"Boştayken ekran koruyucu"}else{"Screensaver when idle"},
            355 => if tr {"Medya bilgilerini göster"}else{"Show media metadata"},
            369 => "GitHub · MehmetNY/GxOled",
            ID_KOFI => "Ko-fi · Support GxOled",
            371 => if tr {"GxOled'den çık"}else{"Quit GxOled"},
            372 => if tr {"Profili düzenle..."}else{"Edit selected profile..."},
            ID_PROFILE_SELECT => if tr {"Profili uygula"}else{"Use selected"},
            ID_PROFILE_NEW => if tr {"Yeni profil"}else{"New Profile"},
            ID_PROFILE_IMPORT => if tr {"İçe aktar"}else{"Import"},
            ID_PROFILE_EXPORT => if tr {"Dışa aktar"}else{"Export"},
            ID_PROFILE_DELETE => if tr {"Sil"}else{"Delete"},
            ID_STARTUP => if tr {"Windows ile başlat"}else{"Start with Windows"},
            ID_GIF_ADD => if tr {"GIF ekle..."}else{"Add GIF..."},
            ID_GIF_TOGGLE => if tr {"GIF başlat / durdur"}else{"Start / Stop GIF"},
            ID_ANIM_STOP => if tr {"Animasyonu durdur / Saat"}else{"Stop animation / Clock"},
            380 => "English", 381 => "Türkçe", 390 => "Light", 391 => "Dark",
            710 => if tr {"GxOled kapanırken"}else{"When GxOled closes"},
            711 => if tr {"GIF oynatma"}else{"GIF playback"},
            718 => if tr {"GIF kitaplığı"}else{"GIF Library"},
            719 => if tr {"128×64 GIF oynatma kontrolleri. Gelecek düzenleme araçları burada yer alacak."}else{"128×64 GIF playback controls. Future editing tools will live here."},
            523 => if tr {"Yeni Profil boş bir JSON ile açılır. Geçerli bir şablon için Düzenleyicide Sıfırla kullanın."}else{"New Profile opens an empty JSON. Use Reset in the editor for a starter template."},
            712 => if tr {"OLED ekran kaydırma"}else{"OLED screen shift"},
            713 => if tr {"Saat yanık izi koruması"}else{"Clock burn-in protection"},
            714 => if tr {"Bağlantı durumu simgesi"}else{"Connection status icon"},
            715 => if tr {"Medya ekranı düzeni"}else{"Media screen layout"},
            717 => if tr {"GIF başlat / durdur"}else{"Start / Stop GIF"},
            _ => continue,
        };
        let wide=wide(caption);
        unsafe {SetWindowTextW(*h as Handle,wide.as_ptr());}
    }
}

fn wide(s:&str)->Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
// A classic Windows multi-line EDIT expects CRLF, whereas serde_json produces LF.
fn win_edit_text(s:&str)->Vec<u16> { wide(&s.replace("\r\n", "\n").replace('\n', "\r\n")) }
fn pretty_json(s:&str)->String {
    serde_json::from_str::<serde_json::Value>(s)
        .ok().and_then(|v| serde_json::to_string_pretty(&v).ok())
        .unwrap_or_else(||s.to_owned())
}

// Fixed classic two-column layout: controls keep their original geometry.
// The Windows settings window is not resizable by design. Only the FORM column
// can scroll vertically; the OLED preview/QR remains visible on the right.
// This deliberately reverts proportional column widths and DPI/window zoom.
fn main_layout(s:&State)->(i32,i32,i32,i32,i32,bool){
    let mut r=Rect::default();
    unsafe {GetClientRect(s.hwnd as Handle,&mut r);}
    let cw=(r.right-r.left).max(1);
    let ch=(r.bottom-r.top).max(1);
    (cw,ch,SETTINGS_LEFT,SETTINGS_FORM,100,true)
}
fn qr_geometry(s:&State,cw:i32,left:i32,form:i32,sc:i32)->(i32,i32,i32,i32){
    let px=|n:i32|n*sc/100;
    let x=left+form+px(14);
    let mut client=Rect::default();
    unsafe{GetClientRect(s.hwnd as Handle,&mut client);}
    let available_h=(client.bottom-px(103)-px(95)).max(px(80));
    let max_by_height=s.qr.as_ref().map(|(w,h,_)|{
        (available_h as i64 * *w as i64 / *h as i64) as i32
    }).unwrap_or(px(360));
    let width=(cw-x-px(14)).max(1).min(px(360)).min(max_by_height.max(1));
    let h=s.qr.as_ref().map(|(w,h,_)| {
        (width as i64 * *h as i64 / *w as i64) as i32
    }).unwrap_or(0);
    (x,px(103),width,h)
}
fn page_bottom(s:&State, sc:i32,ch:i32,cw:i32,left:i32,form:i32)->i32{
    let px=|n:i32|n*sc/100;
    let last=s.placements.iter().filter(|p|p.page==s.page && p.id!=ID_QR_METHOD && p.id!=ID_QR_HINT)
        .map(|p|px(52+p.y-75+p.h+24)).max().unwrap_or(0);
    if s.page==5 {
        let (_,qy,_,qh)=qr_geometry(s,cw,left,form,sc);
        last.max(qy+qh+px(115)).max(ch)
    }else{ last.max(ch) }
}
fn layout_window(){
    if LAYOUT_ACTIVE.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){return;}
    let _guard=LayoutGuard;
    let Some(state)=STATE.get() else{return};
    let (parent,navigation,controls,scroll_info)={
        let Ok(mut s)=state.lock() else{return};
        let (cw,ch,left,form,sc,_)=main_layout(&s);
        let px=|n:i32|n*sc/100;
        // Navigation order is independent from the historical page IDs.
        // GIF Studio follows Animations, while all previous actions retain their IDs.
        let nav_order=[200u32,201,202,206,203,204,205];
        let navigation=s.buttons.iter().filter(|(id,_)|(200..207).contains(id) || *id==371)
            .map(|(id,h)| {
                let order=nav_order.iter().position(|n|n==id).unwrap_or(6) as i32;
                let y=if *id==371 {ch-px(51)}else{px(66)+order*px(52)};
                (*h,px(12),y,if *id==371 {px(118)} else {(left-px(27)).max(px(80))},
                    if *id==371 {px(32)}else{px(42)})
            }).collect::<Vec<_>>();
        let page=s.page;
        let bottom=page_bottom(&s,sc,ch,cw,left,form);
        let maximum=(bottom-ch+px(8)).max(0);
        s.scroll[page]=s.scroll[page].clamp(0,maximum);
        let scroll=s.scroll[page];
        let scroll_info=ScrollInfo{size:std::mem::size_of::<ScrollInfo>() as u32,
            mask:SIF_RANGE|SIF_PAGE|SIF_POS,min:0,max:bottom.max(ch),page:ch.max(1) as u32,
            pos:scroll,track_pos:0};
        let (qx,qy,_qw,qh)=qr_geometry(&s,cw,left,form,sc);
        let controls=s.placements.iter().map(|ctl| {
            let (x,y,w,h)=if ctl.id==ID_QR_METHOD || ctl.id==ID_QR_HINT {
                let y=qy+qh+px(if ctl.id==ID_QR_METHOD {14}else{40});
                (qx,y,(cw-qx-px(8)).max(1),px(24))
            }else{
                let x=left+px(ctl.x-244);
                let y=px(52+ctl.y-75)-scroll;
                let remaining=(form-px(ctl.x-244)-px(7)).max(1);
                let w=if ctl.class=="STATIC" {remaining} else {px(ctl.w).min(remaining)};
                (x,y,w,px(ctl.h))
            };
            let visible=ctl.page==page && y+h>px(38) && y<ch-px(4) && w>0;
            (ctl.hwnd,x,y,w,h,visible)
        }).collect::<Vec<_>>();
        (s.hwnd,navigation,controls,scroll_info)
    };
    // Win32 calls may synchronously enter wnd_proc; never keep STATE locked here.
    unsafe{
        for (h,x,y,w,height) in navigation {
            SetWindowPos(h as Handle,ptr::null_mut(),x,y,w,height,SWP_NOZORDER|SWP_NOACTIVATE);
        }
        SetScrollInfo(parent as Handle,SB_VERT,&scroll_info,1);
        for (h,x,y,w,height,visible) in controls {
            SetWindowPos(h as Handle,ptr::null_mut(),x,y,w,height,SWP_NOZORDER|SWP_NOACTIVATE);
            if (IsWindowVisible(h as Handle)!=0)!=visible {
                ShowWindow(h as Handle,if visible{SW_SHOW}else{SW_HIDE});
            }
        }
        // Moving native STATIC/BUTTON child windows during scrolling can leave
        // old glyphs behind. Redraw the complete client *and* child windows
        // after all geometry changes, not just the parent background.
        RedrawWindow(parent as Handle,ptr::null(),ptr::null_mut(),
            RDW_INVALIDATE|RDW_ERASE|RDW_ALLCHILDREN|RDW_UPDATENOW);
        // LISTBOX scroll artifacts: it has its own internal paint surface.
        // Invalidate after the parent and after its final position is applied.
        if let Some(profile)=STATE.get().and_then(|lock|lock.lock().ok().map(|st|st.profile_list)) {
            if !profile.eq(&0usize) && IsWindowVisible(profile as Handle)!=0 {
                RedrawWindow(profile as Handle,ptr::null(),ptr::null_mut(),
                    RDW_INVALIDATE|RDW_ERASE|RDW_UPDATENOW);
            }
        }

    }
}
fn move_page_scroll(delta:i32){
    let Some(state)=STATE.get() else{return};
    if let Ok(mut s)=state.lock(){
        let (cw,ch,left,form,sc,_)=main_layout(&s);
        let bottom=page_bottom(&s,sc,ch,cw,left,form);
        let maximum=(bottom-ch+8*sc/100).max(0);
        let page=s.page;
        s.scroll[page]=(s.scroll[page]+delta).clamp(0,maximum);
    }
    layout_window();
}
fn layout_editor(hwnd:Handle){
    let Some(editor)=EDITOR.get() else{return};
    let Ok(ed)=editor.lock() else{return};
    if ed.hwnd!=hwnd as usize{return;}
    let mut r=Rect::default();unsafe{GetClientRect(hwnd,&mut r);}
    let w=r.right.max(800);let h=r.bottom.max(400);
    let margin=18;let toolbar=52;let gap=10;
    let n=ed.controls.len() as i32;
    let bw=((w-2*margin-(n-1)*gap)/n).min(170);
    let row_width=n*bw+(n-1)*gap;
    let left=margin+(w-2*margin-row_width)/2;
    unsafe {SetWindowPos(ed.edit as Handle,ptr::null_mut(),margin,margin,w-2*margin,h-96,SWP_NOZORDER|SWP_NOACTIVATE);}
    for (i,(_,button)) in ed.controls.iter().enumerate(){
        unsafe{SetWindowPos(*button as Handle,ptr::null_mut(),left+i as i32*(bw+gap),h-toolbar,
            bw,34,SWP_NOZORDER|SWP_NOACTIVATE);}
    }
}


// Prevent hovering + scrolling from silently changing OLED brightness.
// Keyboard and direct mouse dragging continue to use the native trackbar.
unsafe extern "system" fn brightness_slider_proc(hwnd:Handle, message:u32, w:usize, l:isize)->isize {
    if message==WM_MOUSEWHEEL { return 0; }
    let previous=PREV_BRIGHTNESS_PROC.load(Ordering::Acquire);
    if previous!=0 { unsafe {CallWindowProcW(previous,hwnd,message,w,l)} }
    else { unsafe {DefWindowProcW(hwnd,message,w,l)} }
}

// The classic multiline Win32 EDIT does not consistently select all with Ctrl+A.
// Handle both the key and translated control character; preserve every other key.
unsafe extern "system" fn animation_editor_proc(hwnd:Handle,msg:u32,w:usize,l:isize)->isize {
    if msg==WM_KEYDOWN && (w as u32)==0x41 && unsafe {GetKeyState(VK_CONTROL)} < 0 {
        unsafe{SendMessageW(hwnd,EM_SETSEL,0,-1);}
        return 0;
    }
    if msg==WM_CHAR && w==1 {return 0;}
    let prior=PREV_EDITOR_PROC.load(Ordering::Acquire);
    if prior!=0 {unsafe{CallWindowProcW(prior,hwnd,msg,w,l)}}
    else {unsafe{DefWindowProcW(hwnd,msg,w,l)}}
}


// Native Win32/GDI visual shell inspired by OLED creator tools:
// dark cards, restrained cyan/purple accents and a permanent preview panel.
// No browser, WebView, Electron process, or changes to the USB/render engine.
fn paint_card(dc:Handle,rect:Rect,fill:u32,outline:u32,radius:i32){
    unsafe{
        let brush=CreateSolidBrush(fill);
        let pen=CreatePen(0,1,outline);
        let old_brush=SelectObject(dc,brush);
        let old_pen=SelectObject(dc,pen);
        RoundRect(dc,rect.left,rect.top,rect.right,rect.bottom,radius,radius);
        SelectObject(dc,old_pen);SelectObject(dc,old_brush);
        DeleteObject(brush);DeleteObject(pen);
    }
}
fn paint_text(dc:Handle,x:i32,y:i32,text:&str,size:i32,weight:i32,color:u32){
    let face=wide("Segoe UI");
    let font=unsafe{CreateFontW(-size,0,0,0,weight,0,0,0,1,0,0,5,0,face.as_ptr())};
    unsafe{
        let old=if font.is_null(){ptr::null_mut()}else{SelectObject(dc,font)};
        SetBkMode(dc,TRANSPARENT);SetTextColor(dc,color);
        let label=wide(text);
        TextOutW(dc,x,y,label.as_ptr(),(label.len()-1) as i32);
        if !font.is_null(){SelectObject(dc,old);DeleteObject(font);}
    }
}
fn paint_interface_chrome(dc:Handle,cw:i32,ch:i32,page:usize,dark:bool){
    let bg=if dark{COLOR_DARK_BG}else{rgb(238,243,247)};
    let card=if dark{COLOR_CARD_BG}else{rgb(255,255,255)};
    let border=if dark{COLOR_BORDER}else{rgb(205,216,223)};
    let foreground=if dark{COLOR_DARK_TEXT}else{rgb(34,47,58)};
    let muted=if dark{COLOR_MUTED}else{rgb(95,112,130)};
    unsafe{
        let background=CreateSolidBrush(bg);
        let rect=Rect{left:0,top:0,right:cw,bottom:ch};
        FillRect(dc,&rect,background);DeleteObject(background);
        let rail=CreateSolidBrush(if dark{COLOR_SIDEBAR}else{rgb(225,234,241)});
        let rect=Rect{left:0,top:0,right:178,bottom:ch};
        FillRect(dc,&rect,rail);DeleteObject(rail);
    }
    paint_card(dc,Rect{left:181,top:43,right:527,bottom:ch-11},card,border,18);
    paint_card(dc,Rect{left:536,top:70,right:cw-15,bottom:(ch-16).max(390)},card,border,18);
    paint_text(dc,17,10,"GxOled",23,700,accent());
    paint_text(dc,187,13,"STEELSERIES DAC OLED CONTROL",14,700,foreground);
    let subtitle=match page {
        0=>"GENERAL / PREFERENCES",1=>"CLOCK / STYLES",2=>"ANIMATION / PROFILES",
        3=>"DISPLAY / PROTECTION",4=>"MEDIA / PLAYBACK",5=>"ABOUT / SUPPORT",
        _=>"GIF STUDIO / LIBRARY",
    };
    paint_text(dc,549,52,subtitle,12,700,muted);
    paint_text(dc,549,87,"LIVE OLED  ·  128 × 64",12,700,accent());
    paint_text(dc,549,(ch-56).max(394),"128 x 64  |  STEELSERIES OLED",11,400,muted);
    let preview_line=Rect{left:538,top:72,right:cw-17,bottom:75};
    let highlight=unsafe{CreateSolidBrush(accent_secondary())};
    unsafe{FillRect(dc,&preview_line,highlight);DeleteObject(highlight);}
    let left_accent=Rect{left:182,top:44,right:527,bottom:47};
    let accent=unsafe{CreateSolidBrush(accent())};
    unsafe{FillRect(dc,&left_accent,accent);DeleteObject(accent);}
}
// Lightweight illustrative thumbnails for the clock picker; the 128x64
// panel on the right is the *real* live preview. These tiles never feed OLED.
fn paint_clock_gallery(dc:Handle,scroll:i32,ch:i32,dark:bool){
    let tile=if dark{rgb(19,24,31)}else{rgb(246,249,252)};
    let edge=if dark{COLOR_BORDER}else{rgb(200,216,224)};
    let ink=if dark{accent()}else{rgb(12,133,143)};
    for i in 0..5 {
        let col=i%2;let row=i/2;
        let x=186+col*168;
        let y=82+row*105-scroll;
        if y<43 || y+94>ch-4 {continue;}
        let r=Rect{left:x,top:y,right:x+157,bottom:y+95};
        paint_card(dc,r,tile,edge,12);
        match i {
            0=>paint_text(dc,x+30,y+43,"12:45",27,400,ink),
            1=>paint_text(dc,x+20,y+39,"09:42",34,700,accent_secondary()),
            2|3=>{
                let pen=unsafe{CreatePen(0,2,ink)};
                let brush=unsafe{CreateSolidBrush(tile)};
                unsafe{
                    let old_pen=SelectObject(dc,pen);let old_brush=SelectObject(dc,brush);
                    if i==2 {Ellipse(dc,x+56,y+39,x+104,y+86);}
                    else {RoundRect(dc,x+54,y+38,x+106,y+87,10,10);}
                    MoveToEx(dc,x+80,y+63,ptr::null_mut());LineTo(dc,x+80,y+44);
                    MoveToEx(dc,x+80,y+63,ptr::null_mut());LineTo(dc,x+98,y+71);
                    SelectObject(dc,old_brush);SelectObject(dc,old_pen);
                    DeleteObject(pen);DeleteObject(brush);
                }
            }
            _=>{
                paint_text(dc,x+31,y+38,"CASIO",11,700,ink);
                paint_text(dc,x+32,y+51,"04:17",22,700,ink);
            }
        }
    }
}

fn draw_owner_button(info:&DrawItem){
    let id=info.ctl_id;
    let nav=(200..207).contains(&id);
    let quit=id==371;
    let is_active=nav && ACTIVE_PAGE.load(Ordering::Relaxed)==(id-200) as u8;
    let dark=THEME.load(Ordering::Relaxed)==1;
    let pressed=info.item_state&ODS_SELECTED!=0;
    let card=if dark{COLOR_CARD_BG}else{rgb(245,247,250)};
    let accent_color=if matches!(id, ID_PROFILE_SELECT|ID_EDITOR_SAVE|ID_GIF_ADD){accent_secondary()}else{accent()};
    let highlighted=is_active || pressed;
    let fill=if highlighted{if dark{accent_surface()}else{rgb(215,249,246)}}else if quit{if dark{rgb(48,35,40)}else{rgb(252,233,235)}}else{card};
    let border=if highlighted{if dark{accent_color}else{rgb(13,126,137)}}else{if dark{COLOR_BORDER}else{rgb(198,209,221)}};
    let text_color=if dark{COLOR_DARK_TEXT}else{rgb(26,39,48)};
    let mut area=Rect{left:info.rect.left+1,top:info.rect.top+1,
        right:info.rect.right-1,bottom:info.rect.bottom-1};
    if area.right<=area.left || area.bottom<=area.top{return;}
    paint_card(info.dc,area,fill,border,10);
    if is_active{
        let brush=unsafe{CreateSolidBrush(accent())};
        let bar=Rect{left:area.left+1,top:area.top+8,right:area.left+4,bottom:area.bottom-8};
        unsafe{FillRect(info.dc,&bar,brush);DeleteObject(brush);}
    }
    let h=info.hwnd_item;
    let len=unsafe{GetWindowTextLengthW(h)}.clamp(0,2048) as usize;
    let mut buffer=vec![0u16;len+1];
    let count=unsafe{GetWindowTextW(h,buffer.as_mut_ptr(),(len+1) as i32)};
    if count<=0{return;}
    unsafe{
        SetBkMode(info.dc,TRANSPARENT);
        SetTextColor(info.dc,if is_active{if dark{accent()}else{rgb(0,112,121)}}else if quit{if dark{rgb(243,166,166)}else{rgb(134,39,53)}}else{text_color});
        let font=SendMessageW(h,WM_GETFONT,0,0) as Handle;
        let old=if font.is_null(){ptr::null_mut()}else{SelectObject(info.dc,font)};
        area.left+=if nav{9}else{4};area.right-=4;
        DrawTextW(info.dc,buffer.as_ptr(),count,&mut area,
            DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS);
        if !font.is_null(){SelectObject(info.dc,old);}
    }
}

unsafe extern "system" fn wnd_proc(hwnd:Handle, message:u32, w:usize, l:isize)->isize {
    match message {
        WM_ERASEBKGND if THEME.load(Ordering::Relaxed)==1 => {
            let mut rect=Rect::default();
            unsafe { GetClientRect(hwnd,&mut rect); }
            let brush=*DARK_BRUSH.get_or_init(||unsafe{CreateSolidBrush(COLOR_DARK_BG)} as usize);
            unsafe {FillRect(w as Handle,&rect,brush as Handle);}
            return 1;
        }
        WM_DRAWITEM if l!=0 => {
            let info=unsafe{&*(l as *const DrawItem)};
            if info.ctl_type==4 {draw_owner_button(info);return 1;}
        }
        WM_GETMINMAXINFO if l!=0 => {
            // Keep the JSON editor's five actions readable. Main settings is
            // already fixed size and is intentionally left unchanged.
            let editor_window=EDITOR.get().and_then(|lock|lock.lock().ok()
                .map(|editor|editor.hwnd==hwnd as usize)).unwrap_or(false);
            if editor_window {
                let sizes=unsafe{&mut *(l as *mut MinMaxInfo)};
                sizes.min_track_size.x=850;
                sizes.min_track_size.y=470;
                return 0;
            }
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX if THEME.load(Ordering::Relaxed)==1 => {
            unsafe{SetTextColor(w as Handle,COLOR_DARK_TEXT);SetBkColor(w as Handle,COLOR_EDITOR_BG);}
            return *EDIT_BRUSH.get_or_init(||unsafe{CreateSolidBrush(COLOR_EDITOR_BG)} as usize) as isize;
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN if THEME.load(Ordering::Relaxed)==1 => {
            let id=unsafe{GetDlgCtrlID(l as Handle)};
            let heading=[500,503,510,511,520,530,531,540,710,711,712,713,714,715,718].contains(&id);
            let color=if heading{accent()}else if [501,521,522,523,716,719,ID_ABOUT_CREDITS as i32,ID_ABOUT_COFFEE as i32,ID_QR_HINT as i32].contains(&id){COLOR_MUTED}else{COLOR_DARK_TEXT};
            unsafe {SetTextColor(w as Handle,color);SetBkColor(w as Handle,COLOR_CARD_BG);}
            return *CARD_BRUSH.get_or_init(||unsafe{CreateSolidBrush(COLOR_CARD_BG)} as usize) as isize;
        }
        WM_SIZE | WM_DPICHANGED => {
            if STATE.get().is_some() {layout_window();}
            if EDITOR.get().is_some(){layout_editor(hwnd);}
            return 0;
        }
        WM_HSCROLL if l!=0 => {
            let slider=STATE.get().and_then(|state|state.lock().ok().and_then(|s|
                s.buttons.iter().find(|(id,_)|*id==ID_BRIGHT_SLIDER).map(|(_,h)|*h)));
            if slider==Some(l as usize) {
                let value=unsafe{SendMessageW(l as Handle,TBM_GETPOS,0,0)}.clamp(1,10) as u8;
                if let Some(tx)=TX.get(){let _=tx.send(MenuEvent::SetBrightness(value));}
                return 0;
            }
            let studio=STATE.get().and_then(|state|state.lock().ok().and_then(|s|
                s.buttons.iter().find(|(id,h)|*h==l as usize && (*id==ID_STUDIO_THRESHOLD || *id==ID_STUDIO_BRIGHTNESS))
                .map(|(id,_)|*id)));
            if let Some(id)=studio {
                let val=unsafe{SendMessageW(l as Handle,TBM_GETPOS,0,0)} as i32;
                if let Some(tx)=TX.get(){let _=tx.send(if id==ID_STUDIO_THRESHOLD {
                    MenuEvent::SetGifStudioThreshold(val.clamp(0,255) as u8)
                }else {MenuEvent::SetGifStudioBrightness(val.clamp(-100,100) as i16)});}
                return 0;
            }
        }
        WM_VSCROLL if l==0 => {
            let command=(w&0xffff) as u32;
            let pos=((w>>16)&0xffff) as i32;
            match command {
                0=>move_page_scroll(-35),1=>move_page_scroll(35),
                2=>move_page_scroll(-240),3=>move_page_scroll(240),
                4|5=>{
                    let Some(state)=STATE.get() else{return 0;};
                    if let Ok(mut s)=state.lock(){let page=s.page;s.scroll[page]=pos;}
                    layout_window();
                }
                _=>{},
            }
            return 0;
        }
        WM_MOUSEWHEEL => {
            let delta=((w >> 16) as i16) as i32;
            move_page_scroll(if delta>0 {-52}else{52});return 0;
        }
        WM_KEYDOWN if w==0x21 || w==0x22 => {
            move_page_scroll(if w==0x21 {-260}else{260});return 0;
        }
        WM_CLOSE => { unsafe { ShowWindow(hwnd, SW_HIDE); } return 0; }
        WM_COMMAND => {
            let id = (w & 0xffff) as u32;
            if id==ID_EDITOR_SAVE || id==ID_EDITOR_RESET { apply_profile_edit(id==ID_EDITOR_RESET); return 0; }
            if id==ID_EDITOR_IMPORT {editor_import();return 0;}
            if id==ID_EDITOR_EXPORT {editor_export();return 0;}
            if id==ID_EDITOR_FORMAT {editor_format();return 0;}
            if id==ID_PROFILE_LIST {return 0;}
            if id==ID_PROFILE_SELECT {select_profile();return 0;}
            if id==ID_PROFILE_NEW {profile_new_blank();return 0;}
            if id==ID_PROFILE_IMPORT {profile_import();return 0;}
            if id==ID_PROFILE_EXPORT {profile_export();return 0;}
            if id==ID_PROFILE_DELETE {profile_delete();return 0;}
            if (200..207).contains(&id) { select_tab((id - 200) as usize); return 0; }
            if let Some(tx)=TX.get() {
                let event = match id {
                    300 => Some(MenuEvent::SetUiMode(ConfigUiMode::Tray)),
                    301 => Some(MenuEvent::SetUiMode(ConfigUiMode::Window)),
                    302 => Some(MenuEvent::SetUiMode(ConfigUiMode::Hybrid)),
                    310 => Some(MenuEvent::SetClockStyle(ConfigClockStyle::DigitalOriginal)),
                    311 => Some(MenuEvent::SetClockStyle(ConfigClockStyle::DigitalBold)),
                    312 => Some(MenuEvent::SetClockStyle(ConfigClockStyle::AnalogRound)),
                    313 => Some(MenuEvent::SetClockStyle(ConfigClockStyle::AnalogSquare)),
                    314 => Some(MenuEvent::SetClockStyle(ConfigClockStyle::CasioF91W)),
                    320 => Some(MenuEvent::SetTimeMode(ConfigTimeMode::Off)),
                    321 => Some(MenuEvent::SetTimeMode(ConfigTimeMode::H24)),
                    322 => Some(MenuEvent::SetTimeMode(ConfigTimeMode::H12)),
                    330 => Some(MenuEvent::SetGoAnimation(None)),
                    331 => Some(MenuEvent::SetGoAnimation(Some(super::GoAnimation::Matrix))),
                    332 => Some(MenuEvent::SetGoAnimation(Some(super::GoAnimation::Hyperspace))),
                    333 => Some(MenuEvent::SetGoAnimation(Some(super::GoAnimation::StarWars))),
                    334 => Some(MenuEvent::SetGoAnimation(Some(super::GoAnimation::GameOfLife))),
                    380 => Some(MenuEvent::SetLanguage(ConfigLanguage::English)),
                    381 => Some(MenuEvent::SetLanguage(ConfigLanguage::Turkish)),
                    390 => Some(MenuEvent::SetTheme(ConfigTheme::Light)),
                    391 => Some(MenuEvent::SetTheme(ConfigTheme::Dark)),
                    6090 => Some(MenuEvent::SetUiSkin(ConfigUiSkin::StudioOrange)),
                    6091 => Some(MenuEvent::SetUiSkin(ConfigUiSkin::StudioCyan)),
                    6092 => Some(MenuEvent::SetUiSkin(ConfigUiSkin::StudioViolet)),
                    ID_ENGINE_CLASSIC => Some(MenuEvent::SetGifEngine(GifEngine::Classic)),
                    ID_ENGINE_STUDIO => Some(MenuEvent::SetGifEngine(GifEngine::Studio)),
                    6080..=6083 => Some(MenuEvent::SetGifStudioPreset((id-6080) as u8)),
                    6100=>Some(MenuEvent::SetGifStudioFit(FitMode::Contain)),
                    6101=>Some(MenuEvent::SetGifStudioFit(FitMode::Cover)),
                    6102=>Some(MenuEvent::SetGifStudioFit(FitMode::Stretch)),
                    6110=>Some(MenuEvent::SetGifStudioDither(DitherMode::FloydSteinberg)),
                    6111=>Some(MenuEvent::SetGifStudioDither(DitherMode::Atkinson)),
                    6112=>Some(MenuEvent::SetGifStudioDither(DitherMode::Bayer)),
                    6113=>Some(MenuEvent::SetGifStudioDither(DitherMode::Threshold)),
                    6114=>Some(MenuEvent::SetGifStudioDither(DitherMode::None)),
                    ID_STUDIO_INVERT=>Some(MenuEvent::ToggleGifStudioInvert),
                    ID_STUDIO_IMPORT=>{studio_import();None},
                    ID_STUDIO_EXPORT=>{studio_export();None},
                    369 => { open_github(); None },
                    ID_KOFI => { open_kofi(); None },
                    371 => Some(MenuEvent::Quit),
                    372 => { open_profile_editor(); None },
                    ID_STARTUP => Some(MenuEvent::ToggleStartupFromWindow),
                    ID_GIF_ADD => Some(MenuEvent::OpenGifPickerFromWindow),
                    ID_GIF_TOGGLE => Some(MenuEvent::ToggleGifFromWindow),
                    ID_ANIM_STOP => Some(MenuEvent::SetGoAnimation(None)),
                    620 => Some(MenuEvent::SetShiftMode(ConfigShiftMode::Off)),
                    621 => Some(MenuEvent::SetShiftMode(ConfigShiftMode::Low)),
                    622 => Some(MenuEvent::SetShiftMode(ConfigShiftMode::High)),
                    625 => Some(MenuEvent::SetClockBurnInMode(ClockBurnInMode::Off)),
                    626 => Some(MenuEvent::SetClockBurnInMode(ClockBurnInMode::Wide)),
                    627 => Some(MenuEvent::SetClockBurnInMode(ClockBurnInMode::NoOverlap)),
                    651 => Some(MenuEvent::SetStatusNotifyMode(StatusNotifyMode::Off)),
                    652 => Some(MenuEvent::SetStatusNotifyMode(StatusNotifyMode::Notify)),
                    653 => Some(MenuEvent::SetStatusNotifyMode(StatusNotifyMode::Always)),
                    654 => Some(MenuEvent::SetStatusNotifyMode(StatusNotifyMode::WhenConnected)),
                    655 => Some(MenuEvent::SetStatusNotifyMode(StatusNotifyMode::WhenDisconnected)),
                    660 => Some(MenuEvent::SetQuitMode(ConfigQuitMode::Screensaver)),
                    661 => Some(MenuEvent::SetQuitMode(ConfigQuitMode::StockUi)),
                            354 => Some(MenuEvent::ToggleIdleFromWindow),
                    355 => Some(MenuEvent::ToggleMediaFromWindow),
                    _ => None,
                };
                if let Some(event) = event { let _ = tx.send(event); }
            }
            return 0;
        }
        WM_PAINT => {
            if let Some(state)=STATE.get() {
                if let Ok(guard)=state.lock() {
                    if guard.hwnd != hwnd as usize {
                        let mut paint=Paint {dc:ptr::null_mut(),erase:0,rect:Rect::default(),restore:0,inc_update:0,reserved:[0;32]};
                        unsafe {BeginPaint(hwnd,&mut paint);EndPaint(hwnd,&paint);}
                        return 0;
                    }
                    let mut ps = Paint {dc:ptr::null_mut(),erase:0,rect:Rect::default(),restore:0,inc_update:0,reserved:[0;32]};
                    let dc = unsafe { BeginPaint(hwnd,&mut ps) };
                    if !dc.is_null() {
                        let (cw,ch,left,form,sc,_preview_right)=main_layout(&guard);
                        let scale=|n:i32|n*sc/100;
                        paint_interface_chrome(dc,cw,ch,guard.page,guard.theme==ConfigTheme::Dark);
                        if guard.page==1 {
                            paint_clock_gallery(dc,guard.scroll[1],ch,guard.theme==ConfigTheme::Dark);
                        }
                        unsafe {SetStretchBltMode(dc,3);} // COLORONCOLOR keeps OLED / QR hard edges.
                        if guard.page!=5 {
                            let x=left+form+scale(14);
                            let dst_w=(cw-x-scale(14)).max(1).min(scale(360));
                            let dst_h=dst_w/2;
                            let y=scale(105);
                            if y<ch && y+dst_h>scale(38) {
                                let border=Rect{left:x-scale(4),top:y-scale(4),
                                    right:x+dst_w+scale(4),bottom:y+dst_h+scale(4)};
                                let bg=unsafe{CreateSolidBrush(0x00141414)};
                                unsafe{FillRect(dc,&border,bg);DeleteObject(bg);}
                                let info=BitmapInfo{header:BitmapInfoHeader{
                                    size:40,width:128,height:-64,planes:1,bit_count:32,compression:0,
                                    image_size:128*64*4,x_pixels_per_meter:0,y_pixels_per_meter:0,
                                    used:0,important:0},colors:[0]};
                                unsafe{StretchDIBits(dc,x,y,dst_w,dst_h,0,0,128,64,
                                    guard.framebuffer.as_ptr().cast(),&info,0,SRCCOPY);}
                                let label=wide("OLED PREVIEW · 128 × 64");
                                unsafe{SetBkMode(dc,1);
                                    SetTextColor(dc,if guard.theme==ConfigTheme::Dark{0x00dddddd}else{0x00333333});
                                    TextOutW(dc,x,y-scale(23),label.as_ptr(),(label.len()-1) as i32);}
                            }
                        } else if let Some((w,h,bytes))=&guard.qr {
                            let (x,base_y,show_w,show_h)=qr_geometry(&guard,cw,left,form,sc);
                            let y=base_y;
                            if y<ch && y+show_h>scale(38){
                                let info=BitmapInfo{header:BitmapInfoHeader{
                                    size:40,width:*w as i32,height:-(*h as i32),planes:1,bit_count:32,
                                    compression:0,image_size:(bytes.len()*4) as u32,
                                    x_pixels_per_meter:0,y_pixels_per_meter:0,used:0,important:0},colors:[0]};
                                unsafe{StretchDIBits(dc,x,y,show_w,show_h,0,0,*w as i32,*h as i32,
                                    bytes.as_ptr().cast(),&info,0,SRCCOPY);}
                            }
                        }
                    }
                    unsafe { EndPaint(hwnd,&ps); }
                    return 0;
                }
            }
        }
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd,message,w,l) }
}

fn select_tab(tab: usize) {
    if let Some(state) = STATE.get() {
        if let Ok(mut s) = state.lock() {
            if tab >= s.tabs.len() { return; }
            s.page = tab;
        }
    }
    ACTIVE_PAGE.store(tab as u8,Ordering::Relaxed);
    // layout_window shows/hides controls. Never ShowWindow while holding STATE.
    layout_window();
}
fn control(parent:Handle, class:&str, text:&str, id:u32, x:i32,y:i32,w:i32,h:i32,style:u32)->Handle {
    let klass=wide(class);let caption=wide(text);
    // Only ordinary push buttons are owner drawn. Native radio/check boxes
    // must retain BM_SETCHECK behavior for configuration synchronization.
    let appearance=if class=="BUTTON" && style & 0x000f==0 {style|BS_OWNERDRAW}else{style};
    unsafe { CreateWindowExW(if class=="EDIT" || class=="LISTBOX" {0x00000200}else{0},klass.as_ptr(),caption.as_ptr(),appearance | WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
        x,y,w,h,parent,id as usize as Handle,GetModuleHandleW(ptr::null()),ptr::null()) }
}

pub fn create(tx:Sender<MenuEvent>) -> Result<(),String> {
    if STATE.get().is_some(){return Ok(());}
    TX.set(tx).map_err(|_|"settings sender already initialized".to_owned())?;
    let class=wide("GxOledNativeSettings2026");
    let inst=unsafe{GetModuleHandleW(ptr::null())};
    let cls=WndClass{style:0,proc:Some(wnd_proc),cls_extra:0,wnd_extra:0,instance:inst,
        icon:ptr::null_mut(),cursor:unsafe{LoadCursorW(ptr::null_mut(),32512usize as *const u16)},
        background:unsafe{GetSysColorBrush(COLOR_WINDOW as i32)},
        menu_name:ptr::null(),class_name:class.as_ptr()};
    if unsafe{RegisterClassW(&cls)}==0{return Err("RegisterClassW failed".into());}
    // Restore a non-resizable main window. Custom editor windows retain their
    // independent resizable style; only the main settings uses a fixed width.
    let sw=unsafe{GetSystemMetrics(0)};
    let sh=unsafe{GetSystemMetrics(1)};
    let start_x=((sw-SETTINGS_WIDTH)/2).max(0);
    let start_y=((sh-SETTINGS_HEIGHT)/2).max(0);
    let hwnd=unsafe{CreateWindowExW(0,class.as_ptr(),wide("GxOled  —  Settings").as_ptr(),
        WS_CAPTION|WS_SYSMENU|WS_MINIMIZEBOX|WS_CLIPCHILDREN|WS_VSCROLL,
        start_x,start_y,SETTINGS_WIDTH,SETTINGS_HEIGHT,
        ptr::null_mut(),ptr::null_mut(),inst,ptr::null())};
    if hwnd.is_null(){return Err("CreateWindowExW failed".into());}
    let font_name=wide("Segoe UI");
    let font=unsafe{CreateFontW(-18,0,0,0,400,0,0,0,1,0,0,5,0,font_name.as_ptr())};
    let section_font=unsafe{CreateFontW(-18,0,0,0,700,0,0,0,1,0,0,5,0,font_name.as_ptr())};
    let mut tabs=vec![Vec::new();7];
    let mut buttons=Vec::new();
    let mut placements=Vec::new();
    for (idx,label) in ["General","Clock","Animations","Display","Media","About","GIF Studio"].iter().enumerate(){
        let ctrl=control(hwnd,"BUTTON",label,200+idx as u32,15,55+idx as i32*52,190,40,0);
        if !font.is_null(){unsafe{SendMessageW(ctrl,WM_SETFONT,font as usize,1);}}
        buttons.push((200+idx as u32,ctrl as usize));
    }
    // Quit belongs to the navigation rail, not the General page.
    let quit=control(hwnd,"BUTTON","Quit GxOled",371,8,365,114,31,WS_TABSTOP);
    if !font.is_null(){unsafe{SendMessageW(quit,WM_SETFONT,font as usize,1);}}
    buttons.push((371,quit as usize));
    let mut make=|tab:usize,kind:&str,label:&str,id:u32,x:i32,y:i32,w:i32,h:i32,style:u32| {
        let ctrl=control(hwnd,kind,label,id,x,y,w,h,style);
        let heading=[500,503,510,511,520,530,531,540,710,711,712,713,714,715,718]
            .contains(&id) && kind=="STATIC";
        let selected_font=if heading && !section_font.is_null(){section_font}else{font};
        if !selected_font.is_null(){unsafe{SendMessageW(ctrl,WM_SETFONT,selected_font as usize,1);}}
        tabs[tab].push(ctrl as usize);
        placements.push(ControlPlacement{hwnd:ctrl as usize,id,page:tab,x,y,w,h,class:kind.to_owned()});
        if id>0{buttons.push((id,ctrl as usize));}
        ctrl
    };
    // General
    make(0,"STATIC","App opening mode",500,244,75,320,26,0);
    for (i,name) in ["Tray menu","Window","Hybrid"].iter().enumerate(){
        make(0,"BUTTON",name,300+i as u32,244,112+i as i32*36,245,30,4|WS_TABSTOP);
    }
    make(0,"STATIC","Window: tray icon opens settings.\r\nHybrid: left = window, right = menu.",501,244,231,395,62,0);
    make(0,"STATIC","Language / Dil",0,244,302,220,25,0);
    make(0,"BUTTON","English",380,244,333,108,29,4|WS_TABSTOP);
    make(0,"BUTTON","Türkçe",381,362,333,110,29,4|WS_TABSTOP);
    make(0,"STATIC","Appearance / Görünüm",503,244,388,290,26,0);
    make(0,"BUTTON","Light",390,244,421,108,29,4|WS_TABSTOP);
    make(0,"BUTTON","Dark",391,362,421,110,29,4|WS_TABSTOP);
    make(0,"BUTTON","Start with Windows",ID_STARTUP,244,469,250,30,3|WS_TABSTOP);
    make(0,"STATIC","When GxOled closes",710,244,519,290,26,0);
    make(0,"BUTTON","Screensaver",ID_QUIT_BASE,244,554,235,29,4|WS_TABSTOP);
    make(0,"BUTTON","SteelSeries stock UI",ID_QUIT_BASE+1,244,590,235,29,4|WS_TABSTOP);
    make(0,"STATIC","Design theme",570,244,635,310,27,0);
    for (i,name) in ["Studio Orange","Studio Cyan","Studio Violet"].iter().enumerate() {
        make(0,"BUTTON",name,ID_STYLE_BASE+i as u32,244,675+i as i32*37,245,32,4|WS_TABSTOP);
    }
    // Clock
    make(1,"STATIC","CLOCK STYLE",510,244,75,290,26,0);
    // Two-column clock gallery, preserving the native radio ID/check state.
    for (i,name) in ["Digital Standard","Digital Big","Analog Round","Analog Square","Casio Digital Dial"].iter().enumerate(){
        let col=(i%2) as i32;let row=(i/2) as i32;
        make(1,"BUTTON",name,310+i as u32,244+col*168,114+row*105,154,29,4|WS_TABSTOP);
    }
    make(1,"STATIC","Time display",511,244,452,280,27,0);
    for (i,name) in ["Off","24-hour","12-hour"].iter().enumerate(){
        make(1,"BUTTON",name,320+i as u32,244,493+i as i32*35,250,29,4|WS_TABSTOP);
    }
    // Animation profiles are file-backed. There is no fixed limit.
    make(2,"STATIC","128×64 animation profiles",520,244,75,320,26,0);
    let profile_list=make(2,"LISTBOX","",ID_PROFILE_LIST,244,112,380,213,WS_VSCROLL|LBS_NOTIFY|WS_TABSTOP);
    make(2,"BUTTON","Use selected",ID_PROFILE_SELECT,244,341,245,31,0);
    make(2,"BUTTON","Stop animation / Clock",ID_ANIM_STOP,244,379,245,31,0);
    make(2,"BUTTON","Edit selected profile...",ID_EDIT_PROFILE,244,418,245,31,0);
    make(2,"BUTTON","New Profile",ID_PROFILE_NEW,244,462,109,31,0);
    make(2,"BUTTON","Import",ID_PROFILE_IMPORT,360,462,109,31,0);
    make(2,"BUTTON","Export",ID_PROFILE_EXPORT,244,502,109,31,0);
    make(2,"BUTTON","Delete",ID_PROFILE_DELETE,360,502,109,31,0);
    make(2,"STATIC","Create any number of presets or import/export JSON.",521,244,544,380,47,0);
    let folder_note=make(2,"STATIC","Custom profiles folder:\n%APPDATA%\\GxOled\\\nprofiles\\custom",522,244,596,380,76,0);
    let note_font=unsafe{CreateFontW(-14,0,0,0,400,0,0,0,1,0,0,5,0,font_name.as_ptr())};
    if !note_font.is_null(){unsafe{SendMessageW(folder_note,WM_SETFONT,note_font as usize,1);}}
    make(2,"STATIC","New Profile opens an empty JSON. Use Reset in the editor for a starter template.",523,244,690,345,55,0);
    // Native Windows trackbar requires common-controls initialization.
    let controls_init=InitCommonControls{size:std::mem::size_of::<InitCommonControls>() as u32,
        classes:ICC_BAR_CLASSES};
    if unsafe{InitCommonControlsEx(&controls_init)}==0 {
        return Err("Unable to initialize Windows slider controls".to_owned());
    }
    // GIF has its own category; playback stays unchanged and is no longer
    // crowded into animation profile management.
    make(6,"STATIC","GIF Library",718,244,75,330,28,0);
    make(6,"STATIC","128×64 GIF playback controls. Future editing tools will live here.",719,244,121,340,62,0);
    make(6,"BUTTON","Add GIF...",ID_GIF_ADD,244,204,245,36,0);
    make(6,"BUTTON","Start / Stop GIF",ID_GIF_TOGGLE,244,251,245,36,0);
    make(6,"STATIC","GIF processing engine",750,244,310,310,27,0);
    make(6,"BUTTON","Classic GIF",ID_ENGINE_CLASSIC,244,343,245,30,4|WS_TABSTOP);
    make(6,"BUTTON","Studio GIF",ID_ENGINE_STUDIO,244,379,245,30,4|WS_TABSTOP);
    make(6,"STATIC","Studio options",751,244,430,310,27,0);
    for (i,name) in ["Default","Crisp","Pixel Art","Soft Dither"].iter().enumerate(){
        make(6,"BUTTON",name,ID_STUDIO_PRESET_BASE+i as u32,244,464+i as i32*36,245,30,4|WS_TABSTOP);
    }
    make(6,"STATIC","Source fit",752,244,619,300,26,0);
    for (i,name) in ["Contain","Cover","Stretch"].iter().enumerate(){
        make(6,"BUTTON",name,ID_STUDIO_FIT_BASE+i as u32,244,650+i as i32*34,245,29,4|WS_TABSTOP);
    }
    make(6,"STATIC","Dithering",753,244,770,310,26,0);
    for (i,name) in ["Floyd–Steinberg","Atkinson","Bayer 4×4","Threshold","None"].iter().enumerate(){
        make(6,"BUTTON",name,ID_STUDIO_DITHER_BASE+i as u32,244,800+i as i32*32,245,27,4|WS_TABSTOP);
    }
    make(6,"BUTTON","Invert output",ID_STUDIO_INVERT,244,982,255,29,3|WS_TABSTOP);
    make(6,"STATIC","Threshold",754,244,1021,300,26,0);
    let thr=make(6,"msctls_trackbar32","",ID_STUDIO_THRESHOLD,244,1052,245,40,WS_TABSTOP|0x0001);
    unsafe{SendMessageW(thr,TBM_SETRANGE,1,(255i32<<16) as isize);SendMessageW(thr,TBM_SETPOS,1,128);}
    make(6,"STATIC","Brightness adjustment",755,244,1109,320,26,0);
    let bri=make(6,"msctls_trackbar32","",ID_STUDIO_BRIGHTNESS,244,1139,245,40,WS_TABSTOP|0x0001);
    unsafe{SendMessageW(bri,TBM_SETRANGE,1,((100i32<<16)|(-100i16 as u16 as i32)) as isize);SendMessageW(bri,TBM_SETPOS,1,0);}
    make(6,"BUTTON","Import Studio settings",ID_STUDIO_IMPORT,244,1199,245,32,0);
    make(6,"BUTTON","Export Studio settings",ID_STUDIO_EXPORT,244,1238,245,32,0);
    make(6,"STATIC","Changes take effect when GIF is restarted.",756,244,1290,340,48,0);
    // Display: everything available in tray also exists here.
    make(3,"STATIC","Display controls",530,244,75,320,26,0);
    make(3,"BUTTON","Screensaver when idle",ID_IDLE,244,113,240,30,3|WS_TABSTOP);
    make(3,"STATIC","OLED screen shift",712,244,163,300,25,0);
    for (i,name) in ["Off","Low","High"].iter().enumerate(){
        make(3,"BUTTON",name,ID_SHIFT_BASE+i as u32,244,194+i as i32*32,245,28,4|WS_TABSTOP);
    }
    make(3,"STATIC","Clock burn-in protection",713,244,302,310,26,0);
    for (i,name) in ["Off","Random Movements","No overlapping Pixels"].iter().enumerate(){
        make(3,"BUTTON",name,ID_BURN_BASE+i as u32,244,336+i as i32*33,245,29,4|WS_TABSTOP);
    }
    make(3,"STATIC","Screen brightness",531,244,458,330,28,0);
    let slider=make(3,"msctls_trackbar32","",ID_BRIGHT_SLIDER,244,493,320,42,WS_TABSTOP|0x0001);
    let old_proc=unsafe{SetWindowLongPtrW(slider,GWLP_WNDPROC,brightness_slider_proc as *const () as usize as isize)};
    if old_proc!=0 {PREV_BRIGHTNESS_PROC.store(old_proc,Ordering::Release);}
    unsafe {
        SendMessageW(slider,TBM_SETRANGE,1,((10i32<<16)|1) as isize);
        SendMessageW(slider,TBM_SETPOS,1,5);
    }
    make(3,"STATIC","Device default (move slider to adjust)",ID_BRIGHT_VALUE,244,538,350,29,0);
    make(3,"STATIC","Connection status icon",714,244,597,330,28,0);
    for (i,name) in ["Off","Notify","Always","When connected","When disconnected"].iter().enumerate(){
        make(3,"BUTTON",name,ID_NOTIFY_BASE+i as u32,244,633+i as i32*34,250,29,4|WS_TABSTOP);
    }
    // Media
    make(4,"STATIC","Media controls",540,244,75,310,26,0);
    make(4,"BUTTON","Show media metadata",ID_MEDIA,244,120,250,32,3|WS_TABSTOP);
    make(4,"STATIC","Media replaces the clock while music is playing. The clock returns after playback stops. Animations remain exclusive.",716,244,176,355,95,0);
    // About: open-source credits on the left, embedded publisher QR on the right.
    make(5,"STATIC","GxOled  /  MehmetNY",0,244,75,350,32,0);
    make(5,"BUTTON","GitHub · MehmetNY/GxOled",369,244,119,355,37,0);
    make(5,"BUTTON","Ko-fi · Support GxOled",ID_KOFI,244,163,355,37,0);
    make(5,"STATIC","GxOled builds on JerwuQu/ggoled; animations come from pozitronik/steelclock-go. Their original developers deserve the biggest thanks.",ID_ABOUT_CREDITS,244,220,385,155,0);
    make(5,"STATIC","Still want to buy me a coffee? I wouldn't say no!",ID_ABOUT_COFFEE,244,390,385,96,0);
    let qr_method=make(5,"STATIC","USDT · TRON (TRC20)",ID_QR_METHOD,244,480,325,24,0);
    let qr_hint=make(5,"STATIC","Scan the QR to buy me a coffee.",ID_QR_HINT,244,513,325,24,0);
    let small_font=unsafe{CreateFontW(-13,0,0,0,400,0,0,0,1,0,0,5,0,font_name.as_ptr())};
    if !small_font.is_null(){for h in [qr_method,qr_hint]{unsafe{SendMessageW(h,WM_SETFONT,small_font as usize,1);}}}
    STATE.set(Mutex::new(State{
        hwnd:hwnd as usize,tabs,buttons,placements,profile_entries:Vec::new(),
        profile_list:profile_list as usize,scroll:[0;7],running_profile_key:None,last_sync:None,
        animation:None,page:0,framebuffer:vec![0;128*64],language:ConfigLanguage::English,
        theme:ConfigTheme::Dark,qr:load_embedded_qr(),
    })).map_err(|_|"settings state already initialized".to_owned())?;
    refresh_profile_list(None);
    select_tab(0);
    Ok(())
}

pub fn poll() {
    let mut msg=Message{hwnd:ptr::null_mut(),message:0,wparam:0,lparam:0,time:0,point_x:0,point_y:0,private:0};
    unsafe{while PeekMessageW(&mut msg,ptr::null_mut(),0,0,PM_REMOVE)!=0 {
        TranslateMessage(&msg); DispatchMessageW(&msg);
    }}
}
pub fn open() {
    // ShowWindow can synchronously deliver WM_SIZE to wnd_proc. Taking the
    // State lock here and keeping it across ShowWindow deadlocks the UI.
    let hwnd = STATE.get().and_then(|state| state.lock().ok().map(|s| s.hwnd));
    if let Some(hwnd) = hwnd {
        unsafe {
            ShowWindow(hwnd as Handle, SW_SHOW);
            SetForegroundWindow(hwnd as Handle);
        }
    }
}
pub fn sync(config:&Config){
    let mode_id=match config.ui_mode{ConfigUiMode::Tray=>300,ConfigUiMode::Window=>301,ConfigUiMode::Hybrid=>302};
    let style_id=match config.clock_style{ConfigClockStyle::DigitalOriginal=>310,ConfigClockStyle::DigitalBold=>311,
        ConfigClockStyle::AnalogRound=>312,ConfigClockStyle::AnalogSquare=>313,ConfigClockStyle::CasioF91W=>314};
    let anim_id=match config.go_animation{None=>330,Some(GoAnimation::Matrix)=>331,
        Some(GoAnimation::Hyperspace)=>332,Some(GoAnimation::StarWars)=>333,
        Some(GoAnimation::GameOfLife)=>334};
    let time_id=match config.time_mode{ConfigTimeMode::Off=>320,ConfigTimeMode::H24=>321,ConfigTimeMode::H12=>322};
    let shift_id=match config.oled_shift {ConfigShiftMode::Off=>620,ConfigShiftMode::Low=>621,ConfigShiftMode::High=>622};
    let burn_id=match config.clock_burn_in_mode(){ClockBurnInMode::Off=>625,ClockBurnInMode::Wide=>626,ClockBurnInMode::NoOverlap=>627};
    let notify_id=match config.status_notify{StatusNotifyMode::Off=>651,StatusNotifyMode::Notify=>652,
        StatusNotifyMode::Always=>653,StatusNotifyMode::WhenConnected=>654,StatusNotifyMode::WhenDisconnected=>655};
    let quit_id=match config.quit_mode {ConfigQuitMode::Screensaver=>660,ConfigQuitMode::StockUi=>661};
    let Some(state) = STATE.get() else { return; };
    let (checks, profile_selection, brightness_ui) = {
        let Ok(mut s) = state.lock() else { return; };
        let snapshot=UiSnapshot::capture(config);
        if s.last_sync.as_ref()==Some(&snapshot) {return;}
        s.last_sync=Some(snapshot);
        s.animation=config.go_animation;
        let mut checks = Vec::new();
        for (id,handle) in &s.buttons {
            if [300,301,302,310,311,312,313,314,320,321,322,330,331,332,333,334,
                ID_IDLE,ID_MEDIA,380,381,390,391,ID_STARTUP,620,621,622,
                625,626,627,651,652,653,654,655,660,661,
                6090,6091,6092,ID_ENGINE_CLASSIC,ID_ENGINE_STUDIO,
                6100,6101,6102,6110,6111,6112,6113,6114,ID_STUDIO_INVERT].contains(id) {
                let checked=*id==mode_id || *id==style_id || *id==anim_id || *id==time_id
                    || *id==shift_id || *id==burn_id || *id==notify_id || *id==quit_id
                    || (*id==ID_IDLE && config.idle_timeout)
                    || (*id==ID_MEDIA && config.show_media)
                    || (*id==ID_STARTUP && config.startup)
                    || (*id==380 && config.language==ConfigLanguage::English)
                    || (*id==381 && config.language==ConfigLanguage::Turkish)
                    || (*id==6090 && config.ui_skin==ConfigUiSkin::StudioOrange)
                    || (*id==6091 && config.ui_skin==ConfigUiSkin::StudioCyan)
                    || (*id==6092 && config.ui_skin==ConfigUiSkin::StudioViolet)
                    || (*id==ID_ENGINE_CLASSIC && config.gif_engine==GifEngine::Classic)
                    || (*id==ID_ENGINE_STUDIO && config.gif_engine==GifEngine::Studio)
                    || (*id==6100 && config.gif_studio.fit==FitMode::Contain)
                    || (*id==6101 && config.gif_studio.fit==FitMode::Cover)
                    || (*id==6102 && config.gif_studio.fit==FitMode::Stretch)
                    || (*id==6110 && config.gif_studio.dither==DitherMode::FloydSteinberg)
                    || (*id==6111 && config.gif_studio.dither==DitherMode::Atkinson)
                    || (*id==6112 && config.gif_studio.dither==DitherMode::Bayer)
                    || (*id==6113 && config.gif_studio.dither==DitherMode::Threshold)
                    || (*id==6114 && config.gif_studio.dither==DitherMode::None)
                    || (*id==ID_STUDIO_INVERT && config.gif_studio.invert_output)
                    || (*id==390 && config.theme==ConfigTheme::Light)
                    || (*id==391 && config.theme==ConfigTheme::Dark);
                checks.push((*handle, checked));
            }
        }
        let key=config.go_animation.map(|a|(a,config.go_profile_filename.clone()));
        let mut profile_selection = None;
        if s.running_profile_key != key {
            s.running_profile_key = key.clone();
            if let Some((kind, filename)) = key {
                if let Some(index) = s.profile_entries.iter().position(|p|p.animation==kind && p.custom==filename) {
                    profile_selection = Some((s.profile_list, index));
                }
            }
        }
        let slider=s.buttons.iter().find(|(id,_)|*id==ID_BRIGHT_SLIDER).map(|(_,h)|*h);
        let label=s.buttons.iter().find(|(id,_)|*id==ID_BRIGHT_VALUE).map(|(_,h)|*h);
        (checks, profile_selection, (slider,label,config.screen_brightness,s.language))
    }; // STATE released before synchronous Win32 messages.
    unsafe {
        for (handle, checked) in checks {
            SendMessageW(handle as Handle, BM_SETCHECK, if checked {BST_CHECKED} else {0}, 0);
        }
        if let Some((handle, index)) = profile_selection {
            SendMessageW(handle as Handle, LB_SETCURSEL, index, 0);
        }
        let (slider,label,brightness,lang)=brightness_ui;
        if let (Some(handle),Some(level))=(slider,brightness){
            SendMessageW(handle as Handle,TBM_SETPOS,1,level as isize);
        }
        if let Some(handle)=label {
            let message=match brightness {
                Some(level)=>format!("{}: {}%",if lang==ConfigLanguage::Turkish{"Parlaklık"}else{"Brightness"},level*10),
                None=>if lang==ConfigLanguage::Turkish{"Cihaz varsayılanı (ayarlamak için kaydırın)".to_string()}
                    else{"Device default (move slider to adjust)".to_string()},
            };
            SetWindowTextW(handle as Handle,wide(&message).as_ptr());
        }
        let params=STATE.get().and_then(|state|state.lock().ok().map(|s| {
            let get=|id:u32|s.buttons.iter().find(|(key,_)|*key==id).map(|(_,h)|*h);
            (get(ID_STUDIO_THRESHOLD),get(ID_STUDIO_BRIGHTNESS))
        }));
        if let Some((threshold,brightness))=params {
            if let Some(h)=threshold {SendMessageW(h as Handle,TBM_SETPOS,1,config.gif_studio.threshold as isize);}
            if let Some(h)=brightness {SendMessageW(h as Handle,TBM_SETPOS,1,config.gif_studio.brightness as isize);}
        }
    }
}
pub fn preview(bitmap:&gxoled_lib::Bitmap) {
    if bitmap.w!=128 || bitmap.h!=64 {return;}
    let hwnd = STATE.get().and_then(|state| state.lock().ok().map(|mut s| {
        for (i,pixel) in bitmap.data.iter().enumerate(){ s.framebuffer[i]=if pixel{0x00ffffff}else{0}; }
        s.hwnd
    }));
    if let Some(hwnd) = hwnd { unsafe{InvalidateRect(hwnd as Handle,ptr::null(),0);} }
}

fn studio_import() {
    let Some(path)=rfd::FileDialog::new().add_filter("Studio JSON", &["json"]).pick_file() else{return;};
    let result=(||->Result<StudioSettings,String>{
        let data=std::fs::read(&path).map_err(|e|e.to_string())?;
        if data.len()>64*1024{return Err("Studio settings exceed 64KiB".into());}
        let settings:StudioSettings=serde_json::from_slice(&data).map_err(|e|e.to_string())?;
        settings.validate().map_err(|e|e.to_string())?;
        Ok(settings)
    })();
    match result {
        Ok(settings)=>if let Some(tx)=TX.get(){let _=tx.send(MenuEvent::SetGifStudioSettings(settings));},
        Err(error)=>profile_error("Invalid Studio settings",&error),
    }
}
fn studio_export() {
    let settings=STATE.get().and_then(|state|state.lock().ok().and_then(|s|
        s.last_sync.as_ref().map(|snapshot|snapshot.gif_studio))).unwrap_or_default();
    let Some(path)=rfd::FileDialog::new().set_file_name("gxoled-studio.json")
        .add_filter("Studio JSON", &["json"]).save_file() else{return;};
    match serde_json::to_string_pretty(&settings) {
        Ok(text)=>if let Err(error)=std::fs::write(path,text){profile_error("Export error",&error.to_string());},
        Err(error)=>profile_error("Export error",&error.to_string()),
    }
}

fn open_github() {
    let url=wide("https://github.com/MehmetNY/GxOled-v0.4.0");
    let op=wide("open");
    unsafe { ShellExecuteW(ptr::null_mut(),op.as_ptr(),url.as_ptr(),ptr::null(),ptr::null(),1); }
}
fn open_kofi() {
    let url=wide("https://ko-fi.com/");
    let op=wide("open");
    unsafe { ShellExecuteW(ptr::null_mut(),op.as_ptr(),url.as_ptr(),ptr::null(),ptr::null(),1); }
}

fn profile_error(title:&str,message:&str){
    let name=wide(title);let content=wide(message);
    unsafe {MessageBoxW(ptr::null_mut(),content.as_ptr(),name.as_ptr(),0x10);}
}

fn editor_content(ed:&Editor)->Result<String,String> {
    let len=unsafe{GetWindowTextLengthW(ed.edit as Handle)};
    if len<0 || len>65536{return Err("Profile must be smaller than 64 KiB".into());}
    let mut data=vec![0u16;len as usize+1];
    let read=unsafe{GetWindowTextW(ed.edit as Handle,data.as_mut_ptr(),len+1)};
    Ok(String::from_utf16_lossy(&data[..read as usize]).replace("\r\n", "\n"))
}
fn apply_profile_edit(reset: bool) {
    let Some(lock) = EDITOR.get() else { return; };
    let Some((animation, target, edit, text_result)) = lock.lock().ok().map(|ed| {
        (ed.animation, ed.target.clone(), ed.edit,
         if reset {Ok(profile_default(ed.animation).to_owned())} else {editor_content(&ed)})
    }) else { return; };
    let content = match text_result {
        Ok(content) => content,
        Err(err) => {profile_error("GxOled", &err); return;}
    };
    let kind = match valid_profile_text(&content) {
        Ok(kind) => kind,
        Err(err) => {profile_error("Invalid profile", &err); return;}
    };
    if kind != animation {
        profile_error("Invalid profile", "This editor's widget type cannot be changed. Import a new preset instead.");
        return;
    }
    if reset && custom_directory().is_some_and(|d| target.starts_with(d)) {
        if let Err(err) = std::fs::write(&target, &content) {
            profile_error("GxOled", &err.to_string()); return;
        }
    } else if reset {
        if let Err(err) = std::fs::remove_file(&target) {
            if err.kind() != std::io::ErrorKind::NotFound {
                profile_error("GxOled", &err.to_string()); return;
            }
        }
    } else {
        if let Some(dir) = target.parent() {
            if let Err(err) = std::fs::create_dir_all(dir) {
                profile_error("GxOled", &err.to_string()); return;
            }
        }
        if let Err(err) = std::fs::write(&target, content.as_bytes()) {
            profile_error("GxOled", &err.to_string()); return;
        }
    }
    if reset {
        let formatted = win_edit_text(&pretty_json(&content));
        unsafe {SetWindowTextW(edit as Handle, formatted.as_ptr());}
    }
    let selected = target.file_name().and_then(|x| x.to_str()).map(str::to_owned);
    let custom = custom_directory().is_some_and(|d| target.starts_with(d));
    let name = if custom {selected} else {None};
    if let Some(tx) = TX.get() {let _ = tx.send(MenuEvent::SetGoProfile(animation, name.clone()));}
    refresh_profile_list(name);
}
fn editor_format() {
    let Some(state) = EDITOR.get() else { return; };
    let Some((edit, result)) = state.lock().ok().map(|ed| (ed.edit, editor_content(&ed))) else {return;};
    let content = match result {Ok(t)=>t, Err(err)=>{profile_error("JSON", &err);return;}};
    if let Err(err) = valid_profile_text(&content) {profile_error("JSON validation", &err);return;}
    let value:serde_json::Value = match serde_json::from_str(&content) {
        Ok(v)=>v, Err(err)=>{profile_error("JSON", &err.to_string());return;}
    };
    if let Ok(formatted)=serde_json::to_string_pretty(&value) {
        let formatted=win_edit_text(&formatted);
        unsafe{SetWindowTextW(edit as Handle, formatted.as_ptr());}
    }
}
fn editor_import() {
    let Some(path)=rfd::FileDialog::new().add_filter("Animation JSON", &["json"]).pick_file() else {return;};
    let content=match std::fs::read_to_string(path){Ok(t)=>t,Err(err)=>{profile_error("Import",&err.to_string());return;}};
    let kind=match valid_profile_text(&content){Ok(k)=>k,Err(err)=>{profile_error("Import",&err);return;}};
    let handles=EDITOR.get().and_then(|state|state.lock().ok().map(|ed|(ed.animation,ed.edit)));
    if let Some((animation, edit))=handles {
        if kind != animation {
            profile_error("Import", "This JSON contains a different animation. Use 'Import' on the Animations page instead.");
            return;
        }
        let text=win_edit_text(&pretty_json(&content));
        unsafe{SetWindowTextW(edit as Handle,text.as_ptr());}
    }
}
fn editor_export() {
    let Some(editor)=EDITOR.get() else {return;};
    let Some(result)=editor.lock().ok().map(|ed| editor_content(&ed)) else {return;};
    let content=match result {Ok(t)=>t,Err(err)=>{profile_error("Export",&err);return;}};
    if let Err(err)=valid_profile_text(&content){profile_error("Export",&err);return;}
    if let Some(dest)=rfd::FileDialog::new().add_filter("Animation JSON", &["json"]).set_file_name("gxoled-profile.json").save_file(){
        if let Err(err)=std::fs::write(dest,pretty_json(&content)){profile_error("Export",&err.to_string());}
    }
}
fn open_profile_editor(){
    let entry=chosen_profile().unwrap_or(ProfileEntry{label:"Matrix".to_owned(),animation:GoAnimation::Matrix,custom:None});
    let Some(target)=selected_profile_path(&entry) else{return};
    let content=std::fs::read_to_string(&target).unwrap_or_else(|_|profile_default(entry.animation).to_owned());
    open_profile_editor_with(entry,target,pretty_json(&content));
}
fn open_profile_editor_with(entry:ProfileEntry,target:std::path::PathBuf,content:String){
    if let Some(lock) = EDITOR.get() {
        let handles = if let Ok(mut ed) = lock.lock() {
            ed.animation = entry.animation;
            ed.target = target;
            ed.title = entry.label.clone();
            // Do not call SetWindowTextW or ShowWindow while holding EDITOR;
            // those calls can synchronously dispatch WM_SIZE.
            Some((ed.hwnd, ed.edit))
        } else { None };
        if let Some((hwnd, edit)) = handles {
            let title = wide(&format!("GxOled — {}", entry.label));
            let content_w = win_edit_text(&content);
            unsafe {
                SetWindowTextW(hwnd as Handle, title.as_ptr());
                SetWindowTextW(edit as Handle, content_w.as_ptr());
                ShowWindow(hwnd as Handle, SW_SHOW);
                SetForegroundWindow(hwnd as Handle);
                SetFocus(edit as Handle);
            }
        }
        return;
    }
    let cls=wide("GxOledProfileEdit2026");
    let inst=unsafe{GetModuleHandleW(ptr::null())};
    let wnd=WndClass{style:0,proc:Some(wnd_proc),cls_extra:0,wnd_extra:0,instance:inst,
        icon:ptr::null_mut(),cursor:unsafe{LoadCursorW(ptr::null_mut(),32512usize as *const u16)},
        background:unsafe{GetSysColorBrush(COLOR_WINDOW as i32)},menu_name:ptr::null(),class_name:cls.as_ptr()};
    if unsafe{RegisterClassW(&wnd)}==0{profile_error("GxOled","Unable to register editor window");return;}
    let sw=unsafe{GetSystemMetrics(0)}.max(640);
    let sh=unsafe{GetSystemMetrics(1)}.max(480);
    let hwnd=unsafe{CreateWindowExW(0,cls.as_ptr(),wide("GxOled — Advanced Animation Editor").as_ptr(),
        WS_OVERLAPPEDWINDOW|WS_CLIPCHILDREN,60,60,900.min(sw-30).max(850),710.min(sh-40).max(470),
        ptr::null_mut(),ptr::null_mut(),inst,ptr::null())};
    if hwnd.is_null(){profile_error("GxOled","Unable to create profile editor");return;}
    let edit=control(hwnd,"EDIT","",400,16,16,728,520,
        WS_VSCROLL|ES_MULTILINE|ES_AUTOVSCROLL|ES_WANTRETURN|WS_TABSTOP);
    unsafe{
        SendMessageW(edit,EM_SETLIMITTEXT,65_536,0);
        SetWindowTextW(edit,win_edit_text(&content).as_ptr());
        let prior=SetWindowLongPtrW(edit,GWLP_WNDPROC,animation_editor_proc as *const () as usize as isize);
        if prior!=0 {PREV_EDITOR_PROC.store(prior,Ordering::Release);}
    }
    let font_name=wide("Consolas");
    let font=unsafe{CreateFontW(-15,0,0,0,400,0,0,0,1,0,0,5,0,font_name.as_ptr())};
    if !font.is_null(){unsafe{SendMessageW(edit,WM_SETFONT,font as usize,1);}}
    let mut controls=Vec::new();
    for (id,caption) in [
        (ID_EDITOR_SAVE,"Save & Apply"),(ID_EDITOR_FORMAT,"Validate / Format"),
        (ID_EDITOR_IMPORT,"Import JSON"),(ID_EDITOR_EXPORT,"Export JSON"),
        (ID_EDITOR_RESET,"Reset")
    ] {
        let ctrl=control(hwnd,"BUTTON",caption,id,15,570,150,32,0);
        if !font.is_null(){unsafe{SendMessageW(ctrl,WM_SETFONT,font as usize,1);}}
        controls.push((id,ctrl as usize));
    }
    if EDITOR.set(Mutex::new(Editor{
        hwnd:hwnd as usize,edit:edit as usize,animation:entry.animation,
        target,title:entry.label,controls,
    })).is_ok(){
        if THEME.load(Ordering::Relaxed)==1{
            let dark=1u32;
            unsafe{DwmSetWindowAttribute(hwnd,20,&dark,4);}
        }
        layout_editor(hwnd);
        unsafe{ShowWindow(hwnd,SW_SHOW);SetForegroundWindow(hwnd);}
    }
}

fn decode_qr_png(contents:&[u8])->Result<(u32,u32,Vec<u32>),String>{
    if contents.len()>2_000_000 {return Err("QR image exceeds 2 MB".into());}
    let image=image::load_from_memory_with_format(contents,image::ImageFormat::Png).map_err(|err|err.to_string())?.to_rgba8();
    let (w,h)=image.dimensions();
    if w==0 || h==0 || w>1024 || h>1024 {return Err("QR image must be 1–1024 pixels per side".into());}
    let pixels=image.pixels().map(|p| {
        let a=p.0[3] as u32;
        let r=(p.0[0] as u32*a + 255*(255-a))/255;
        let g=(p.0[1] as u32*a + 255*(255-a))/255;
        let b=(p.0[2] as u32*a + 255*(255-a))/255;
        (r<<16)|(g<<8)|b
    }).collect();
    Ok((w,h,pixels))
}
/// The publisher-provided QR is bundled in the binary, not chosen by the end user.
/// Legacy per-user donation-qr.png overrides are intentionally ignored.
fn load_embedded_qr()->Option<(u32,u32,Vec<u32>)>{
    decode_qr_png(include_bytes!("../assets/donation-usdt-tron.png")).ok()
}
