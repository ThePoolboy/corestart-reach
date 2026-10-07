//! Backups: everything in Reach in one encrypted file, for moving to another
//! computer or getting your setup back after a reinstall.
//!
//! A backup holds the data file's contents plus the choices the interface keeps in
//! its own storage (theme, open folders). It's encrypted with the data file's key,
//! so it opens with the master password Reach had when it was made, and importing
//! it replaces everything, that master password included. The plaintext header
//! (format, Reach version, date) lets a backup from a newer Reach be refused before
//! anything is decrypted.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use chacha20poly1305::XNonce;
use chacha20poly1305::aead::{Aead, Payload};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::{Error, Result, msg};
use crate::model::VaultData;
use crate::vault::{self, KdfParams, Unlocked};

const FORMAT: &str = "corestart-reach-backup";
const VERSION: u32 = 1;
/// Not the data file's, so neither kind of file can pass for the other.
const AAD: &[u8] = b"corestart-reach-backup-v1";
const CIPHER: &str = "xchacha20poly1305";
/// Far bigger than any real backup.
const MAX_SIZE: u64 = 64 * 1024 * 1024;

/// What the interface remembers in its own storage, by key (`reach.theme`…).
pub type UiState = BTreeMap<String, String>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupFile {
    format: String,
    version: u32,
    /// The Reach version that made it.
    app_version: String,
    /// When it was made, in seconds since 1970.
    created: u64,
    kdf: KdfParams,
    cipher: String,
    nonce: String,
    data: String,
}

/// The encrypted part.
#[derive(Serialize)]
struct ContentsRef<'a> {
    data: &'a VaultData,
    ui: &'a UiState,
}

#[derive(Deserialize)]
struct Contents {
    data: VaultData,
    #[serde(default)]
    ui: UiState,
}

/// What's in a backup, shown before it replaces everything.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub created: u64,
    pub app_version: String,
    pub folders: usize,
    pub connections: usize,
    pub credentials: usize,
}

/// A backup that has been decrypted and checked, waiting to be restored.
pub struct Opened {
    vault: Unlocked,
    ui: UiState,
    pub summary: Summary,
}

/// Write everything in `vault`, plus `ui`, to `path`. `password` must be the
/// master password, so nobody can take a backup from a Reach left unlocked.
pub fn export(vault: &Unlocked, password: &str, ui: &UiState, app_version: &str, path: &Path) -> Result<()> {
    if !vault.is_password(password)? {
        return Err(msg("Wrong master password."));
    }
    if !sane_ui(ui) {
        return Err(msg("Reach couldn't read its interface settings for the backup."));
    }
    let plain = Zeroizing::new(serde_json::to_vec(&ContentsRef { data: &vault.data, ui })?);
    let nonce = vault::random::<24>()?;
    let body = vault::cipher(&vault.key)
        .encrypt(&XNonce::from(nonce), Payload { msg: &plain, aad: AAD })
        .map_err(|_| msg("Encryption failed."))?;
    let file = BackupFile {
        format: FORMAT.into(),
        version: VERSION,
        app_version: app_version.into(),
        created: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        kdf: vault.kdf.clone(),
        cipher: CIPHER.into(),
        nonce: B64.encode(nonce),
        data: B64.encode(body),
    };
    write_private(path, &serde_json::to_vec_pretty(&file)?)
}

/// Decrypt and check a backup. Nothing changes until [`Opened::restore`].
pub fn open(path: &Path, password: &str, app_version: &str) -> Result<Opened> {
    if fs::metadata(path)?.len() > MAX_SIZE {
        return Err(not_a_backup());
    }
    let raw = fs::read(path)?;
    let file: BackupFile = serde_json::from_slice(&raw).map_err(|_| not_a_backup())?;
    if file.format != FORMAT {
        return Err(not_a_backup());
    }
    if file.version > VERSION || is_newer(&file.app_version, app_version) {
        return Err(msg(format!(
            "This backup was made by Corestart Reach {}. Update Reach to import it.",
            file.app_version
        )));
    }
    if file.cipher != CIPHER {
        return Err(damaged());
    }
    let nonce: [u8; 24] = B64
        .decode(&file.nonce)
        .ok()
        .and_then(|n| n.try_into().ok())
        .ok_or_else(damaged)?;
    let body = B64.decode(&file.data).map_err(|_| damaged())?;

    // Argon2 only refuses key settings that were damaged or tampered with.
    let key = vault::derive_key(password, &file.kdf).map_err(|_| damaged())?;
    let plain = vault::cipher(&key)
        .decrypt(&XNonce::from(nonce), Payload { msg: &body, aad: AAD })
        .map_err(|_| msg("Wrong password. Use the master password Reach had when the backup was made."))?;
    let plain = Zeroizing::new(plain);
    let contents: Contents = serde_json::from_slice(&plain).map_err(|_| damaged())?;
    if contents.data.settings.validate().is_err() || !sane_ui(&contents.ui) {
        return Err(damaged());
    }

    let d = &contents.data;
    let summary = Summary {
        created: file.created,
        app_version: file.app_version,
        folders: d.folders.len(),
        connections: d.connections.len(),
        credentials: d.credentials.len(),
    };
    Ok(Opened { vault: Unlocked { key, kdf: file.kdf, data: contents.data }, ui: contents.ui, summary })
}

