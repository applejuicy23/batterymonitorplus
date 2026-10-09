//! Razer battery over HID, using the OpenRazer feature-report protocol.
//!
//! Each Razer HID interface is asked for "get battery" (class 0x07, id 0x80)
//! and "get charging" (0x07 / 0x84). Only read commands are sent.

use std::collections::HashSet;
use std::thread;
use std::time::Duration;

use hidapi::{HidApi, HidDevice, HidResult};

use crate::{Reading, Source};

pub const VID: u16 = 0x1532;

const CLASS_POWER: u8 = 0x07;
const CMD_BATTERY: u8 = 0x80;
const CMD_CHARGING: u8 = 0x84;
/// Different Razer generations answer on different transaction ids.
const TRANSACTION_IDS: [u8; 4] = [0x1F, 0xFF, 0x3F, 0x9F];
const REPORT_LEN: usize = 90;
const STATUS_OK: u8 = 0x02;

/// Asks every connected Razer device for its charge and charging state,
/// one reading per product (a device exposes several HID interfaces).
pub fn read_razer_batteries() -> HidResult<Vec<Reading>> {
    let api = HidApi::new()?;
    let mut out = Vec::new();
    let mut done = HashSet::new();
    for info in api.device_list().filter(|d| d.vendor_id() == VID) {
        let pid = info.product_id();
        if done.contains(&pid) {
            continue;
        }
        let Ok(dev) = api.open_path(info.path()) else {
            continue;
        };
        let Some((tid, raw)) = query_razer_power(&dev, None, CMD_BATTERY) else {
            continue;
        };
        let charging = query_razer_power(&dev, Some(tid), CMD_CHARGING).map(|(_, v)| v != 0);
        done.insert(pid);
        out.push(Reading {
            id: format!("razer:{pid:04x}"),
            name: info
                .product_string()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Razer {pid:04x}")),
            source: Source::Razer,
            percent: ((raw as u32 * 100 + 127) / 255) as u8,
            charging,
        });
    }
    Ok(out)
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
