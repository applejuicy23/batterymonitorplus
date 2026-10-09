//! Battery levels of connected peripherals.
//!
//! Two sources:
//! - [`bluetooth`]: the charge Windows itself shows in Settings → Bluetooth.
//! - [`razer`]: Razer mice/keyboards queried over HID (OpenRazer protocol),
//!   which covers the 2.4 GHz dongle and USB cable.

pub mod bluetooth;
pub mod razer;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Bluetooth,
    Razer,
}

#[derive(Debug, Clone, Serialize)]
pub struct Reading {
    /// Stable per-device key, usable in config to pick/rename/hide devices.
    pub id: String,
    pub name: String,
    pub source: Source,
    pub percent: u8,
    pub charging: Option<bool>,
}

/// Polls every source once and merges the results; if one source fails,
/// the others still show up.
pub fn read_all_batteries() -> Vec<Reading> {
    let mut out = bluetooth::read_bluetooth_batteries().unwrap_or_default();
    out.extend(razer::read_razer_batteries().unwrap_or_default());
    out
}
