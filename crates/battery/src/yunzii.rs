//! YUNZII keyboards over their 2.4 GHz dongle (`056D:C077`, I-CHIP).
//!
//! The dongle has a vendor HID channel (usage page 0xFF00, report id 0x0A,
//! 63 bytes each way). Command 0x05 is a read-only status query; in the
//! answer byte 14 is the battery percent and byte 15 the voltage in tenths of
//! a volt. The dongle caches what the keyboard reported when it last
//! connected, so the value only refreshes on reconnect — and the reply is
//! byte-for-byte the same with the keyboard switched off, so whether it is
//! connected right now can't be told.
//!
//! Protocol from https://lee-it.co.uk/linux/2026/05/03/wireless-peripheral-battery-linux.html
//! (YUNZII B87, same dongle), confirmed on a B75 PRO MAX.

use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice, HidResult};

use crate::{Device, DeviceKind, Source, unix_now};

pub const DONGLE_VID: u16 = 0x056D;
pub const DONGLE_PID: u16 = 0xC077;

const VENDOR_USAGE_PAGE: u16 = 0xFF00;
const VENDOR_USAGE: u16 = 0x01;
const REPORT_ID: u8 = 0x0A;
const CMD_STATUS: u8 = 0x05;
const REPORT_LEN: usize = 64;
const BATTERY_BYTE: usize = 14;
const REPLY_TIMEOUT: Duration = Duration::from_millis(500);

/// Finds a YUNZII 2.4G dongle and asks it for the keyboard's battery.
pub fn detect_yunzii_devices() -> HidResult<Vec<Device>> {
    let api = HidApi::new()?;
    let Some(info) = api.device_list().find(|d| {
        d.vendor_id() == DONGLE_VID
            && d.product_id() == DONGLE_PID
            && d.usage_page() == VENDOR_USAGE_PAGE
            && d.usage() == VENDOR_USAGE
    }) else {
        return Ok(Vec::new());
    };

    let name = info
        .product_string()
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches(" 2.4G").to_owned())
        .unwrap_or_else(|| "YUNZII keyboard".to_owned());
    let battery = api
        .open_path(info.path())
        .ok()
        .and_then(|dev| query_yunzii_battery(&dev));

    Ok(vec![Device {
        id: format!("yunzii:{DONGLE_PID:04x}"),
        name,
        kind: DeviceKind::Keyboard,
        source: Source::Yunzii,
        battery,
        charging: None,
        connected: None,
        updated_at: battery.map(|_| unix_now()),
    }])
}

/// Sends the status query and waits for its answer; returns the percent.
fn query_yunzii_battery(dev: &HidDevice) -> Option<u8> {
    let mut request = [0u8; REPORT_LEN];
    request[0] = REPORT_ID;
    request[1] = CMD_STATUS;
    dev.write(&request).ok()?;

    let deadline = Instant::now() + REPLY_TIMEOUT;
    let mut reply = [0u8; REPORT_LEN];
    while Instant::now() < deadline {
        let n = dev.read_timeout(&mut reply, 100).ok()?;
        if n > BATTERY_BYTE && reply[0] == REPORT_ID && reply[1] == CMD_STATUS {
            return Some(reply[BATTERY_BYTE].min(100));
        }
    }
    None
}
