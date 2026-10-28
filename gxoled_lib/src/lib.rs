pub mod bitmap;
use anyhow::bail;
pub use bitmap::Bitmap;
use hidapi::{HidApi, HidDevice, MAX_REPORT_DESCRIPTOR_SIZE};
use std::{cmp::min, time::Duration};

// NOTE: these work for Arctis Nova Pro but might not for different products!
const SCREEN_REPORT_SPLIT_SZ: usize = 64;
const SCREEN_REPORT_SIZE: usize = 1024;
const SCREEN_WIDTH: usize = 128;
const SCREEN_HEIGHT: usize = 64;

type DrawReport = [u8; SCREEN_REPORT_SIZE];

struct ReportDrawable<'a> {
    bitmap: &'a Bitmap,
    w: usize,
    h: usize,
    dst_x: usize,
    dst_y: usize,
    src_x: usize,
    src_y: usize,
}

#[derive(Debug)]
pub enum DeviceEvent {
    /// Current volume level returned by a device information query.
    Volume {
        volume: u8,
    },
    /// A physical volume-dial change, which triggers the base station's native OLED OSD.
    VolumeDial {
        volume: u8,
    },
    Battery {
        headset: u8,
        charging: u8,
    },
    HeadsetConnection {
        wireless: bool,
        bluetooth: bool,
        bluetooth_on: bool,
    },
    /// Reply to the `0x80` "UX settings" query. Field layout taken from the device spec as
    /// documented by loteran/Arctis-Sound-Manager (`ux_settings`): every field is one raw byte.
    UxSettings {
        dim_timer: u8,
        oled_brightness: u8,
        idle_brightness: u8,
        /// 0 = detailed home screen, 1 = simple.
        home_screen_type: u8,
        home_screen_mic_mute: u8,
        /// Whether the base station believes SteelSeries Sonar is present (`0x8d`).
        sonar_present: u8,
    },
    /// A report that the application does not interpret. Only produced so diagnostics can
    /// log it; the draw thread never forwards it to the UI.
    Unknown {
        raw: [u8; 16],
    },
}

// Parse one 64-byte HID report using the report ID exposed by this device's info
// collection. Keep this function pure so the physical-dial/queried-volume distinction can
// be regression-tested without requiring a connected headset base station.
fn parse_event_report(buf: &[u8; 64], info_report_id: u8) -> Vec<DeviceEvent> {
    match (buf[0], buf[1]) {
        // Unsolicited physical volume-dial event. This is what activates the base station's
        // native volume overlay; routine status responses below must not be treated as this.
        (0x07, 0x25) => vec![DeviceEvent::VolumeDial {
            volume: 0x38u8.saturating_sub(buf[2]),
        }],

        // Unsolicited headset connection state.
        (0x07, 0xb5) => vec![DeviceEvent::HeadsetConnection {
            wireless: buf[4] == 8,
            bluetooth: buf[3] == 1,
            bluetooth_on: buf[2] == 4,
        }],

        // Unsolicited battery state.
        (0x07, 0xb7) => vec![DeviceEvent::Battery {
            headset: buf[2],
            charging: buf[3],
        }],

        // Version info; not needed by the application.
        (id, 0x10) if id == info_report_id => vec![],

        // Routine information/query response. This is intentionally `Volume`, not
        // `VolumeDial`, so it does not keep the native OSD suppression timer alive.
        (id, 0x20) if id == info_report_id => vec![DeviceEvent::Volume {
            volume: 0x38u8.saturating_sub(buf[3]),
        }],

        // UX settings reply (dim timer, brightness, home screen type, Sonar flag).
        (id, 0x80) if id == info_report_id => vec![DeviceEvent::UxSettings {
            dim_timer: buf[2],
            oled_brightness: buf[3],
            idle_brightness: buf[4],
            home_screen_type: buf[5],
            home_screen_mic_mute: buf[6],
            sonar_present: buf[7],
        }],

        // General device state query response.
        (id, 0xb0) if id == info_report_id => vec![
            DeviceEvent::HeadsetConnection {
                wireless: buf[15] == 8,
                bluetooth: buf[5] == 1,
                bluetooth_on: buf[4] == 4,
            },
            DeviceEvent::Battery {
                headset: buf[6],
                charging: buf[7],
            },
        ],

        // Query variant of the unsolicited battery event.
        (id, 0xb7) if id == info_report_id => vec![DeviceEvent::Battery {
            headset: buf[2],
            charging: buf[3],
        }],

        _ => {
            let mut raw = [0u8; 16];
            raw.copy_from_slice(&buf[..16]);
            vec![DeviceEvent::Unknown { raw }]
        }
    }
}

