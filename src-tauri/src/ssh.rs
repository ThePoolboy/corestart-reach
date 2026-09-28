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
use russh::keys::agent::AgentIdentity;
use russh::keys::agent::client::{AgentClient, AgentStream};
use russh::keys::{self, Algorithm, HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKey, PublicKeyOrCertificate};
use russh::{ChannelMsg, ChannelWriteHalf, MethodKind, Pty};
use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::net::TcpStream;
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
    /// `exited` is true when the remote shell exited by itself.
    Closed { message: String, exited: bool },
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
                Ok(closed) => closed,
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

/// Run one session from connect to close. `Ok` carries the `Closed` event.
async fn run(
    ui: &Ui,
    target: Resolved,
    cols: u32,
    rows: u32,
    output: Channel<InvokeResponseBody>,
) -> std::result::Result<SshEvent, String> {
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
    // Only the TCP connection gets a timeout: the SSH handshake below can wait
    // on you reading and answering the host-key question.
    let tcp = TcpStream::connect((target.host.as_str(), target.port));
    let tcp = tokio::time::timeout(Duration::from_secs(15), tcp)
        .await
        .map_err(|_| format!("Timed out connecting to {}:{}.", target.host, target.port))?
        .map_err(|e| format!("Couldn't connect to {}:{}: {e}", target.host, target.port))?;
    let _ = tcp.set_nodelay(true);
    let mut handle = match client::connect_stream(config, tcp, handler).await {
        Ok(h) => h,
        Err(e) => {
            let reason = rejected.lock().unwrap_or_else(|e| e.into_inner()).take();
            return Err(reason.unwrap_or_else(|| err(e)));
        }
    };

    let user = if target.username.is_empty() {
        ui.prompt("login as: ", false).await.filter(|u| !u.is_empty()).ok_or("Cancelled.")?
    } else {
        target.username.clone()
    };
    authenticate(&mut handle, ui, &target, &user).await?;

    let channel = handle.channel_open_session().await.map_err(err)?;
    // Backspace sends DEL (127) and input is UTF-8, matching the xterm.js terminal.
    let modes = [(Pty::VERASE, 127), (Pty::IUTF8, 1)];
    channel
        .request_pty(false, "xterm-256color", cols, rows, 0, 0, &modes)
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
            // EOF only means "no more output"; the exit status can still follow it.
            ChannelMsg::Close => break,
            _ => {}
        }
    }
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "en").await;
    Ok(match exit_status {
        // The shell exited (`exit`, `logout`, Ctrl+D): the window closes itself.
        Some(code) => SshEvent::Closed {
            message: format!("Logged out (exit code {code})."),
            exited: true,
        },
        // No exit status: the server went away (reboot, network drop).
        None => SshEvent::Closed { message: "Connection lost.".into(), exited: false },
    })
}

/// At most this many keys are offered, so a full ssh-agent can't use up the
/// server's MaxAuthTries (OpenSSH default 6) before the password gets a turn.
const MAX_KEYS: usize = 4;

/// Key files OpenSSH tries when none is configured, in its order.
const DEFAULT_KEYS: &[&str] = &["id_ed25519", "id_ecdsa", "id_rsa"];

