//! GxOled Studio GIF processing: 128×64 output, independent from Classic GIF.
//! Architecture inspired by MIT-licensed GGIF-Studio (D4rkTT), without
//! its Electron renderer or GameSense device backend.
use crate::{Frame, MAX_GIF_FILE_BYTES, MAX_GIF_DIMENSION, MAX_GIF_FRAMES, MAX_GIF_TOTAL_PIXELS};
use anyhow::{Context, bail};
use gxoled_lib::Bitmap;
use image::{AnimationDecoder, ImageDecoder, ImageFormat, ImageReader, RgbaImage, imageops, codecs::gif::GifDecoder};
use serde::{Deserialize, Serialize};
use std::{io::BufReader, sync::Arc, time::Duration};

pub const WIDTH: u32 = 128;
pub const HEIGHT: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GifEngine { #[default] Classic, Studio }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FitMode { #[default] Contain, Cover, Stretch }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DitherMode { #[default] FloydSteinberg, Atkinson, Bayer, Threshold, None }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StudioSettings {
    pub fit: FitMode,
    pub dither: DitherMode,
    pub rotate_quarters: u8,
    pub flip_h: bool,
    pub flip_v: bool,
    pub zoom: f32,
    pub offset_x: i16,
    pub offset_y: i16,
    pub white_background: bool,
    pub invert_source: bool,
    pub brightness: f32,   // -100..100
    pub contrast: f32,     // -90..90
    pub gamma: f32,        // 0.1..5
    pub normalize: bool,
    pub threshold: u8,
    pub invert_output: bool,
    pub playback_speed: f32,
    pub minimum_frame_ms: u16,
    pub trim_start: usize,
    pub trim_end: Option<usize>,
}
impl Default for StudioSettings {
    fn default() -> Self {
        Self {fit:FitMode::Contain,dither:DitherMode::FloydSteinberg,
            rotate_quarters:0,flip_h:false,flip_v:false,zoom:1.0,
            offset_x:0,offset_y:0,white_background:false,invert_source:false,
            brightness:0.0,contrast:0.0,gamma:1.0,normalize:false,threshold:128,
            invert_output:false,playback_speed:1.0,minimum_frame_ms:30,
            trim_start:0,trim_end:None}
    }
}
impl StudioSettings {
    pub fn validate(&self)->anyhow::Result<()> {
        if !self.zoom.is_finite() || !(0.25..=8.0).contains(&self.zoom) {bail!("Studio zoom must be 0.25–8.0")}
        if !self.brightness.is_finite() || !(-100.0..=100.0).contains(&self.brightness){bail!("Invalid brightness")}
        if !self.contrast.is_finite() || !(-90.0..=90.0).contains(&self.contrast){bail!("Invalid contrast")}
        if !self.gamma.is_finite() || !(0.1..=5.0).contains(&self.gamma){bail!("Invalid gamma")}
        if !self.playback_speed.is_finite() || !(0.1..=5.0).contains(&self.playback_speed){bail!("Invalid playback speed")}
        if self.rotate_quarters>3 {bail!("Invalid rotation")}
        if self.offset_x.unsigned_abs()>1024 || self.offset_y.unsigned_abs()>1024 {bail!("Offsets are too large")}
        if self.minimum_frame_ms<10 {bail!("Minimum frame duration must be at least 10ms")}
        Ok(())
    }
    /// Starter looks for instant selection without overwriting custom settings.
    pub fn preset(index:u8)->Self {
        let mut s=Self::default();
        match index {
            1=>{s.dither=DitherMode::Threshold;s.contrast=30.0;},
            2=>{s.dither=DitherMode::Bayer;s.fit=FitMode::Cover;s.contrast=15.0;},
            3=>{s.dither=DitherMode::Atkinson;s.normalize=true;},
            _=>{},
        }
        s
    }
}

fn resize_frame(src:&RgbaImage,s:&StudioSettings)->RgbaImage {
    let mut image=match s.rotate_quarters {
        1=>imageops::rotate90(src),2=>imageops::rotate180(src),3=>imageops::rotate270(src),_=>src.clone(),
    };
    if s.flip_h{image=imageops::flip_horizontal(&image);}
    if s.flip_v{image=imageops::flip_vertical(&image);}
    let (w,h)=(image.width().max(1),image.height().max(1));
    let sx=WIDTH as f32/w as f32;let sy=HEIGHT as f32/h as f32;
    let scale=match s.fit{FitMode::Contain=>sx.min(sy),FitMode::Cover=>sx.max(sy),FitMode::Stretch=>1.0}*s.zoom;
    let tw=if s.fit==FitMode::Stretch {(WIDTH as f32*s.zoom).round() as u32}else{(w as f32*scale).round() as u32};
    let th=if s.fit==FitMode::Stretch {(HEIGHT as f32*s.zoom).round() as u32}else{(h as f32*scale).round() as u32};
    let tw=tw.clamp(1,768);let th=th.clamp(1,768);
    let small=imageops::resize(&image,tw,th,imageops::FilterType::Triangle);
    let base=if s.white_background{[255,255,255,255]}else{[0,0,0,255]};
    let mut target=RgbaImage::from_pixel(WIDTH,HEIGHT,image::Rgba(base));
    let left=(WIDTH as i32-tw as i32)/2+s.offset_x as i32;
    let top=(HEIGHT as i32-th as i32)/2+s.offset_y as i32;
    for (x,y,pixel) in small.enumerate_pixels(){
        let dx=left+x as i32;let dy=top+y as i32;
        if dx<0||dy<0||dx>=WIDTH as i32||dy>=HEIGHT as i32{continue;}
        let a=pixel[3] as u16;
        let mut rgb=[0u8;4];
        for i in 0..3 {rgb[i]=((pixel[i] as u16*a+base[i] as u16*(255-a))/255) as u8;}
        rgb[3]=255;
        target.put_pixel(dx as u32,dy as u32,image::Rgba(rgb));
    }
    target
}

