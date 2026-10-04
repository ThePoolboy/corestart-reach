// No console window behind the app in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
#[cfg(test)]
mod demo;
mod error;
mod model;
mod rdp;
mod ssh;
mod update;
mod vault;

use tauri::{
    AppHandle, Emitter, Manager, Monitor, PhysicalSize, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use commands::AppState;

/// The screen a window is on, or the primary one.
fn screen_of(window: &WebviewWindow) -> Option<Monitor> {
    match window.current_monitor() {
        Ok(Some(m)) => Some(m),
        _ => window.primary_monitor().ok().flatten(),
    }
}

/// The part of a screen windows can use. Wayland may not report a work area;
/// then it's the full screen.
fn usable_area(monitor: &Monitor) -> PhysicalSize<u32> {
    let area = monitor.work_area().size;
    if area.width > 0 && area.height > 0 { area } else { *monitor.size() }
}

/// `share` of a screen area, e.g. 0.8 for 80%.
fn share_of(area: PhysicalSize<u32>, share: f64) -> PhysicalSize<u32> {
    let part = |px: u32| (f64::from(px) * share).round() as u32;
    PhysicalSize::new(part(area.width), part(area.height))
}

/// Size a window that was built hidden to `share` of its screen, centre it and
/// show it. SSH windows use 0.8, the size of an RDP window in Window mode.
pub fn show_sized(window: &WebviewWindow, share: f64) {
    if let Some(monitor) = screen_of(window) {
        let _ = window.set_size(share_of(usable_area(&monitor), share));
        let _ = window.center();
    }
    let _ = window.show();
}

/// Shrink a window that would be bigger than its screen (small laptop panels,
/// high display scaling) to 90% of the screen, and centre it.
pub fn fit_to_screen(window: &WebviewWindow) {
    let (Some(monitor), Ok(size)) = (screen_of(window), window.outer_size()) else {
        return;
    };
    let screen = usable_area(&monitor);
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

    // Wayland and X11 match windows to the desktop file (and its icon) by the
    // program name, which would be "corestart-reach". Without a match KDE shows a
    // generic Wayland icon. Name it after the desktop file before GTK starts.
    #[cfg(target_os = "linux")]
    glib::set_prgname(Some("io.github.thepoolboy.corestart-reach"));

    let builder = tauri::Builder::default();
    #[cfg(windows)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(update::Pending::default());

    builder
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
            update::update_supported,
            update::update_check,
            update::update_install,
            update::ssh_window_count,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Corestart Reach");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_windows_take_a_share_of_the_screen() {
        assert_eq!(share_of(PhysicalSize::new(1920, 1040), 0.8), PhysicalSize::new(1536, 832));
        assert_eq!(share_of(PhysicalSize::new(1366, 728), 0.8), PhysicalSize::new(1093, 582));
    }
}
