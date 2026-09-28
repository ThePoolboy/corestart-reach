// No console window behind the app in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod model;
mod rdp;
mod ssh;
mod vault;

use tauri::{Manager, WindowEvent};

use commands::AppState;

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
            commands::save_credential,
            commands::delete_credential,
            commands::connect,
            commands::ssh_title,
            commands::ssh_start,
            commands::ssh_write,
            commands::ssh_resize,
            commands::ssh_answer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Corestart Reach");
}