impl Opened {
    /// Make this backup Reach's data file at `path`. The current file is copied to
    /// [`before_import_path`] first, so a mistaken import can still be undone.
    pub fn restore(self, path: &Path) -> Result<(Unlocked, UiState)> {
        if vault::exists(path) {
            fs::copy(path, before_import_path(path))?;
        }
        self.vault.save(path)?;
        Ok((self.vault, self.ui))
    }
}

/// `vault.before-import.json`, next to the data file.
pub fn before_import_path(path: &Path) -> PathBuf {
    path.with_extension("before-import.json")
}

fn not_a_backup() -> Error {
    msg("This file isn't a Reach backup.")
}

fn damaged() -> Error {
    msg("This backup is damaged and can't be opened.")
}

/// A few short `reach.*` entries, like the interface writes.
fn sane_ui(ui: &UiState) -> bool {
    ui.len() <= 32
        && ui
            .iter()
            .all(|(k, v)| k.starts_with("reach.") && k.len() <= 64 && v.len() <= 64 * 1024)
}

/// Whether version `a` ("0.10.0") is newer than `b` ("0.9.2"). Anything
/// unreadable counts as not newer.
fn is_newer(a: &str, b: &str) -> bool {
    fn parse(v: &str) -> Option<Vec<u64>> {
        v.split(['-', '+']).next()?.split('.').map(|p| p.parse().ok()).collect()
    }
    matches!((parse(a), parse(b)), (Some(a), Some(b)) if a > b)
}

