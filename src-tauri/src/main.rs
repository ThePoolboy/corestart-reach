// No console window behind the app in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod model;
mod rdp;
mod ssh;
mod vault;

use tauri::{Manager, PhysicalSize, WebviewWindow, WindowEvent};

use commands::AppState;

/// Shrink a window that would be bigger than its screen (small laptop panels,
/// high display scaling) to 90% of the screen, and centre it.
pub fn fit_to_screen(window: &WebviewWindow) {
    let monitor = match window.current_monitor() {
        Ok(Some(m)) => Some(m),
        _ => window.primary_monitor().ok().flatten(),
    };
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else {
        return;
    };
    // Wayland may not report a work area; fall back to the full screen.
    let area = monitor.work_area().size;
    let screen = if area.width > 0 && area.height > 0 { area } else { *monitor.size() };
    let max_w = screen.width * 9 / 10;
    let max_h = screen.height * 9 / 10;
    if size.width > max_w || size.height > max_h {
        let _ = window.set_size(PhysicalSize::new(size.width.min(max_w), size.height.min(max_h)));
        let _ = window.center();
    }
}

fn main() {
    // WebKitGTK's DMA-BUF renderer crashes on NVIDIA + Wayland ("Error 71 (Protocol
    // error) dispatching to Wayland display"). Turn it off unless the user chose.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: first thing in main, before any other thread exists.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(AppState::new(dir.join("vault.json")));
            if let Some(main) = app.get_webview_window("main") {
                fit_to_screen(&main);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing an SSH window ends its session.
            if let WindowEvent::Destroyed = event {
                if let Some(session) = window.label().strip_prefix("ssh-") {
                    window.state::<AppState>().ssh().close(session);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::vault_create,
            commands::vault_unlock,
            commands::vault_lock,
            commands::get_tree,
            commands::save_connection,
            commands::duplicate_connection,
            commands::delete_connection,
            commands::save_folder,
            commands::delete_folder,
            commands::move_item,
            commands::save_credential,
            commands::delete_credential,
            commands::connect,
            commands::quick_connect,
            commands::ssh_title,
            commands::ssh_start,
            commands::ssh_write,
            commands::ssh_resize,
            commands::ssh_answer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Corestart Reach");
}
