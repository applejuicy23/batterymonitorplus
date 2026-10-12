//! BatteryMonitor+ desktop app: tray icon, background battery polling and
//! the settings window.

mod commands;
mod monitor;
mod tray;

use tauri::WindowEvent;

use crate::monitor::{MonitorState, start_polling};
use crate::tray::{MAIN_WINDOW, create_tray};

/// Builds and runs the app; closing the window only hides it to the tray.
pub fn run_app() {
    tauri::Builder::default()
        .manage(MonitorState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::refresh_devices,
            commands::update_device,
            commands::set_poll_seconds,
        ])
        .setup(|app| {
            create_tray(app.handle())?;
            start_polling(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == MAIN_WINDOW
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running BatteryMonitor+");
}
