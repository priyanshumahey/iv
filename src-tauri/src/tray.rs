//! System tray management

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Debug, PartialEq)]
pub enum TrayIconState {
    Idle,
    Recording,
    Transcribing,
}

fn get_icon_path(state: &TrayIconState) -> &'static str {
    match state {
        TrayIconState::Idle => "icons/tray_idle.png",
        TrayIconState::Recording => "icons/tray_recording.png",
        TrayIconState::Transcribing => "icons/tray_transcribing.png",
    }
}

/// Create the system tray icon and menu
pub fn create_tray(app: &AppHandle) -> Result<TrayIcon, String> {
    let menu = build_tray_menu(app, &TrayIconState::Idle)?;

    let icon_path = app
        .path()
        .resolve(
            get_icon_path(&TrayIconState::Idle),
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| format!("Failed to resolve icon path: {}", e))?;

    let icon =
        Image::from_path(&icon_path).map_err(|e| format!("Failed to load tray icon: {}", e))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click { button, .. } = event {
                if button == tauri::tray::MouseButton::Left {
                    // Left click opens the main window
                    if let Some(window) = tray.app_handle().get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)
        .map_err(|e| format!("Failed to build tray icon: {}", e))
}

fn build_tray_menu(app: &AppHandle, state: &TrayIconState) -> Result<Menu<tauri::Wry>, String> {
    let version_label = format!("IndexVoice v{}", env!("CARGO_PKG_VERSION"));

    let version_item = MenuItem::with_id(app, "version", &version_label, false, None::<&str>)
        .map_err(|e| format!("Failed to create menu item: {}", e))?;

    let settings_item = MenuItem::with_id(app, "settings", "Settings...", true, Some("Ctrl+,"))
        .map_err(|e| format!("Failed to create menu item: {}", e))?;

    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, Some("Ctrl+Q"))
        .map_err(|e| format!("Failed to create menu item: {}", e))?;

    let separator = PredefinedMenuItem::separator(app)
        .map_err(|e| format!("Failed to create separator: {}", e))?;

    let separator2 = PredefinedMenuItem::separator(app)
        .map_err(|e| format!("Failed to create separator: {}", e))?;

    match state {
        TrayIconState::Recording | TrayIconState::Transcribing => {
            let cancel_item =
                MenuItem::with_id(app, "cancel", "Cancel Recording", true, None::<&str>)
                    .map_err(|e| format!("Failed to create menu item: {}", e))?;

            Menu::with_items(
                app,
                &[
                    &version_item,
                    &separator,
                    &cancel_item,
                    &separator2,
                    &settings_item,
                    &quit_item,
                ],
            )
            .map_err(|e| format!("Failed to create menu: {}", e))
        }
        TrayIconState::Idle => Menu::with_items(
            app,
            &[&version_item, &separator, &settings_item, &quit_item],
        )
        .map_err(|e| format!("Failed to create menu: {}", e)),
    }
}

/// Handle tray menu events
fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "settings" => {
            log::info!("Settings menu clicked");
            if let Some(window) = app.get_webview_window("main") {
                log::info!("Found main window, showing it");
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                log::warn!("Main window not found!");
            }
        }
        "cancel" => {
            let _ = app.emit("cancel-recording", ());
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

pub fn change_tray_icon(app: &AppHandle, state: TrayIconState) {
    if let Some(tray) = app.tray_by_id("main") {
        let icon_path = match app
            .path()
            .resolve(get_icon_path(&state), tauri::path::BaseDirectory::Resource)
        {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to resolve icon path: {}", e);
                return;
            }
        };

        if let Ok(icon) = Image::from_path(&icon_path) {
            let _ = tray.set_icon(Some(icon));
        }

        if let Ok(menu) = build_tray_menu(app, &state) {
            let _ = tray.set_menu(Some(menu));
        }
    } else {
        log::warn!("Tray icon not found");
    }
}
