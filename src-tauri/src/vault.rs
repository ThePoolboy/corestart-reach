//! The encrypted vault file.
//!
//! One JSON file holds a small plaintext header (format, KDF settings, nonce) and the
//! encrypted body. The body is the whole [`VaultData`] (hosts, usernames, passwords
//! and notes), so nothing about your servers is readable without the master password.
//!
//! - Key derivation: Argon2id, 64 MiB, 3 passes, from the master password + random salt.
//! - Encryption: XChaCha20-Poly1305 with a fresh random 24-byte nonce on every save.

use std::fs;
use std::io::Write;
use std::path::Path;

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::{Result, msg};
use crate::model::VaultData;

const FORMAT: &str = "corestart-reach-vault";
const VERSION: u32 = 1;
const AAD: &[u8] = b"corestart-reach-vault-v1";

const DEFAULT_M_KIB: u32 = 64 * 1024;
const DEFAULT_T: u32 = 3;
const DEFAULT_P: u32 = 1;

pub const MIN_PASSWORD_LEN: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct KdfParams {
    alg: String,
    m_kib: u32,
    t: u32,
    p: u32,
    salt: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VaultFile {
    format: String,
    version: u32,
    kdf: KdfParams,
    cipher: String,
    nonce: String,
    data: String,
}

/// An open vault: the derived key and the decrypted contents.
pub struct Unlocked {
    pub(crate) key: Zeroizing<[u8; 32]>,
    pub(crate) kdf: KdfParams,
    pub data: VaultData,
}

pub(crate) fn random<const N: usize>() -> Result<[u8; N]> {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).map_err(|e| msg(format!("No secure random source: {e}")))?;
    Ok(buf)
}

pub(crate) fn derive_key(password: &str, kdf: &KdfParams) -> Result<Zeroizing<[u8; 32]>> {
    if kdf.alg != "argon2id" {
        return Err(msg(format!("Unsupported key derivation '{}'.", kdf.alg)));
    }
    // Bound the settings so a tampered file can't make us allocate gigabytes.
    if kdf.m_kib > 4 * 1024 * 1024 || kdf.t > 100 || kdf.p > 64 {
        return Err(msg("The vault file has unreasonable key settings and may be damaged."));
    }
    let salt = B64.decode(&kdf.salt).map_err(|_| msg("The vault file is damaged (salt)."))?;
    let params = Params::new(kdf.m_kib, kdf.t, kdf.p, Some(32))
        .map_err(|e| msg(format!("Bad key settings: {e}")))?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), &salt, key.as_mut())
        .map_err(|e| msg(format!("Key derivation failed: {e}")))?;
    Ok(key)
}

pub(crate) fn cipher(key: &[u8; 32]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(&Key::from(*key))
}

pub fn exists(path: &Path) -> bool {
    path.is_file()
}

/// Create a brand-new, empty vault. Fails if one already exists.
pub fn create(path: &Path, password: &str) -> Result<Unlocked> {
    if exists(path) {
        return Err(msg("A vault already exists."));
    }
    check_new_password(password)?;
    let kdf = new_kdf()?;
    let key = derive_key(password, &kdf)?;
    let vault = Unlocked { key, kdf, data: VaultData::default() };
    vault.save(path)?;
    Ok(vault)
}

fn check_new_password(password: &str) -> Result<()> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(msg(format!(
            "Use a master password of at least {MIN_PASSWORD_LEN} characters."
        )));
    }
    Ok(())
}

/// Current key settings with a fresh random salt.
fn new_kdf() -> Result<KdfParams> {
    Ok(KdfParams {
        alg: "argon2id".into(),
        m_kib: DEFAULT_M_KIB,
        t: DEFAULT_T,
        p: DEFAULT_P,
        salt: B64.encode(random::<16>()?),
    })
}