enum DeviceMerge {
    Merged(HidDevice),
    Separate { oled: HidDevice, info: HidDevice },
}
impl DeviceMerge {
    fn oled(&self) -> &HidDevice {
        match self {
            DeviceMerge::Merged(dev) => dev,
            DeviceMerge::Separate { oled, .. } => oled,
        }
    }
    fn info(&self) -> &HidDevice {
        match self {
            DeviceMerge::Merged(dev) => dev,
            DeviceMerge::Separate { info, .. } => info,
        }
    }
}

pub struct Device {
    dev: DeviceMerge,
    pub width: usize,
    pub height: usize,
    oled_report_id: u8,
    info_report_id: u8,
    product_id: u16,
}
impl Device {
    /// Connect to a SteelSeries GG device.
    pub fn connect() -> anyhow::Result<Device> {
        let api = HidApi::new()?;

        // Find all connected devices matching given Vendor/Product IDs and interface
        let device_infos: Vec<_> = api
            .device_list()
            .filter(|d| {
                d.vendor_id() == 0x1038 // SteelSeries
        && [
            0x12cb, // Arctis Nova Pro Wired
            0x12cd, // Arctis Nova Pro Wired (Xbox)
            0x12e0, // Arctis Nova Pro Wireless
            0x12e5, // Arctis Nova Pro Wireless (Xbox)
            0x225d, // Arctis Nova Pro Wireless (Xbox White)
            0x2244, // Arctis Nova Elite (base station, unofficial/untested - see issue #26)
        ].contains(&d.product_id())
            // Nova Elite base station exposes the OLED/info collections on interface 3, not 4
            // (interface 4 there is an unrelated consumer-control/media-key interface).
            && d.interface_number() == if d.product_id() == 0x2244 { 3 } else { 4 }
            })
            .collect();

        // We're expecting to find exactly two devices with different HID descriptors
        if device_infos.is_empty() {
            bail!("No matching devices connected");
        } else if device_infos.len() < 2 {
            bail!("Too few matching devices connected");
        } else if device_infos.len() > 2 {
            bail!("Too many matching devices connected");
        }

        // The HID report IDs differ by product; Nova Pro uses 6 for both the OLED and
        // info collections, but the Nova Elite base station declares report ID 1 for its
        // OLED collection (Col01) and report ID 7 for its info collection (Col02).
        let is_nova_elite = device_infos[0].product_id() == 0x2244;
        let oled_report_id: u8 = if is_nova_elite { 0x01 } else { 0x06 };
        let info_report_id: u8 = if is_nova_elite { 0x07 } else { 0x06 };

        // On Linux, both devices can get put under the same hidraw interface, meaning we use the same device for both
        let dev = if device_infos[0].path() == device_infos[1].path() {
            let Ok(dev) = device_infos[0].open_device(&api) else {
                bail!("Failed to connect to USB device");
            };
            DeviceMerge::Merged(dev)
        // On Windows (and maybe some Linux variants), they are separate interfaces and have to be opened separately
        } else {
            // Open both devices
            let Ok(mut devices) = device_infos
                .iter()
                .map(|info| anyhow::Ok(info.open_device(&api)?))
                .collect::<anyhow::Result<Vec<_>>>()
            else {
                bail!("Failed to connect to USB device");
            };

            // Get descriptors
            let Ok(mut device_reports) = devices
                .iter()
                .map(|dev| {
                    let mut buf = [0u8; MAX_REPORT_DESCRIPTOR_SIZE];
                    let sz = dev.get_report_descriptor(&mut buf)?;
                    if sz > buf.len() {
                        bail!("HID report descriptor exceeded the supplied buffer");
                    }
                    anyhow::Ok(Vec::from(&buf[..sz]))
                })
                .collect::<anyhow::Result<Vec<_>>>()
            else {
                bail!("Failed to get USB device HID reports");
            };

            // Identify and open the two devices by their descriptors
            let Some(oled_dev_idx) = device_reports.iter().position(|desc| desc.get(1) == Some(&0xc0)) else {
                bail!("No OLED device found");
            };
            _ = device_reports.swap_remove(oled_dev_idx);
            let oled_dev = devices.swap_remove(oled_dev_idx);
            let Some(info_dev_idx) = device_reports.iter().position(|desc| desc.get(1) == Some(&0x00)) else {
                bail!("No info device found");
            };
            _ = device_reports.swap_remove(info_dev_idx);
            let info_dev = devices.swap_remove(info_dev_idx);

            DeviceMerge::Separate {
                oled: oled_dev,
                info: info_dev,
            }
        };

        Ok(Device {
            dev,
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            oled_report_id,
            info_report_id,
            product_id: device_infos[0].product_id(),
        })
    }

