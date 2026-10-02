//! Updates. On Windows Reach updates itself from GitHub Releases: it reads
//! `latest.json` from the newest published release and installs the setup.exe
//! it names, which must carry a signature from the project's updater key
//! (public half in tauri.conf.json). On Linux, Flatpak updates Reach from the
//! project's Flatpak repository instead.

use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
}

/// Whether this build updates itself. False on Linux, where Flatpak does it.
#[tauri::command]
pub fn update_supported() -> bool {
    cfg!(windows)
}

/// SSH windows that installing an update would close. RDP sessions run in
/// their own programs and keep going.
#[tauri::command]
pub fn ssh_window_count(app: AppHandle) -> usize {
    app.webview_windows().keys().filter(|label| label.starts_with("ssh-")).count()
}

#[cfg(windows)]
pub use self::imp::*;

#[cfg(windows)]
mod imp {
    use std::sync::Mutex;

    use tauri::{AppHandle, State};
    use tauri_plugin_updater::{Update, UpdaterExt};

    use super::UpdateInfo;
    use crate::error::{Result, msg};

    /// The update found by the last check, so installing gets exactly that one.
    #[derive(Default)]
    pub struct Pending(Mutex<Option<Update>>);

    /// Ask GitHub whether a newer version is published. Returns None when this
    /// is the latest.
    #[tauri::command]
    pub async fn update_check(app: AppHandle, pending: State<'_, Pending>) -> Result<Option<UpdateInfo>> {
        let updater = app.updater().map_err(|e| msg(format!("Couldn't check for updates: {e}")))?;
        let update = updater.check().await.map_err(|e| msg(format!("Couldn't check for updates: {e}")))?;
        let info = update.as_ref().map(|u| UpdateInfo { version: u.version.clone() });
        *pending.0.lock().unwrap_or_else(|e| e.into_inner()) = update;
        Ok(info)
    }

    /// Download the update, check its signature and start the installer. Reach
    /// exits as the installer starts, and the installer opens it again.
    #[tauri::command]
    pub async fn update_install(pending: State<'_, Pending>) -> Result<()> {
        let update = pending.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let update = update.ok_or_else(|| msg("No update to install. Check for updates first."))?;
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|e| msg(format!("Couldn't install the update: {e}")))
    }
}

#[cfg(not(windows))]
#[tauri::command]
pub fn update_check() -> crate::error::Result<Option<UpdateInfo>> {
    Err(crate::error::msg("Reach is updated by your software center (Flatpak)."))
}

#[cfg(not(windows))]
#[tauri::command]
pub fn update_install() -> crate::error::Result<()> {
    Err(crate::error::msg("Reach is updated by your software center (Flatpak)."))
}
