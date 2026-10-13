//! Commands the settings window calls (`invoke(...)` on the JS side).
//!
//! Commands are `async` so Tauri runs them off the main thread — device
//! detection takes a few hundred ms and must not freeze the window.

use bmp_battery::DeviceKind;
use bmp_config::merge::find_main_id;
use bmp_config::{DeviceSettings, TileLayout, load_config, save_config};
use serde::Deserialize;
use tauri::AppHandle;

use crate::monitor::{Snapshot, build_snapshot, publish_snapshot, refresh_devices_now};
use crate::taskbar::{
    embed_tiles_window, move_tiles_window, place_tiles_window, read_tiles_offset,
    resize_tiles_window,
};
use crate::tray::show_main_window;

/// One change to a device's settings, sent by the settings window.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum DeviceChange {
    /// `null` goes back to the default (shown when the battery is live).
    Show(Option<bool>),
    /// `null` or empty goes back to the detected name.
    Name(Option<String>),
    Kind(Option<DeviceKind>),
    Order(Option<i32>),
    /// Merge into the device with this id.
    Link(String),
    Unlink,
    Reset,
}

/// Returns the latest snapshot without detecting again.
#[tauri::command]
pub async fn get_snapshot(app: AppHandle) -> Snapshot {
    build_snapshot(&app)
}

/// Detects devices right now and returns the fresh snapshot.
#[tauri::command]
pub async fn refresh_devices(app: AppHandle) -> Snapshot {
    refresh_devices_now(&app)
}

/// Applies one settings change to a device, saves the config and returns
/// the updated snapshot.
#[tauri::command]
pub async fn update_device(
    app: AppHandle,
    id: String,
    change: DeviceChange,
) -> Result<Snapshot, String> {
    let mut config = load_config().map_err(|e| e.to_string())?;
    let link_target = match &change {
        DeviceChange::Link(target) => {
            let main_id = find_main_id(&config, target);
            if main_id == id {
                return Err("a device can't be merged into itself".to_owned());
            }
            Some(main_id)
        }
        _ => None,
    };
    let settings = config.devices.entry(id).or_default();
    apply_device_change(settings, change, link_target);
    save_config(&config).map_err(|e| e.to_string())?;
    Ok(publish_snapshot(&app))
}

/// Sets how often batteries are re-read, in seconds.
#[tauri::command]
pub async fn set_poll_seconds(app: AppHandle, seconds: u64) -> Result<Snapshot, String> {
    let mut config = load_config().map_err(|e| e.to_string())?;
    config.poll_seconds = Some(seconds);
    save_config(&config).map_err(|e| e.to_string())?;
    Ok(publish_snapshot(&app))
}

/// Saves how the taskbar tiles are drawn.
#[tauri::command]
pub async fn set_tile_layout(app: AppHandle, layout: TileLayout) -> Result<Snapshot, String> {
    let mut config = load_config().map_err(|e| e.to_string())?;
    config.tile_layout = Some(layout);
    save_config(&config).map_err(|e| e.to_string())?;
    Ok(publish_snapshot(&app))
}

/// Takes the width the tiles page needs and re-docks the tiles window.
#[tauri::command]
pub async fn set_tiles_width(app: AppHandle, width: f64) {
    resize_tiles_window(&app, width);
}

/// Shifts the tiles sideways while they're being dragged.
#[tauri::command]
pub async fn move_tiles_by(app: AppHandle, dx: f64) {
    move_tiles_window(&app, dx);
}

/// Saves where the tiles were dropped after a drag.
#[tauri::command]
pub async fn save_tiles_position(app: AppHandle) -> Result<Snapshot, String> {
    let offset = read_tiles_offset(&app).map(|o| o.round() as i32);
    let mut config = load_config().map_err(|e| e.to_string())?;
    config.tile_offset = offset;
    save_config(&config).map_err(|e| e.to_string())?;
    Ok(publish_snapshot(&app))
}

/// Puts the tiles back in the automatic spot next to the tray.
#[tauri::command]
pub async fn reset_tiles_position(app: AppHandle) -> Result<Snapshot, String> {
    let mut config = load_config().map_err(|e| e.to_string())?;
    config.tile_offset = None;
    save_config(&config).map_err(|e| e.to_string())?;
    place_tiles_window(&app, None);
    Ok(publish_snapshot(&app))
}

/// Switches the tiles between living inside the taskbar and floating over it.
#[tauri::command]
pub async fn set_tile_embed(app: AppHandle, embed: bool) -> Result<Snapshot, String> {
    let mut config = load_config().map_err(|e| e.to_string())?;
    config.tile_embed = Some(embed);
    save_config(&config).map_err(|e| e.to_string())?;
    embed_tiles_window(&app, embed);
    Ok(publish_snapshot(&app))
}

/// Opens the settings window (clicking a taskbar tile).
#[tauri::command]
pub async fn open_settings(app: AppHandle) {
    show_main_window(&app);
}

/// Writes one change into a device's settings.
fn apply_device_change(
    settings: &mut DeviceSettings,
    change: DeviceChange,
    link_target: Option<String>,
) {
    match change {
        DeviceChange::Show(show) => settings.show = show,
        DeviceChange::Name(name) => {
            settings.name = name.map(|n| n.trim().to_owned()).filter(|n| !n.is_empty());
        }
        DeviceChange::Kind(kind) => settings.kind = kind,
        DeviceChange::Order(order) => settings.order = order,
        DeviceChange::Link(_) => {
            settings.same_as = link_target;
            settings.keep_separate = None;
        }
        DeviceChange::Unlink => {
            settings.same_as = None;
            settings.keep_separate = Some(true);
        }
        DeviceChange::Reset => {
            settings.show = None;
            settings.name = None;
            settings.kind = None;
            settings.icon = None;
            settings.order = None;
            settings.same_as = None;
            settings.keep_separate = None;
        }
    }
}
