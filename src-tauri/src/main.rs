// No console window behind the app in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
#[cfg(test)]
mod demo;
mod error;
mod model;
mod rdp;
mod ssh;
mod vault;

use tauri::{AppHandle, Emitter, Manager, PhysicalSize, WebviewWindow, WebviewWindowBuilder, WindowEvent};

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

/// Bring the main window forward, or reopen it if it was closed while SSH
/// windows kept the app running.
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let Some(config) = app.config().app.windows.iter().find(|w| w.label == "main") else {
        return;
    };
    if let Ok(window) = WebviewWindowBuilder::from_config(app, config).and_then(|b| b.build()) {
        fit_to_screen(&window);
        let _ = window.set_focus();
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

    // Linux Mint tells every GTK app to load its xapp module, which isn't inside
    // the Flatpak, so GTK prints "Failed to load module". Harmless; skip it.
    #[cfg(target_os = "linux")]
    if let Ok(modules) = std::env::var("GTK_MODULES") {
        let kept: Vec<&str> = modules.split(':').filter(|m| !m.is_empty() && *m != "xapp-gtk3-module").collect();
        // SAFETY: still before any other thread exists.
        unsafe {
            if kept.is_empty() {
                std::env::remove_var("GTK_MODULES");
            } else {
                std::env::set_var("GTK_MODULES", kept.join(":"));
            }
        }
    }

    tauri::Builder::default()
        // One copy of Reach at a time: two copies saving the same vault would
        // overwrite each other's changes. Starting it again brings this one forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main_window(app)))
        .plugin(tauri_plugin_clipboard_manager::init())
        // File picker for SSH key files; goes through the desktop portal in a Flatpak.
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(AppState::new(dir.join("vault.json")));
            if let Some(main) = app.get_webview_window("main") {
                fit_to_screen(&main);
            }

            // Auto-lock: check every few seconds; the main window shows the lock
            // screen when told the vault was locked.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    if let Some(minutes) = handle.state::<AppState>().lock_if_idle() {
                        let _ = handle.emit("vault-locked", minutes);
                    }
                }
            });
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
            commands::vault_touch,
            commands::vault_change_password,
            commands::save_settings,
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
            commands::ssh_start,
            commands::ssh_write,
            commands::ssh_resize,
            commands::ssh_answer,
            commands::open_link,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Corestart Reach");
}