/// Write `bytes` to `path`, readable only by you on Linux. Written in place, not
/// through a temporary file: in a Flatpak the save dialog only grants access to
/// the file you picked.
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        // An existing file you chose to replace keeps its permissions otherwise.
        if path.exists() {
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
        }
    }
    let mut f = options.open(path)?;
    f.write_all(bytes)?;
    // Best effort: not every file system behind a save dialog supports it.
    let _ = f.sync_all();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Credential, Folder};
    use uuid::Uuid;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("reach-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A data file with a folder, a saved login and changed settings.
    fn sample(dir: &Path, password: &str) -> (PathBuf, Unlocked) {
        let path = dir.join("vault.json");
        let mut v = vault::create(&path, password).unwrap();
        v.data.folders.push(Folder { id: Uuid::new_v4(), name: "Servers".into(), parent_id: None });
        v.data.credentials.push(Credential {
            id: Uuid::new_v4(),
            name: "Admin".into(),
            username: "admin".into(),
            domain: String::new(),
            password: Some("s3cret".into()),
        });
        v.data.settings.auto_lock_minutes = 60;
        v.data.settings.shortcuts.insert("quickConnect".into(), "Ctrl+J".into());
        v.save(&path).unwrap();
        (path, v)
    }

    fn ui() -> UiState {
        UiState::from([("reach.theme".into(), "light".into())])
    }

    #[test]
    fn export_and_open_round_trip() {
        let dir = temp_dir();
        let (_, v) = sample(&dir, "home password");
        let file = dir.join("Reach backup.reachbackup");

        assert!(export(&v, "not the password", &ui(), "0.3.0", &file).is_err());
        assert!(!file.exists(), "nothing is written without the master password");
        export(&v, "home password", &ui(), "0.3.0", &file).unwrap();

        let raw = fs::read_to_string(&file).unwrap();
        assert!(!raw.contains("Servers") && !raw.contains("s3cret"), "contents must be encrypted");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
        }

        assert!(open(&file, "home password2", "0.3.0").is_err());
        let opened = open(&file, "home password", "0.3.0").unwrap();
        assert_eq!(opened.summary.folders, 1);
        assert_eq!(opened.summary.connections, 0);
        assert_eq!(opened.summary.credentials, 1);
        assert_eq!(opened.summary.app_version, "0.3.0");
        assert!(opened.summary.created > 0);
        assert_eq!(opened.ui, ui());
        assert_eq!(opened.vault.data.settings, v.data.settings);
        assert_eq!(opened.vault.data.credentials[0].password.as_deref(), Some("s3cret"));
    }

    #[test]
    fn restore_replaces_everything_and_keeps_a_copy() {
        let dir = temp_dir();
        let (_, home) = sample(&dir, "home password");
        let file = dir.join("b.reachbackup");
        export(&home, "home password", &ui(), "0.3.0", &file).unwrap();

        // Another computer, with its own connections and master password.
        let other = temp_dir();
        let path = other.join("vault.json");
        let mut v = vault::create(&path, "work password").unwrap();
        v.data.folders.push(Folder { id: Uuid::new_v4(), name: "Work".into(), parent_id: None });
        v.save(&path).unwrap();

        let (restored, ui_state) = open(&file, "home password", "0.3.0").unwrap().restore(&path).unwrap();
        assert_eq!(restored.data.folders[0].name, "Servers");
        assert_eq!(ui_state, ui());

        // The master password came with the backup.
        assert!(vault::unlock(&path, "work password").is_err());
        let now = vault::unlock(&path, "home password").unwrap();
        assert_eq!(now.data.folders[0].name, "Servers");
        assert_eq!(now.data.settings.auto_lock_minutes, 60);

        // What was there before is kept, with its own password.
        let before = vault::unlock(&before_import_path(&path), "work password").unwrap();
        assert_eq!(before.data.folders[0].name, "Work");
    }

    #[test]
    fn restore_on_a_fresh_install() {
        let dir = temp_dir();
        let (_, home) = sample(&dir, "home password");
        let file = dir.join("b.reachbackup");
        export(&home, "home password", &UiState::new(), "0.3.0", &file).unwrap();

        let path = temp_dir().join("vault.json");
        open(&file, "home password", "0.3.0").unwrap().restore(&path).unwrap();
        assert!(vault::unlock(&path, "home password").is_ok());
        assert!(!before_import_path(&path).exists());
    }

    #[test]
    fn data_files_and_backups_are_not_interchangeable() {
        let dir = temp_dir();
        let (path, v) = sample(&dir, "home password");
        let file = dir.join("b.reachbackup");
        export(&v, "home password", &ui(), "0.3.0", &file).unwrap();

        assert_eq!(open(&path, "home password", "0.3.0").err().unwrap().to_string(), not_a_backup().to_string());
        assert!(vault::unlock(&file, "home password").is_err());
        let junk = dir.join("notes.txt");
        fs::write(&junk, "hello").unwrap();
        assert!(open(&junk, "home password", "0.3.0").is_err());
    }

    #[test]
    fn backups_from_a_newer_reach_are_refused() {
        let dir = temp_dir();
        let (_, v) = sample(&dir, "home password");
        let file = dir.join("b.reachbackup");
        export(&v, "home password", &ui(), "0.4.0", &file).unwrap();

        let e = open(&file, "home password", "0.3.9").err().unwrap().to_string();
        assert!(e.contains("Reach 0.4.0") && e.contains("Update"), "{e}");
        assert!(open(&file, "home password", "0.4.0").is_ok());
        assert!(open(&file, "home password", "1.0.0").is_ok());
    }

    #[test]
    fn interface_settings_are_sanity_checked() {
        let dir = temp_dir();
        let (_, v) = sample(&dir, "home password");
        let file = dir.join("b.reachbackup");
        let bad = UiState::from([("other.key".into(), "x".into())]);
        assert!(export(&v, "home password", &bad, "0.3.0", &file).is_err());
        let huge = UiState::from([("reach.theme".into(), "x".repeat(70_000))]);
        assert!(export(&v, "home password", &huge, "0.3.0", &file).is_err());
    }

    #[test]
    fn version_order() {
        assert!(is_newer("0.10.0", "0.9.2"));
        assert!(is_newer("1.0.0", "0.99.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.3.0"));
        assert!(!is_newer("0.3.0-beta", "0.3.0"));
        assert!(!is_newer("nonsense", "0.3.0"));
    }
}
