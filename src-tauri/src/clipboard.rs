//! Clipboard handling and pasting functionality

use crate::input::{self};
use crate::settings::{get_settings, ClipboardHandling, PasteMethod};
use log::info;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Pastes text using the clipboard: saves current content, writes text, sends paste keystroke, restores clipboard.
fn paste_via_clipboard(
    app_handle: &AppHandle,
    text: &str,
    paste_method: &PasteMethod,
) -> Result<(), String> {
    let mut enigo = input::new_enigo()?;

    let clipboard = app_handle.clipboard();

    // Save current clipboard content
    let original_content = clipboard.read_text().unwrap_or_default();

    // Write our text to clipboard
    clipboard
        .write_text(text)
        .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

    // Small delay to ensure clipboard is ready
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Send paste keystroke
    match paste_method {
        PasteMethod::CtrlV => input::send_paste_ctrl_v(&mut enigo)?,
        PasteMethod::CtrlShiftV => input::send_paste_ctrl_shift_v(&mut enigo)?,
        PasteMethod::ShiftInsert => input::send_paste_shift_insert(&mut enigo)?,
        _ => return Err("Invalid paste method for clipboard paste".into()),
    }

    // Small delay after paste
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Restore original clipboard content
    if !original_content.is_empty() {
        clipboard
            .write_text(&original_content)
            .map_err(|e| format!("Failed to restore clipboard: {}", e))?;
    }

    Ok(())
}

/// Main paste function - routes to appropriate paste method based on settings
pub fn paste(text: String, app_handle: &AppHandle) -> Result<(), String> {
    let settings = get_settings(app_handle);
    let paste_method = settings.paste_method;

    // Append trailing space if setting is enabled
    let text = if settings.append_trailing_space {
        format!("{} ", text)
    } else {
        text
    };

    info!("Using paste method: {:?}", paste_method);

    // Perform the paste operation
    match paste_method {
        PasteMethod::None => {
            info!("PasteMethod::None selected - skipping paste action");
        }
        PasteMethod::Direct => {
            let mut enigo = input::new_enigo()?;
            input::paste_text_direct(&mut enigo, &text)?;
        }
        PasteMethod::CtrlV | PasteMethod::CtrlShiftV | PasteMethod::ShiftInsert => {
            paste_via_clipboard(app_handle, &text, &paste_method)?;
        }
    }

    // After pasting, optionally copy to clipboard based on settings
    if settings.clipboard_handling == ClipboardHandling::CopyToClipboard {
        let clipboard = app_handle.clipboard();
        clipboard
            .write_text(&text)
            .map_err(|e| format!("Failed to copy to clipboard: {}", e))?;
    }

    Ok(())
}
