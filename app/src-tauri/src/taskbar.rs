//! The tiles window docked on the Windows taskbar, just left of the tray.
//!
//! Windows 11 has no API to put custom panels into the taskbar, so the tiles
//! live in a borderless, transparent, always-on-top tool window that sits
//! over the empty part of the taskbar. A worker re-docks it every second,
//! because the taskbar can move, change height or come back after an
//! explorer restart, and clicking the taskbar pushes it below.

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewWindow};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GWL_EXSTYLE, GetWindowLongPtrW, GetWindowRect, HWND_TOPMOST,
    SWP_NOACTIVATE, SetWindowLongPtrW, SetWindowPos, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};
use windows::core::w;

pub const TILES_WINDOW: &str = "tiles";

const DOCK_EVERY: Duration = Duration::from_secs(1);
/// Gap between the tiles and the tray, in logical pixels.
const TRAY_GAP: f64 = 4.0;

/// Width the tiles page asked for, in logical pixels (0 = nothing to show).
#[derive(Default)]
pub struct TilesState {
    pub width: Mutex<f64>,
}

/// Where the tiles window should go, in physical screen pixels.
struct DockPlace {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

/// Turns the tiles window into a tool window that never takes focus, then
/// starts the worker that keeps it docked.
pub fn start_docking(app: AppHandle) {
    if let Some(window) = app.get_webview_window(TILES_WINDOW) {
        make_window_unfocusable(&window);
    }
    thread::spawn(move || {
        loop {
            dock_tiles_window(&app);
            thread::sleep(DOCK_EVERY);
        }
    });
}

/// Stores the width the tiles page needs and re-docks right away.
pub fn resize_tiles_window(app: &AppHandle, width: f64) {
    *app.state::<TilesState>().width.lock().unwrap() = width;
    dock_tiles_window(app);
}

/// Moves the tiles window next to the tray and keeps it on top; hides it
/// when there is nothing to show or no taskbar (explorer restarting).
pub fn dock_tiles_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(TILES_WINDOW) else {
        return;
    };
    let width = *app.state::<TilesState>().width.lock().unwrap();
    let scale = window.scale_factor().unwrap_or(1.0);
    let place = if width >= 1.0 {
        find_dock_place(width * scale, TRAY_GAP * scale)
    } else {
        None
    };
    let Some(place) = place else {
        let _ = window.hide();
        return;
    };
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            place.x,
            place.y,
            place.width,
            place.height,
            SWP_NOACTIVATE,
        );
    }
    if !window.is_visible().unwrap_or(false) {
        let _ = window.show();
    }
}

/// Works out the spot left of the tray area, as tall as the taskbar.
fn find_dock_place(width: f64, gap: f64) -> Option<DockPlace> {
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok()?;
    let taskbar_rect = read_window_rect(taskbar)?;
    let tray = unsafe { FindWindowExW(Some(taskbar), None, w!("TrayNotifyWnd"), None) }.ok();
    let tray_left = tray
        .and_then(read_window_rect)
        .map(|r| r.left)
        .unwrap_or(taskbar_rect.right);
    let width = width.ceil() as i32;
    Some(DockPlace {
        x: tray_left - width - gap.round() as i32,
        y: taskbar_rect.top,
        width,
        height: taskbar_rect.bottom - taskbar_rect.top,
    })
}

/// Reads a window's rectangle on screen.
fn read_window_rect(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
    Some(rect)
}

/// Adds the tool-window and no-activate styles: no Alt+Tab entry, and
/// clicking a tile doesn't steal focus from the app you're working in.
fn make_window_unfocusable(window: &WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let extra = (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | extra);
    }
}

/// Checks whether the taskbar uses the light theme (it can differ from the
/// apps theme, so the tiles can't rely on `prefers-color-scheme`).
pub fn is_taskbar_light() -> bool {
    let mut value: u32 = 0;
    let mut size = size_of::<u32>() as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    result.is_ok() && value != 0
}