/// Log in the way OpenSSH does: ask the server which methods it accepts, then
/// try keys (the connection's key file, or else ssh-agent and the default key
/// files), then the password, then keyboard-interactive (which also covers 2FA
/// prompts). Anything not saved is asked for in the terminal.
async fn authenticate(
    handle: &mut client::Handle<Client>,
    ui: &Ui,
    target: &Resolved,
    user: &str,
) -> std::result::Result<(), String> {
    let mut methods: Vec<MethodKind> = Vec::new();
    // "none" auth fails on any real server but tells us what it accepts.
    let probe = handle.authenticate_none(user).await.map_err(err)?;
    if succeeded(&probe, &mut methods) {
        return Ok(());
    }
    if methods.is_empty() {
        methods = vec![MethodKind::PublicKey, MethodKind::Password, MethodKind::KeyboardInteractive];
    }

    if methods.contains(&MethodKind::PublicKey) {
        let accepted = if target.ssh_key_path.is_empty() {
            try_usual_keys(handle, ui, user, &mut methods).await?
        } else {
            try_key_file(handle, ui, user, target, &mut methods).await?
        };
        if accepted {
            return Ok(());
        }
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
    if methods == [MethodKind::PublicKey] {
        return Err(format!(
            "{user}@{}: Permission denied (publickey).\r\n\
             This server only accepts SSH keys. Set a private key file on the connection, \
             or load your key into ssh-agent.",
            target.host
        ));
    }
    Err(format!("{user}@{}: Permission denied.", target.host))
}

/// The key file set on the connection. Asks for its passphrase if it isn't saved.
async fn try_key_file(
    handle: &mut client::Handle<Client>,
    ui: &Ui,
    user: &str,
    target: &Resolved,
    methods: &mut Vec<MethodKind>,
) -> std::result::Result<bool, String> {
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
    if offer_key(handle, user, key, methods).await? {
        return Ok(true);
    }
    ui.status("Server refused the key.");
    Ok(false)
}

/// No key file set: like OpenSSH, try ssh-agent, then ~/.ssh/id_ed25519,
/// id_ecdsa and id_rsa. An encrypted key file asks for its passphrase, and an
/// empty answer skips it.
async fn try_usual_keys(
    handle: &mut client::Handle<Client>,
    ui: &Ui,
    user: &str,
    methods: &mut Vec<MethodKind>,
) -> std::result::Result<bool, String> {
    let rsa_hash = handle.best_supported_rsa_hash().await.map_err(err)?.flatten();
    let mut offered: Vec<PublicKey> = Vec::new();

    if let Some(mut agent) = connect_agent().await {
        let identities = agent.request_identities().await.unwrap_or_default();
        for identity in identities {
            if offered.len() >= MAX_KEYS || !methods.contains(&MethodKind::PublicKey) {
                return Ok(false);
            }
            // Certificates need the matching private key's cert login; skip them.
            let AgentIdentity::PublicKey { key, .. } = identity else { continue };
            offered.push(key.clone());
            let hash = if matches!(key.algorithm(), Algorithm::Rsa { .. }) { rsa_hash } else { None };
            let result = handle
                .authenticate_publickey_with(user, key, hash, &mut agent)
                .await
                .map_err(|e| format!("ssh-agent: {e}"))?;
            if succeeded(&result, methods) {
                return Ok(true);
            }
        }
    }

    let Some(ssh_dir) = home_dir().map(|h| h.join(".ssh")) else { return Ok(false) };
    for name in DEFAULT_KEYS {
        if offered.len() >= MAX_KEYS || !methods.contains(&MethodKind::PublicKey) {
            break;
        }
        let path = ssh_dir.join(name);
        if !path.is_file() {
            continue;
        }
        // Skip keys the agent already offered, without asking for a passphrase.
        if let Ok(public) = PublicKey::read_openssh_file(path.with_extension("pub")) {
            if offered.iter().any(|k| k.key_data() == public.key_data()) {
                continue;
            }
        }
        let key = match keys::load_secret_key(&path, None) {
            Ok(k) => k,
            Err(keys::Error::KeyIsEncrypted) => {
                let prompt = format!("Enter passphrase for key '{}' (Enter to skip): ", path.display());
                match ui.prompt(prompt, true).await {
                    None => return Err("Cancelled.".into()),
                    Some(p) if p.is_empty() => continue,
                    Some(p) => match keys::load_secret_key(&path, Some(&p)) {
                        Ok(k) => k,
                        Err(_) => {
                            ui.status("Wrong passphrase; skipping that key.");
                            continue;
                        }
                    },
                }
            }
            Err(_) => continue, // unreadable or unsupported: OpenSSH skips it too
        };
        offered.push(key.public_key().clone());
        if offer_key(handle, user, key, methods).await? {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn offer_key(
    handle: &mut client::Handle<Client>,
    user: &str,
    key: PrivateKey,
    methods: &mut Vec<MethodKind>,
) -> std::result::Result<bool, String> {
    let hash = handle.best_supported_rsa_hash().await.map_err(err)?.flatten();
    let result = handle
        .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
        .await
        .map_err(err)?;
    Ok(succeeded(&result, methods))
}

/// The user's running SSH agent: `SSH_AUTH_SOCK` on Linux, the Windows
/// OpenSSH agent service's pipe on Windows. `None` if there isn't one.
async fn connect_agent() -> Option<AgentClient<Box<dyn AgentStream + Send + Unpin>>> {
    #[cfg(unix)]
    let agent = AgentClient::connect_env().await.ok()?.dynamic();
    #[cfg(windows)]
    let agent = AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await.ok()?.dynamic();
    Some(agent)
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
