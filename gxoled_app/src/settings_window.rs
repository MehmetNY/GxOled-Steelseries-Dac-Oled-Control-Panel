//! Native Win32 settings window (no browser/server, no additional UI runtime).
//! SDL's existing tray continues to be owned by the main thread.
#![cfg(target_os = "windows")]

use std::{ffi::c_void, ptr, sync::{mpsc::Sender, Mutex, OnceLock, atomic::{AtomicBool, AtomicU8, AtomicIsize, Ordering}}};
use super::{MenuEvent, ConfigClockStyle, ConfigUiMode, ConfigLanguage, ConfigTheme, ConfigTimeMode, ConfigShiftMode, ClockBurnInMode, StatusNotifyMode, ConfigQuitMode, Config, GoAnimation};

type Handle = *mut c_void;
const WM_ERASEBKGND: u32 = 0x0014;
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
const SETTINGS_WIDTH: i32 = 1050;
const SETTINGS_HEIGHT: i32 = 710;
const SETTINGS_LEFT: i32 = 184;
const SETTINGS_FORM: i32 = 417;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_CHILD: u32 = 0x4000_0000;
const WS_CLIPSIBLINGS:u32=0x0400_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WM_CLOSE: u32 = 0x0010;
const WM_COMMAND: u32 = 0x0111;
const WM_PAINT: u32 = 0x000f;
const WM_DRAWITEM: u32 = 0x002b;
const WM_GETMINMAXINFO: u32 = 0x0024;
const BS_OWNERDRAW: u32 = 0x000b;
const ODS_SELECTED: u32 = 0x0001;
const DT_CENTER:u32=0x0001;
const DT_VCENTER:u32=0x0004;
const DT_SINGLELINE:u32=0x0020;
const DT_END_ELLIPSIS:u32=0x00008000;
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
const COLOR_DARK_BG: u32 = 0x001b1918; // #18191B
const COLOR_EDITOR_BG:u32=0x00272422; // #222427
const COLOR_DARK_TEXT: u32 = 0x00f5f4ee;
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
#[repr(C)] #[derive(Default)]
struct Rect {left:i32, top:i32, right:i32, bottom:i32}
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
struct WinPoint {x:i32,y:i32}
#[repr(C)]
struct MinMaxInfo {reserved:WinPoint,max_size:WinPoint,max_pos:WinPoint,min_track:WinPoint,max_track:WinPoint}
#[repr(C)]
struct DrawItem {ctl_type:u32,ctl_id:u32,item_id:u32,item_action:u32,item_state:u32,
    hwnd_item:Handle,hdc:Handle,rect:Rect,item_data:usize}
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
    fn GetWindowTextW(hwnd:Handle,text:*mut u16,max:i32)->i32;
    fn GetDlgCtrlID(hwnd:Handle)->i32;
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
    fn DrawTextW(dc:Handle,text:*const u16,length:i32,rect:*mut Rect,format:u32)->i32;
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
    fn SelectObject(dc:Handle,obj:Handle)->Handle;
    fn RoundRect(dc:Handle,left:i32,top:i32,right:i32,bottom:i32,ellipse_w:i32,ellipse_h:i32)->i32;
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
    language:ConfigLanguage,theme:ConfigTheme,shift:ConfigShiftMode,
    brightness:Option<u8>,burn_wide:bool,burn_no_overlap:bool,
    media:bool,idle:bool,startup:bool,
    status:StatusNotifyMode,quit:ConfigQuitMode,
}
impl UiSnapshot {
    fn capture(c:&Config)->Self {Self{
        mode:c.ui_mode,style:c.clock_style,time:c.time_mode,
        animation:c.go_animation,profile:c.go_profile_filename.clone(),
        language:c.language,theme:c.theme,shift:c.oled_shift,
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
static EDIT_BRUSH: OnceLock<usize> = OnceLock::new();
static PANEL_BRUSH: OnceLock<usize> = OnceLock::new();
static UI_FONT: OnceLock<usize> = OnceLock::new();
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
        .map(|p|px(115+p.y-75+p.h+24)).max().unwrap_or(0);
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
        let navigation=s.buttons.iter().filter(|(id,_)|(200..207).contains(id) || *id==371)
            .map(|(id,h)| {
                // Visual order is General, Clock, Animations, GIF Studio, Display, Media, About.
                // Control IDs and existing page indices intentionally remain unchanged.
                let nav_order=match *id {200=>0,201=>1,202=>2,206=>3,203=>4,204=>5,205=>6,_=>0};
                let step=px(49);
                let y=if *id==371 {ch-px(53)}else{px(76)+nav_order*step};
                let height=if *id==371 {px(32)}else{px(39)};
                (*h,px(14),y,if *id==371 {px(128)} else {(left-px(30)).max(px(92))},height)
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
                let y=px(115+ctl.y-75)-scroll;
                let remaining=(form-px(ctl.x-244)-px(7)).max(1);
                let w=if ctl.class=="STATIC" {remaining} else {px(ctl.w).min(remaining)};
                (x,y,w,px(ctl.h))
            };
            let visible=ctl.page==page && y>=px(110) && y<ch-px(10) && w>0;
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
            SetWindowPos(h as Handle,ptr::null_mut(),x,y,w,height,SWP_NOZORDER|SWP_NOACTIVATE|SWP_NOREDRAW);
            if (IsWindowVisible(h as Handle)!=0)!=visible {
                ShowWindow(h as Handle,if visible{SW_SHOW}else{SW_HIDE});
            }
        }
        // Moving native STATIC/BUTTON child windows during scrolling can leave
        // old glyphs behind. Redraw the complete client *and* child windows
        // after all geometry changes, not just the parent background.
        RedrawWindow(parent as Handle,ptr::null(),ptr::null_mut(),
            RDW_INVALIDATE|RDW_ERASE|RDW_ALLCHILDREN|RDW_UPDATENOW);
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
    let w=r.right.max(820);let h=r.bottom.max(460);
    let margin=18;
    let gap=10;
    let count=ed.controls.len() as i32;
    let bw=((w-2*margin-(count-1)*gap)/count).max(148);
    unsafe {SetWindowPos(ed.edit as Handle,ptr::null_mut(),margin,margin,w-2*margin,h-90,SWP_NOZORDER|SWP_NOACTIVATE);}
    for (i,(_,button)) in ed.controls.iter().enumerate(){
        unsafe{SetWindowPos(*button as Handle,ptr::null_mut(),margin+i as i32*(bw+gap),h-52,
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

// UI drawing only. All widget handlers, OLED frames and saved preferences stay unchanged.
// Win32 COLORREF is 0x00BBGGRR (not RGB).
fn color(r:u8,g:u8,b:u8)->u32 {(r as u32)|((g as u32)<<8)|((b as u32)<<16)}
fn selected_nav(id:u32)->bool {
    let Some(state)=STATE.get() else{return false;};
    state.try_lock().ok().is_some_and(|s|id==200+s.page as u32)
}
unsafe fn rounded(dc:Handle,rect:&Rect,fill:u32,border:u32,radius:i32){
    unsafe {
        let brush=CreateSolidBrush(fill);
        let pen=CreatePen(0,1,border);
        let old_brush=SelectObject(dc,brush);
        let old_pen=SelectObject(dc,pen);
        RoundRect(dc,rect.left,rect.top,rect.right,rect.bottom,radius,radius);
        SelectObject(dc,old_pen);
        SelectObject(dc,old_brush);
        DeleteObject(pen);
        DeleteObject(brush);
    }
}
unsafe fn draw_label(dc:Handle, caption:&str,rect:&Rect,shade:u32,centered:bool){
    let text=wide(caption);
    let mut rc=Rect{left:rect.left,top:rect.top,right:rect.right,bottom:rect.bottom};
    unsafe {
        SetBkMode(dc,1);
        SetTextColor(dc,shade);
        let old_font=UI_FONT.get().map(|font|SelectObject(dc,*font as Handle));
        DrawTextW(dc,text.as_ptr(),(text.len()-1) as i32,&mut rc,
            DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS|(if centered{DT_CENTER}else{0}));
        if let Some(font)=old_font{SelectObject(dc,font);}
    }
}
unsafe fn draw_action_button(draw:*const DrawItem)->isize {
    if draw.is_null(){return 0;}
    let info=unsafe{&*draw};
    let id=info.ctl_id;
    let dark=THEME.load(Ordering::Relaxed)==1;
    let nav=(200..207).contains(&id);
    let active=nav&&selected_nav(id);
    let pressed=(info.item_state&ODS_SELECTED)!=0;
    let prominent=[ID_PROFILE_SELECT,ID_PROFILE_NEW,ID_GIF_ADD,ID_EDITOR_SAVE].contains(&id);
    let accent=if prominent{color(145,66,223)}else{color(42,217,199)};
    let (fill,border,text_color)=if dark {
        if active {(color(25,61,61),color(43,203,183),color(235,255,251))}
        else if pressed {(color(42,60,69),accent,color(255,255,255))}
        else if prominent {(color(51,33,69),color(126,67,169),color(249,235,255))}
        else {(color(35,38,44),color(60,67,77),color(237,242,245))}
    } else if active {(color(211,243,238),color(16,153,136),color(18,50,48))}
        else {(color(246,248,252),color(187,197,206),color(35,42,51))};
    let r=&info.rect;
    unsafe {
        rounded(info.hdc,r,fill,border,12);
        if active {
            let bar=Rect{left:r.left+5,top:r.top+7,right:r.left+9,bottom:r.bottom-7};
            let brush=CreateSolidBrush(color(42,217,199));
            FillRect(info.hdc,&bar,brush);
            DeleteObject(brush);
        }
        let mut buffer=[0u16;260];
        let len=GetWindowTextW(info.hwnd_item,buffer.as_mut_ptr(),buffer.len() as i32);
        let caption=String::from_utf16_lossy(&buffer[..len.max(0) as usize]);
        let mut area=Rect{left:r.left+if nav{18}else{6},top:r.top+1,right:r.right-6,bottom:r.bottom-1};
        if nav {area.right=r.right-5;}
        draw_label(info.hdc,&caption,&area,text_color,!nav);
    }
    1
}
fn page_label(page:usize,tr:bool)->&'static str {
    match page {
        0=>if tr{"GENEL BAKIŞ"}else{"GENERAL CONTROL"},
        1=>if tr{"SAAT GALERİSİ"}else{"CLOCK GALLERY"},
        2=>if tr{"ANİMASYON PROFİLLERİ"}else{"ANIMATION PROFILES"},
        6=>"GIF STUDIO",
        3=>if tr{"OLED EKRAN AYARLARI"}else{"OLED DISPLAY"},
        4=>if tr{"MEDYA"}else{"MEDIA CONTROLS"},
        _=>if tr{"HAKKINDA"}else{"ABOUT GXOLED"},
    }
}
fn paint_panels(dc:Handle, s:&State,cw:i32,ch:i32){
    let dark=s.theme==ConfigTheme::Dark;
    let (bg,rail,panel,border,main_text,muted,teal)=if dark {
        (color(19,20,23),color(15,17,21),color(26,28,33),color(57,63,74),
            color(240,245,247),color(152,163,173),color(42,217,199))
    }else{(color(241,245,249),color(233,239,243),color(255,255,255),color(193,205,214),
            color(33,40,49),color(89,103,119),color(19,150,135))};
    let full=Rect{left:0,top:0,right:cw,bottom:ch};
    unsafe {
        let brush=CreateSolidBrush(bg); FillRect(dc,&full,brush);DeleteObject(brush);
        rounded(dc,&Rect{left:7,top:60,right:SETTINGS_LEFT-7,bottom:ch-9},rail,border,20);
        rounded(dc,&Rect{left:SETTINGS_LEFT-7,top:60,right:SETTINGS_LEFT+SETTINGS_FORM+5,bottom:ch-9},panel,border,20);
        rounded(dc,&Rect{left:SETTINGS_LEFT+SETTINGS_FORM+12,top:60,right:cw-11,bottom:ch-9},panel,border,20);
        draw_label(dc,"GxOled",&Rect{left:18,top:8,right:150,bottom:39},teal,false);
        draw_label(dc,"128 x 64   /   OLED CONTROL",&Rect{left:181,top:8,right:570,bottom:39},muted,false);
        draw_label(dc,page_label(s.page,s.language==ConfigLanguage::Turkish),
            &Rect{left:SETTINGS_LEFT+14,top:67,right:SETTINGS_LEFT+SETTINGS_FORM-12,bottom:94},main_text,false);
        let rule=Rect{left:SETTINGS_LEFT+14,top:99,right:SETTINGS_LEFT+70,bottom:102};
        let brush=CreateSolidBrush(teal);FillRect(dc,&rule,brush);DeleteObject(brush);
        let p_x=SETTINGS_LEFT+SETTINGS_FORM+25;
        draw_label(dc,if s.page==5{"DONATIONS   /   USDT TRON"}else{"LIVE OLED   /   MONOCHROME"},
            &Rect{left:p_x,top:75,right:cw-16,bottom:101},muted,false);
        if s.page!=5 {
            draw_label(dc,"SteelSeries  •  128 x 64",
                &Rect{left:p_x,top:346,right:cw-18,bottom:374},muted,false);
            draw_label(dc,"GxOled Advanced v0.5.0",
                &Rect{left:p_x,top:395,right:cw-20,bottom:422},muted,false);
            rounded(dc,&Rect{left:p_x-4,top:458,right:cw-25,bottom:555},
                if dark{color(30,35,43)}else{color(240,249,248)},border,16);
            let info=match s.page {
                0=>"APP MODE  /  QUICK ACCESS",
                1=>"CLOCK  /  LIVE STYLE PREVIEW",
                2=>"ANIMATIONS  /  128 x 64",
                6=>"GIF STUDIO  /  PLAYBACK",
                3=>"DISPLAY  /  BURN-IN CONTROL",
                _=>"MEDIA  /  SYSTEM PLAYBACK",
            };
            draw_label(dc,info,&Rect{left:p_x+10,top:470,right:cw-30,bottom:498},teal,false);
            let hint=match s.page {
                0=>"Tray, Window or Hybrid opening mode",
                1=>"Select a style to update this preview",
                2=>"Native Go-rendered animation profiles",
                6=>"GIF files, playback and OLED preview",
                3=>"Brightness and OLED pixel protection",
                _=>"Track details in a separate OLED view",
            };
            draw_label(dc,hint,&Rect{left:p_x+10,top:507,right:cw-30,bottom:537},muted,false);
        }
    }
}
unsafe extern "system" fn wnd_proc(hwnd:Handle, message:u32, w:usize, l:isize)->isize {
    match message {
        WM_DRAWITEM => {
            // Owner-drawn pushbuttons keep native keyboard/click behavior.
            return unsafe{draw_action_button(l as *const DrawItem)};
        }
        WM_GETMINMAXINFO => {
            // JSON editor controls require enough width for five readable actions.
            if let Some(editor)=EDITOR.get() {
                if editor.try_lock().ok().is_some_and(|ed|ed.hwnd==hwnd as usize) && l!=0 {
                    let bounds=unsafe{&mut *(l as *mut MinMaxInfo)};
                    bounds.min_track.x=860;
                    bounds.min_track.y=540;
                    return 0;
                }
            }
        }
        WM_ERASEBKGND if THEME.load(Ordering::Relaxed)==1 => {
            let mut rect=Rect::default();
            unsafe { GetClientRect(hwnd,&mut rect); }
            let brush=*DARK_BRUSH.get_or_init(||unsafe{CreateSolidBrush(COLOR_DARK_BG)} as usize);
            unsafe {FillRect(w as Handle,&rect,brush as Handle);}
            return 1;
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX if THEME.load(Ordering::Relaxed)==1 => {
            unsafe{SetTextColor(w as Handle,COLOR_DARK_TEXT);SetBkColor(w as Handle,COLOR_EDITOR_BG);}
            return *EDIT_BRUSH.get_or_init(||unsafe{CreateSolidBrush(COLOR_EDITOR_BG)} as usize) as isize;
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN if THEME.load(Ordering::Relaxed)==1 => {
            let panel=color(26,28,33);
            let ctrl_id=unsafe{GetDlgCtrlID(l as Handle)} as u32;
            let heading=message==WM_CTLCOLORSTATIC &&
                [500,503,510,511,520,718,530,531,713,540,710,712,714].contains(&ctrl_id);
            let fg=if heading{color(49,224,205)}else{COLOR_DARK_TEXT};
            unsafe {SetTextColor(w as Handle,fg);SetBkColor(w as Handle,panel);}
            return *PANEL_BRUSH.get_or_init(||unsafe{CreateSolidBrush(panel)} as usize) as isize;
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
                        paint_panels(dc,&guard,cw,ch);
                        unsafe {SetStretchBltMode(dc,3);} // COLORONCOLOR keeps OLED / QR hard edges.
                        if guard.page!=5 {
                            let x=left+form+scale(14);
                            let dst_w=(cw-x-scale(14)).max(1).min(scale(360));
                            let dst_h=dst_w/2;
                            let y=scale(145);
                            if y<ch && y+dst_h>scale(38) {
                                let info=BitmapInfo{header:BitmapInfoHeader{
                                    size:40,width:128,height:-64,planes:1,bit_count:32,compression:0,
                                    image_size:128*64*4,x_pixels_per_meter:0,y_pixels_per_meter:0,
                                    used:0,important:0},colors:[0]};
                                let border=Rect{left:x-3,top:y-3,right:x+dst_w+3,bottom:y+dst_h+3};
                                unsafe {rounded(dc,&border,color(14,16,19),color(31,123,114),12);}
                                // The OLED bitmap is strictly monochrome, irrespective of UI accent colours.
                                unsafe{StretchDIBits(dc,x,y,dst_w,dst_h,0,0,128,64,
                                    guard.framebuffer.as_ptr().cast(),&info,0,SRCCOPY);}
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
    // layout_window shows/hides controls. Never ShowWindow while holding STATE.
    layout_window();
    if let Some(s)=STATE.get() {
        if let Ok(guard)=s.lock(){
            unsafe { for (id,h) in &guard.buttons {
                if (200..207).contains(id) {InvalidateRect(*h as Handle,ptr::null(),1);}
            }}
        }
    }
}
fn control(parent:Handle, class:&str, text:&str, id:u32, x:i32,y:i32,w:i32,h:i32,style:u32)->Handle {
    let klass=wide(class);let caption=wide(text);
    let effective_style=if class=="BUTTON" && (style&0x000f)==0 {style|BS_OWNERDRAW}else{style};
    unsafe { CreateWindowExW(if class=="EDIT" || class=="LISTBOX" {0x00000200}else{0},klass.as_ptr(),caption.as_ptr(),effective_style | WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
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
    let font=unsafe{CreateFontW(-18,0,0,0,500,0,0,0,1,0,0,5,0,font_name.as_ptr())};
    if !font.is_null(){let _=UI_FONT.set(font as usize);}
    let mut tabs=vec![Vec::new();7];
    let mut buttons=Vec::new();
    let mut placements=Vec::new();
    for (idx,label) in ["General","Clock","Animations","Display","Media","About","GIF Studio"].iter().enumerate(){
        let ctrl=control(hwnd,"BUTTON",label,200+idx as u32,15,55+idx as i32*52,155,40,0);
        if !font.is_null(){unsafe{SendMessageW(ctrl,WM_SETFONT,font as usize,1);}}
        buttons.push((200+idx as u32,ctrl as usize));
    }
    // Quit belongs to the navigation rail, not the General page.
    let quit=control(hwnd,"BUTTON","Quit GxOled",371,8,365,114,31,WS_TABSTOP);
    if !font.is_null(){unsafe{SendMessageW(quit,WM_SETFONT,font as usize,1);}}
    buttons.push((371,quit as usize));
    let mut make=|tab:usize,kind:&str,label:&str,id:u32,x:i32,y:i32,w:i32,h:i32,style:u32| {
        let ctrl=control(hwnd,kind,label,id,x,y,w,h,style);
        if !font.is_null(){unsafe{SendMessageW(ctrl,WM_SETFONT,font as usize,1);}}
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
    // Clock
    make(1,"STATIC","CLOCK STYLE",510,244,75,290,26,0);
    for (i,name) in ["Digital Standard","Digital Big","Analog Round","Analog Square","Casio Digital Dial"].iter().enumerate(){
        make(1,"BUTTON",name,310+i as u32,244,114+i as i32*38,250,31,4|WS_TABSTOP);
    }
    make(1,"STATIC","Time display",511,244,325,280,27,0);
    for (i,name) in ["Off","24-hour","12-hour"].iter().enumerate(){
        make(1,"BUTTON",name,320+i as u32,244,365+i as i32*38,250,31,4|WS_TABSTOP);
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
    // GIF has its own category; playback stays unchanged and is no longer
    // crowded into animation profile management.
    make(6,"STATIC","GIF Library",718,244,75,330,28,0);
    make(6,"STATIC","128×64 GIF playback controls. Future editing tools will live here.",719,244,121,340,62,0);
    make(6,"BUTTON","Add GIF...",ID_GIF_ADD,244,204,245,36,0);
    make(6,"BUTTON","Start / Stop GIF",ID_GIF_TOGGLE,244,251,245,36,0);
    // Native Windows trackbar requires common-controls initialization.
    let controls_init=InitCommonControls{size:std::mem::size_of::<InitCommonControls>() as u32,
        classes:ICC_BAR_CLASSES};
    if unsafe{InitCommonControlsEx(&controls_init)}==0 {
        return Err("Unable to initialize Windows slider controls".to_owned());
    }
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
                625,626,627,651,652,653,654,655,660,661].contains(id) {
                let checked=*id==mode_id || *id==style_id || *id==anim_id || *id==time_id
                    || *id==shift_id || *id==burn_id || *id==notify_id || *id==quit_id
                    || (*id==ID_IDLE && config.idle_timeout)
                    || (*id==ID_MEDIA && config.show_media)
                    || (*id==ID_STARTUP && config.startup)
                    || (*id==380 && config.language==ConfigLanguage::English)
                    || (*id==381 && config.language==ConfigLanguage::Turkish)
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
    }
}
pub fn preview(bitmap:&gxoled_lib::Bitmap) {
    if bitmap.w!=128 || bitmap.h!=64 {return;}
    let hwnd = STATE.get().and_then(|state| state.lock().ok().map(|mut s| {
        for (i,pixel) in bitmap.data.iter().enumerate(){ s.framebuffer[i]=if pixel{0x00ffffff}else{0}; }
        s.hwnd
    }));
    if let Some(hwnd) = hwnd {
        let region=Rect{left:SETTINGS_LEFT+SETTINGS_FORM+11,top:134,right:SETTINGS_WIDTH-12,bottom:335};
        unsafe{InvalidateRect(hwnd as Handle,&region,0);}
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
        WS_OVERLAPPEDWINDOW|WS_CLIPCHILDREN,60,60,930.min(sw-30),680.min(sh-40),
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
    let font=unsafe{CreateFontW(-17,0,0,0,400,0,0,0,1,0,0,5,0,font_name.as_ptr())};
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
