//! Commands the frontend calls through `invoke`.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::error::{Result, msg};
use crate::model::{
    ConnectionInput, CredentialInput, FolderInput, Protocol, RdpScreen, Resolved, Settings, Tree,
    VaultData, parse_address,
};
use crate::ssh::{Sessions, SshAnswer, SshEvent};
use crate::{rdp, vault};

pub struct AppState {
    vault_path: PathBuf,
    vault: Mutex<Option<vault::Unlocked>>,
    ssh: Sessions,
    /// Last time you used the main window, for auto-lock. Wall-clock time, so
    /// time spent asleep or suspended counts as idle.
    last_activity: Mutex<SystemTime>,
}

impl AppState {
    pub fn new(vault_path: PathBuf) -> Self {
        Self {
            vault_path,
            vault: Mutex::new(None),
            ssh: Sessions::default(),
            last_activity: Mutex::new(SystemTime::now()),
        }
    }

    pub fn ssh(&self) -> &Sessions {
        &self.ssh
    }

    fn lock(&self) -> MutexGuard<'_, Option<vault::Unlocked>> {
        self.vault.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn touch(&self) {
        *self.last_activity.lock().unwrap_or_else(|e| e.into_inner()) = SystemTime::now();
    }

    /// Lock the vault if it has been idle for longer than the auto-lock setting.
    /// Returns that setting (in minutes) when it locked. Open SSH and RDP
    /// sessions keep running; only opening new ones needs the password again.
    pub fn lock_if_idle(&self) -> Option<u32> {
        let mut guard = self.lock();
        let minutes = guard.as_ref()?.data.settings.auto_lock_minutes;
        if minutes == 0 {
            return None;
        }
        let last = *self.last_activity.lock().unwrap_or_else(|e| e.into_inner());
        // A clock set backwards reads as "just now", never as a reason to lock.
        let idle = SystemTime::now().duration_since(last).unwrap_or_default();
        if idle < Duration::from_secs(u64::from(minutes) * 60) {
            return None;
        }
        *guard = None;
        Some(minutes)
    }

    fn read<T>(&self, f: impl FnOnce(&VaultData) -> Result<T>) -> Result<T> {
        let guard = self.lock();
        let v = guard.as_ref().ok_or_else(|| msg("The vault is locked."))?;
        f(&v.data)
    }

    /// Apply a change, save the vault, and return the new tree. If saving fails
    /// the change is rolled back so memory and disk never disagree.
    fn modify(&self, f: impl FnOnce(&mut VaultData) -> Result<()>) -> Result<Tree> {
        self.touch();
        let mut guard = self.lock();
        let v = guard.as_mut().ok_or_else(|| msg("The vault is locked."))?;
        let before = v.data.clone();
        if let Err(e) = f(&mut v.data).and_then(|_| v.save(&self.vault_path)) {
            v.data = before;
            return Err(e);
        }
        Ok(Tree::from(&v.data))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    exists: bool,
    unlocked: bool,
    path: String,
    min_password_length: usize,
}

#[tauri::command]
pub async fn vault_status(state: State<'_, AppState>) -> Result<VaultStatus> {
    Ok(VaultStatus {
        exists: vault::exists(&state.vault_path),
        unlocked: state.lock().is_some(),
        path: state.vault_path.display().to_string(),
        min_password_length: vault::MIN_PASSWORD_LEN,
    })
}

#[tauri::command]
pub async fn vault_create(state: State<'_, AppState>, password: String) -> Result<Tree> {
    let password = Zeroizing::new(password);
    let path = state.vault_path.clone();
    let opened = tauri::async_runtime::spawn_blocking(move || vault::create(&path, &password))
        .await
        .map_err(|e| msg(e.to_string()))??;
    let tree = Tree::from(&opened.data);
    *state.lock() = Some(opened);
    state.touch();
    Ok(tree)
}

#[tauri::command]
pub async fn vault_unlock(state: State<'_, AppState>, password: String) -> Result<Tree> {
    let password = Zeroizing::new(password);
    let path = state.vault_path.clone();
    // Key derivation takes a moment on purpose; keep it off the UI thread.
    let opened = tauri::async_runtime::spawn_blocking(move || vault::unlock(&path, &password))
        .await
        .map_err(|e| msg(e.to_string()))??;
    let tree = Tree::from(&opened.data);
    *state.lock() = Some(opened);
    state.touch();
    Ok(tree)
}

#[tauri::command]
pub async fn vault_lock(state: State<'_, AppState>) -> Result<()> {
    *state.lock() = None;
    Ok(())
}

/// The main window reports that you're using it (throttled), for auto-lock.
#[tauri::command]
pub async fn vault_touch(state: State<'_, AppState>) -> Result<()> {
    state.touch();
    Ok(())
}

#[tauri::command]
pub async fn vault_change_password(app: AppHandle, current: String, new: String) -> Result<()> {
    let current = Zeroizing::new(current);
    let new = Zeroizing::new(new);
    // Two key derivations, about a second: off the UI thread.
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        state.touch();
        let mut guard = state.lock();
        let v = guard.as_mut().ok_or_else(|| msg("The vault is locked."))?;
        v.change_password(&state.vault_path, &current, &new)
    })
    .await
    .map_err(|e| msg(e.to_string()))?
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<Tree> {
    settings.validate()?;
    state.modify(|d| {
        d.settings = settings;
        Ok(())
    })
}

