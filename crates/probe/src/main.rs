//! `bmp-probe` — BatteryMonitor+ device detector and settings, before there's a UI.
//!
//!   bmp-probe                      all devices, shown or hidden
//!   bmp-probe tiles [--json]       only what the taskbar tiles will show
//!   bmp-probe hide yunzii          devices can be picked by id or part of the name
//!   bmp-probe rename viper "Mouse"

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use bmp_battery::{Device, DeviceKind, detect_all_devices};
use bmp_config::{
    Config, apply_overrides, arrange_devices, choose_icon, find_config_path, is_device_shown,
    load_config, remember_devices, save_config,
};
use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "bmp-probe",
    about = "BatteryMonitor+ device detector and settings"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// All detected and remembered devices, shown or hidden (default)
    List {
        #[arg(long)]
        json: bool,
    },
    /// Only what the taskbar tiles will show, in tile order
    Tiles {
        #[arg(long)]
        json: bool,
    },
    /// Show a device in the tiles
    Show { device: String },
    /// Hide a device from the tiles
    Hide { device: String },
    /// Give a device your own name
    Rename { device: String, name: String },
    /// Override the device kind (laptop, mouse, keyboard, headset, gamepad, other)
    Kind { device: String, kind: DeviceKind },
    /// Set the icon: a built-in name or a path to an SVG/PNG
    Icon { device: String, icon: String },
    /// Set the tile position; lower comes first
    Order { device: String, order: i32 },
    /// Forget your settings for a device (show/name/kind/icon/order)
    Reset { device: String },
    /// Print where the config file is
    Config,
}

/// One row of `list --json`.
#[derive(Serialize)]
struct ListEntry {
    id: String,
    name: String,
    kind: DeviceKind,
    icon: String,
    shown: bool,
    /// Found right now (vs. only remembered in the config).
    detected: bool,
    device: Option<Device>,
}

/// One tile of `tiles --json`, the shape the taskbar UI will consume.
#[derive(Serialize)]
struct TileEntry {
    #[serde(flatten)]
    device: Device,
    icon: String,
    live: bool,
}

