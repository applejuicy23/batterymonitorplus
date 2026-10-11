//! One physical device seen by several sources, e.g. a keyboard that is
//! paired over Bluetooth and also has a 2.4 GHz dongle, becomes one tile.
//!
//! The link is stored in the config as `same_as = "<main id>"` on the other
//! entries, so settings keep working when only one source is visible.
//! Links are made automatically for devices of the same kind whose names
//! match ignoring case, spaces and punctuation; `keep_separate = true`
//! stops that for a device.

use std::collections::BTreeMap;

use bmp_battery::{Device, Source};

use crate::Config;

/// Lowercases a name and drops everything but letters and digits, so
/// "YUNZII B75PROMAX" and "YUNZII B75 PRO MAX" compare equal.
pub fn normalize_device_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Finds the id a device is merged into, or its own id if it isn't linked.
pub fn find_main_id(config: &Config, id: &str) -> String {
    config
        .devices
        .get(id)
        .and_then(|s| s.same_as.clone())
        .filter(|main| !main.is_empty() && main != id)
        .unwrap_or_else(|| id.to_owned())
}

/// Links devices detected together that look like the same physical device.
/// The vendor source (Razer, YUNZII dongle) becomes the main entry because
/// its id is tied to the product. Returns `true` if the config changed.
pub fn link_same_devices(config: &mut Config, devices: &[Device]) -> bool {
    let mut changed = false;
    for (i, a) in devices.iter().enumerate() {
        for b in &devices[i + 1..] {
            if a.kind != b.kind {
                continue;
            }
            let name = normalize_device_name(&a.name);
            if name.is_empty() || name != normalize_device_name(&b.name) {
                continue;
            }
            let (main, other) = if rank_source(b.source) > rank_source(a.source) {
                (b, a)
            } else {
                (a, b)
            };
            let main_id = find_main_id(config, &main.id);
            let settings = config.devices.entry(other.id.clone()).or_default();
            if settings.keep_separate == Some(true) || settings.same_as.is_some() {
                continue;
            }
            if main_id != other.id {
                settings.same_as = Some(main_id);
                changed = true;
            }
        }
    }
    changed
}

/// Folds linked devices into one each: the id is the main id, the name and
/// kind come from the main entry, and the battery from the freshest source.
pub fn merge_same_devices(config: &Config, devices: Vec<Device>) -> Vec<Device> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Vec<Device>> = BTreeMap::new();
    for device in devices {
        let main_id = find_main_id(config, &device.id);
        if !groups.contains_key(&main_id) {
            order.push(main_id.clone());
        }
        groups.entry(main_id).or_default().push(device);
    }

    let mut out = Vec::new();
    for main_id in order {
        let members = groups.remove(&main_id).unwrap_or_default();
        let Some(best) = members.iter().max_by_key(|d| rank_reading(d)) else {
            continue;
        };
        let mut merged = best.clone();
        if let Some(main) = members.iter().find(|d| d.id == main_id) {
            merged.name = main.name.clone();
            merged.kind = main.kind;
        } else if let Some(settings) = config.devices.get(&main_id) {
            if let Some(name) = &settings.detected_name {
                merged.name = name.clone();
            }
            if let Some(kind) = settings.detected_kind {
                merged.kind = kind;
            }
        }
        merged.id = main_id;
        out.push(merged);
    }
    out
}

/// Vendor sources have product-bound ids, so they make the better main entry.
fn rank_source(source: Source) -> u8 {
    match source {
        Source::Razer | Source::Yunzii => 2,
        Source::Bluetooth => 1,
        Source::System => 0,
    }
}

/// Scores how trustworthy a battery value is: connected beats unknown beats
/// stale beats none; ties go to the newer value.
fn rank_reading(device: &Device) -> (u8, u64) {
    let trust = match (device.battery, device.connected) {
        (None, _) => 0,
        (Some(_), Some(false)) => 1,
        (Some(_), None) => 2,
        (Some(_), Some(true)) => 3,
    };
    (trust, device.updated_at.unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeviceSettings;
    use bmp_battery::DeviceKind;

    /// Builds a test keyboard from the given source.
    fn make_keyboard(
        id: &str,
        name: &str,
        source: Source,
        battery: u8,
        connected: Option<bool>,
    ) -> Device {
        Device {
            id: id.to_owned(),
            name: name.to_owned(),
            kind: DeviceKind::Keyboard,
            source,
            battery: Some(battery),
            charging: None,
            connected,
            updated_at: Some(1),
        }
    }

    /// Checks the dongle entry becomes main and the stale BT value loses.
    #[test]
    fn merges_bluetooth_and_dongle_keyboard() {
        let devices = vec![
            make_keyboard(
                "bt:E78E",
                "YUNZII B75PROMAX",
                Source::Bluetooth,
                100,
                Some(false),
            ),
            make_keyboard(
                "yunzii:c077",
                "YUNZII B75 PRO MAX",
                Source::Yunzii,
                80,
                None,
            ),
        ];
        let mut config = Config::default();
        assert!(link_same_devices(&mut config, &devices));
        assert_eq!(
            config.devices["bt:E78E"].same_as.as_deref(),
            Some("yunzii:c077")
        );
        assert!(!link_same_devices(&mut config, &devices));

        let merged = merge_same_devices(&config, devices);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, "yunzii:c077");
        assert_eq!(merged[0].name, "YUNZII B75 PRO MAX");
        assert_eq!(merged[0].battery, Some(80));
    }

    /// Checks a live Bluetooth value wins once the keyboard is used over BT,
    /// even when the dongle isn't plugged in.
    #[test]
    fn live_bluetooth_wins_under_main_id() {
        let mut config = Config::default();
        config.devices.insert(
            "bt:E78E".into(),
            DeviceSettings {
                same_as: Some("yunzii:c077".into()),
                ..Default::default()
            },
        );
        config.devices.insert(
            "yunzii:c077".into(),
            DeviceSettings {
                detected_name: Some("YUNZII B75 PRO MAX".into()),
                ..Default::default()
            },
        );
        let devices = vec![make_keyboard(
            "bt:E78E",
            "YUNZII B75PROMAX",
            Source::Bluetooth,
            64,
            Some(true),
        )];
        let merged = merge_same_devices(&config, devices);
        assert_eq!(merged[0].id, "yunzii:c077");
        assert_eq!(merged[0].name, "YUNZII B75 PRO MAX");
        assert_eq!(merged[0].battery, Some(64));
    }

    /// Checks `keep_separate` stops the automatic link.
    #[test]
    fn keep_separate_blocks_linking() {
        let devices = vec![
            make_keyboard("bt:A", "Pad", Source::Bluetooth, 10, Some(true)),
            make_keyboard("yunzii:c077", "Pad", Source::Yunzii, 20, None),
        ];
        let mut config = Config::default();
        config.devices.insert(
            "bt:A".into(),
            DeviceSettings {
                keep_separate: Some(true),
                ..Default::default()
            },
        );
        assert!(!link_same_devices(&mut config, &devices));
        assert_eq!(merge_same_devices(&config, devices).len(), 2);
    }
}