#[tauri::command]
pub async fn get_tree(state: State<'_, AppState>) -> Result<Tree> {
    state.read(|d| Ok(Tree::from(d)))
}

#[tauri::command]
pub async fn save_connection(state: State<'_, AppState>, input: ConnectionInput) -> Result<Saved> {
    let mut id = Uuid::nil();
    let tree = state.modify(|d| {
        id = d.save_connection(input)?;
        Ok(())
    })?;
    Ok(Saved { id, tree })
}

#[tauri::command]
pub async fn duplicate_connection(state: State<'_, AppState>, id: Uuid) -> Result<Saved> {
    let mut new_id = Uuid::nil();
    let tree = state.modify(|d| {
        new_id = d.duplicate_connection(id)?;
        Ok(())
    })?;
    Ok(Saved { id: new_id, tree })
}

#[tauri::command]
pub async fn delete_connection(state: State<'_, AppState>, id: Uuid) -> Result<Tree> {
    state.modify(|d| {
        d.delete_connection(id);
        Ok(())
    })
}

#[tauri::command]
pub async fn save_folder(state: State<'_, AppState>, input: FolderInput) -> Result<Saved> {
    let mut id = Uuid::nil();
    let tree = state.modify(|d| {
        id = d.save_folder(input)?;
        Ok(())
    })?;
    Ok(Saved { id, tree })
}

#[tauri::command]
pub async fn delete_folder(state: State<'_, AppState>, id: Uuid) -> Result<Tree> {
    state.modify(|d| {
        d.delete_folder(id);
        Ok(())
    })
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Connection,
    Folder,
}

/// Drag and drop / "Move to…": put a connection or folder into `folder_id`
/// (`None` = top level).
#[tauri::command]
pub async fn move_item(
    state: State<'_, AppState>,
    kind: ItemKind,
    id: Uuid,
    folder_id: Option<Uuid>,
) -> Result<Tree> {
    state.modify(|d| match kind {
        ItemKind::Connection => d.move_connection(id, folder_id),
        ItemKind::Folder => d.move_folder(id, folder_id),
    })
}

#[tauri::command]
pub async fn save_credential(state: State<'_, AppState>, input: CredentialInput) -> Result<Saved> {
    let mut id = Uuid::nil();
    let tree = state.modify(|d| {
        id = d.save_credential(input)?;
        Ok(())
    })?;
    Ok(Saved { id, tree })
}

#[tauri::command]
pub async fn delete_credential(state: State<'_, AppState>, id: Uuid) -> Result<Tree> {
    state.modify(|d| {
        d.delete_credential(id);
        Ok(())
    })
}

/// A saved item's id plus the updated tree.
#[derive(Serialize)]
pub struct Saved {
    id: Uuid,
    tree: Tree,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ConnectOutcome {
    Launched,
    /// Nothing saved to log in with; the UI should ask and call `connect` again.
    #[serde(rename_all = "camelCase")]
    NeedCredentials { username: String, domain: String },
}

/// A login typed into the "log in" prompt. Used for one launch, never saved.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginOverride {
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

impl LoginOverride {
    fn apply(self, target: &mut Resolved) {
        if let Some(u) = self.username.map(|u| u.trim().to_string()).filter(|u| !u.is_empty()) {
            target.username = u;
        }
        if let Some(d) = self.domain {
            target.domain = d.trim().to_string();
        }
        if self.password.is_some() {
            target.password = self.password;
        }
    }
}

/// Open a saved connection. `login` overrides the saved login for this launch
/// only (used after the "log in" prompt).
#[tauri::command]
pub async fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Uuid,
    login: Option<LoginOverride>,
) -> Result<ConnectOutcome> {
    let mut target = state.read(|d| d.resolve(id))?;
    if let Some(login) = login {
        login.apply(&mut target);
    }
    launch(&app, &state, target)
}

