//! SSH sessions shown in their own terminal window.
//!
//! The main window asks for a session ([`Sessions::prepare`]) and opens an `ssh-<id>`
//! window. That window calls `ssh_start` with two channels: raw terminal output
//! and [`SshEvent`]s (status lines, host-key and password prompts). Keystrokes come
//! back through `ssh_write`, answers to prompts through `ssh_answer`.
//!
//! Host keys are checked against the user's `~/.ssh/known_hosts`, shared with OpenSSH.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::client::{self, AuthResult, KeyboardInteractiveAuthResponse};
use russh::keys::{self, HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh::{ChannelMsg, ChannelWriteHalf, MethodKind};
use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::sync::oneshot;

use crate::error::{Result, msg};
use crate::model::Resolved;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SshEvent {
    /// A progress or warning line to print in the terminal.
    Status { message: String },
    /// First connection to this host: ask the user to trust the key.
    HostKey { host: String, port: u16, algorithm: String, fingerprint: String },
    /// Ask the user to type something (password, passphrase, 2FA code).
    Prompt { prompt: String, secret: bool },
    /// The shell is open; keystrokes now go to the server.
    Connected,
    /// The session ended normally.
    Closed { message: String },
    /// The session could not be opened, or ended with an error.
    Failed { message: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SshAnswer {
    HostKey { accept: bool },
    /// `None` means the user cancelled.
    Prompt { value: Option<String> },
}

struct Entry {
    target: Resolved,
    writer: Option<Arc<ChannelWriteHalf<client::Msg>>>,
    answer: Option<oneshot::Sender<SshAnswer>>,
    task: Option<JoinHandle<()>>,
}

#[derive(Clone, Default)]
pub struct Sessions(Arc<Mutex<HashMap<String, Entry>>>);

impl Sessions {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Register a session for a window that is about to open. Returns its id.
    pub fn prepare(&self, target: Resolved) -> String {
        let id = uuid::Uuid::new_v4().simple().to_string();
        self.lock().insert(id.clone(), Entry { target, writer: None, answer: None, task: None });
        id
    }

    pub fn title(&self, id: &str) -> Option<String> {
        self.lock().get(id).map(|e| e.target.name.clone())
    }

    /// Connect (or reconnect) the session, streaming into the window's channels.
    pub fn start(
        &self,
        id: &str,
        cols: u32,
        rows: u32,
        output: Channel<InvokeResponseBody>,
        events: Channel<SshEvent>,
    ) -> Result<()> {
        let target = {
            let mut map = self.lock();
            let entry = map.get_mut(id).ok_or_else(|| msg("This session has expired."))?;
            if let Some(task) = entry.task.take() {
                task.abort();
            }
            entry.writer = None;
            entry.answer = None;
            entry.target.clone()
        };
        let ui = Ui { id: id.to_string(), sessions: self.clone(), events };
        let sessions = self.clone();
        let session_id = id.to_string();
        let task = tauri::async_runtime::spawn(async move {
            let result = run(&ui, target, cols, rows, output).await;
            if let Some(entry) = sessions.lock().get_mut(&session_id) {
                entry.writer = None;
                entry.answer = None;
            }
            ui.emit(match result {
                Ok(message) => SshEvent::Closed { message },
                Err(message) => SshEvent::Failed { message },
            });
        });
        if let Some(entry) = self.lock().get_mut(id) {
            entry.task = Some(task);
        }
        Ok(())
    }

    fn writer(&self, id: &str) -> Option<Arc<ChannelWriteHalf<client::Msg>>> {
        self.lock().get(id).and_then(|e| e.writer.clone())
    }

    pub async fn write(&self, id: &str, data: Vec<u8>) -> Result<()> {
        if let Some(w) = self.writer(id) {
            w.data_bytes(data).await.map_err(|e| msg(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn resize(&self, id: &str, cols: u32, rows: u32) -> Result<()> {
        if let Some(w) = self.writer(id) {
            w.window_change(cols, rows, 0, 0).await.map_err(|e| msg(e.to_string()))?;
        }
        Ok(())
    }

    pub fn answer(&self, id: &str, answer: SshAnswer) {
        if let Some(tx) = self.lock().get_mut(id).and_then(|e| e.answer.take()) {
            let _ = tx.send(answer);
        }
    }

    /// The window was closed: drop the connection and forget the session.
    pub fn close(&self, id: &str) {
        if let Some(entry) = self.lock().remove(id) {
            if let Some(task) = entry.task {
                task.abort();
            }
        }
    }
}

/// Talks to the session window.
#[derive(Clone)]
struct Ui {
    id: String,
    sessions: Sessions,
    events: Channel<SshEvent>,
}

impl Ui {
    fn emit(&self, event: SshEvent) {
        let _ = self.events.send(event);
    }

    fn status(&self, message: impl Into<String>) {
        self.emit(SshEvent::Status { message: message.into() });
    }

    /// Send a question to the window and wait for the answer.
    async fn ask(&self, event: SshEvent) -> Option<SshAnswer> {
        let (tx, rx) = oneshot::channel();
        match self.sessions.lock().get_mut(&self.id) {
            Some(entry) => entry.answer = Some(tx),
            None => return None,
        }
        self.emit(event);
        rx.await.ok()
    }

    async fn prompt(&self, prompt: impl Into<String>, secret: bool) -> Option<String> {
        match self.ask(SshEvent::Prompt { prompt: prompt.into(), secret }).await {
            Some(SshAnswer::Prompt { value }) => value,
            _ => None,
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    #[allow(deprecated)] // fine on every platform we ship; no extra crate needed
    std::env::home_dir()
}

fn known_hosts_file() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".ssh").join("known_hosts"))
}

fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")), home_dir()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(path),
    }
}

struct Client {
    ui: Ui,
    host: String,
    port: u16,
    /// Why we refused the server's key, reported instead of russh's generic error.
    rejected: Arc<Mutex<Option<String>>>,
}

impl Client {
    fn reject(&self, reason: String) -> std::result::Result<bool, russh::Error> {
        *self.rejected.lock().unwrap_or_else(|e| e.into_inner()) = Some(reason);
        Ok(false)
    }
}

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_key: &PublicKeyOrCertificate,
    ) -> std::result::Result<bool, Self::Error> {
        let key = server_key.public_key();
        let Some(file) = known_hosts_file() else {
            return self.reject("Can't find your home folder to check known_hosts.".into());
        };
        match keys::check_known_hosts_path(&self.host, self.port, &key, &file) {
            Ok(true) => Ok(true),
            Ok(false) => {
                let answer = self
                    .ui
                    .ask(SshEvent::HostKey {
                        host: self.host.clone(),
                        port: self.port,
                        algorithm: key.algorithm().to_string(),
                        fingerprint: key.fingerprint(HashAlg::Sha256).to_string(),
                    })
                    .await;
                if !matches!(answer, Some(SshAnswer::HostKey { accept: true })) {
                    return self.reject("Host key not accepted.".into());
                }
                if let Err(e) =
                    keys::known_hosts::learn_known_hosts_path(&self.host, self.port, &key, &file)
                {
                    self.ui.status(format!("Warning: couldn't save the host key: {e}"));
                }
                Ok(true)
            }
            Err(keys::Error::KeyChanged { line }) => self.reject(format!(
                "WARNING: THE HOST KEY FOR {} HAS CHANGED!\r\n\
                 Someone could be intercepting this connection, or the server was reinstalled.\r\n\
                 New key fingerprint: {}\r\n\
                 If you trust this change, remove line {line} of {}.",
                self.host,
                key.fingerprint(HashAlg::Sha256),
                file.display(),
            )),
            Err(e) => self.reject(format!("Couldn't read {}: {e}", file.display())),
        }
    }
}

fn err(e: russh::Error) -> String {
    e.to_string()
}

/// Run one session from connect to close. `Ok` carries the closing message.
async fn run(
    ui: &Ui,
    target: Resolved,
    cols: u32,
    rows: u32,
    output: Channel<InvokeResponseBody>,
) -> std::result::Result<String, String> {
    ui.status(format!("Connecting to {}:{}…", target.host, target.port));

    let config = Arc::new(client::Config {
        keepalive_interval: Some(Duration::from_secs(30)),
        ..Default::default()
    });
    let rejected = Arc::new(Mutex::new(None));
    let handler = Client {
        ui: ui.clone(),
        host: target.host.clone(),
        port: target.port,
        rejected: rejected.clone(),
    };
    let connect = client::connect(config, (target.host.as_str(), target.port), handler);
    let mut handle = match tokio::time::timeout(Duration::from_secs(20), connect).await {
        Err(_) => return Err(format!("Timed out connecting to {}.", target.host)),
        Ok(Err(e)) => {
            let reason = rejected.lock().unwrap_or_else(|e| e.into_inner()).take();
            return Err(reason.unwrap_or_else(|| err(e)));
        }
        Ok(Ok(h)) => h,
    };

    let user = if target.username.is_empty() {
        ui.prompt("login as: ", false).await.filter(|u| !u.is_empty()).ok_or("Cancelled.")?
    } else {
        target.username.clone()
    };
    authenticate(&mut handle, ui, &target, &user).await?;

    let channel = handle.channel_open_session().await.map_err(err)?;
    channel
        .request_pty(false, "xterm-256color", cols, rows, 0, 0, &[])
        .await
        .map_err(err)?;
    channel.request_shell(false).await.map_err(err)?;
    let (mut reader, writer) = channel.split();
    if let Some(entry) = ui.sessions.lock().get_mut(&ui.id) {
        entry.writer = Some(Arc::new(writer));
    }
    ui.emit(SshEvent::Connected);

    let mut exit_status = None;
    while let Some(message) = reader.wait().await {
        match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                if output.send(InvokeResponseBody::Raw(data.to_vec())).is_err() {
                    break; // window is gone
                }
            }
            ChannelMsg::ExitStatus { exit_status: code } => exit_status = Some(code),
            ChannelMsg::Eof | ChannelMsg::Close => break,
            _ => {}
        }
    }
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "en").await;
    Ok(match exit_status {
        Some(0) | None => "Connection closed.".into(),
        Some(code) => format!("Connection closed (exit code {code})."),
    })
}

