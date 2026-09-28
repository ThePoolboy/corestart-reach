//! Commands the frontend calls through `invoke`.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, State, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::error::{Result, msg};
use crate::model::{
    ConnectionInput, CredentialInput, FolderInput, Protocol, Tree, VaultData,
};
use crate::ssh::{Sessions, SshAnswer, SshEvent};
use crate::{rdp, vault};

pub struct AppState {
    vault_path: PathBuf,
    vault: Mutex<Option<vault::Unlocked>>,
    ssh: Sessions,
}

impl AppState {
    pub fn new(vault_path: PathBuf) -> Self {
        Self { vault_path, vault: Mutex::new(None), ssh: Sessions::default() }
    }

    pub fn ssh(&self) -> &Sessions {
        &self.ssh
    }

    fn lock(&self) -> MutexGuard<'_, Option<vault::Unlocked>> {
        self.vault.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn read<T>(&self, f: impl FnOnce(&VaultData) -> Result<T>) -> Result<T> {
        let guard = self.lock();
        let v = guard.as_ref().ok_or_else(|| msg("The vault is locked."))?;
        f(&v.data)
    }

    /// Apply a change, save the vault, and return the new tree. If saving fails
    /// the change is rolled back so memory and disk never disagree.
    fn modify(&self, f: impl FnOnce(&mut VaultData) -> Result<()>) -> Result<Tree> {
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
    Ok(tree)
}

#[tauri::command]
pub async fn vault_lock(state: State<'_, AppState>) -> Result<()> {
    *state.lock() = None;
    Ok(())
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

/// Open a connection. `username`/`password` override the saved ones for this
/// launch only (used after the "enter credentials" prompt).
#[tauri::command]
pub async fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Uuid,
    username: Option<String>,
    password: Option<String>,
) -> Result<ConnectOutcome> {
    let mut target = state.read(|d| d.resolve(id))?;
    if let Some(u) = username.filter(|u| !u.trim().is_empty()) {
        target.username = u.trim().to_string();
    }
    if password.is_some() {
        target.password = password;
    }

    match target.protocol {
        Protocol::Rdp => {
            if rdp::needs_credentials(&target) {
                return Ok(ConnectOutcome::NeedCredentials {
                    username: target.username,
                    domain: target.domain,
                });
            }
            rdp::launch(&app, &target)?;
        }
        Protocol::Ssh => {
            let title = format!("{} — SSH", target.name);
            let session = state.ssh.prepare(target);
            let url = format!("index.html?view=ssh&session={session}");
            let built = WebviewWindowBuilder::new(&app, format!("ssh-{session}"), WebviewUrl::App(url.into()))
                .title(title)
                .inner_size(960.0, 600.0)
                .min_inner_size(420.0, 260.0)
                .build();
            if let Err(e) = built {
                state.ssh.close(&session);
                return Err(e.into());
            }
        }
    }
    Ok(ConnectOutcome::Launched)
}

#[tauri::command]
pub async fn ssh_title(state: State<'_, AppState>, session: String) -> Result<String> {
    state.ssh.title(&session).ok_or_else(|| msg("This session has expired."))
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