/// Converts one fully composited source frame to the exact monochrome output
/// the existing HID driver accepts, without adding another device connection.
pub fn render_frame(src:&RgbaImage,s:&StudioSettings)->Bitmap {
    let img=resize_frame(src,s);
    let mut gray:Vec<f32>=img.pixels().map(|p|{
        let y=0.299*p[0] as f32+0.587*p[1] as f32+0.114*p[2] as f32;
        if s.invert_source {255.0-y}else{y}
    }).collect();
    if s.normalize {
        let (mut lo,mut hi)=(255.0f32,0.0f32);
        for &v in &gray {lo=lo.min(v);hi=hi.max(v)}
        if hi-lo>1.0 {for v in &mut gray{*v=(*v-lo)*255.0/(hi-lo);}}
    }
    let contrast=(1.0+s.contrast/100.0)/(1.0-s.contrast/100.0);
    for v in &mut gray {
        *v=((255.0*(*v/255.0).clamp(0.0,1.0).powf(1.0/s.gamma)+s.brightness*2.55-128.0)*contrast+128.0).clamp(0.0,255.0);
    }
    let mut out=Bitmap::new(WIDTH as usize,HEIGHT as usize,false);
    let bayer=[[0u8,8,2,10],[12,4,14,6],[3,11,1,9],[15,7,13,5]];
    for y in 0..HEIGHT as usize {
        for x in 0..WIDTH as usize {
            let i=y*WIDTH as usize+x;
            let level=gray[i];
            let threshold=if s.dither==DitherMode::Bayer {
                (bayer[y%4][x%4] as f32+0.5)*16.0
            } else if s.dither==DitherMode::None {128.0}else{s.threshold as f32};
            let white=level>=threshold;
            out.data.set(i,white ^ s.invert_output);
            if matches!(s.dither,DitherMode::FloydSteinberg|DitherMode::Atkinson) {
                let error = level - if white {255.0} else {0.0};
                let mut spread=|dx:isize,dy:isize,weight:f32|{
                    let nx=x as isize+dx;let ny=y as isize+dy;
                    if nx>=0&&ny>=0&&nx<WIDTH as isize&&ny<HEIGHT as isize{
                        gray[ny as usize*WIDTH as usize+nx as usize]+=error*weight;
                    }
                };
                if s.dither==DitherMode::Atkinson {
                    for (dx,dy) in [(1,0),(2,0),(-1,1),(0,1),(1,1),(0,2)]{spread(dx,dy,1.0/8.0);}
                }else{
                    for (dx,dy,weight) in [(1,0,7.0/16.0),(-1,1,3.0/16.0),(0,1,5.0/16.0),(1,1,1.0/16.0)]{spread(dx,dy,weight);}
                }
            }
        }
    }
    out
}

pub fn decode_studio_gif(path:&str,s:&StudioSettings)->anyhow::Result<Vec<Frame>> {
    s.validate()?;
    let file=std::fs::File::open(path).with_context(||format!("Could not open {path}"))?;
    if file.metadata()?.len()>MAX_GIF_FILE_BYTES{bail!("GIF exceeds 64MiB")}
    let reader=ImageReader::new(BufReader::new(file)).with_guessed_format()?;
    if reader.format()!=Some(ImageFormat::Gif){bail!("Studio currently accepts GIF files")}
    let decoder=GifDecoder::new(reader.into_inner())?;
    let (w,h)=decoder.dimensions();
    if w==0||h==0||w>MAX_GIF_DIMENSION||h>MAX_GIF_DIMENSION{bail!("Invalid source dimensions")}
    let mut frames=Vec::new();let mut total=0u64;
    for (index,result) in decoder.into_frames().enumerate() {
        if index>=MAX_GIF_FRAMES{bail!("GIF exceeds 2,000 frames")}
        if index>=s.trim_end.unwrap_or(usize::MAX){break;}
        let decoded=result.context("Corrupt GIF frame")?;
        total+=u64::from(decoded.buffer().width())*u64::from(decoded.buffer().height());
        if total>MAX_GIF_TOTAL_PIXELS{bail!("Too many decoded pixels")}
        if index<s.trim_start {continue;}
        let (num,denom)=decoded.delay().numer_denom_ms();
        let millis=(num as f32/denom.max(1) as f32/s.playback_speed)
            .round().max(s.minimum_frame_ms as f32) as u64;
        let bitmap=render_frame(decoded.buffer(),s);
        frames.push(Frame{bitmap:Arc::new(bitmap),delay:Some(Duration::from_millis(millis))});
    }
    if frames.is_empty(){bail!("No frames in Studio selection")}
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn monochrome_dimensions_and_invert(){
        let src=RgbaImage::from_pixel(10,5,image::Rgba([255,255,255,255]));
        let clean=StudioSettings{dither:DitherMode::Threshold,..Default::default()};
        let a=render_frame(&src,&clean);
        assert_eq!((a.w,a.h),(128,64));
        let b=render_frame(&src,&StudioSettings{invert_output:true,..clean});
        assert_eq!(a.data.len(),8192);
        assert_ne!(a.data[4096],b.data[4096]);
    }
    #[test] fn rejects_nonfinite_settings(){
        assert!(StudioSettings{gamma:f32::NAN,..Default::default()}.validate().is_err());
    }
}