/// Try the private key, then the password, then keyboard-interactive (which also
/// covers 2FA prompts). Asks the user for anything that isn't saved.
async fn authenticate(
    handle: &mut client::Handle<Client>,
    ui: &Ui,
    target: &Resolved,
    user: &str,
) -> std::result::Result<(), String> {
    let mut methods: Vec<MethodKind> =
        vec![MethodKind::PublicKey, MethodKind::Password, MethodKind::KeyboardInteractive];

    if !target.ssh_key_path.is_empty() {
        let path = expand_home(&target.ssh_key_path);
        let key = match keys::load_secret_key(&path, target.ssh_key_passphrase.as_deref()) {
            Ok(k) => k,
            Err(keys::Error::KeyIsEncrypted) => {
                let pass = ui
                    .prompt(format!("Enter passphrase for key '{}': ", path.display()), true)
                    .await
                    .ok_or("Cancelled.")?;
                keys::load_secret_key(&path, Some(&pass))
                    .map_err(|e| format!("Couldn't unlock {}: {e}", path.display()))?
            }
            Err(e) => return Err(format!("Couldn't read key {}: {e}", path.display())),
        };
        let hash = handle.best_supported_rsa_hash().await.map_err(err)?.flatten();
        let result = handle
            .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
            .await
            .map_err(err)?;
        if succeeded(&result, &mut methods) {
            return Ok(());
        }
        ui.status("Server refused the key.");
    }

    let mut saved = target.password.clone();
    for _ in 0..3 {
        let can_password = methods.contains(&MethodKind::Password);
        let can_keyboard = methods.contains(&MethodKind::KeyboardInteractive);
        if !can_password && !can_keyboard {
            break;
        }
        if can_password {
            let password = match saved.take() {
                Some(p) => p,
                None => ui
                    .prompt(format!("{user}@{}'s password: ", target.host), true)
                    .await
                    .ok_or("Cancelled.")?,
            };
            let result = handle.authenticate_password(user, password).await.map_err(err)?;
            if succeeded(&result, &mut methods) {
                return Ok(());
            }
        } else if keyboard_interactive(handle, ui, user, saved.take()).await? {
            return Ok(());
        }
        ui.status("Permission denied, please try again.");
    }
    Err(format!("{user}@{}: Permission denied.", target.host))
}

