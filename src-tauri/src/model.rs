//! Everything stored inside the vault, plus the secret-free views sent to the UI.
//!
//! Passwords never leave the Rust side: the UI only sees `has_password` flags and
//! sends back a [`SecretUpdate`] saying whether to keep, clear or replace a secret.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Result, msg};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultData {
    #[serde(default)]
    pub folders: Vec<Folder>,
    #[serde(default)]
    pub connections: Vec<Connection>,
    #[serde(default)]
    pub credentials: Vec<Credential>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Rdp,
    Ssh,
}

impl Protocol {
    pub fn default_port(self) -> u16 {
        match self {
            Protocol::Rdp => 3389,
            Protocol::Ssh => 22,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RdpScreen {
    #[default]
    Window,
    Fullscreen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub folder_id: Option<Uuid>,
    pub protocol: Protocol,
    pub host: String,
    #[serde(default)]
    pub port: Option<u16>,
    /// When set, username/domain/password come from this saved credential.
    #[serde(default)]
    pub credential_id: Option<Uuid>,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub ssh_key_path: String,
    #[serde(default)]
    pub ssh_key_passphrase: Option<String>,
    #[serde(default)]
    pub rdp_screen: RdpScreen,
    #[serde(default)]
    pub notes: String,
}

/// A reusable username/password, e.g. a domain admin account shared by many servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credential {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub password: Option<String>,
}

// ---------------------------------------------------------------------------
// Views sent to the UI (no secrets)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub id: Uuid,
    pub name: String,
    pub folder_id: Option<Uuid>,
    pub protocol: Protocol,
    pub host: String,
    pub port: Option<u16>,
    pub credential_id: Option<Uuid>,
    pub username: String,
    pub domain: String,
    pub has_password: bool,
    pub ssh_key_path: String,
    pub has_key_passphrase: bool,
    pub rdp_screen: RdpScreen,
    pub notes: String,
}

impl From<&Connection> for ConnectionView {
    fn from(c: &Connection) -> Self {
        Self {
            id: c.id,
            name: c.name.clone(),
            folder_id: c.folder_id,
            protocol: c.protocol,
            host: c.host.clone(),
            port: c.port,
            credential_id: c.credential_id,
            username: c.username.clone(),
            domain: c.domain.clone(),
            has_password: c.password.is_some(),
            ssh_key_path: c.ssh_key_path.clone(),
            has_key_passphrase: c.ssh_key_passphrase.is_some(),
            rdp_screen: c.rdp_screen,
            notes: c.notes.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialView {
    pub id: Uuid,
    pub name: String,
    pub username: String,
    pub domain: String,
    pub has_password: bool,
}

impl From<&Credential> for CredentialView {
    fn from(c: &Credential) -> Self {
        Self {
            id: c.id,
            name: c.name.clone(),
            username: c.username.clone(),
            domain: c.domain.clone(),
            has_password: c.password.is_some(),
        }
    }
}

/// The whole vault as the UI sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tree {
    pub folders: Vec<Folder>,
    pub connections: Vec<ConnectionView>,
    pub credentials: Vec<CredentialView>,
}

impl From<&VaultData> for Tree {
    fn from(d: &VaultData) -> Self {
        Self {
            folders: d.folders.clone(),
            connections: d.connections.iter().map(Into::into).collect(),
            credentials: d.credentials.iter().map(Into::into).collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Input from the UI
// ---------------------------------------------------------------------------

/// What to do with a stored secret: `"keep"`, `"clear"` or `{"set": "..."}`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretUpdate {
    #[default]
    Keep,
    Clear,
    Set(String),
}

impl SecretUpdate {
    fn apply(self, slot: &mut Option<String>) {
        match self {
            SecretUpdate::Keep => {}
            SecretUpdate::Clear => *slot = None,
            SecretUpdate::Set(value) => *slot = Some(value),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    pub id: Option<Uuid>,
    pub name: String,
    pub folder_id: Option<Uuid>,
    pub protocol: Protocol,
    pub host: String,
    pub port: Option<u16>,
    pub credential_id: Option<Uuid>,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub password: SecretUpdate,
    #[serde(default)]
    pub ssh_key_path: String,
    #[serde(default)]
    pub ssh_key_passphrase: SecretUpdate,
    #[serde(default)]
    pub rdp_screen: RdpScreen,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderInput {
    pub id: Option<Uuid>,
    pub name: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialInput {
    pub id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub password: SecretUpdate,
}

// ---------------------------------------------------------------------------
// Mutations
// ---------------------------------------------------------------------------

impl VaultData {
    /// Insert or update a connection. Returns its id.
    pub fn save_connection(&mut self, input: ConnectionInput) -> Result<Uuid> {
        let host = input.host.trim().to_string();
        if host.is_empty() || host.contains(char::is_whitespace) {
            return Err(msg("Enter a host name or IP address (no spaces)."));
        }
        if input.port == Some(0) {
            return Err(msg("Port must be between 1 and 65535."));
        }
        self.check_folder(input.folder_id)?;
        if let Some(cid) = input.credential_id {
            if !self.credentials.iter().any(|c| c.id == cid) {
                return Err(msg("That saved credential no longer exists."));
            }
        }
        let name = match input.name.trim() {
            "" => host.clone(),
            n => n.to_string(),
        };

        let id = input.id.unwrap_or_else(Uuid::new_v4);
        let index = match self.connections.iter().position(|c| c.id == id) {
            Some(i) => i,
            None => {
                self.connections.push(Connection {
                    id,
                    name: String::new(),
                    folder_id: None,
                    protocol: input.protocol,
                    host: String::new(),
                    port: None,
                    credential_id: None,
                    username: String::new(),
                    domain: String::new(),
                    password: None,
                    ssh_key_path: String::new(),
                    ssh_key_passphrase: None,
                    rdp_screen: RdpScreen::default(),
                    notes: String::new(),
                });
                self.connections.len() - 1
            }
        };
        let c = &mut self.connections[index];
        c.name = name;
        c.folder_id = input.folder_id;
        c.protocol = input.protocol;
        c.host = host;
        c.port = input.port;
        c.credential_id = input.credential_id;
        c.username = input.username.trim().to_string();
        c.domain = input.domain.trim().to_string();
        input.password.apply(&mut c.password);
        c.ssh_key_path = input.ssh_key_path.trim().to_string();
        input.ssh_key_passphrase.apply(&mut c.ssh_key_passphrase);
        c.rdp_screen = input.rdp_screen;
        c.notes = input.notes;
        Ok(id)
    }

    pub fn duplicate_connection(&mut self, id: Uuid) -> Result<Uuid> {
        let mut copy = self.connection(id)?.clone();
        copy.id = Uuid::new_v4();
        copy.name = format!("{} (copy)", copy.name);
        let new_id = copy.id;
        self.connections.push(copy);
        Ok(new_id)
    }

    pub fn delete_connection(&mut self, id: Uuid) {
        self.connections.retain(|c| c.id != id);
    }

    pub fn save_folder(&mut self, input: FolderInput) -> Result<Uuid> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(msg("Folder name can't be empty."));
        }
        self.check_folder(input.parent_id)?;
        let id = input.id.unwrap_or_else(Uuid::new_v4);
        // Refuse to move a folder inside itself or one of its own subfolders.
        let mut cursor = input.parent_id;
        while let Some(p) = cursor {
            if p == id {
                return Err(msg("A folder can't be moved inside itself."));
            }
            cursor = self.folders.iter().find(|f| f.id == p).and_then(|f| f.parent_id);
        }
        match self.folders.iter_mut().find(|f| f.id == id) {
            Some(f) => {
                f.name = name.to_string();
                f.parent_id = input.parent_id;
            }
            None => self.folders.push(Folder {
                id,
                name: name.to_string(),
                parent_id: input.parent_id,
            }),
        }
        Ok(id)
    }

    /// Delete a folder. Its contents move up to the folder's parent.
    pub fn delete_folder(&mut self, id: Uuid) {
        let parent = self.folders.iter().find(|f| f.id == id).and_then(|f| f.parent_id);
        for f in &mut self.folders {
            if f.parent_id == Some(id) {
                f.parent_id = parent;
            }
        }
        for c in &mut self.connections {
            if c.folder_id == Some(id) {
                c.folder_id = parent;
            }
        }
        self.folders.retain(|f| f.id != id);
    }

    pub fn save_credential(&mut self, input: CredentialInput) -> Result<Uuid> {
        let name = match input.name.trim() {
            "" if input.username.trim().is_empty() => {
                return Err(msg("Give the credential a name or a username."));
            }
            "" => input.username.trim().to_string(),
            n => n.to_string(),
        };
        let id = input.id.unwrap_or_else(Uuid::new_v4);
        let index = match self.credentials.iter().position(|c| c.id == id) {
            Some(i) => i,
            None => {
                self.credentials.push(Credential {
                    id,
                    name: String::new(),
                    username: String::new(),
                    domain: String::new(),
                    password: None,
                });
                self.credentials.len() - 1
            }
        };
        let c = &mut self.credentials[index];
        c.name = name;
        c.username = input.username.trim().to_string();
        c.domain = input.domain.trim().to_string();
        input.password.apply(&mut c.password);
        Ok(id)
    }

    /// Delete a saved credential. Connections using it fall back to their own fields.
    pub fn delete_credential(&mut self, id: Uuid) {
        for c in &mut self.connections {
            if c.credential_id == Some(id) {
                c.credential_id = None;
            }
        }
        self.credentials.retain(|c| c.id != id);
    }

    pub fn connection(&self, id: Uuid) -> Result<&Connection> {
        self.connections
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| msg("That connection no longer exists."))
    }

    fn check_folder(&self, id: Option<Uuid>) -> Result<()> {
        match id {
            Some(id) if !self.folders.iter().any(|f| f.id == id) => {
                Err(msg("That folder no longer exists."))
            }
            _ => Ok(()),
        }
    }

    /// Everything needed to open a session, with saved credentials merged in.
    pub fn resolve(&self, id: Uuid) -> Result<Resolved> {
        let c = self.connection(id)?;
        let (username, domain, password) = match c.credential_id {
            Some(cid) => {
                let cred = self
                    .credentials
                    .iter()
                    .find(|x| x.id == cid)
                    .ok_or_else(|| msg("The saved credential for this connection is missing."))?;
                (cred.username.clone(), cred.domain.clone(), cred.password.clone())
            }
            None => (c.username.clone(), c.domain.clone(), c.password.clone()),
        };
        Ok(Resolved {
            name: c.name.clone(),
            protocol: c.protocol,
            host: c.host.clone(),
            port: c.port.unwrap_or(c.protocol.default_port()),
            username,
            domain,
            password,
            ssh_key_path: c.ssh_key_path.clone(),
            ssh_key_passphrase: c.ssh_key_passphrase.clone(),
            rdp_screen: c.rdp_screen,
        })
    }
}

/// A connection ready to launch.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub domain: String,
    pub password: Option<String>,
    pub ssh_key_path: String,
    pub ssh_key_passphrase: Option<String>,
    pub rdp_screen: RdpScreen,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(host: &str) -> ConnectionInput {
        ConnectionInput {
            id: None,
            name: String::new(),
            folder_id: None,
            protocol: Protocol::Ssh,
            host: host.into(),
            port: None,
            credential_id: None,
            username: "admin".into(),
            domain: String::new(),
            password: SecretUpdate::Set("pw".into()),
            ssh_key_path: String::new(),
            ssh_key_passphrase: SecretUpdate::Keep,
            rdp_screen: RdpScreen::Window,
            notes: String::new(),
        }
    }

    #[test]
    fn secrets_are_kept_unless_changed() {
        let mut d = VaultData::default();
        let id = d.save_connection(input("srv1")).unwrap();
        let mut update = input("srv1");
        update.id = Some(id);
        update.password = SecretUpdate::Keep;
        d.save_connection(update).unwrap();
        assert_eq!(d.connection(id).unwrap().password.as_deref(), Some("pw"));
        assert_eq!(d.connection(id).unwrap().name, "srv1");
    }

    #[test]
    fn saved_credential_overrides_inline() {
        let mut d = VaultData::default();
        let cred = d
            .save_credential(CredentialInput {
                id: None,
                name: "Domain admin".into(),
                username: "CORP\\admin".into(),
                domain: String::new(),
                password: SecretUpdate::Set("secret".into()),
            })
            .unwrap();
        let mut i = input("srv1");
        i.credential_id = Some(cred);
        let id = d.save_connection(i).unwrap();
        let r = d.resolve(id).unwrap();
        assert_eq!(r.username, "CORP\\admin");
        assert_eq!(r.password.as_deref(), Some("secret"));
        assert_eq!(r.port, 22);

        d.delete_credential(cred);
        assert_eq!(d.resolve(id).unwrap().username, "admin");
    }

    #[test]
    fn deleting_folder_moves_contents_up() {
        let mut d = VaultData::default();
        let outer = d.save_folder(FolderInput { id: None, name: "Outer".into(), parent_id: None }).unwrap();
        let inner = d
            .save_folder(FolderInput { id: None, name: "Inner".into(), parent_id: Some(outer) })
            .unwrap();
        let mut i = input("srv1");
        i.folder_id = Some(inner);
        let id = d.save_connection(i).unwrap();
        d.delete_folder(inner);
        assert_eq!(d.connection(id).unwrap().folder_id, Some(outer));
        assert!(
            d.save_folder(FolderInput { id: Some(outer), name: "Outer".into(), parent_id: Some(outer) })
                .is_err()
        );
    }
}
