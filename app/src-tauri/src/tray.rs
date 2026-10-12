//! The tray icon: left click opens the settings window, the menu has
//! "Settings", "Refresh now" and "Quit", and the tooltip lists the batteries.

use bmp_config::rows::Tile;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::monitor::refresh_devices_now;

pub const TRAY_ID: &str = "main";
pub const MAIN_WINDOW: &str = "main";

/// Creates the tray icon with its menu and click handling.
pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refresh now", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &refresh, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("BatteryMonitor+")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => show_main_window(app),
            "refresh" => {
                let app = app.clone();
                std::thread::spawn(move || {
                    refresh_devices_now(&app);
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Brings the settings window to the front, un-hiding it if needed.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Puts a one-line battery summary into the tray tooltip,
/// e.g. "Razer Viper V3 Pro SE 16% · YUNZII B75 PRO MAX 80%".
pub fn update_tray_tooltip(app: &AppHandle, tiles: &[Tile]) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let parts: Vec<String> = tiles
        .iter()
        .filter_map(|t| t.device.battery.map(|b| format!("{} {b}%", t.device.name)))
        .collect();
    let text = if parts.is_empty() {
        "BatteryMonitor+".to_owned()
    } else {
        parts.join(" · ")
    };
    let _ = tray.set_tooltip(Some(text));
}
