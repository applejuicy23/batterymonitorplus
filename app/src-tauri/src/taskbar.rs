//! The tiles window docked on the Windows taskbar.
//!
//! Windows 11 has no API to put custom panels into the taskbar. By default
//! the tiles window is made a child of the taskbar window (`Shell_TrayWnd`),
//! so it moves and auto-hides together with it and never ends up below it.
//! The user can switch that off; then the tiles float over the taskbar as a
//! borderless, always-on-top tool window instead.
//!
//! A worker re-docks the window twice a second, because the taskbar can
//! change size or come back after an explorer restart. An explorer restart
//! also destroys the embedded tiles window (children die with their
//! parent), so the worker re-creates it from the app config.
//!
//! By default the tiles go left of the tray. Taskbar restylers (Windhawk's
//! taskbar styler, etc.) move the tray visually while Windows still reports
//! the old spot, so the user can drag the tiles and the spot is saved as an
//! offset from the taskbar's right edge.

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GA_PARENT, GWL_EXSTYLE, GWL_STYLE, GetAncestor,
    GetWindowLongPtrW, GetWindowRect, HWND_TOP, HWND_TOPMOST, IsWindowVisible, SWP_NOACTIVATE,
    SetParent, SetWindowLongPtrW, SetWindowPos, WS_CHILD, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_POPUP,
};
use windows::core::w;

pub const TILES_WINDOW: &str = "tiles";

const DOCK_EVERY: Duration = Duration::from_millis(500);
/// Gap between the tiles and the tray, in logical pixels.
const TRAY_GAP: f64 = 4.0;
/// Below this many visible pixels the taskbar counts as auto-hidden.
const MIN_VISIBLE_TASKBAR: i32 = 8;

/// What the tiles window needs to know to dock itself.
#[derive(Default)]
pub struct TilesState {
    /// Width the tiles page asked for, in logical pixels (0 = nothing to show).
    pub width: Mutex<f64>,
    /// Logical pixels from the taskbar's right edge; `None` = next to the tray.
    pub offset: Mutex<Option<f64>>,
    /// Live inside the taskbar window instead of floating over it.
    pub embed: Mutex<bool>,
}

/// Where the tiles window should go, in physical screen pixels.
struct DockPlace {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

/// The taskbar and the tray area as Windows reports them.
struct TaskbarRects {
    hwnd: HWND,
    taskbar: RECT,
    tray_left: i32,
}

/// Loads the saved spot and mode, sets up the tiles window and starts the
/// worker that keeps it docked.
pub fn start_docking(app: AppHandle, offset: Option<i32>, embed: bool) {
    let state = app.state::<TilesState>();
    *state.offset.lock().unwrap() = offset.map(f64::from);
    *state.embed.lock().unwrap() = embed;
    if let Some(window) = app.get_webview_window(TILES_WINDOW) {
        make_window_unfocusable(&window);
    }
    thread::spawn(move || {
        loop {
            if app.get_webview_window(TILES_WINDOW).is_none() {
                recreate_tiles_window(&app);
            }
            dock_tiles_window(&app);
            thread::sleep(DOCK_EVERY);
        }
    });
}

/// Builds the tiles window again from `tauri.conf.json` after explorer took
/// it down with the old taskbar.
fn recreate_tiles_window(app: &AppHandle) {
    let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == TILES_WINDOW)
    else {
        return;
    };
    *app.state::<TilesState>().width.lock().unwrap() = 0.0;
    if let Ok(window) = WebviewWindowBuilder::from_config(app, config).and_then(|b| b.build()) {
        make_window_unfocusable(&window);
    }
}

/// Stores the width the tiles page needs and re-docks right away.
pub fn resize_tiles_window(app: &AppHandle, width: f64) {
    *app.state::<TilesState>().width.lock().unwrap() = width;
    dock_tiles_window(app);
}

/// Switches between living inside the taskbar and floating over it.
pub fn embed_tiles_window(app: &AppHandle, embed: bool) {
    *app.state::<TilesState>().embed.lock().unwrap() = embed;
    dock_tiles_window(app);
}

/// Shifts the tiles sideways while the user drags them (`dx` in logical
/// pixels, positive = right) and returns the new offset.
pub fn move_tiles_window(app: &AppHandle, dx: f64) -> Option<f64> {
    let current = read_tiles_offset(app)?;
    let max = read_max_offset(app).unwrap_or(f64::MAX);
    let moved = (current - dx).clamp(0.0, max.max(0.0));
    *app.state::<TilesState>().offset.lock().unwrap() = Some(moved);
    dock_tiles_window(app);
    Some(moved)
}

/// Sets the saved spot (`None` puts the tiles back next to the tray).
pub fn place_tiles_window(app: &AppHandle, offset: Option<i32>) {
    *app.state::<TilesState>().offset.lock().unwrap() = offset.map(f64::from);
    dock_tiles_window(app);
}