/// Connect to a host that isn't saved: `server`, `server:3390`, `10.0.0.5`,
/// `[fe80::1]:22`, or for SSH `user@server`. Nothing is written to the vault.
#[tauri::command]
pub async fn quick_connect(
    app: AppHandle,
    state: State<'_, AppState>,
    protocol: Protocol,
    address: String,
    login: Option<LoginOverride>,
) -> Result<ConnectOutcome> {
    let address = address.trim();
    // `user@host` is SSH shorthand. For RDP an @ belongs to a UPN login, not the address.
    let (user, address) = match (protocol, address.rsplit_once('@')) {
        (Protocol::Ssh, Some((user, host))) if !user.is_empty() => (user, host),
        _ => ("", address),
    };
    let (host, port) = parse_address(address, protocol.default_port())?;
    let name = if port == protocol.default_port() { host.clone() } else { format!("{host}:{port}") };
    let mut target = Resolved {
        name,
        protocol,
        host,
        port,
        username: user.to_string(),
        domain: String::new(),
        password: None,
        ssh_key_path: String::new(),
        ssh_key_passphrase: None,
        rdp_screen: RdpScreen::Window,
    };
    if let Some(login) = login {
        login.apply(&mut target);
    }
    launch(&app, &state, target)
}

fn launch(app: &AppHandle, state: &AppState, target: Resolved) -> Result<ConnectOutcome> {
    match target.protocol {
        Protocol::Rdp => {
            if rdp::needs_credentials(&target) {
                return Ok(ConnectOutcome::NeedCredentials {
                    username: target.username,
                    domain: target.domain,
                });
            }
            rdp::launch(app, &target)?;
        }
        Protocol::Ssh => {
            let title = format!("{} — SSH", target.name);
            let session = state.ssh.prepare(target);
            let url = format!("index.html?view=ssh&session={session}");
            let built = WebviewWindowBuilder::new(app, format!("ssh-{session}"), WebviewUrl::App(url.into()))
                .title(title)
                .inner_size(960.0, 600.0)
                .min_inner_size(420.0, 260.0)
                .build();
            match built {
                Ok(window) => crate::fit_to_screen(&window),
                Err(e) => {
                    state.ssh.close(&session);
                    return Err(e.into());
                }
            }
        }
    }
    Ok(ConnectOutcome::Launched)
}

#[tauri::command]
pub async fn ssh_start(
    state: State<'_, AppState>,
    session: String,
    cols: u32,
    rows: u32,
    output: Channel<InvokeResponseBody>,
    events: Channel<SshEvent>,
) -> Result<()> {
    state.ssh.start(&session, cols, rows, output, events)
}

#[tauri::command]
pub async fn ssh_write(state: State<'_, AppState>, session: String, data: String) -> Result<()> {
    state.ssh.write(&session, data.into_bytes()).await
}

#[tauri::command]
pub async fn ssh_resize(
    state: State<'_, AppState>,
    session: String,
    cols: u32,
    rows: u32,
) -> Result<()> {
    state.ssh.resize(&session, cols, rows).await
}

#[tauri::command]
pub async fn ssh_answer(
    state: State<'_, AppState>,
    session: String,
    answer: SshAnswer,
) -> Result<()> {
    state.ssh.answer(&session, answer);
    Ok(())
}

/// Web pages the About section links to. The interface names one of these
/// rather than passing a URL, so it can't open anything else.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Link {
    Source,
    Kofi,
    Releases,
}

impl Link {
    fn url(&self) -> &'static str {
        match self {
            Link::Source => "https://github.com/ThePoolboy/corestart-reach",
            Link::Kofi => "https://ko-fi.com/thepoolboy",
            Link::Releases => "https://github.com/ThePoolboy/corestart-reach/releases/latest",
        }
    }
}

/// Open a link in the default web browser (through the desktop portal in a Flatpak).
#[tauri::command]
pub fn open_link(link: Link) -> Result<()> {
    open::that_detached(link.url()).map_err(|e| msg(format!("Couldn't open the web browser: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlocked_state(minutes: u32) -> AppState {
        let dir = std::env::temp_dir().join(format!("reach-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vault.json");
        let mut v = vault::create(&path, "password1").unwrap();
        v.data.settings.auto_lock_minutes = minutes;
        let state = AppState::new(path);
        *state.lock() = Some(v);
        state
    }

    fn idle_for(state: &AppState, minutes: u64) {
        *state.last_activity.lock().unwrap() = SystemTime::now() - Duration::from_secs(minutes * 60);
    }

    #[test]
    fn auto_lock_after_idle() {
        let state = unlocked_state(15);
        state.touch();
        assert_eq!(state.lock_if_idle(), None);
        idle_for(&state, 14);
        assert_eq!(state.lock_if_idle(), None, "not idle long enough yet");
        idle_for(&state, 16);
        assert_eq!(state.lock_if_idle(), Some(15));
        assert!(state.lock().is_none(), "vault must be locked");
        assert_eq!(state.lock_if_idle(), None, "already locked");
    }

    #[test]
    fn auto_lock_never() {
        let state = unlocked_state(0);
        idle_for(&state, 60 * 24 * 30);
        assert_eq!(state.lock_if_idle(), None);
        assert!(state.lock().is_some());
    }
}
