//! Loopback-only transfer from the bundled, offline HTML Studio editor.
//! The old Classic GIF loader and device writer remain the sole OLED owner.
#![cfg(target_os = "windows")]
use std::{ffi::c_void, io::{Read,Write}, net::{TcpListener,TcpStream}, path::PathBuf,
    sync::{mpsc::Sender,OnceLock}, thread, time::Duration};
use super::MenuEvent;
use gxoled_draw::gif_studio::GifEngine;

static ENDPOINT: OnceLock<(u16,String)> = OnceLock::new();
const MAX_BYTES:usize=64*1024*1024;
const MAX_HEADERS:usize=16*1024;
const TEMPLATE:&str=include_str!("../../studio/editor.html");

#[link(name="bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(algorithm:*mut c_void, buffer:*mut u8, length:u32, flags:u32)->i32;
}
#[link(name="shell32")]
unsafe extern "system" {
    fn ShellExecuteW(hwnd:*mut c_void, operation:*const u16, file:*const u16,
        parameters:*const u16, directory:*const u16, show:i32)->*mut c_void;
}
fn wide(s:&str)->Vec<u16>{s.encode_utf16().chain(std::iter::once(0)).collect()}
fn studio_dir()->PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir)
        .join("GxOled").join("Studio")
}
fn response(stream:&mut TcpStream,code:&str,body:&str){
    let _=write!(stream,"HTTP/1.1 {code}\r\nAccess-Control-Allow-Origin: null\r\nVary: Origin\r\nAccess-Control-Allow-Methods: POST, OPTIONS\r\nAccess-Control-Allow-Headers: X-GxOled-Token, Content-Type\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
}
fn handle(mut stream:TcpStream, token:&str, tx:&Sender<MenuEvent>){
    let _=stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _=stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut bytes=Vec::new();let mut buf=[0u8;8192];
    let hdr_end=loop {
        if bytes.len()>MAX_HEADERS {response(&mut stream,"431 Headers Too Large","Headers too large");return;}
        match stream.read(&mut buf){
            Ok(0)|Err(_)=>return,
            Ok(n)=>{bytes.extend_from_slice(&buf[..n]);
                if let Some(i)=bytes.windows(4).position(|w|w==b"\r\n\r\n"){break i+4;}}
        }
    };
    let Ok(header)=std::str::from_utf8(&bytes[..hdr_end]) else {response(&mut stream,"400 Bad Request","Invalid headers");return;};
    let mut lines=header.split("\r\n");let request=lines.next().unwrap_or("");
    if request=="OPTIONS /gif HTTP/1.1" {response(&mut stream,"204 No Content","");return;}
    if request!="POST /gif HTTP/1.1" {response(&mut stream,"404 Not Found","Unknown endpoint");return;}
    let mut length=None;let mut authorized=false;let mut origin_ok=false;
    for line in lines {
        if let Some((k,v))=line.split_once(':'){
            let v=v.trim();
            if k.eq_ignore_ascii_case("Content-Length"){length=v.parse::<usize>().ok();}
            if k.eq_ignore_ascii_case("X-GxOled-Token"){authorized=v==token;}
            if k.eq_ignore_ascii_case("Origin"){origin_ok=v=="null";}
        }
    }
    if !authorized || !origin_ok {response(&mut stream,"403 Forbidden","Unauthorized local Studio request");return;}
    let Some(len)=length else {response(&mut stream,"411 Length Required","Content-Length required");return;};
    if len<14||len>MAX_BYTES {response(&mut stream,"413 Payload Too Large","GIF must be at most 64 MiB");return;}
    let mut body=bytes.split_off(hdr_end);
    if body.len()>len {body.truncate(len);}
    while body.len()<len {
        match stream.read(&mut buf){Ok(0)|Err(_)=>{response(&mut stream,"400 Bad Request","Incomplete GIF");return;},
            Ok(n)=>{let take=(len-body.len()).min(n);body.extend_from_slice(&buf[..take]);}}
    }
    if !body.starts_with(b"GIF89a")&&!body.starts_with(b"GIF87a") {
        response(&mut stream,"415 Unsupported Media Type","Invalid GIF header");return;
    }
    let width=u16::from_le_bytes([body[6],body[7]]);
    let height=u16::from_le_bytes([body[8],body[9]]);
    if width!=128||height!=64 {response(&mut stream,"422 Unprocessable Entity","GxOled Studio requires 128x64 output");return;}
    let dir=studio_dir();
    if std::fs::create_dir_all(&dir).is_err(){response(&mut stream,"500 Internal Server Error","Cannot create Studio directory");return;}
    let tmp=dir.join("last-export.tmp");let dest=dir.join("last-export.gif");
    if std::fs::write(&tmp,body).is_err() || std::fs::rename(&tmp,&dest).is_err() {
        response(&mut stream,"500 Internal Server Error","Cannot save Studio GIF");return;
    }
    // Send in order: the classic engine has no second HID connection and is
    // already capable of playing the exact monochrome frames exported above.
    let _=tx.send(MenuEvent::SetGifEngine(GifEngine::Classic));
    let _=tx.send(MenuEvent::GifSelected(dest));
    let _=tx.send(MenuEvent::StartImportedGif);
    response(&mut stream,"200 OK","Queued GIF playback");
}

pub fn start(tx:Sender<MenuEvent>){
    if ENDPOINT.get().is_some(){return;}
    let Ok(listener)=TcpListener::bind(("127.0.0.1",0)) else {return;};
    let Ok(address)=listener.local_addr() else {return;};
    let mut token=[0u8;24];
    if unsafe{BCryptGenRandom(std::ptr::null_mut(),token.as_mut_ptr(),token.len() as u32,2)}!=0{return;}
    let token=token.iter().map(|v|format!("{v:02x}")).collect::<String>();
    if ENDPOINT.set((address.port(),token.clone())).is_err(){return;}
    thread::spawn(move || {for stream in listener.incoming(){match stream {
        Ok(stream)=>handle(stream,&token,&tx), Err(_)=>break,
    }}});
}

pub fn launch_studio(){
    let (port,token)=ENDPOINT.get().map(|(p,t)|(p.to_string(),t.clone()))
        .unwrap_or_else(||("offline".to_owned(),"offline".to_owned()));
    let html=TEMPLATE.replace("__GXOLED_PORT__",&port).replace("__GXOLED_TOKEN__",&token);
    let dir=studio_dir();
    if std::fs::create_dir_all(&dir).is_err(){return;}
    let path=dir.join("editor.html");
    if std::fs::write(&path,html).is_err(){return;}
    let open=wide("open");
    let url=format!("file:///{}",path.to_string_lossy().replace('\\',"/"));
    let params=wide(&format!("--app=\"{url}\""));
    let edge=wide("msedge.exe");
    let result=unsafe{ShellExecuteW(std::ptr::null_mut(),open.as_ptr(),edge.as_ptr(),params.as_ptr(),std::ptr::null(),1)};
    if (result as isize)<=32 {
        let filename=wide(&path.to_string_lossy());
        unsafe{ShellExecuteW(std::ptr::null_mut(),open.as_ptr(),filename.as_ptr(),std::ptr::null(),std::ptr::null(),1);}
    }
}
