//! Ready-to-show views of the devices, shared by the app and `bmp-probe`:
//! tiles (what the taskbar shows) and rows (the settings list).

use bmp_battery::{Device, DeviceKind};
use serde::Serialize;

use crate::merge::find_main_id;
use crate::{Config, DeviceSettings, apply_overrides, arrange_devices, choose_icon};

/// One taskbar tile.
#[derive(Debug, Clone, Serialize)]
pub struct Tile {
    #[serde(flatten)]
    pub device: Device,
    pub icon: String,
    /// The battery value is from a device that is connected now.
    pub live: bool,
}

/// One line of the settings list: a device the config knows, found right
/// now or only remembered.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceRow {
    pub id: String,
    /// Name with the user's override applied.
    pub name: String,
    pub kind: DeviceKind,
    pub icon: String,
    /// Goes into the tiles (as itself, not merged into another entry).
    pub shown: bool,
    /// Found right now (vs. only remembered in the config).
    pub detected: bool,
    /// Main entry this one is merged into, if any.
    pub merged_into: Option<String>,
    pub device: Option<Device>,
    pub settings: DeviceSettings,
}

/// Builds the tiles: devices merged, hidden ones dropped, overrides applied,
/// in tile order, each with its icon.
pub fn build_tiles(config: &Config, devices: Vec<Device>) -> Vec<Tile> {
    arrange_devices(config, devices)
        .into_iter()
        .map(|d| Tile {
            icon: choose_icon(config, &d),
            live: d.has_live_battery(),
            device: d,
        })
        .collect()
}

/// Builds one row per device the config knows: shown first, then hidden,
/// then merged entries, each group ordered by kind.
pub fn build_device_rows(config: &Config, devices: &[Device]) -> Vec<DeviceRow> {
    let tile_ids: Vec<String> = arrange_devices(config, devices.to_vec())
        .into_iter()
        .map(|d| d.id)
        .collect();

    let mut rows = Vec::new();
    for (id, settings) in &config.devices {
        let main_id = find_main_id(config, id);
        let merged_into = (&main_id != id).then_some(main_id);
        let shown = merged_into.is_none() && tile_ids.contains(id);
        let detected = devices.iter().find(|d| &d.id == id);
        let (name, kind, icon) = match detected {
            Some(device) => {
                let shown_as = apply_overrides(config, device.clone());
                let icon = choose_icon(config, &shown_as);
                (shown_as.name, shown_as.kind, icon)
            }
            None => {
                let kind = settings
                    .kind
                    .or(settings.detected_kind)
                    .unwrap_or(DeviceKind::Other);
                let name = settings
                    .name
                    .clone()
                    .or_else(|| settings.detected_name.clone())
                    .unwrap_or_else(|| id.clone());
                let icon = settings
                    .icon
                    .clone()
                    .unwrap_or_else(|| kind.as_str().to_owned());
                (name, kind, icon)
            }
        };
        rows.push(DeviceRow {
            id: id.clone(),
            name,
            kind,
            icon,
            shown,
            detected: detected.is_some(),
            merged_into,
            device: detected.cloned(),
            settings: settings.clone(),
        });
    }
    rows.sort_by(|a, b| {
        b.shown
            .cmp(&a.shown)
            .then(a.merged_into.is_some().cmp(&b.merged_into.is_some()))
            .then(a.kind.cmp(&b.kind))
    });
    rows
}
