//! Input handling and keyboard simulation

use enigo::{Enigo, Key, Keyboard, Settings};
use tauri::AppHandle;

#[cfg(not(target_os = "macos"))]
use enigo::Mouse;

pub fn new_enigo() -> Result<Enigo, String> {
    Enigo::new(&Settings::default()).map_err(|e| format!("Failed to initialize Enigo: {}", e))
}

/// Get the current mouse cursor position.
/// On macOS, uses NSEvent::mouseLocation for accurate multi-monitor coordinates.
/// Returns None if getting the location fails.
#[cfg(target_os = "macos")]
pub fn get_cursor_position(_app_handle: &AppHandle) -> Option<(i32, i32)> {
    use tauri_nspanel::objc2_app_kit::{NSEvent, NSScreen};
    use tauri_nspanel::objc2_foundation::MainThreadMarker;

    // NSEvent::mouseLocation returns coordinates in screen coordinates
    // where (0,0) is bottom-left of the primary screen
    let mouse_location = NSEvent::mouseLocation();

    // Get the primary screen height to convert from bottom-left origin to top-left origin
    let mtm = MainThreadMarker::new()?;
    let screens = NSScreen::screens(mtm);

    if screens.is_empty() {
        return None;
    }

    // Primary screen is first in the array
    let primary_screen = screens.firstObject()?;
    let primary_frame = primary_screen.frame();
    let primary_height = primary_frame.size.height;

    // Convert from Cocoa coordinates (bottom-left origin) to screen coordinates (top-left origin)
    let x = mouse_location.x as i32;
    let y = (primary_height - mouse_location.y) as i32;

    Some((x, y))
}

/// Get the current mouse cursor position using Enigo.
/// Returns None if the state is not available or if getting the location fails.
#[cfg(not(target_os = "macos"))]
pub fn get_cursor_position(_app_handle: &AppHandle) -> Option<(i32, i32)> {
    let enigo = new_enigo().ok()?;
    enigo.location().ok()
}

/// Sends a Ctrl+V paste command using platform-specific virtual key codes.
/// On Windows, uses VK_V (0x56) for correct behavior regardless of keyboard layout.
pub fn send_paste_ctrl_v(enigo: &mut Enigo) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Other(0x56)); // VK_V on Windows

    #[cfg(target_os = "macos")]
    let (modifier_key, v_key_code) = (Key::Meta, Key::Other(9)); // Cmd+V on macOS

    #[cfg(target_os = "linux")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Unicode('v'));

    // Press modifier + V
    enigo
        .key(modifier_key, enigo::Direction::Press)
        .map_err(|e| format!("Failed to press modifier key: {}", e))?;

    enigo
        .key(v_key_code, enigo::Direction::Click)
        .map_err(|e| format!("Failed to click V key: {}", e))?;

    std::thread::sleep(std::time::Duration::from_millis(100));

    enigo
        .key(modifier_key, enigo::Direction::Release)
        .map_err(|e| format!("Failed to release modifier key: {}", e))?;

    Ok(())
}

/// Sends a Ctrl+Shift+V paste command (commonly used in terminals).
pub fn send_paste_ctrl_shift_v(enigo: &mut Enigo) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let (ctrl_key, shift_key, v_key_code) = (Key::Control, Key::Shift, Key::Other(0x56));

    #[cfg(target_os = "macos")]
    let (ctrl_key, shift_key, v_key_code) = (Key::Meta, Key::Shift, Key::Other(9));

    #[cfg(target_os = "linux")]
    let (ctrl_key, shift_key, v_key_code) = (Key::Control, Key::Shift, Key::Unicode('v'));

    // Press Ctrl + Shift + V
    enigo
        .key(ctrl_key, enigo::Direction::Press)
        .map_err(|e| format!("Failed to press Ctrl: {}", e))?;

    enigo
        .key(shift_key, enigo::Direction::Press)
        .map_err(|e| format!("Failed to press Shift: {}", e))?;

    enigo
        .key(v_key_code, enigo::Direction::Click)
        .map_err(|e| format!("Failed to click V: {}", e))?;

    std::thread::sleep(std::time::Duration::from_millis(100));

    enigo
        .key(shift_key, enigo::Direction::Release)
        .map_err(|e| format!("Failed to release Shift: {}", e))?;

    enigo
        .key(ctrl_key, enigo::Direction::Release)
        .map_err(|e| format!("Failed to release Ctrl: {}", e))?;

    Ok(())
}

/// Sends a Shift+Insert paste command (legacy paste method).
pub fn send_paste_shift_insert(enigo: &mut Enigo) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // VK_INSERT = 0x2D
        enigo
            .key(Key::Shift, enigo::Direction::Press)
            .map_err(|e| format!("Failed to press Shift: {}", e))?;

        enigo
            .key(Key::Other(0x2D), enigo::Direction::Click)
            .map_err(|e| format!("Failed to click Insert: {}", e))?;

        std::thread::sleep(std::time::Duration::from_millis(100));

        enigo
            .key(Key::Shift, enigo::Direction::Release)
            .map_err(|e| format!("Failed to release Shift: {}", e))?;

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = enigo;
        Err("Shift+Insert paste is only supported on Windows in this build".into())
    }
}

/// Types text directly character by character.
/// This is slower but works in more applications.
pub fn paste_text_direct(enigo: &mut Enigo, text: &str) -> Result<(), String> {
    enigo
        .text(text)
        .map_err(|e| format!("Failed to type text: {}", e))?;

    Ok(())
}