/// True on success; on failure, remember which methods the server still offers.
fn succeeded(result: &AuthResult, methods: &mut Vec<MethodKind>) -> bool {
    match result {
        AuthResult::Success => true,
        AuthResult::Failure { remaining_methods, .. } => {
            *methods = remaining_methods.iter().copied().collect();
            false
        }
    }
}

/// Answer the server's prompts. A saved password answers the first hidden prompt.
async fn keyboard_interactive(
    handle: &mut client::Handle<Client>,
    ui: &Ui,
    user: &str,
    mut password: Option<String>,
) -> std::result::Result<bool, String> {
    let mut response = handle
        .authenticate_keyboard_interactive_start(user, None::<String>)
        .await
        .map_err(err)?;
    loop {
        match response {
            KeyboardInteractiveAuthResponse::Success => return Ok(true),
            KeyboardInteractiveAuthResponse::Failure { .. } => return Ok(false),
            KeyboardInteractiveAuthResponse::InfoRequest { instructions, prompts, .. } => {
                if !instructions.trim().is_empty() {
                    ui.status(instructions.trim().to_string());
                }
                let mut answers = Vec::with_capacity(prompts.len());
                for p in prompts {
                    let answer = match (!p.echo).then(|| password.take()).flatten() {
                        Some(saved) => saved,
                        None => ui.prompt(p.prompt, !p.echo).await.ok_or("Cancelled.")?,
                    };
                    answers.push(answer);
                }
                response = handle
                    .authenticate_keyboard_interactive_respond(answers)
                    .await
                    .map_err(err)?;
            }
        }
    }
}