/// Works out the tiles' current offset from the taskbar's right edge, even
/// when they're still in the automatic spot.
pub fn read_tiles_offset(app: &AppHandle) -> Option<f64> {
    if let Some(offset) = *app.state::<TilesState>().offset.lock().unwrap() {
        return Some(offset);
    }
    let window = app.get_webview_window(TILES_WINDOW)?;
    let rect = read_window_rect(window.hwnd().ok()?)?;
    let rects = find_taskbar_rects(false)?;
    let scale = window.scale_factor().unwrap_or(1.0);
    Some(f64::from(rects.taskbar.right - rect.right) / scale)
}

/// Largest offset that still keeps the tiles on the taskbar.
fn read_max_offset(app: &AppHandle) -> Option<f64> {
    let window = app.get_webview_window(TILES_WINDOW)?;
    let rects = find_taskbar_rects(false)?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let width = *app.state::<TilesState>().width.lock().unwrap();
    Some(f64::from(rects.taskbar.right - rects.taskbar.left) / scale - width)
}

/// Moves the tiles window to its spot (inside or over the taskbar); hides it
/// when there is nothing to show or no taskbar to sit on.
pub fn dock_tiles_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(TILES_WINDOW) else {
        return;
    };
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let state = app.state::<TilesState>();
    let width = *state.width.lock().unwrap();
    let offset = *state.offset.lock().unwrap();
    let embed = *state.embed.lock().unwrap();
    let scale = window.scale_factor().unwrap_or(1.0);

    // An embedded window hides with the taskbar by itself; a floating one
    // has to be hidden by hand when the taskbar slides away.
    let rects = if width >= 1.0 {
        find_taskbar_rects(!embed)
    } else {
        None
    };
    let Some(rects) = rects else {
        let _ = window.hide();
        return;
    };
    let place = find_dock_place(&rects, width * scale, offset.map(|o| o * scale), TRAY_GAP * scale);

    if embed {
        attach_to_taskbar(hwnd, rects.hwnd);
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOP),
                place.x - rects.taskbar.left,
                place.y - rects.taskbar.top,
                place.width,
                place.height,
                SWP_NOACTIVATE,
            );
        }
    } else {
        detach_from_taskbar(hwnd);
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
    }
    if !window.is_visible().unwrap_or(false) {
        let _ = window.show();
    }
}

/// Makes the tiles window a child of the taskbar, unless it already is.
fn attach_to_taskbar(hwnd: HWND, taskbar: HWND) {
    unsafe {
        if GetAncestor(hwnd, GA_PARENT) == taskbar {
            return;
        }
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let child_style = (style & !(WS_POPUP.0 as isize)) | WS_CHILD.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_STYLE, child_style);
        let _ = SetParent(hwnd, Some(taskbar));
    }
}

/// Turns the tiles window back into a free-floating popup, if it was a
/// child of the taskbar.
fn detach_from_taskbar(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        if style & WS_CHILD.0 as isize == 0 {
            return;
        }
        let _ = SetParent(hwnd, None);
        let popup_style = (style & !(WS_CHILD.0 as isize)) | WS_POPUP.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_STYLE, popup_style);
    }
}

/// Works out the tiles' rectangle on screen: at the saved offset from the
/// taskbar's right edge, or left of the tray; always as tall as the taskbar.
fn find_dock_place(rects: &TaskbarRects, width: f64, offset: Option<f64>, gap: f64) -> DockPlace {
    let taskbar = rects.taskbar;
    let width = width.ceil() as i32;
    let right = match offset {
        Some(offset) => taskbar.right - offset.round() as i32,
        None => rects.tray_left - gap.round() as i32,
    };
    let x = (right - width).clamp(taskbar.left, (taskbar.right - width).max(taskbar.left));
    DockPlace {
        x,
        y: taskbar.top,
        width,
        height: taskbar.bottom - taskbar.top,
    }
}

/// Finds the taskbar and its tray; `None` when the taskbar is missing or
/// hidden, or (with `require_on_screen`) slid off the screen by auto-hide.
fn find_taskbar_rects(require_on_screen: bool) -> Option<TaskbarRects> {
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok()?;
    if !unsafe { IsWindowVisible(taskbar) }.as_bool() {
        return None;
    }
    let taskbar_rect = read_window_rect(taskbar)?;
    if require_on_screen && !is_taskbar_on_screen(taskbar, &taskbar_rect) {
        return None;
    }
    let tray = unsafe { FindWindowExW(Some(taskbar), None, w!("TrayNotifyWnd"), None) }.ok();
    let tray_left = tray
        .and_then(read_window_rect)
        .map(|r| r.left)
        .unwrap_or(taskbar_rect.right);
    Some(TaskbarRects {
        hwnd: taskbar,
        taskbar: taskbar_rect,
        tray_left,
    })
}

/// Checks that the taskbar is really showing, not slid off the edge of its
/// monitor by auto-hide.
fn is_taskbar_on_screen(taskbar: HWND, rect: &RECT) -> bool {
    let monitor = unsafe { MonitorFromWindow(taskbar, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return true;
    }
    let screen = info.rcMonitor;
    let visible_top = rect.top.max(screen.top);
    let visible_bottom = rect.bottom.min(screen.bottom);
    let fully_visible = visible_bottom - visible_top >= rect.bottom - rect.top;
    fully_visible && visible_bottom - visible_top >= MIN_VISIBLE_TASKBAR
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
