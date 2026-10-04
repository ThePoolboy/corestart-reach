//! Sample vault for screenshots. Not part of the app; run it on purpose with
//! `npm run demo-vault` (see the README). It refuses to overwrite a vault.
//!
//! Every name and address is made up (example.com, 10.x addresses).

use std::path::PathBuf;

use uuid::Uuid;

use crate::model::{
    ConnectionInput, CredentialInput, FolderInput, Protocol, RdpScreen, RdpSize, SecretUpdate, VaultData,
};
use crate::vault;

const PASSWORD: &str = "demo-password";

/// Where the vault goes: `REACH_DEMO_VAULT`, or where a build from source keeps it.
fn vault_path() -> PathBuf {
    if let Some(p) = std::env::var_os("REACH_DEMO_VAULT") {
        return p.into();
    }
    let base = if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA").expect("APPDATA is not set"))
    } else if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(xdg)
    } else {
        PathBuf::from(std::env::var_os("HOME").expect("HOME is not set")).join(".local/share")
    };
    base.join("io.github.thepoolboy.corestart-reach").join("vault.json")
}

fn folder(data: &mut VaultData, name: &str, parent: Option<Uuid>) -> Uuid {
    data.save_folder(FolderInput {
        id: None,
        name: name.into(),
        parent_id: parent,
    })
    .unwrap()
}

fn credential(data: &mut VaultData, name: &str, username: &str, domain: &str) -> Uuid {
    data.save_credential(CredentialInput {
        id: None,
        name: name.into(),
        username: username.into(),
        domain: domain.into(),
        password: SecretUpdate::Set(PASSWORD.into()),
    })
    .unwrap()
}

struct Conn<'a> {
    name: &'a str,
    protocol: Protocol,
    host: &'a str,
    port: Option<u16>,
    credential: Option<Uuid>,
    username: &'a str,
    key: &'a str,
    notes: &'a str,
}

impl Conn<'_> {
    fn rdp<'a>(name: &'a str, host: &'a str, credential: Uuid) -> Conn<'a> {
        Conn {
            name,
            protocol: Protocol::Rdp,
            host,
            port: None,
            credential: Some(credential),
            username: "",
            key: "",
            notes: "",
        }
    }

    fn ssh<'a>(name: &'a str, host: &'a str, username: &'a str) -> Conn<'a> {
        Conn {
            name,
            protocol: Protocol::Ssh,
            host,
            port: None,
            credential: None,
            username,
            key: "",
            notes: "",
        }
    }

    fn save(self, data: &mut VaultData, folder: Option<Uuid>) {
        data.save_connection(ConnectionInput {
            id: None,
            name: self.name.into(),
            folder_id: folder,
            protocol: self.protocol,
            host: self.host.into(),
            port: self.port,
            credential_id: self.credential,
            username: self.username.into(),
            domain: String::new(),
            // SSH entries with a key log in with it; the rest show a saved password.
            password: if self.key.is_empty() && self.credential.is_none() {
                SecretUpdate::Set(PASSWORD.into())
            } else {
                SecretUpdate::Keep
            },
            ssh_key_path: self.key.into(),
            ssh_key_passphrase: SecretUpdate::Keep,
            rdp_screen: RdpScreen::Window,
            rdp_size: RdpSize::default(),
            notes: self.notes.into(),
        })
        .unwrap();
    }
}

fn fill(data: &mut VaultData) {
    let admin = credential(data, "Domain admin", "it-admin", "EXAMPLE");
    let helpdesk = credential(data, "Helpdesk", "helpdesk", "EXAMPLE");

    let hq = folder(data, "Headquarters", None);
    let dcs = folder(data, "Domain controllers", Some(hq));
    Conn {
        notes: "Primary DC. Holds the FSMO roles.",
        ..Conn::rdp("DC-01", "10.10.0.10", admin)
    }
    .save(data, Some(dcs));
    Conn::rdp("DC-02", "10.10.0.11", admin).save(data, Some(dcs));
    Conn::rdp("FS-01 (file server)", "fs-01.corp.example.com", admin).save(data, Some(hq));
    Conn::rdp("PRINT-01", "print-01.corp.example.com", helpdesk).save(data, Some(hq));
    Conn::rdp("Reception PC", "10.10.20.31", helpdesk).save(data, Some(hq));

    let web = folder(data, "Web", None);
    Conn {
        key: "~/.ssh/id_ed25519",
        ..Conn::ssh("web-01", "web-01.example.com", "deploy")
    }
    .save(data, Some(web));
    Conn {
        key: "~/.ssh/id_ed25519",
        ..Conn::ssh("web-02", "web-02.example.com", "deploy")
    }
    .save(data, Some(web));
    Conn {
        notes: "HAProxy in front of web-01 and web-02.",
        ..Conn::ssh("lb-01", "10.20.0.5", "ops")
    }
    .save(data, Some(web));

    let db = folder(data, "Databases", None);
    Conn::ssh("db-primary", "10.20.1.10", "ops").save(data, Some(db));
    Conn::ssh("db-replica", "10.20.1.11", "ops").save(data, Some(db));

    let lab = folder(data, "Lab", None);
    Conn {
        port: Some(2222),
        ..Conn::ssh("pve-01 (Proxmox)", "192.168.50.2", "root")
    }
    .save(data, Some(lab));
    Conn::rdp("win11-test", "192.168.50.21", admin).save(data, Some(lab));
    Conn::ssh("fedora-test", "192.168.50.22", "tester").save(data, Some(lab));

    Conn {
        port: Some(2222),
        key: "~/.ssh/id_ed25519",
        notes: "Jump host for the DMZ.",
        ..Conn::ssh("Bastion", "bastion.example.com", "ops")
    }
    .save(data, None);
}

#[test]
#[ignore = "writes a sample vault for screenshots; run with `npm run demo-vault`"]
fn write_demo_vault() {
    let path = vault_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut v = vault::create(&path, PASSWORD)
        .unwrap_or_else(|e| panic!("{e} Move {} aside first.", path.display()));
    fill(&mut v.data);
    v.save(&path).unwrap();
    println!(
        "Sample vault written to {}\nMaster password: {PASSWORD}",
        path.display()
    );
}

#[test]
fn sample_data_is_valid() {
    let mut data = VaultData::default();
    fill(&mut data);
    assert_eq!(data.folders.len(), 5);
    assert_eq!(data.connections.len(), 14);
}
