//! The user's choices about devices: which ones show up in the tiles, under
//! what name, with what kind/icon and in what order.
//!
//! Stored as TOML in `%APPDATA%\BatteryMonitorPlus\config.toml`
//! (or the path in the `BMP_CONFIG` environment variable), e.g.:
//!
//! ```toml
//! [devices."razer:00df"]
//! detected_name = "Razer Viper V3 Pro SE"
//! detected_kind = "mouse"
//! name = "Viper"
//! order = 1
//!
//! [devices."bt:E78E379B2DB7"]
//! detected_name = "YUNZII B75PROMAX"
//! detected_kind = "keyboard"
//! show = false
//! ```
//!
//! `detected_*` are written by the app so the file is readable by hand;
//! everything else is the user's.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use bmp_battery::{Device, DeviceKind};
use serde::{Deserialize, Serialize};

const APP_DIR: &str = "BatteryMonitorPlus";
const FILE_NAME: &str = "config.toml";
const PATH_OVERRIDE_VAR: &str = "BMP_CONFIG";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Per-device settings keyed by device id (`bt:...`, `razer:...`).
    #[serde(default)]
    pub devices: BTreeMap<String, DeviceSettings>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_kind: Option<DeviceKind>,
    /// Show in the tiles; when unset, devices that report a battery are shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<DeviceKind>,
    /// Built-in icon name or a path to an SVG/PNG; defaults to the kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Lower comes first; unset devices go after, ordered by kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
}

/// Works out where the config file lives.
pub fn find_config_path() -> PathBuf {
    if let Some(path) = std::env::var_os(PATH_OVERRIDE_VAR) {
        return PathBuf::from(path);
    }
    let base = std::env::var_os("APPDATA").map_or_else(|| PathBuf::from("."), PathBuf::from);
    base.join(APP_DIR).join(FILE_NAME)
}

/// Reads the config from disk; a missing file means an empty config.
pub fn load_config() -> io::Result<Config> {
    let path = find_config_path();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(e) => return Err(e),
    };
    toml::from_str(&text).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {e}", path.display()),
        )
    })
}

/// Writes the config to disk, creating the folder if needed.
pub fn save_config(config: &Config) -> io::Result<()> {
    let path = find_config_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = toml::to_string_pretty(config).map_err(io::Error::other)?;
    fs::write(path, text)
}

/// Records every detected device in the config (name and kind as detected),
/// so devices stay listed and editable even while they're off.
/// Returns `true` if anything changed and the config should be saved.
pub fn remember_devices(config: &mut Config, devices: &[Device]) -> bool {
    let mut changed = false;
    for device in devices {
        let settings = config.devices.entry(device.id.clone()).or_default();
        let before = settings.clone();
        settings.detected_name = Some(device.name.clone());
        settings.detected_kind = Some(device.kind);
        changed |= *settings != before;
    }
    changed
}

/// Tells whether a device goes into the tiles: the user's choice if set,
/// otherwise only devices that have reported a battery.
pub fn is_device_shown(config: &Config, device: &Device) -> bool {
    config
        .devices
        .get(&device.id)
        .and_then(|s| s.show)
        .unwrap_or(device.battery.is_some())
}

/// Applies the user's name/kind overrides to a detected device.
pub fn apply_overrides(config: &Config, mut device: Device) -> Device {
    if let Some(settings) = config.devices.get(&device.id) {
        if let Some(name) = &settings.name {
            device.name = name.clone();
        }
        if let Some(kind) = settings.kind {
            device.kind = kind;
        }
    }
    device
}

/// Picks the icon for a device: the user's icon, or the one for its kind.
pub fn choose_icon(config: &Config, device: &Device) -> String {
    config
        .devices
        .get(&device.id)
        .and_then(|s| s.icon.clone())
        .unwrap_or_else(|| device.kind.as_str().to_owned())
}

/// Turns detected devices into what the tiles show: hidden ones dropped,
/// overrides applied, sorted by the user's order, then kind, then name.
pub fn arrange_devices(config: &Config, devices: Vec<Device>) -> Vec<Device> {
    let mut shown: Vec<(i32, Device)> = devices
        .into_iter()
        .filter(|d| is_device_shown(config, d))
        .map(|d| {
            let order = config
                .devices
                .get(&d.id)
                .and_then(|s| s.order)
                .unwrap_or(i32::MAX);
            (order, apply_overrides(config, d))
        })
        .collect();
    shown.sort_by(|(oa, a), (ob, b)| {
        oa.cmp(ob)
            .then(a.kind.cmp(&b.kind))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    shown.into_iter().map(|(_, d)| d).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_battery::Source;

    /// Builds a test device with the given id, kind and battery.
    fn make_device(id: &str, kind: DeviceKind, battery: Option<u8>) -> Device {
        Device {
            id: id.to_owned(),
            name: id.to_owned(),
            kind,
            source: Source::Bluetooth,
            battery,
            charging: None,
            connected: Some(true),
            updated_at: None,
        }
    }

    /// Checks default visibility, user hiding, ordering and renaming.
    #[test]
    fn arranges_devices_with_user_choices() {
        let mut config = Config::default();
        config.devices.insert(
            "kb".into(),
            DeviceSettings {
                show: Some(false),
                ..Default::default()
            },
        );
        config.devices.insert(
            "pad".into(),
            DeviceSettings {
                order: Some(0),
                name: Some("Pad".into()),
                ..Default::default()
            },
        );
        let devices = vec![
            make_device("kb", DeviceKind::Keyboard, Some(50)),
            make_device("phone", DeviceKind::Other, None),
            make_device("mouse", DeviceKind::Mouse, Some(40)),
            make_device("pad", DeviceKind::Gamepad, Some(90)),
        ];
        let names: Vec<String> = arrange_devices(&config, devices)
            .into_iter()
            .map(|d| d.name)
            .collect();
        assert_eq!(names, ["Pad", "mouse"]);
    }

    /// Checks that remembering devices only reports a change when needed.
    #[test]
    fn remembers_devices_once() {
        let mut config = Config::default();
        let devices = vec![make_device("mouse", DeviceKind::Mouse, Some(40))];
        assert!(remember_devices(&mut config, &devices));
        assert!(!remember_devices(&mut config, &devices));
        assert_eq!(
            config.devices["mouse"].detected_kind,
            Some(DeviceKind::Mouse)
        );
    }

    /// Checks that the config survives a TOML round trip.
    #[test]
    fn round_trips_through_toml() {
        let mut config = Config::default();
        config.devices.insert(
            "bt:E78E379B2DB7".into(),
            DeviceSettings {
                detected_name: Some("YUNZII B75PROMAX".into()),
                kind: Some(DeviceKind::Keyboard),
                show: Some(true),
                ..Default::default()
            },
        );
        let text = toml::to_string_pretty(&config).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.devices, config.devices);
    }
}