    /// USB product ID of the connected base station (for diagnostics).
    pub fn product_id(&self) -> u16 {
        self.product_id
    }

    /// Dump the full device tree info for all SteelSeries devices to stdout for debug purposes
    pub fn dump_devices() {
        let Ok(api) = HidApi::new() else {
            eprintln!("Failed to initialize HID API.");
            return;
        };

        let device_infos: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == 0x1038) // SteelSeries
            .collect();
        if device_infos.is_empty() {
            println!("No devices.");
            return;
        }

        println!("-----");
        for info in device_infos {
            println!("product={}", info.product_string().unwrap_or("?"));
            println!("pid={:#04x}", info.product_id());
            println!("interface={}", info.interface_number());
            println!("path={}", info.path().to_string_lossy());
            println!("usage={}", info.usage());
            if let Ok(dev) = info.open_device(&api) {
                let mut buf = [0u8; MAX_REPORT_DESCRIPTOR_SIZE];
                if let Ok(sz) = dev.get_report_descriptor(&mut buf) {
                    let valid_len = sz.min(buf.len());
                    let preview_len = valid_len.min(16);
                    println!("report desc sz={sz}, first 16 bytes: {:02x?}", &buf[..preview_len]);
                } else {
                    println!("getting report descriptor failed");
                }
            } else {
                println!("opening device failed");
            }
            println!("-----");
        }
    }

    /// Reconnect to a device.
    pub fn reconnect(&mut self) -> anyhow::Result<()> {
        *self = Self::connect()?;
        Ok(())
    }

    // Creates a HID report for a `ReportDrawable`
    // The Bitmap must already be within the report limits (from `split_for_report`)
    fn create_report(&self, d: &ReportDrawable) -> DrawReport {
        let mut report: DrawReport = [0; SCREEN_REPORT_SIZE];
        report[0] = self.oled_report_id; // hid report id
        report[1] = 0x93; // command id
        report[2] = d.dst_x as u8;
        report[3] = d.dst_y as u8;
        // Pad height to multiple of 8 to align with device blocks.
        let padded_h = d.h.div_ceil(8) * 8;
        report[4] = d.w as u8;
        report[5] = padded_h as u8;
        for y in 0..d.h {
            for x in 0..d.w {
                // NOTE: report has columns rather than rows
                let ri = x * padded_h + y;
                let pi = (d.src_y + y) * d.bitmap.w + (d.src_x + x);
                report[(ri / 8) + 6] |= (d.bitmap.data[pi] as u8) << (ri % 8);
            }
        }
        report
    }

    // Splits up a `Bitmap` to be appropriately sized for being able to send over USB HID
    fn prepare_for_report<'a>(&self, bitmap: &'a Bitmap, x: isize, y: isize) -> Vec<ReportDrawable<'a>> {
        let mut w = bitmap.w;
        let mut h = bitmap.h;

        // Handle negative x/y by moving src_x/src_y
        let (x, src_x) = if x < 0 {
            w = w.saturating_sub((-x) as usize);
            (0, (-x) as usize)
        } else {
            (x as usize, 0)
        };
        let (y, src_y) = if y < 0 {
            h = h.saturating_sub((-y) as usize);
            (0, (-y) as usize)
        } else {
            (y as usize, 0)
        };

        // Crop size to screen
        let x = min(x, self.width);
        let y = min(y, self.height);
        if x + w >= self.width {
            w = self.width - x;
        }
        if y + h >= self.height {
            h = self.height - y;
        }

        // Split
        let mut vec = Vec::<ReportDrawable<'a>>::new();
        let splits = w.div_ceil(SCREEN_REPORT_SPLIT_SZ);
        for i in 0..splits {
            vec.push(ReportDrawable {
                bitmap,
                w: min(SCREEN_REPORT_SPLIT_SZ, w - i * SCREEN_REPORT_SPLIT_SZ),
                h,
                dst_x: x + (i * SCREEN_REPORT_SPLIT_SZ),
                dst_y: y,
                src_x: src_x + i * SCREEN_REPORT_SPLIT_SZ,
                src_y,
            });
        }
        vec
    }

    /// Draw a `Bitmap` at the given location.
    pub fn draw(&self, bitmap: &Bitmap, x: isize, y: isize) -> anyhow::Result<()> {
        let drawables = self.prepare_for_report(bitmap, x, y);
        for drawable in drawables {
            let report = self.create_report(&drawable);
            self.retry_report(&report)?;
        }
        Ok(())
    }

    fn retry_report(&self, data: &[u8]) -> anyhow::Result<()> {
        let mut i: u64 = 0;
        loop {
            match self.dev.oled().send_feature_report(data) {
                Ok(_) => return Ok(()),
                Err(err) => {
                    if i == 10 {
                        return Err(err.into());
                    }
                    i += 1;
                    spin_sleep::sleep(Duration::from_millis(i.pow(2)));
                }
            }
        }
    }

    /// Set screen brightness.
    pub fn set_brightness(&self, value: u8) -> anyhow::Result<()> {
        if value < 1 {
            bail!("brightness too low");
        } else if value > 0x0a {
            bail!("brightness too high");
        }
        let mut report = [0; 64];
        report[0] = self.oled_report_id; // hid report id
        report[1] = 0x85; // command id
        report[2] = value;
        self.dev.oled().write(&report)?;
        Ok(())
    }

    /// Probe device to fetch current state.
    /// Data is received via events.
    pub fn probe(&self) -> anyhow::Result<()> {
        let mut report = [0; 64];
        report[0] = self.info_report_id; // hid report id
        report[1] = 0xb0; // command id, get various data
        self.dev.info().write(&report)?;
        report[1] = 0x20; // command id, get volume info
        self.dev.info().write(&report)?;
        Ok(())
    }

    /// Return to SteelSeries UI.
    pub fn return_to_ui(&self) -> anyhow::Result<()> {
        let mut report = [0; 64];
        report[0] = self.oled_report_id; // hid report id
        report[1] = 0x95; // command id
        self.dev.oled().write(&report)?;
        Ok(())
    }

    /// Send a 64-byte command report on the OLED collection (the same route as brightness
    /// `0x85` and return-to-UI `0x95`).
    pub fn send_oled_command(&self, cmd: u8, args: &[u8]) -> anyhow::Result<()> {
        let mut report = [0u8; 64];
        report[0] = self.oled_report_id;
        report[1] = cmd;
        for (slot, value) in report[2..].iter_mut().zip(args) {
            *slot = *value;
        }
        self.dev.oled().write(&report)?;
        Ok(())
    }

    /// Send a 64-byte command report on the info collection (the same route as the `0x20` /
    /// `0xb0` queries).
    pub fn send_info_command(&self, cmd: u8, args: &[u8]) -> anyhow::Result<()> {
        let mut report = [0u8; 64];
        report[0] = self.info_report_id;
        report[1] = cmd;
        for (slot, value) in report[2..].iter_mut().zip(args) {
            *slot = *value;
        }
        self.dev.info().write(&report)?;
        Ok(())
    }

    /// Ask the base station for its UX settings (`0x80`); the answer arrives as
    /// [`DeviceEvent::UxSettings`]. Read-only.
    pub fn query_ux_settings(&self) -> anyhow::Result<()> {
        self.send_info_command(0x80, &[])
    }

    /// Experimental (`GXOLED_OSD_LAB`): `0x8d` "set Sonar present". The command exists in the
    /// device's published opcode list; which of the two collections the firmware listens on is
    /// not documented for this model, so the caller decides and logs the outcome.
    pub fn set_sonar_present_on(&self, present: bool, oled_collection: bool) -> anyhow::Result<()> {
        let args = [present as u8];
        if oled_collection {
            self.send_oled_command(0x8d, &args)
        } else {
            self.send_info_command(0x8d, &args)
        }
    }

    /// Experimental (`GXOLED_OSD_LAB`): `0x89` home screen type (0 detailed, 1 simple).
    pub fn set_home_screen_type_on(&self, value: u8, oled_collection: bool) -> anyhow::Result<()> {
        let args = [value];
        if oled_collection {
            self.send_oled_command(0x89, &args)
        } else {
            self.send_info_command(0x89, &args)
        }
    }

    // `buf[0]` is the incoming report ID. Nova Pro command replies use the info report ID
    // sent with the query, while unsolicited dial events use report ID 7.
    fn parse_event(&self, buf: &[u8; 64]) -> Vec<DeviceEvent> {
        #[cfg(debug_assertions)]
        println!("parse_event: {:x?}", buf);
        parse_event_report(buf, self.info_report_id)
    }

    /// Poll events from the device. This blocks until an event is returned.
    pub fn poll_event(&self) -> anyhow::Result<Vec<DeviceEvent>> {
        let mut buf = [0u8; 64];
        self.dev.info().set_blocking_mode(true)?;
        _ = self.dev.info().read(&mut buf)?;
        Ok(self.parse_event(&buf))
    }

    /// Return any pending events from the device. Non-blocking.
    pub fn get_events(&self) -> anyhow::Result<Vec<DeviceEvent>> {
        self.dev.info().set_blocking_mode(false)?;
        let mut events = vec![];
        loop {
            let mut buf = [0u8; 64];
            let len = self.dev.info().read(&mut buf)?;
            if len == 0 {
                break;
            } else {
                events.append(&mut self.parse_event(&buf));
            }
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::{DeviceEvent, parse_event_report};

    #[test]
    fn physical_volume_dial_is_distinct_from_volume_query_reply() {
        let mut dial_report = [0u8; 64];
        dial_report[0] = 0x07;
        dial_report[1] = 0x25;
        dial_report[2] = 0x20;
        let dial_events = parse_event_report(&dial_report, 0x06);
        assert!(matches!(dial_events.as_slice(), [DeviceEvent::VolumeDial { volume: 0x18 }]));

        let mut query_report = [0u8; 64];
        query_report[0] = 0x06;
        query_report[1] = 0x20;
        query_report[3] = 0x20;
        let query_events = parse_event_report(&query_report, 0x06);
        assert!(matches!(query_events.as_slice(), [DeviceEvent::Volume { volume: 0x18 }]));
    }

    #[test]
    fn ux_settings_reply_is_decoded_field_by_field() {
        let mut report = [0u8; 64];
        report[0] = 0x06;
        report[1] = 0x80;
        report[2..8].copy_from_slice(&[1, 8, 3, 0, 1, 1]);
        let events = parse_event_report(&report, 0x06);
        assert!(matches!(
            events.as_slice(),
            [DeviceEvent::UxSettings {
                dim_timer: 1,
                oled_brightness: 8,
                idle_brightness: 3,
                home_screen_type: 0,
                home_screen_mic_mute: 1,
                sonar_present: 1,
            }]
        ));
    }

    #[test]
    fn unrecognised_reports_are_preserved_for_diagnostics() {
        let mut report = [0u8; 64];
        report[0] = 0x07;
        report[1] = 0x42;
        report[2] = 0xaa;
        let events = parse_event_report(&report, 0x06);
        match events.as_slice() {
            [DeviceEvent::Unknown { raw }] => assert_eq!(&raw[..3], &[0x07, 0x42, 0xaa]),
            other => panic!("unexpected: {other:?}"),
        }
    }
}