/// Compare keys without an early exit, so timing doesn't reveal how much matched.
fn same_key(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

/// Open an existing vault with the master password.
pub fn unlock(path: &Path, password: &str) -> Result<Unlocked> {
    let raw = fs::read(path)?;
    let file: VaultFile =
        serde_json::from_slice(&raw).map_err(|_| msg("The vault file is damaged or not a vault."))?;
    if file.format != FORMAT {
        return Err(msg("This file is not a Corestart Reach vault."));
    }
    if file.version > VERSION {
        return Err(msg("This vault was made by a newer version of Corestart Reach. Please update."));
    }
    if file.cipher != "xchacha20poly1305" {
        return Err(msg(format!("Unsupported cipher '{}'.", file.cipher)));
    }
    let nonce: [u8; 24] = B64
        .decode(&file.nonce)
        .ok()
        .and_then(|n| n.try_into().ok())
        .ok_or_else(|| msg("The vault file is damaged (nonce)."))?;
    let body = B64.decode(&file.data).map_err(|_| msg("The vault file is damaged (data)."))?;

    let key = derive_key(password, &file.kdf)?;
    let plain = cipher(&key)
        .decrypt(&XNonce::from(nonce), Payload { msg: &body, aad: AAD })
        .map_err(|_| msg("Wrong master password."))?;
    let plain = Zeroizing::new(plain);
    let data: VaultData = serde_json::from_slice(&plain)?;
    Ok(Unlocked { key, kdf: file.kdf, data })
}

impl Unlocked {
    /// Encrypt and write the vault. Writes to a temp file first and keeps the
    /// previous version as `<name>.bak`, so a crash mid-save can't lose data.
    pub fn save(&self, path: &Path) -> Result<()> {
        let plain = Zeroizing::new(serde_json::to_vec(&self.data)?);
        let nonce = random::<24>()?;
        let body = cipher(&self.key)
            .encrypt(&XNonce::from(nonce), Payload { msg: &plain, aad: AAD })
            .map_err(|_| msg("Encryption failed."))?;
        let file = VaultFile {
            format: FORMAT.into(),
            version: VERSION,
            kdf: self.kdf.clone(),
            cipher: "xchacha20poly1305".into(),
            nonce: B64.encode(nonce),
            data: B64.encode(body),
        };

        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        let _ = fs::remove_file(&tmp); // leftover from a crash; recreate it private
        {
            let mut f = create_private(&tmp)?;
            f.write_all(&serde_json::to_vec_pretty(&file)?)?;
            f.sync_all()?;
        }
        if path.exists() {
            fs::copy(path, path.with_extension("json.bak"))?;
        }
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Whether `password` is this vault's master password.
    pub fn is_password(&self, password: &str) -> Result<bool> {
        let entered = derive_key(password, &self.kdf)?;
        Ok(same_key(&entered, &self.key))
    }

    /// Re-encrypt the vault under a new master password, with a new salt.
    ///
    /// The backup is replaced too: otherwise `vault.json.bak` would still open
    /// with the old password, which matters if you're changing it because it leaked.
    pub fn change_password(&mut self, path: &Path, current: &str, new: &str) -> Result<()> {
        if !self.is_password(current)? {
            return Err(msg("The current master password is wrong."));
        }
        check_new_password(new)?;
        if current == new {
            return Err(msg("The new password is the same as the current one."));
        }
        let kdf = new_kdf()?;
        let key = derive_key(new, &kdf)?;
        let old_key = std::mem::replace(&mut self.key, key);
        let old_kdf = std::mem::replace(&mut self.kdf, kdf);
        if let Err(e) = self.save(path) {
            self.key = old_key;
            self.kdf = old_kdf;
            return Err(e);
        }
        fs::copy(path, path.with_extension("json.bak"))?;
        Ok(())
    }
}

/// A new file only its owner can read (0600 on Linux). Windows already keeps
/// files under the user's AppData private to that user.
fn create_private(path: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Folder;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("reach-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn round_trip_and_wrong_password() {
        let path = temp_path("vault.json");
        let mut v = create(&path, "correct horse").unwrap();
        v.data.folders.push(Folder { id: uuid::Uuid::new_v4(), name: "Servers".into(), parent_id: None });
        v.save(&path).unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("Servers"), "folder name must be encrypted");

        let opened = unlock(&path, "correct horse").unwrap();
        assert_eq!(opened.data.folders[0].name, "Servers");
        assert!(unlock(&path, "wrong password").is_err());
        assert!(create(&path, "correct horse").is_err());
        assert!(path.with_extension("json.bak").exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "vault must be readable by its owner only");
        }
    }

    #[test]
    fn changing_the_master_password() {
        let path = temp_path("vault.json");
        let mut v = create(&path, "old password").unwrap();
        v.data.folders.push(Folder { id: uuid::Uuid::new_v4(), name: "Kept".into(), parent_id: None });
        v.save(&path).unwrap();

        assert!(v.change_password(&path, "not the password", "new password").is_err());
        assert!(v.change_password(&path, "old password", "short").is_err());
        assert!(v.change_password(&path, "old password", "old password").is_err());
        v.change_password(&path, "old password", "new password").unwrap();

        assert!(unlock(&path, "old password").is_err());
        assert_eq!(unlock(&path, "new password").unwrap().data.folders[0].name, "Kept");
        // The backup must not open with the old password either.
        let bak = path.with_extension("json.bak");
        assert!(unlock(&bak, "old password").is_err());
        assert!(unlock(&bak, "new password").is_ok());
    }

    #[test]
    fn short_password_rejected() {
        assert!(create(&temp_path("v.json"), "short").is_err());
    }
}
