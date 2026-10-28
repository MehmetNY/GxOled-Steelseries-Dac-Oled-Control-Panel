// This is a wrapper around `gxoled_lib` that has high-level draw functions and additional events.
// Heavily specialised for the GxOled CLI and desktop app, and is therefore not recommended for general use.

mod casio;
mod osd_lab;

use anyhow::{Context, bail};
use chrono::{Local, Timelike};
use gxoled_lib::{Bitmap, Device, DeviceEvent, bitmap::BitVec};
use image::{AnimationDecoder, ImageDecoder, ImageFormat, ImageReader, codecs::gif::GifDecoder};
use parking_lot::{Mutex, MutexGuard};
use rusttype::{Font, Scale, point};
use std::{
    collections::BTreeMap,
    io::BufReader,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{Receiver, Sender, channel},
    },
    time::{Duration, Instant},
};

pub struct TextRenderer {
    font: Font<'static>,
    size: f32,
}
impl TextRenderer {
    pub fn load_from_file(path: &PathBuf, size: f32) -> anyhow::Result<Self> {
        const MAX_CUSTOM_FONT_BYTES: u64 = 32 * 1024 * 1024;

        if !size.is_finite() || !(1.0..=64.0).contains(&size) {
            bail!("Custom font size must be finite and between 1 and 64 pixels");
        }
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > MAX_CUSTOM_FONT_BYTES {
            bail!("Custom font file exceeds the 32 MiB safety limit");
        }
        let data = std::fs::read(path)?;
        // Recheck the bytes actually read in case the file changed after metadata lookup.
        if data.len() as u64 > MAX_CUSTOM_FONT_BYTES {
            bail!("Custom font file exceeds the 32 MiB safety limit");
        }
        let Some(font) = Font::try_from_vec(data) else {
            bail!("Failed to load font");
        };
        Ok(Self { font, size })
    }
    pub fn new_pixel_operator() -> Self {
        Self {
            font: Font::try_from_bytes(include_bytes!("../fonts/PixelOperator.ttf")).unwrap(),
            size: 16.0,
        }
    }
    pub fn line_height(&self) -> usize {
        self.line_height_at_size(self.size)
    }
    pub fn line_height_at_size(&self, size: f32) -> usize {
        let v_metrics = self.font.v_metrics(Scale::uniform(size));
        (v_metrics.ascent - v_metrics.descent).ceil() as usize
    }
    pub fn size(&self) -> f32 {
        self.size
    }
    pub fn render_lines(&self, text: &str) -> Vec<Bitmap> {
        self.render_lines_at_size(text, self.size)
    }
    pub fn render_lines_at_size(&self, text: &str, size: f32) -> Vec<Bitmap> {
        let clean_text = text.replace('\r', "");
        let text_lines = clean_text.split('\n');
        let scale = Scale::uniform(size);
        text_lines
            .map(|text_line| {
                let glyphs: Vec<_> = self.font.layout(text_line, scale, point(0.0, 0.0)).collect();
                let mut line_w_offset = 0;
                let mut line_h_offset = 0;
                let mut line_w = 0;
                let mut line_h = 0;
                for bb in glyphs.iter().filter_map(|g| g.pixel_bounding_box()) {
                    line_w_offset = line_w_offset.max(-bb.min.x);
                    line_h_offset = line_h_offset.max(-bb.min.y);
                    line_w = line_w.max(bb.max.x + 1);
                    line_h = line_h.max(bb.max.y + 1);
                }
                let line_w = (line_w + line_w_offset) as usize;
                let line_h = (line_h + line_h_offset) as usize;
                let mut bitmap = Bitmap::new(line_w, line_h, false);
                for glyph in glyphs {
                    if let Some(bb) = glyph.pixel_bounding_box() {
                        glyph.draw(|x, y, v| {
                            if v > 0.5 {
                                let px = (x as i32 + line_w_offset + bb.min.x) as usize;
                                let py = (y as i32 + line_h_offset + bb.min.y) as usize;
                                bitmap.data.set(py * line_w + px, true);
                            }
                        })
                    }
                }
                bitmap
            })
            .collect()
    }
}

fn bitmap_from_image(img: &image::RgbaImage, threshold: u8) -> Bitmap {
    let t3 = (threshold as usize) * 3 * 255;
    Bitmap {
        w: img.width() as usize,
        h: img.height() as usize,
        data: img
            .pixels()
            .map(|p| {
                let r = p.0[0] as usize * p.0[3] as usize;
                let g = p.0[1] as usize * p.0[3] as usize;
                let b = p.0[2] as usize * p.0[3] as usize;
                (r + g + b) >= t3
            })
            .collect::<BitVec>(),
    }
}
fn bitmap_from_dynimage(img: &image::DynamicImage, threshold: u8) -> Bitmap {
    bitmap_from_image(&img.to_rgba8(), threshold)
}
pub fn bitmap_from_memory(buf: &[u8], threshold: u8) -> anyhow::Result<Bitmap> {
    let img = image::load_from_memory(buf)?;
    Ok(bitmap_from_dynimage(&img, threshold))
}

#[derive(Clone)]
pub struct Frame {
    pub bitmap: Arc<Bitmap>,
    pub delay: Option<Duration>,
}

pub fn decode_frames(path: &str, threshold: u8) -> Vec<Frame> {
    let reader = ImageReader::open(path).expect("Failed to open image");
    if matches!(reader.format().unwrap(), ImageFormat::Gif) {
        let gif = GifDecoder::new(reader.into_inner()).expect("Failed to decode gif");
        let frames = gif.into_frames();
        frames
            .map(|frame| {
                let frame = frame.expect("Failed to decode gif frame");
                let bitmap = Arc::new(bitmap_from_image(frame.buffer(), threshold));
                Frame {
                    bitmap,
                    delay: Some({
                        let (numer, denom) = frame.delay().numer_denom_ms();
                        Duration::from_millis(numer as u64 / denom.max(1) as u64)
                    }),
                }
            })
            .collect()
    } else {
        let img = reader.decode().expect("Failed to decode image");
        let bitmap = Arc::new(bitmap_from_dynimage(&img, threshold));
        vec![Frame { bitmap, delay: None }]
    }
}

// Guard the interactive application against accidentally selecting an enormous or malformed
// GIF. The OLED is only 128x64; these limits are intentionally generous for source artwork while
// placing a bound on disk, decode-time, frame-count, and retained bitmap memory.
const MAX_GIF_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_GIF_DIMENSION: u32 = 2048;
const MAX_GIF_FRAMES: usize = 2000;
const MAX_GIF_TOTAL_PIXELS: u64 = 64 * 1024 * 1024;

