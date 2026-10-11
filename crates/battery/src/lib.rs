//! Detects peripherals that may have a battery and reads their charge.
//!
//! Sources:
//! - [`system`]: the laptop's own battery ("This PC").
//! - [`bluetooth`]: every paired Bluetooth device, with the charge Windows
//!   shows in Settings → Bluetooth.
//! - [`razer`]: Razer mice/keyboards over HID (OpenRazer protocol), which
//!   covers the 2.4 GHz dongle and USB cable.

pub mod bluetooth;
pub mod kind;
pub mod razer;
pub mod system;

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

pub use kind::DeviceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    System,
    Bluetooth,
    Razer,
}

#[derive(Debug, Clone, Serialize)]
pub struct Device {
    /// Stable per-device key, used in the config to show/hide/rename.
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub source: Source,
    /// Last known charge, 0..=100; `None` if the device never reported one.
    pub battery: Option<u8>,
    pub charging: Option<bool>,
    /// Connected right now; `None` if it can't be told. When `false`,
    /// `battery` is the last value seen, from `updated_at`.
    pub connected: Option<bool>,
    /// When `battery` was measured, unix seconds.
    pub updated_at: Option<u64>,
}

impl Device {
    /// Tells whether the battery value is from a device that is connected
    /// now, i.e. safe to show as live.
    pub fn has_live_battery(&self) -> bool {
        self.battery.is_some() && self.connected != Some(false)
    }
}

/// Runs every detector once and merges the results; if one source fails,
/// the others still show up.
pub fn detect_all_devices() -> Vec<Device> {
    let mut out: Vec<Device> = system::read_system_battery().into_iter().collect();
    out.extend(bluetooth::detect_bluetooth_devices().unwrap_or_default());
    out.extend(razer::detect_razer_devices().unwrap_or_default());
    out
}

/// Returns the current time as unix seconds.
pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
