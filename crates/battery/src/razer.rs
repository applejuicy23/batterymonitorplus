//! Razer devices over HID, using the OpenRazer feature-report protocol.
//!
//! Each Razer HID interface is asked for "get battery" (class 0x07, id 0x80)
//! and "get charging" (0x07 / 0x84). Only read commands are sent. A Razer
//! device that doesn't answer (e.g. dongle plugged in, mouse switched off)
//! is still listed, just without a battery value.

use std::collections::{BTreeMap, HashSet};
use std::thread;
use std::time::Duration;

use hidapi::{DeviceInfo, HidApi, HidDevice, HidResult};

use crate::kind::guess_kind_from_name;
use crate::{Device, DeviceKind, Source, unix_now};

pub const VID: u16 = 0x1532;

const CLASS_POWER: u8 = 0x07;
const CMD_BATTERY: u8 = 0x80;
const CMD_CHARGING: u8 = 0x84;
/// Different Razer generations answer on different transaction ids.
const TRANSACTION_IDS: [u8; 4] = [0x1F, 0xFF, 0x3F, 0x9F];
const REPORT_LEN: usize = 90;
const STATUS_OK: u8 = 0x02;

const USAGE_PAGE_GENERIC_DESKTOP: u16 = 0x01;
const USAGE_MOUSE: u16 = 0x02;

/// Finds every connected Razer product and asks it for charge and charging
/// state, one device per product (a product exposes several HID interfaces).
pub fn detect_razer_devices() -> HidResult<Vec<Device>> {
    let api = HidApi::new()?;
    let mut interfaces: BTreeMap<u16, Vec<&DeviceInfo>> = BTreeMap::new();
    for info in api.device_list().filter(|d| d.vendor_id() == VID) {
        interfaces.entry(info.product_id()).or_default().push(info);
    }

    let mut out = Vec::new();
    for (pid, infos) in interfaces {
        let name = infos
            .iter()
            .filter_map(|i| i.product_string())
            .find(|s| !s.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Razer {pid:04x}"));
        let kind = guess_razer_kind(&infos, &name);
        let power = read_razer_power(&api, &infos);
        out.push(Device {
            id: format!("razer:{pid:04x}"),
            name,
            kind,
            source: Source::Razer,
            battery: power.map(|(percent, _)| percent),
            charging: power.and_then(|(_, charging)| charging),
            connected: Some(power.is_some()),
            updated_at: power.map(|_| unix_now()),
        });
    }
    Ok(out)
}

/// A Razer product with a HID mouse interface is a mouse (its keyboard
/// interface is just for macros); otherwise the name decides.
fn guess_razer_kind(infos: &[&DeviceInfo], name: &str) -> DeviceKind {
    let usages: HashSet<(u16, u16)> = infos.iter().map(|i| (i.usage_page(), i.usage())).collect();
    if usages.contains(&(USAGE_PAGE_GENERIC_DESKTOP, USAGE_MOUSE)) {
        return DeviceKind::Mouse;
    }
    guess_kind_from_name(name)
}

/// Tries each interface of a product until one answers the battery query;
/// returns `(percent, charging)`.
fn read_razer_power(api: &HidApi, infos: &[&DeviceInfo]) -> Option<(u8, Option<bool>)> {
    for info in infos {
        let Ok(dev) = api.open_path(info.path()) else {
            continue;
        };
        let Some((tid, raw)) = query_razer_power(&dev, None, CMD_BATTERY) else {
            continue;
        };
        let charging = query_razer_power(&dev, Some(tid), CMD_CHARGING).map(|(_, v)| v != 0);
        let percent = ((raw as u32 * 100 + 127) / 255) as u8;
        return Some((percent, charging));
    }
    None
}

/// Sends one power-class command and waits for the answer. Tries each known
/// transaction id until one replies; returns `(transaction_id, value)`.
fn query_razer_power(dev: &HidDevice, tid: Option<u8>, command: u8) -> Option<(u8, u8)> {
    let tids = tid.map_or(TRANSACTION_IDS.to_vec(), |t| vec![t]);
    for tid in tids {
        let mut req = [0u8; REPORT_LEN + 1];
        req[1..].copy_from_slice(&build_razer_report(tid, CLASS_POWER, command, 0x02));
        if dev.send_feature_report(&req).is_err() {
            continue;
        }
        thread::sleep(Duration::from_millis(60));
        let mut resp = [0u8; REPORT_LEN + 1];
        let Ok(n) = dev.get_feature_report(&mut resp) else {
            continue;
        };
        let r = if n > REPORT_LEN {
            &resp[1..]
        } else {
            &resp[..n]
        };
        if r.len() >= 10 && r[0] == STATUS_OK && r[6] == CLASS_POWER && r[7] == command {
            return Some((tid, r[9]));
        }
    }
    None
}

/// Builds a 90-byte Razer request (OpenRazer `razer_report` layout);
/// the checksum is the XOR of bytes 2..88.
fn build_razer_report(tid: u8, class: u8, command: u8, size: u8) -> [u8; REPORT_LEN] {
    let mut buf = [0u8; REPORT_LEN];
    buf[1] = tid;
    buf[5] = size;
    buf[6] = class;
    buf[7] = command;
    buf[88] = buf[2..88].iter().fold(0, |acc, b| acc ^ b);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks the battery request has the right header bytes and checksum.
    #[test]
    fn builds_battery_report_with_checksum() {
        let r = build_razer_report(0x1F, CLASS_POWER, CMD_BATTERY, 0x02);
        assert_eq!(r[1], 0x1F);
        assert_eq!(&r[5..8], &[0x02, 0x07, 0x80]);
        assert_eq!(r[88], 0x02 ^ 0x07 ^ 0x80);
    }
}