pub fn try_decode_gif_frames(path: &str, threshold: u8) -> anyhow::Result<Vec<Frame>> {
    let file = std::fs::File::open(path).with_context(|| format!("Could not open {}", path))?;
    let file_size = file.metadata()?.len();
    if file_size > MAX_GIF_FILE_BYTES {
        bail!("GIF file is larger than the 64 MiB safety limit");
    }

    let reader = ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .context("Could not identify the selected image format")?;
    if reader.format() != Some(ImageFormat::Gif) {
        bail!("The selected file is not a GIF image");
    }

    let decoder = GifDecoder::new(reader.into_inner()).context("Could not decode GIF header")?;
    let (width, height) = decoder.dimensions();
    validate_gif_dimensions(width, height)?;

    let mut frames = Vec::new();
    let mut total_pixels = 0u64;
    for frame_result in decoder.into_frames() {
        if frames.len() >= MAX_GIF_FRAMES {
            bail!("GIF exceeds the 2,000-frame safety limit");
        }
        let frame = frame_result.context("Could not decode a GIF frame")?;
        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay = Duration::from_millis(numer as u64 / denom.max(1) as u64);
        let buffer = frame.into_buffer();
        let frame_pixels = u64::from(buffer.width()) * u64::from(buffer.height());
        total_pixels = total_pixels
            .checked_add(frame_pixels)
            .ok_or_else(|| anyhow::anyhow!("GIF pixel count overflow"))?;
        if total_pixels > MAX_GIF_TOTAL_PIXELS {
            bail!("GIF exceeds the decoded-pixel safety limit");
        }
        frames.push(Frame {
            bitmap: Arc::new(bitmap_from_image(&buffer, threshold)),
            delay: Some(delay),
        });
    }

    if frames.is_empty() {
        bail!("GIF contains no frames");
    }
    Ok(frames)
}

