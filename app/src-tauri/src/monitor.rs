//! Background battery polling and the latest snapshot the UI shows.
//!
//! A worker thread re-detects devices every `poll_seconds`, records them in
//! the config, and pushes a fresh [`Snapshot`] to the window and the tray.
//! The config is always read from disk, so edits made with `bmp-probe` or
//! by hand show up on the next tick.

use std::sync::Mutex;
use std::thread;

use bmp_battery::{Device, detect_all_devices};
use bmp_config::rows::{DeviceRow, Tile, build_device_rows, build_tiles};
use bmp_config::{
    TileLayout, find_config_path, load_config, read_poll_interval, remember_devices, save_config,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::taskbar::is_taskbar_light;
use crate::tray::update_tray_tooltip;

pub const SNAPSHOT_EVENT: &str = "snapshot";

/// Everything the settings window needs to draw itself.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Snapshot {
    pub tiles: Vec<Tile>,
    pub rows: Vec<DeviceRow>,
    pub poll_seconds: u64,
    pub tile_layout: TileLayout,
    /// Saved spot of the tiles (see [`bmp_config::Config::tile_offset`]).
    pub tile_offset: Option<i32>,
    /// Tiles live inside the taskbar window instead of floating over it.
    pub tile_embed: bool,
    /// The taskbar's own theme, which the tiles follow.
    pub taskbar_light: bool,
    pub config_path: String,
    /// Set when the config file couldn't be read or written.
    pub error: Option<String>,
}

/// Devices from the last detection, shared between the worker and commands.
#[derive(Default)]
pub struct MonitorState {
    pub devices: Mutex<Vec<Device>>,
}

/// Starts the worker thread that re-reads batteries forever.
pub fn start_polling(app: AppHandle) {
    thread::spawn(move || {
        loop {
            refresh_devices_now(&app);
            let interval = load_config()
                .map(|c| read_poll_interval(&c))
                .unwrap_or_else(|_| read_poll_interval(&Default::default()));
            thread::sleep(interval);
        }
    });
}

/// Detects devices right now, remembers them, and pushes a new snapshot.
pub fn refresh_devices_now(app: &AppHandle) -> Snapshot {
    let devices = detect_all_devices();
    let mut error = None;
    match load_config() {
        Ok(mut config) => {
            if remember_devices(&mut config, &devices)
                && let Err(e) = save_config(&config)
            {
                error = Some(e.to_string());
            }
        }
        Err(e) => error = Some(e.to_string()),
    }
    *app.state::<MonitorState>().devices.lock().unwrap() = devices;
    let mut snapshot = publish_snapshot(app);
    if snapshot.error.is_none() {
        snapshot.error = error;
    }
    snapshot
}

/// Rebuilds the snapshot from the last detected devices (no new detection)
/// and sends it to the window and the tray tooltip.
pub fn publish_snapshot(app: &AppHandle) -> Snapshot {
    let snapshot = build_snapshot(app);
    let _ = app.emit(SNAPSHOT_EVENT, &snapshot);
    update_tray_tooltip(app, &snapshot.tiles);
    snapshot
}

/// Combines the last detected devices with the config into a snapshot.
pub fn build_snapshot(app: &AppHandle) -> Snapshot {
    let devices = app.state::<MonitorState>().devices.lock().unwrap().clone();
    let config_path = find_config_path().display().to_string();
    match load_config() {
        Ok(config) => Snapshot {
            tiles: build_tiles(&config, devices.clone()),
            rows: build_device_rows(&config, &devices),
            poll_seconds: read_poll_interval(&config).as_secs(),
            tile_layout: config.tile_layout.unwrap_or_default(),
            tile_offset: config.tile_offset,
            tile_embed: config.tile_embed.unwrap_or(true),
            taskbar_light: is_taskbar_light(),
            config_path,
            error: None,
        },
        Err(e) => Snapshot {
            taskbar_light: is_taskbar_light(),
            config_path,
            error: Some(e.to_string()),
            ..Default::default()
        },
    }
}
