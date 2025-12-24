//! Recording overlay window management

use crate::input;
use crate::settings::{self, OverlayPosition};
use log::debug;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindowBuilder};

#[cfg(target_os = "windows")]
use tauri::WebviewWindow;

const OVERLAY_WIDTH: f64 = 200.0;
const OVERLAY_HEIGHT: f64 = 50.0;
const OVERLAY_TOP_OFFSET: f64 = 20.0;
const OVERLAY_BOTTOM_OFFSET: f64 = 60.0;

/// Overlay states
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OverlayState {
    Hidden,
    Recording,
    Transcribing,
}

/// Get the monitor that contains the mouse cursor
fn get_monitor_with_cursor(app_handle: &AppHandle) -> Option<tauri::Monitor> {
    if let Some(mouse_location) = input::get_cursor_position(app_handle) {
        if let Ok(monitors) = app_handle.available_monitors() {
            for monitor in monitors {
                if is_mouse_within_monitor(mouse_location, monitor.position(), monitor.size()) {
                    return Some(monitor);
                }
            }
        }
    }

    // Fallback to primary monitor
    app_handle.primary_monitor().ok().flatten()
}

fn is_mouse_within_monitor(
    mouse_pos: (i32, i32),
    monitor_pos: &PhysicalPosition<i32>,
    monitor_size: &PhysicalSize<u32>,
) -> bool {
    let (mouse_x, mouse_y) = mouse_pos;
    let PhysicalPosition {
        x: monitor_x,
        y: monitor_y,
    } = *monitor_pos;
    let PhysicalSize {
        width: monitor_width,
        height: monitor_height,
    } = *monitor_size;

    mouse_x >= monitor_x
        && mouse_x < (monitor_x + monitor_width as i32)
        && mouse_y >= monitor_y
        && mouse_y < (monitor_y + monitor_height as i32)
}

/// Calculate the overlay position based on settings and monitor
fn calculate_overlay_position(app_handle: &AppHandle) -> Option<(f64, f64)> {
    let monitor = get_monitor_with_cursor(app_handle)?;

    let settings = settings::get_settings(app_handle);

    // Don't show overlay if position is None
    if settings.overlay_position == OverlayPosition::None {
        return None;
    }

    let work_area = monitor.work_area();
    let scale = monitor.scale_factor();
    let work_area_width = work_area.size.width as f64 / scale;
    let work_area_height = work_area.size.height as f64 / scale;
    let work_area_x = work_area.position.x as f64 / scale;
    let work_area_y = work_area.position.y as f64 / scale;

    let x = work_area_x + (work_area_width - OVERLAY_WIDTH) / 2.0;
    let y = match settings.overlay_position {
        OverlayPosition::Top => work_area_y + OVERLAY_TOP_OFFSET,
        OverlayPosition::Bottom | OverlayPosition::None => {
            work_area_y + work_area_height - OVERLAY_HEIGHT - OVERLAY_BOTTOM_OFFSET
        }
    };

    Some((x, y))
}

/// Create the recording overlay window (hidden by default)
pub fn create_recording_overlay(app_handle: &AppHandle) {
    let (x, y) = calculate_overlay_position(app_handle).unwrap_or((100.0, 100.0));

    match WebviewWindowBuilder::new(
        app_handle,
        "recording_overlay",
        tauri::WebviewUrl::App("src/overlay/index.html".into()),
    )
    .title("Recording")
    .position(x, y)
    .resizable(false)
    .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
    .shadow(false)
    .transparent(true)
    .maximizable(false)
    .minimizable(false)
    .closable(false)
    .accept_first_mouse(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .transparent(true)
    .visible(false)
    .focused(false)
    .build()
    {
        Ok(window) => {
            debug!("Recording overlay window created");

            // On Windows, force the window to be topmost
            #[cfg(target_os = "windows")]
            force_overlay_topmost(&window);
        }
        Err(e) => {
            log::error!("Failed to create recording overlay window: {}", e);
        }
    }
}

/// Force overlay to be topmost on Windows
/// Uses raw Win32 API to ensure the window stays above all others
#[cfg(target_os = "windows")]
fn force_overlay_topmost(overlay_window: &WebviewWindow) {
    // Re-apply always_on_top to ensure it takes effect
    let _ = overlay_window.set_always_on_top(true);
}

/// Show the overlay with a specific state
pub fn show_overlay(app_handle: &AppHandle, state: OverlayState) {
    let settings = settings::get_settings(app_handle);

    // Don't show if overlay is disabled
    if settings.overlay_position == OverlayPosition::None {
        return;
    }

    let overlay = match app_handle.get_webview_window("recording_overlay") {
        Some(window) => window,
        None => {
            log::info!("Overlay window not found, creating a new one");
            create_recording_overlay(app_handle);
            match app_handle.get_webview_window("recording_overlay") {
                Some(window) => window,
                None => {
                    log::error!("Failed to create overlay window");
                    return;
                }
            }
        }
    };

    // Update position in case monitor changed
    if let Some((x, y)) = calculate_overlay_position(app_handle) {
        let _ = overlay.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
    }

    // Emit state change to frontend
    let _ = app_handle.emit("overlay-state-change", &state);

    // Show the window
    let _ = overlay.show();

    #[cfg(target_os = "windows")]
    force_overlay_topmost(&overlay);

    debug!("Overlay shown with state: {:?}", state);
}

/// Hide the overlay
pub fn hide_overlay(app_handle: &AppHandle) {
    if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
        let _ = overlay.hide();
        let _ = app_handle.emit("overlay-state-change", OverlayState::Hidden);
        debug!("Overlay hidden");
    }
}

/// Update the overlay state without changing visibility
pub fn update_overlay_state(app_handle: &AppHandle, state: OverlayState) {
    let _ = app_handle.emit("overlay-state-change", &state);
    debug!("Overlay state updated: {:?}", state);
}