fn validate_gif_dimensions(width: u32, height: u32) -> anyhow::Result<()> {
    if width == 0 || height == 0 || width > MAX_GIF_DIMENSION || height > MAX_GIF_DIMENSION {
        bail!("GIF dimensions exceed the 2048 x 2048 safety limit");
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Hash, Eq, Ord)]
pub struct LayerId(usize);
impl LayerId {
    pub fn none() -> LayerId {
        LayerId(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockStyle {
    DigitalOriginal,
    DigitalBold,
    AnalogRound,
    AnalogSquare,
    CasioF91W,
}

pub enum DrawLayer {
    Image {
        bitmap: Arc<Bitmap>,
        x: isize,
        y: isize,
    },
    /// A live clock rendered on the OLED worker thread. The frame therefore keeps
    /// advancing even when the native system-tray menu temporarily blocks the UI loop.
    Clock {
        style: ClockStyle,
        twelve_hour: bool,
        x_offset: isize,
        y_offset: isize,
        media_visible: bool,
        turkish: bool,
    },
    Animation {
        frames: Vec<Frame>,
        x: isize,
        y: isize,
        follow_fps: bool,
    },
    Scroll {
        bitmap: Arc<Bitmap>,
        y: isize,
    },
}

pub enum ShiftMode {
    Off,
    Low,
    High,
}

enum DrawCommand {
    Play,
    Pause,
    SetShiftMode(ShiftMode),
    SetBrightness(u8),
    Stop,
    Probe,
}

#[derive(Debug)]
pub enum DrawEvent {
    DeviceDisconnected,
    DeviceReconnected,
    DeviceEvent(DeviceEvent),
    BrightnessSet { value: u8, error: Option<String> },
}

struct AnimState {
    ticks: usize,
    next_update: Instant,
    timing_initialized: bool,
}

fn animation_frame_delay(frame: &Frame) -> Duration {
    frame
        .delay
        .unwrap_or(Duration::from_secs(1))
        .max(Duration::from_millis(10))
}

/// Initialize the first frame's display duration, then advance at most one frame per render
/// cycle. Scheduling relative to the current render time prevents delayed frames from causing a
/// burst of rapid catch-up frames, which the OLED cannot display usefully at its fixed refresh rate.
fn update_animation_timing(state: &mut AnimState, frames: &[Frame], now: Instant) {
    if frames.is_empty() {
        return;
    }

    if !state.timing_initialized {
        state.next_update = now + animation_frame_delay(&frames[state.ticks % frames.len()]);
        state.timing_initialized = true;
        return;
    }

    if now >= state.next_update {
        state.ticks = (state.ticks + 1) % frames.len();
        state.next_update = now + animation_frame_delay(&frames[state.ticks]);
    }
}
struct ScrollState {
    x: isize,
}

struct DrawLayerState {
    layer: DrawLayer,
    anim: AnimState,
    scroll: ScrollState,
}

const OLED_SHIFT_PERIOD: Duration = Duration::from_secs(90);

fn square_perimeter(out: &mut Vec<(isize, isize)>, r: isize) {
    for x in 0..=r {
        out.push((x, -r));
    }
    for y in (-r + 1)..=r {
        out.push((r, y));
    }
    for x in ((-r)..r).rev() {
        out.push((x, r));
    }
    for y in ((-r)..r).rev() {
        out.push((-r, y));
    }
    for x in (-r + 1)..0 {
        out.push((x, -r));
    }
}
fn square_shift_calc(r: isize) -> Vec<(isize, isize)> {
    let mut result = vec![(0, 0)];
    for r in 1..=r {
        square_perimeter(&mut result, r);
    }
    for r in (1..r).rev() {
        square_perimeter(&mut result, r);
    }
    result
}

const RECONNECT_PERIOD: Duration = Duration::from_secs(1);
// The base station takes over its OLED temporarily to show the volume OSD.
// Stop sending custom frames until the knob has been idle for this long.
const VOLUME_OSD_HOLD: Duration = Duration::from_millis(2500);

/// A few DAC firmware versions repeat identical unsolicited dial events during
/// the built-in OSD. Do not turn a 2.5-second hold into a 5–6-second hold.
/// Real volume changes always extend the hold. The same level becomes a new
/// action only after a full quiet interval with no reports.
fn is_new_volume_dial_action(previous: &mut Option<(u8, Instant)>, level: u8, now: Instant) -> bool {
    let is_new = match *previous {
        Some((last_level, last_report)) => {
            level != last_level || now.saturating_duration_since(last_report) >= VOLUME_OSD_HOLD
        }
        None => true,
    };
    *previous = Some((level, now));
    is_new
}

fn run_draw_device_thread(
    mut dev: Device,
    layers: Arc<Mutex<LayerMap>>,
    cmd_receiver: Receiver<DrawCommand>,
    event_sender: Sender<DrawEvent>,
    texter: Arc<Mutex<TextRenderer>>,
    fps: usize,
) -> Device {
    let fps = fps.max(1);
    let frame_delay = Duration::from_nanos(1_000_000_000 / fps as u64);
    let mut prev_screen = Bitmap::new(0, 0, false);
    let mut playing = false;
    let shift_pos_low = square_shift_calc(1);
    let shift_pos_high = square_shift_calc(4);
    let mut shift_idx_low = 0;
    let mut shift_idx_high = 0;
    let mut last_shift = Instant::now();
    let mut shift_mode = ShiftMode::Off;
    let mut connected = true;
    let mut last_connect_attempt = Instant::now();
    let mut last_frame_time = Instant::now();
    let mut volume_osd_until: Option<Instant> = None;
    let mut volume_osd_started: Option<Instant> = None;
    let mut last_volume_dial_report: Option<(u8, Instant)> = None;
    // Explicitly opt-in diagnostics. Normal releases have no file writes.
    let osd_trace_file = std::env::var_os("GXOLED_OSD_TRACE_FILE");
    let osd_session_start = Instant::now();
    let osd_hold = std::env::var("GXOLED_OSD_HOLD_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|ms| *ms <= 10000)
        .map(Duration::from_millis)
        .unwrap_or(VOLUME_OSD_HOLD);
    let osd_legacy = std::env::var("GXOLED_OSD_LEGACY").ok().as_deref() == Some("1");
    // Opt-in, bounded test. During the first 7s after a physical dial event,
    // resend the current valid OLED frame every 100ms instead of every second.
    // Do not change pixel data or send undocumented HID commands.
    let osd_pulse = std::env::var("GXOLED_OSD_PULSE").ok().as_deref() == Some("1");
    let osd_log = |message: &str| {
        if let Some(path) = &osd_trace_file {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(f, "[{}ms pid={}] {}", osd_session_start.elapsed().as_millis(),
                    std::process::id(), message);
            }
        }
    };
    // Opt-in: burn a 4-digit frame counter into the bottom-right corner of every frame that is
    // actually sent, so a camera can show which HID write the panel finally displayed. The
    // counter is drawn on a copy: `prev_screen` stays unstamped, so the send cadence
    // (change-or-1Hz heartbeat) is identical to the normal build.
    // `GXOLED_OSD_LAB=1` (see `osd_lab`): one experiment per knob turn, stamped on screen.
    let osd_lab_enabled = std::env::var("GXOLED_OSD_LAB").ok().as_deref() == Some("1");
    let osd_stamp = osd_lab_enabled || std::env::var("GXOLED_OSD_STAMP").ok().as_deref() == Some("1");
    let mut osd_seq: u32 = 0;
    let mut osd_last_dial: Option<Instant> = None;
    let mut lab = osd_lab_enabled.then(|| osd_lab::OsdLab::new(osd_lab::default_steps()));
    let mut lab_hold = osd_hold;
    let mut lab_orig_home: Option<u8> = None;
    let mut lab_sonar_changed = false;
    let mut lab_home_changed = false;
    let mut lab_last_ui_cmd: Option<Instant> = None;
    let mut raw_unknown_logged = 0u32;
    osd_log(&format!("START hold_ms={} legacy={} pulse={} stamp={} lab={} pid=0x{:04x} fps={}",
        osd_hold.as_millis(), osd_legacy, osd_pulse, osd_stamp, osd_lab_enabled, dev.product_id(), fps));
    if osd_lab_enabled {
        let names: Vec<&str> = osd_lab::default_steps().iter().map(|step| step.label()).collect();
        osd_log(&format!("LAB_BEGIN steps={names:?}"));
        // Read-only: shows the device's own dim timer / home screen / Sonar flag at start.
        osd_log(&format!("LAB_QUERY_UX at=start result={:?}",
            dev.query_ux_settings().map_err(|e| e.to_string())));
    }
    let mut osd_trace_remaining_frames = 0u8;
    let mut last_accepted_volume_event: Option<Instant> = None;
    loop {
        let time = Instant::now();
        let mut stop_after_frame = false;
        while let Ok(cmd) = cmd_receiver.try_recv() {
            match cmd {
                DrawCommand::Play => playing = true,
                DrawCommand::Pause => playing = false,
                DrawCommand::SetShiftMode(mode) => shift_mode = mode,
                DrawCommand::SetBrightness(value) => {
                    let error = dev.set_brightness(value).err().map(|err| err.to_string());
                    event_sender
                        .send(DrawEvent::BrightnessSet { value, error })
                        .unwrap();
                }
                DrawCommand::Stop => stop_after_frame = true,
                DrawCommand::Probe => _ = dev.probe(),
            }
        }

        // Attempt to reconnect
        if !connected && time.duration_since(last_connect_attempt) >= RECONNECT_PERIOD {
            last_connect_attempt = time;
            if dev.reconnect().is_ok() {
                connected = true;
                event_sender.send(DrawEvent::DeviceReconnected).unwrap();
            }
        }

        // Poll device events BEFORE drawing. Turning the base station's volume dial
        // causes its firmware to show a native volume OSD. Sending a custom OLED frame
        // while that OSD is active makes the two writers race and can produce flicker.
        // Extend the yield period on every volume event, so continuous dial movement
        // keeps the native UI in control until 2.5 seconds after the last event.
        if connected {
            if let Some(lab) = lab.as_mut() {
                if let Some((index, pre)) = lab.due_pre(Instant::now(), osd_session_start.elapsed()) {
                    osd_log(&format!("LAB_PRE_DUE step_index={index} pre={pre:?}"));
                    lab_run_pre(&dev, pre, lab_orig_home, &osd_log);
                    match pre {
                        osd_lab::LabPre::SonarOn => lab_sonar_changed = true,
                        osd_lab::LabPre::SonarOff => lab_sonar_changed = false,
                        osd_lab::LabPre::HomeSimple => lab_home_changed = true,
                        osd_lab::LabPre::HomeRestore => lab_home_changed = false,
                    }
                }
            }
            let read_start = Instant::now();
            let events = dev.get_events().unwrap_or_else(|err| {
                osd_log(&format!("HID_READ_ERROR {err}"));
                connected = false;
                event_sender.send(DrawEvent::DeviceDisconnected).unwrap();
                vec![]
            });
            if osd_trace_file.is_some() && read_start.elapsed() > Duration::from_millis(100) {
                osd_log(&format!("HID_READ_SLOW duration_ms={}", read_start.elapsed().as_millis()));
            }
            for event in events {
                let event_time = Instant::now();
                let mut arm = false;
                match &event {
                    DeviceEvent::VolumeDial { volume } => {
                        let since_last = last_volume_dial_report
                            .map(|(_, t)| event_time.saturating_duration_since(t).as_millis());
                        arm = if osd_legacy {
                            last_volume_dial_report = Some((*volume, event_time));
                            true
                        } else {
                            is_new_volume_dial_action(&mut last_volume_dial_report, *volume, event_time)
                        };
                        osd_last_dial = Some(event_time);
                        osd_log(&format!("DIAL level={volume} accepted={arm} since_previous_ms={since_last:?} last_seq={osd_seq}"));
                        if let Some(lab) = lab.as_mut() {
                            let outcome = lab.on_dial(event_time);
                            if outcome.new_group {
                                lab_hold = outcome.step.hold_override().unwrap_or(osd_hold);
                                lab_last_ui_cmd = None;
                                osd_log(&format!("LAB_GROUP group={} step_index={} step={} hold_ms={}",
                                    outcome.group, outcome.step_index, outcome.step.label(), lab_hold.as_millis()));
                            }
                            let rate_ok = lab_last_ui_cmd
                                .is_none_or(|t| event_time.saturating_duration_since(t) >= Duration::from_millis(300));
                            if (outcome.new_group && outcome.step.ui_at_group_start())
                                || (outcome.step.ui_at_every_dial() && rate_ok)
                            {
                                let result = dev.return_to_ui().map_err(|e| e.to_string());
                                lab_last_ui_cmd = Some(event_time);
                                osd_log(&format!("LAB_UI_CMD when=dial step={} result={result:?}", outcome.step.label()));
                                if outcome.step.ui_at_every_dial() {
                                    spin_sleep::sleep(Duration::from_millis(60));
                                }
                                // Make the next loop iteration send a frame even if nothing changed.
                                prev_screen = Bitmap::new(0, 0, false);
                            }
                        }
                    }
                    DeviceEvent::UxSettings {
                        dim_timer,
                        oled_brightness,
                        idle_brightness,
                        home_screen_type,
                        home_screen_mic_mute,
                        sonar_present,
                    } => {
                        osd_log(&format!("UX_SETTINGS dim_timer={dim_timer} oled_brightness={oled_brightness} \
idle_brightness={idle_brightness} home_screen_type={home_screen_type} mic_mute={home_screen_mic_mute} \
sonar_present={sonar_present}"));
                        if lab.is_some() {
                            lab_orig_home.get_or_insert(*home_screen_type);
                        }
                        continue;
                    }
                    DeviceEvent::Unknown { raw } => {
                        if osd_trace_file.is_some() && raw_unknown_logged < 300 {
                            raw_unknown_logged += 1;
                            osd_log(&format!("RAW_UNKNOWN {raw:02x?}"));
                        }
                        continue;
                    }
                    DeviceEvent::Volume { volume } if osd_legacy => {
                        // 2026-10-18 working behavior: old parser combined unsolicited and
                        // queried volume replies into the same Volume event.
                        arm = true;
                        osd_log(&format!("VOLUME_QUERY level={volume} accepted=true [legacy]"));
                    }
                    DeviceEvent::Volume { volume } => {
                        osd_log(&format!("VOLUME_QUERY level={volume} accepted=false"));
                    }
                    _ => {}
                }
                if arm {
                    last_accepted_volume_event = Some(event_time);
                    volume_osd_started.get_or_insert(event_time);
                    volume_osd_until = Some(event_time + lab_hold);
                    osd_trace_remaining_frames = 3;
                    osd_log(&format!("HOLD_ARM duration_ms={} until_ms={}",
                        lab_hold.as_millis(),
                        event_time.duration_since(osd_session_start).as_millis() + lab_hold.as_millis()));
                }
                event_sender.send(DrawEvent::DeviceEvent(event)).unwrap();
            }
        }

        let volume_osd_active = volume_osd_until.is_some_and(|until| Instant::now() < until);
        if !volume_osd_active {
            if let Some(suppression_started) = volume_osd_started.take() {
                osd_log(&format!("HOLD_END elapsed_ms={}",
                    Instant::now().saturating_duration_since(suppression_started).as_millis()));
                if let Some(step) = lab.as_ref().and_then(|l| l.current_step()) {
                    if step.ui_at_hold_end() {
                        let result = dev.return_to_ui().map_err(|e| e.to_string());
                        osd_log(&format!("LAB_UI_CMD when=hold-end step={} result={result:?}", step.label()));
                        spin_sleep::sleep(Duration::from_millis(40));
                        prev_screen = Bitmap::new(0, 0, false);
                    }
                }
                // Shift animation deadlines by the exact time yielded to the firmware. Without
                // this, GIF playback would rush through the frames that elapsed during the OSD.
                let paused_for = Instant::now().saturating_duration_since(suppression_started);
                let mut layer_states = layers.lock();
                for state in layer_states.values_mut() {
                    if matches!(&state.layer, DrawLayer::Animation { .. }) {
                        state.anim.next_update = state.anim.next_update + paused_for;
                    }
                }
            }
            volume_osd_until = None;
        }

        // Render only when the device is ours to control. During the hardware volume
        // OSD the firmware owns the display, but we keep polling events and keep the
        // current GxOled layers intact so they resume automatically afterwards.
        if connected && playing && !volume_osd_active {
            // Handle OLED shifts
            let (shift_x, shift_y) = match shift_mode {
                ShiftMode::Off => (0, 0),
                ShiftMode::Low => {
                    if time.duration_since(last_shift) >= OLED_SHIFT_PERIOD {
                        shift_idx_low = (shift_idx_low + 1) % shift_pos_low.len();
                        last_shift = time;
                    }
                    shift_pos_low[shift_idx_low]
                }
                ShiftMode::High => {
                    if time.duration_since(last_shift) >= OLED_SHIFT_PERIOD {
                        shift_idx_high = (shift_idx_high + 1) % shift_pos_high.len();
                        last_shift = time;
                    }
                    shift_pos_high[shift_idx_high]
                }
            };

            // Update and blit each layer to the screen
            let mut screen = Bitmap::new(dev.width, dev.height, false);
            let mut layers = layers.lock();
            for (_, state) in layers.iter_mut() {
                match &state.layer {
                    DrawLayer::Image { bitmap, x, y } => screen.blit(bitmap, x + shift_x, y + shift_y, false),
                    DrawLayer::Clock {
                        style,
                        twelve_hour,
                        x_offset,
                        y_offset,
                        media_visible,
                        turkish,
                    } => {
                        let now = Local::now();
                        let texter = texter.lock();
                        let bitmap = render_clock_bitmap(
                            &texter,
                            *style,
                            *twelve_hour,
                            *media_visible,
                            *turkish,
                            now,
                        );
                        let base_x = if bitmap.w >= dev.width {
                            0
                        } else {
                            (dev.width as isize - bitmap.w as isize) / 2
                        };
                        let x = base_x + *x_offset;
                        let y = match *style {
                            ClockStyle::AnalogRound | ClockStyle::AnalogSquare if *media_visible => 4,
                            ClockStyle::AnalogRound | ClockStyle::AnalogSquare => {
                                (dev.height as isize - bitmap.h as isize) / 2
                            }
                            ClockStyle::DigitalOriginal | ClockStyle::DigitalBold if *media_visible => 4,
                            ClockStyle::DigitalOriginal => {
                                (dev.height as isize - texter.line_height() as isize) / 2
                            }
                            ClockStyle::CasioF91W => 0,
                            ClockStyle::DigitalBold => {
                                (dev.height as isize - texter.line_height_at_size(texter.size() * 1.48) as isize) / 2
                            }
                        } + *y_offset;
                        screen.blit(&bitmap, x + shift_x, y + shift_y, false);
                    }
                    DrawLayer::Animation {
                        frames,
                        x,
                        y,
                        follow_fps,
                    } => {
                        if !frames.is_empty() {
                            if *follow_fps {
                                let frame = &frames[state.anim.ticks % frames.len()];
                                screen.blit(&frame.bitmap, x + shift_x, y + shift_y, false);
                                state.anim.ticks = (state.anim.ticks + 1) % frames.len();
                            } else {
                                update_animation_timing(&mut state.anim, frames, time);
                                let frame = &frames[state.anim.ticks % frames.len()];
                                screen.blit(&frame.bitmap, x + shift_x, y + shift_y, false);
                            }
                        }
                    }
                    DrawLayer::Scroll { bitmap, y } => {
                        const MARGIN: isize = 30;
                        let scroll_w = bitmap.w as isize + MARGIN;
                        let dupes = 1 + dev.width / scroll_w as usize;
                        for i in 0..=dupes {
                            screen.blit(
                                bitmap,
                                state.scroll.x + i as isize * scroll_w + shift_x,
                                *y + shift_y,
                                false,
                            );
                        }
                        state.scroll.x -= 1;
                        if state.scroll.x <= -scroll_w {
                            state.scroll.x += scroll_w;
                        }
                    }
                }
            }

            drop(layers);

            // Draw update
            let frame_time = Instant::now();
            // Diagnostic pulse: 10 Hz for a bounded seven-second period after
            // physical knob movement; outside that period, keep 1 Hz heartbeat.
            let recent_dial_pulse = osd_pulse && last_accepted_volume_event.is_some_and(|last| {
                frame_time.saturating_duration_since(last) < Duration::from_secs(7)
            });
            let heartbeat = if recent_dial_pulse {
                Duration::from_millis(100)
            } else {
                Duration::from_secs(1)
            };
            let force_redraw = frame_time.duration_since(last_frame_time) >= heartbeat;
            if screen != prev_screen || force_redraw {
                last_frame_time = frame_time;
                let draw_start = Instant::now();
                let stamped;
                let to_send: &Bitmap = if osd_stamp {
                    osd_seq = osd_seq.wrapping_add(1);
                    let mut copy = screen.crop(0, 0, screen.w, screen.h);
                    // Lab mode: leading digit = experiment step, remaining three = frame counter.
                    let stamp_value = match lab.as_ref() {
                        Some(l) => l.stamp_digit(frame_time) * 1000 + osd_seq % 1000,
                        None => osd_seq,
                    };
                    stamp_sequence(&mut copy, stamp_value);
                    stamped = copy;
                    &stamped
                } else {
                    &screen
                };
                if let Err(err) = dev.draw(to_send, 0, 0) {
                    osd_log(&format!("FRAME_ERROR seq={osd_seq} elapsed_ms={} error={err}",
                        draw_start.elapsed().as_millis()));
                    if connected {
                        connected = false;
                        event_sender.send(DrawEvent::DeviceDisconnected).unwrap();
                    }
                } else {
                    if osd_trace_remaining_frames > 0 {
                        osd_log(&format!("FRAME_OK write_ms={} after_hold=true pulse={}",
                            draw_start.elapsed().as_millis(), recent_dial_pulse));
                        osd_trace_remaining_frames -= 1;
                    }
                    // Stamp mode: one line per sent frame for ten seconds after the last dial
                    // report, so the first counter value visible on camera maps to a send time.
                    if osd_stamp && osd_trace_file.is_some() {
                        if let Some(dial) = osd_last_dial {
                            let since_dial = frame_time.saturating_duration_since(dial);
                            if since_dial < Duration::from_secs(10) {
                                let step = lab.as_ref().and_then(|l| l.current_step()).map_or("-", |s| s.label());
                                osd_log(&format!("FRAME seq={osd_seq} since_dial_ms={} write_ms={} pulse={} step={step}",
                                    since_dial.as_millis(), draw_start.elapsed().as_millis(), recent_dial_pulse));
                            }
                        }
                    }
                    prev_screen = screen;
                }
            }
        }


        // Stop
        if stop_after_frame {
            break;
        }

        // Delay as long as needed based on how long frame rendering took (which will mostly depend on USB speed)
        let frame_duration = Instant::now().duration_since(time);
        // println!("frame: {:?}, {:?}", frame_duration, frame_delay);
        spin_sleep::sleep(frame_delay.saturating_sub(frame_duration));
    }
    if lab.is_some() {
        // Leave the base station the way it was found, even if the cycle was cut short.
        if lab_sonar_changed {
            lab_run_pre(&dev, osd_lab::LabPre::SonarOff, lab_orig_home, &osd_log);
        }
        if lab_home_changed {
            lab_run_pre(&dev, osd_lab::LabPre::HomeRestore, lab_orig_home, &osd_log);
        }
        osd_log("LAB_END");
    }
    dev
}

/// Device side of an [`osd_lab::LabPre`]. Commands go to both HID collections because it is not
/// documented which one the firmware listens on for settings; each outcome is logged, and a
/// follow-up `0x80` query records what the device reports afterwards.
fn lab_run_pre(dev: &Device, pre: osd_lab::LabPre, orig_home: Option<u8>, log: &dyn Fn(&str)) {
    use osd_lab::LabPre;
    match pre {
        LabPre::SonarOn | LabPre::SonarOff => {
            let on = pre == LabPre::SonarOn;
            for (name, oled) in [("info", false), ("oled", true)] {
                let result = dev.set_sonar_present_on(on, oled).map_err(|e| e.to_string());
                log(&format!("LAB_PRE sonar_present={on} collection={name} result={result:?}"));
            }
        }
        LabPre::HomeSimple | LabPre::HomeRestore => {
            let target = if pre == LabPre::HomeSimple { Some(1) } else { orig_home };
            match target {
                Some(value) => {
                    for (name, oled) in [("info", false), ("oled", true)] {
                        let result = dev.set_home_screen_type_on(value, oled).map_err(|e| e.to_string());
                        log(&format!("LAB_PRE home_screen_type={value} collection={name} result={result:?}"));
                    }
                }
                None => log("LAB_PRE skipped: the device never reported its original home screen type"),
            }
        }
    }
    spin_sleep::sleep(Duration::from_millis(150));
    let result = dev.query_ux_settings().map_err(|e| e.to_string());
    log(&format!("LAB_QUERY_UX at=after-{pre:?} result={result:?}"));
}

fn thicken_bitmap(bitmap: &Bitmap) -> Bitmap {
    let mut result = Bitmap::new(bitmap.w.saturating_add(1), bitmap.h, false);
    result.blit(bitmap, 0, 0, false);
    result.blit(bitmap, 1, 0, false);
    result
}

fn clock_digit_rows(digit: u8) -> [u8; 5] {
    match digit {
        0 => [0b111, 0b101, 0b101, 0b101, 0b111],
        1 => [0b010, 0b110, 0b010, 0b010, 0b111],
        2 => [0b111, 0b001, 0b111, 0b100, 0b111],
        3 => [0b111, 0b001, 0b111, 0b001, 0b111],
        4 => [0b101, 0b101, 0b111, 0b001, 0b001],
        5 => [0b111, 0b100, 0b111, 0b001, 0b111],
        6 => [0b111, 0b100, 0b111, 0b101, 0b111],
        7 => [0b111, 0b001, 0b010, 0b010, 0b010],
        8 => [0b111, 0b101, 0b111, 0b101, 0b111],
        9 => [0b111, 0b101, 0b111, 0b001, 0b111],
        _ => [0; 5],
    }
}

/// Diagnostic frame counter (`GXOLED_OSD_STAMP=1`): four decimal digits (`seq % 10000`), each
/// glyph the 3x5 clock font scaled 2x, on a cleared plate in the bottom-right corner.
fn stamp_sequence(bitmap: &mut Bitmap, seq: u32) {
    const SCALE: isize = 2;
    const GLYPH_W: isize = 3 * SCALE;
    const GLYPH_H: isize = 5 * SCALE;
    const GAP: isize = 2;
    const PAD: isize = 2;
    const DIGITS: isize = 4;
    let plate_w = DIGITS * GLYPH_W + (DIGITS - 1) * GAP + 2 * PAD;
    let plate_h = GLYPH_H + 2 * PAD;
    let origin_x = bitmap.w as isize - plate_w;
    let origin_y = bitmap.h as isize - plate_h;
    if origin_x < 0 || origin_y < 0 {
        return;
    }
    for y in 0..plate_h {
        for x in 0..plate_w {
            bitmap
                .data
                .set((origin_y + y) as usize * bitmap.w + (origin_x + x) as usize, false);
        }
    }
    let digits = [(seq / 1000) % 10, (seq / 100) % 10, (seq / 10) % 10, seq % 10];
    for (index, digit) in digits.iter().enumerate() {
        let rows = clock_digit_rows(*digit as u8);
        let glyph_x = origin_x + PAD + index as isize * (GLYPH_W + GAP);
        let glyph_y = origin_y + PAD;
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..3usize {
                if *bits & (0b100u8 >> col) != 0 {
                    for dy in 0..SCALE {
                        for dx in 0..SCALE {
                            put_pixel(
                                bitmap,
                                glyph_x + col as isize * SCALE + dx,
                                glyph_y + row as isize * SCALE + dy,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn put_pixel(bitmap: &mut Bitmap, x: isize, y: isize) {
    if x >= 0 && y >= 0 && x < bitmap.w as isize && y < bitmap.h as isize {
        bitmap.data.set(y as usize * bitmap.w + x as usize, true);
    }
}

fn draw_line(bitmap: &mut Bitmap, mut x0: isize, mut y0: isize, x1: isize, y1: isize) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        put_pixel(bitmap, x0, y0);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn draw_digit(bitmap: &mut Bitmap, x: isize, y: isize, digit: u8) {
    for (row, bits) in clock_digit_rows(digit).iter().enumerate() {
        for col in 0..3usize {
            if *bits & (1u8 << (2 - col)) != 0 {
                put_pixel(bitmap, x + col as isize, y + row as isize);
            }
        }
    }
}

fn draw_two_digit_label(bitmap: &mut Bitmap, x: isize, y: isize, value: u8) {
    draw_digit(bitmap, x, y, value / 10);
    draw_digit(bitmap, x + 4, y, value % 10);
}

fn analog_clock_bitmap(style: ClockStyle, now: chrono::DateTime<Local>, size: usize) -> Bitmap {
    let mut bitmap = Bitmap::new(size, size, false);
    let center = (size as isize - 1) / 2;
    let radius = center - 2;
    if style == ClockStyle::AnalogRound {
        for degree in 0..360 {
            let angle = (degree as f64).to_radians();
            put_pixel(
                &mut bitmap,
                center + (radius as f64 * angle.cos()).round() as isize,
                center + (radius as f64 * angle.sin()).round() as isize,
            );
        }
    } else {
        let low = 2;
        let high = size as isize - 3;
        draw_line(&mut bitmap, low, low, high, low);
        draw_line(&mut bitmap, high, low, high, high);
        draw_line(&mut bitmap, high, high, low, high);
        draw_line(&mut bitmap, low, high, low, low);
    }

    // Twelve dial marks; the cardinal positions are labeled numerically instead.
    for hour in 0..12 {
        if [0, 3, 6, 9].contains(&hour) {
            continue;
        }
        let angle = (hour as f64 * std::f64::consts::TAU / 12.0) - std::f64::consts::FRAC_PI_2;
        let ux = angle.cos();
        let uy = angle.sin();
        let outer_factor = if style == ClockStyle::AnalogSquare {
            (radius as f64) / ux.abs().max(uy.abs()).max(0.0001)
        } else {
            radius as f64
        };
        let outer_x = center + (ux * outer_factor).round() as isize;
        let outer_y = center + (uy * outer_factor).round() as isize;
        let inner_x = center + (ux * (outer_factor - 3.0).max(0.0)).round() as isize;
        let inner_y = center + (uy * (outer_factor - 3.0).max(0.0)).round() as isize;
        draw_line(&mut bitmap, inner_x, inner_y, outer_x, outer_y);
    }

    let hour = (now.hour() % 12) as f64 + now.minute() as f64 / 60.0;
    let minute = now.minute() as f64 + now.second() as f64 / 60.0;
    let second = now.second() as f64;
    let hand_angle = |fraction: f64| fraction * std::f64::consts::TAU / 60.0 - std::f64::consts::FRAC_PI_2;
    let hour_angle = hour * std::f64::consts::TAU / 12.0 - std::f64::consts::FRAC_PI_2;
    let minute_angle = hand_angle(minute);
    let second_angle = hand_angle(second);
    let hand = |angle: f64, length: f64| {
        (
            center + (angle.cos() * length).round() as isize,
            center + (angle.sin() * length).round() as isize,
        )
    };
    let (hx, hy) = hand(hour_angle, radius as f64 * 0.48);
    let (mx, my) = hand(minute_angle, radius as f64 * 0.72);
    let (sx, sy) = hand(second_angle, radius as f64 * 0.88);
    draw_line(&mut bitmap, center, center, hx, hy);
    draw_line(&mut bitmap, center, center, mx, my);
    draw_line(&mut bitmap, center, center, sx, sy);
    for oy in -1..=1 {
        for ox in -1..=1 {
            put_pixel(&mut bitmap, center + ox, center + oy);
        }
    }

    // Compact 3x5 numerals keep 12, 3, 6 and 9 legible on the 128x64 OLED.
    let label_r = radius;
    draw_two_digit_label(&mut bitmap, center - 3, center - label_r + 2, 12);
    draw_digit(&mut bitmap, center + label_r - 5, center - 2, 3);
    draw_digit(&mut bitmap, center - 1, center + label_r - 7, 6);
    draw_digit(&mut bitmap, center - label_r + 2, center - 2, 9);
    bitmap
}

fn render_clock_bitmap(
    texter: &TextRenderer,
    style: ClockStyle,
    twelve_hour: bool,
    media_visible: bool,
    turkish: bool,
    now: chrono::DateTime<Local>,
) -> Bitmap {
    let digital_time = if twelve_hour {
        let suffix = if turkish {if now.hour()<12 {"ÖÖ"} else {"ÖS"}}
            else {if now.hour()<12 {"AM"} else {"PM"}};
        format!("{:02}:{:02}:{:02} {suffix}", (now.hour()+11)%12+1, now.minute(), now.second())
    } else {now.format("%H:%M:%S").to_string()};
    match style {
        ClockStyle::DigitalOriginal => {
            let text = digital_time.clone();
            let lines=if media_visible { texter.render_lines_at_size(&text,texter.size()*0.80) }
                else {texter.render_lines(&text)};
            lines.into_iter().next().unwrap_or_else(|| Bitmap::new(0, 0, false))
        }
        ClockStyle::DigitalBold => {
            let text = digital_time;
            let size = texter.size() * if media_visible { 0.94 } else { 1.48 };
            let lines = texter.render_lines_at_size(&text, size);
            lines.first().map(thicken_bitmap).unwrap_or_else(|| Bitmap::new(0, 0, false))
        }
        ClockStyle::AnalogRound => analog_clock_bitmap(style, now, if media_visible { 32 } else { 54 }),
        ClockStyle::AnalogSquare => analog_clock_bitmap(style, now, if media_visible { 32 } else { 54 }),
        ClockStyle::CasioF91W => casio::render(now, twelve_hour, turkish),
    }
}

type LayerMap = BTreeMap<LayerId, DrawLayerState>;
pub struct DrawDevice {
    width: usize,
    height: usize,
    layers: Arc<Mutex<LayerMap>>,
    layer_counter: usize,
    thread: Option<std::thread::JoinHandle<Device>>,
    cmd_sender: Sender<DrawCommand>,
    event_receiver: Receiver<DrawEvent>,
    pub texter: Arc<Mutex<TextRenderer>>,
}
impl DrawDevice {
    pub fn new(dev: Device, fps: usize) -> DrawDevice {
        let layers: Arc<Mutex<LayerMap>> = Default::default();
        let (cmd_sender, cmd_recver) = channel::<DrawCommand>();
        let (event_sender, event_receiver) = channel::<DrawEvent>();
        let c_layers = layers.clone();
        let texter = Arc::new(Mutex::new(TextRenderer::new_pixel_operator()));
        let render_texter = texter.clone();
        let (width, height) = (dev.width, dev.height);
        let thread = Some(std::thread::spawn(move || {
            run_draw_device_thread(dev, c_layers, cmd_recver, event_sender, render_texter, fps)
        }));
        DrawDevice {
            width,
            height,
            layers,
            layer_counter: 0,
            thread,
            cmd_sender,
            event_receiver,
            texter,
        }
    }
    fn destroy(&mut self) -> Option<Device> {
        if let Some(thread) = self.thread.take() {
            let _ = self.cmd_sender.send(DrawCommand::Stop);
            thread.join().ok()
        } else {
            None
        }
    }
    pub fn stop(mut self) -> Device {
        self.destroy().expect("render thread panicked")
    }
    pub fn try_event(&mut self) -> Option<DrawEvent> {
        self.event_receiver.try_recv().ok()
    }
    pub fn poll_event(&mut self) -> anyhow::Result<DrawEvent> {
        Ok(self.event_receiver.recv()?)
    }
    pub fn center_bitmap(&self, bitmap: &Bitmap) -> (isize, isize) {
        (
            (self.width as isize - bitmap.w as isize) / 2,
            (self.height as isize - bitmap.h as isize) / 2,
        )
    }
    fn add_layer_locked(&mut self, layers: &mut MutexGuard<'_, LayerMap>, layer: DrawLayer) -> LayerId {
        self.layer_counter += 1;
        let id = LayerId(self.layer_counter);
        _ = layers.insert(
            id,
            DrawLayerState {
                layer,
                anim: AnimState {
                    ticks: 0,
                    next_update: Instant::now(),
                    timing_initialized: false,
                },
                scroll: ScrollState { x: 0 },
            },
        );
        id
    }
    pub fn add_layer(&mut self, layer: DrawLayer) -> LayerId {
        self.add_layer_locked(&mut self.layers.clone().lock(), layer)
    }
    pub fn remove_layer(&mut self, id: LayerId) {
        self.layers.lock().remove(&id);
    }
    /// Replace an existing layer atomically without pausing the renderer. If the ID no
    /// longer exists (for example, after an idle clear), return the new layer to the caller.
    pub fn replace_layer(&mut self, id: LayerId, layer: DrawLayer) -> Result<(), DrawLayer> {
        let mut layers = self.layers.lock();
        if let Some(state) = layers.get_mut(&id) {
            state.layer = layer;
            state.anim.ticks = 0;
            state.anim.next_update = Instant::now();
            state.anim.timing_initialized = false;
            state.scroll.x = 0;
            Ok(())
        } else {
            Err(layer)
        }
    }
    pub fn remove_layers(&mut self, ids: &[LayerId]) {
        let mut layers = self.layers.lock();
        for id in ids {
            layers.remove(id);
        }
    }
    pub fn clear_layers(&mut self) {
        self.layers.lock().clear();
    }
    pub fn font_line_height(&self) -> usize {
        self.texter.lock().line_height()
    }
    pub fn font_line_height_at_size(&self, size: f32) -> usize {
        self.texter.lock().line_height_at_size(size)
    }
    pub fn text_size(&self) -> f32 {
        self.texter.lock().size()
    }
    pub fn render_lines(&self, text: &str) -> Vec<Bitmap> {
        self.texter.lock().render_lines(text)
    }
    pub fn render_lines_at_size(&self, text: &str, size: f32) -> Vec<Bitmap> {
        self.texter.lock().render_lines_at_size(text, size)
    }
    pub fn set_text_renderer(&mut self, texter: TextRenderer) {
        *self.texter.lock() = texter;
    }
    /// Return the OLED dimensions in pixels.
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }
    pub fn add_text(&mut self, text: &str, x: Option<isize>, y: Option<isize>) -> Vec<LayerId> {
        self.add_text_with_size(text, x, y, None)
    }

    pub fn add_text_at_size(&mut self, text: &str, x: Option<isize>, y: Option<isize>, size: f32) -> Vec<LayerId> {
        self.add_text_with_size(text, x, y, Some(size))
    }

    fn add_text_with_size(&mut self, text: &str, x: Option<isize>, y: Option<isize>, size: Option<f32>) -> Vec<LayerId> {
        let (bitmaps, line_height) = {
            let texter = self.texter.lock();
            let bitmaps = match size {
                Some(size) => texter.render_lines_at_size(text, size),
                None => texter.render_lines(text),
            };
            let line_height = size.map_or_else(|| texter.line_height(), |size| texter.line_height_at_size(size));
            (bitmaps, line_height)
        };
        let layers = self.layers.clone();
        let mut layers = layers.lock();
        let bitmaps: Vec<_> = bitmaps.into_iter().map(Arc::new).collect();
        let center_y: isize = (self.height as isize - (line_height * bitmaps.len()) as isize) / 2;
        bitmaps
            .into_iter()
            .enumerate()
            .map(|(i, bitmap)| {
                let y = y.unwrap_or(center_y) + (i * line_height) as isize;
                if bitmap.w >= self.width {
                    self.add_layer_locked(&mut layers, DrawLayer::Scroll { bitmap, y })
                } else {
                    let center = self.center_bitmap(&bitmap);
                    self.add_layer_locked(
                        &mut layers,
                        DrawLayer::Image { bitmap, x: x.unwrap_or(center.0), y },
                    )
                }
            })
            .collect()
    }
    pub fn set_shift_mode(&mut self, mode: ShiftMode) {
        self.cmd_sender.send(DrawCommand::SetShiftMode(mode)).unwrap();
    }
    /// Request a hardware brightness level from the render thread.
    /// The result is reported as `DrawEvent::BrightnessSet`.
    pub fn set_brightness(&mut self, value: u8) {
        self.cmd_sender.send(DrawCommand::SetBrightness(value)).unwrap();
    }
    pub fn play(&mut self) {
        self.cmd_sender.send(DrawCommand::Play).unwrap();
    }
    pub fn pause(&mut self) {
        self.cmd_sender.send(DrawCommand::Pause).unwrap();
    }
    pub fn probe(&mut self) {
        self.cmd_sender.send(DrawCommand::Probe).unwrap();
    }
}
impl Drop for DrawDevice {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(test)]
#[rustfmt::skip]
#[test]
fn shift_pos() {
    assert_eq!(square_shift_calc(1), &[
        // center
        (0, 0),
        // r=1
        (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1),
    ]);
    assert_eq!(square_shift_calc(2), &[
        // center
        (0, 0),
        // r=1
        (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1),
        // r=2
        (0, -2), (1, -2), (2, -2), (2, -1), (2, 0), (2, 1), (2, 2), (1, 2), (0, 2), (-1, 2), (-2, 2), (-2, 1), (-2, 0), (-2, -1), (-2, -2), (-1, -2),
        // r=1
        (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1),
    ]);
}

#[cfg(test)]
mod animation_timing_tests {
    use super::{AnimState, Frame, update_animation_timing};
    use gxoled_lib::Bitmap;
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    fn frame(delay: Duration) -> Frame {
        Frame {
            bitmap: Arc::new(Bitmap::new(1, 1, false)),
            delay: Some(delay),
        }
    }

    #[test]
    fn first_frame_keeps_its_delay_and_next_frame_uses_its_own_delay() {
        let frames = vec![
            frame(Duration::from_millis(100)),
            frame(Duration::from_millis(250)),
        ];
        let start = Instant::now();
        let mut state = AnimState {
            ticks: 0,
            next_update: start,
            timing_initialized: false,
        };

        update_animation_timing(&mut state, &frames, start);
        assert_eq!(state.ticks, 0, "the first frame must not be skipped immediately");
        assert_eq!(state.next_update, start + Duration::from_millis(100));

        let first_deadline = state.next_update;
        update_animation_timing(&mut state, &frames, first_deadline);
        assert_eq!(state.ticks, 1);
        assert_eq!(state.next_update, first_deadline + Duration::from_millis(250));
    }
}

/// Pixel-accurate 128x64 preview of the Casio-inspired clock.
pub fn casio_clock_preview(twelve_hour: bool, turkish: bool) -> Bitmap { casio::render(Local::now(), twelve_hour, turkish) }

#[cfg(test)]
mod volume_osd_dedup_tests {
    use super::{is_new_volume_dial_action, VOLUME_OSD_HOLD};
    use std::time::{Duration, Instant};

    #[test]
    fn duplicate_firmware_reports_do_not_extend_hold() {
        let start = Instant::now();
        let mut previous = None;
        assert!(is_new_volume_dial_action(&mut previous, 20, start));
        for milliseconds in [500, 1600, 2400] {
            assert!(!is_new_volume_dial_action(&mut previous, 20, start + Duration::from_millis(milliseconds)));
        }
        assert_eq!(VOLUME_OSD_HOLD, Duration::from_millis(2500));
    }

    #[test]
    fn volume_changes_rearm_timer() {
        let start = Instant::now();
        let mut previous = None;
        assert!(is_new_volume_dial_action(&mut previous, 20, start));
        assert!(is_new_volume_dial_action(&mut previous, 21, start + Duration::from_millis(80)));
        assert!(is_new_volume_dial_action(&mut previous, 22, start + Duration::from_millis(150)));
    }

    #[test]
    fn same_level_after_silence_is_new_dial_action() {
        let start = Instant::now();
        let mut previous = None;
        assert!(is_new_volume_dial_action(&mut previous, 20, start));
        assert!(!is_new_volume_dial_action(&mut previous, 20, start + Duration::from_millis(200)));
        assert!(is_new_volume_dial_action(&mut previous, 20, start + Duration::from_millis(2800)));
    }
}

#[cfg(test)]
mod osd_stamp_tests {
    use super::{Bitmap, stamp_sequence};

    const PLATE_W: usize = 34;
    const PLATE_H: usize = 14;

    fn rendered(seq: u32) -> Bitmap {
        let mut bitmap = Bitmap::new(128, 64, false);
        stamp_sequence(&mut bitmap, seq);
        bitmap
    }

    #[test]
    fn stamp_touches_only_the_bottom_right_plate() {
        let mut bitmap = Bitmap::new(128, 64, true);
        stamp_sequence(&mut bitmap, 1234);
        let mut plate_lit = 0;
        let mut plate_cleared = 0;
        for y in 0..64 {
            for x in 0..128 {
                let inside = x >= 128 - PLATE_W && y >= 64 - PLATE_H;
                let value = bitmap.data[y * 128 + x];
                if inside {
                    if value { plate_lit += 1 } else { plate_cleared += 1 }
                } else {
                    assert!(value, "pixel outside the plate changed at ({x},{y})");
                }
            }
        }
        assert!(plate_lit > 0 && plate_cleared > 0);
    }

    #[test]
    fn different_counters_differ_and_wrap_at_ten_thousand() {
        assert_ne!(rendered(1), rendered(2));
        assert_ne!(rendered(1000), rendered(2000));
        assert_eq!(rendered(0), rendered(10_000));
        assert_eq!(rendered(1234), rendered(11_234));
    }

    #[test]
    fn stamping_a_copy_leaves_the_original_screen_unchanged() {
        let screen = Bitmap::new(128, 64, false);
        let mut copy = screen.crop(0, 0, screen.w, screen.h);
        stamp_sequence(&mut copy, 7);
        assert_ne!(screen, copy);
        assert_eq!(screen, Bitmap::new(128, 64, false));
    }

    #[test]
    fn tiny_bitmaps_are_left_alone() {
        let mut bitmap = Bitmap::new(20, 10, false);
        stamp_sequence(&mut bitmap, 8888);
        assert_eq!(bitmap, Bitmap::new(20, 10, false));
    }
}