/// Detects devices, remembers them in the config, then runs the subcommand.
fn main() -> ExitCode {
    let cli = Cli::parse();
    match run_command(cli.command.unwrap_or(Command::List { json: false })) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Does the actual work of one subcommand.
fn run_command(command: Command) -> Result<(), String> {
    if let Command::Config = command {
        println!("{}", find_config_path().display());
        return Ok(());
    }

    let devices = detect_all_devices();
    let mut config = load_config().map_err(|e| e.to_string())?;
    if remember_devices(&mut config, &devices) {
        save_config(&config).map_err(|e| e.to_string())?;
    }

    match command {
        Command::List { json } => print_device_list(&config, &devices, json),
        Command::Tiles { json } => print_tiles(&config, devices, json),
        Command::Show { device } => {
            change_device_settings(&mut config, &devices, &device, |s| s.show = Some(true))
        }
        Command::Hide { device } => {
            change_device_settings(&mut config, &devices, &device, |s| s.show = Some(false))
        }
        Command::Rename { device, name } => {
            change_device_settings(&mut config, &devices, &device, |s| s.name = Some(name))
        }
        Command::Kind { device, kind } => {
            change_device_settings(&mut config, &devices, &device, |s| s.kind = Some(kind))
        }
        Command::Icon { device, icon } => {
            change_device_settings(&mut config, &devices, &device, |s| s.icon = Some(icon))
        }
        Command::Order { device, order } => {
            change_device_settings(&mut config, &devices, &device, |s| s.order = Some(order))
        }
        Command::Reset { device } => change_device_settings(&mut config, &devices, &device, |s| {
            s.show = None;
            s.name = None;
            s.kind = None;
            s.icon = None;
            s.order = None;
        }),
        Command::Config => unreachable!(),
    }
}

/// Prints every device the config knows (detected now or remembered),
/// marking which ones go into the tiles.
fn print_device_list(config: &Config, devices: &[Device], json: bool) -> Result<(), String> {
    let mut entries = Vec::new();
    for (id, settings) in &config.devices {
        let detected = devices.iter().find(|d| &d.id == id);
        let entry = match detected {
            Some(device) => {
                let shown_as = apply_overrides(config, device.clone());
                ListEntry {
                    id: id.clone(),
                    name: shown_as.name,
                    kind: shown_as.kind,
                    icon: choose_icon(config, device),
                    shown: is_device_shown(config, device),
                    detected: true,
                    device: Some(device.clone()),
                }
            }
            None => ListEntry {
                id: id.clone(),
                name: settings
                    .name
                    .clone()
                    .or_else(|| settings.detected_name.clone())
                    .unwrap_or_else(|| id.clone()),
                kind: settings
                    .kind
                    .or(settings.detected_kind)
                    .unwrap_or(DeviceKind::Other),
                icon: settings.icon.clone().unwrap_or_default(),
                shown: settings.show.unwrap_or(false),
                detected: false,
                device: None,
            },
        };
        entries.push(entry);
    }
    entries.sort_by(|a, b| b.shown.cmp(&a.shown).then(a.kind.cmp(&b.kind)));

    if json {
        let text = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
        println!("{text}");
        return Ok(());
    }
    if entries.is_empty() {
        println!("no devices found");
        return Ok(());
    }
    println!(
        "{:<7} {:<18} {:<9} {:<28} {:>7}  STATE",
        "TILES", "ID", "KIND", "NAME", "BATTERY"
    );
    for e in &entries {
        let battery = e
            .device
            .as_ref()
            .and_then(|d| d.battery)
            .map_or("-".to_owned(), |b| format!("{b}%"));
        let state = e
            .device
            .as_ref()
            .map_or("not found".to_owned(), describe_device_state);
        println!(
            "{:<7} {:<18} {:<9} {:<28} {:>7}  {state}",
            if e.shown { "shown" } else { "hidden" },
            e.id,
            e.kind,
            e.name,
            battery
        );
    }
    println!("\nconfig: {}", find_config_path().display());
    Ok(())
}

/// Prints only the devices that go into the tiles, in tile order.
fn print_tiles(config: &Config, devices: Vec<Device>, json: bool) -> Result<(), String> {
    let tiles: Vec<TileEntry> = arrange_devices(config, devices)
        .into_iter()
        .map(|d| TileEntry {
            icon: choose_icon(config, &d),
            live: d.has_live_battery(),
            device: d,
        })
        .collect();
    if json {
        let text = serde_json::to_string_pretty(&tiles).map_err(|e| e.to_string())?;
        println!("{text}");
        return Ok(());
    }
    if tiles.is_empty() {
        println!("no tiles — every device is hidden or has no battery");
    }
    for t in &tiles {
        let battery = t.device.battery.map_or("-".to_owned(), |b| format!("{b}%"));
        println!(
            "[{}] {:>4}  {:<28} {}",
            t.icon,
            battery,
            t.device.name,
            describe_device_state(&t.device)
        );
    }
    Ok(())
}

/// Finds the device the user meant and applies a settings change to it.
fn change_device_settings(
    config: &mut Config,
    devices: &[Device],
    query: &str,
    change: impl FnOnce(&mut bmp_config::DeviceSettings),
) -> Result<(), String> {
    let id = resolve_device_id(config, devices, query)?;
    change(config.devices.entry(id.clone()).or_default());
    save_config(config).map_err(|e| e.to_string())?;
    let shown = devices
        .iter()
        .find(|d| d.id == id)
        .map(|d| is_device_shown(config, d));
    let settings = &config.devices[&id];
    println!(
        "{id}: {}",
        toml::to_string(settings)
            .unwrap_or_default()
            .trim()
            .replace('\n', ", ")
    );
    if let Some(shown) = shown {
        println!("{}", if shown { "in tiles" } else { "not in tiles" });
    }
    Ok(())
}

/// Matches what the user typed against device ids and names: an exact id
/// wins, otherwise a unique case-insensitive part of an id or name.
fn resolve_device_id(config: &Config, devices: &[Device], query: &str) -> Result<String, String> {
    let mut known: Vec<(String, Vec<String>)> = config
        .devices
        .iter()
        .map(|(id, s)| {
            let names = [s.name.clone(), s.detected_name.clone()]
                .into_iter()
                .flatten()
                .collect();
            (id.clone(), names)
        })
        .collect();
    for d in devices {
        if !known.iter().any(|(id, _)| id == &d.id) {
            known.push((d.id.clone(), vec![d.name.clone()]));
        }
    }

    let wanted = query.to_lowercase();
    if let Some((id, _)) = known.iter().find(|(id, _)| id.to_lowercase() == wanted) {
        return Ok(id.clone());
    }
    let matches: Vec<&String> = known
        .iter()
        .filter(|(id, names)| {
            id.to_lowercase().contains(&wanted)
                || names.iter().any(|n| n.to_lowercase().contains(&wanted))
        })
        .map(|(id, _)| id)
        .collect();
    match matches.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(format!(
            "no device matches '{query}' — run `bmp-probe list`"
        )),
        many => Err(format!(
            "'{query}' matches several devices: {}",
            many.iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Describes connection/charging in a few words, e.g. "disconnected,
/// battery from 3d ago" so stale values are obvious.
fn describe_device_state(device: &Device) -> String {
    match (device.connected, device.charging) {
        (Some(false), _) => match device.updated_at.filter(|_| device.battery.is_some()) {
            Some(at) => format!("disconnected, battery from {}", format_age(at)),
            None => "disconnected".to_owned(),
        },
        (_, Some(true)) => "charging".to_owned(),
        (_, Some(false)) => "on battery".to_owned(),
        (Some(true), None) => "connected".to_owned(),
        (None, None) => "connection unknown".to_owned(),
    }
}

/// Turns a unix timestamp into "5m ago" / "3h ago" / "2d ago".
fn format_age(unix_seconds: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let age = now.saturating_sub(unix_seconds);
    match age {
        0..60 => "just now".to_owned(),
        60..3600 => format!("{}m ago", age / 60),
        3600..86400 => format!("{}h ago", age / 3600),
        _ => format!("{}d ago", age / 86400),
    }
}
