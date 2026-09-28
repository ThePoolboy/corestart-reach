//! Launch RDP sessions in the platform's own client.
//!
//! - Linux: FreeRDP. All arguments, including the password, are written to its stdin
//!   (`/args-from:stdin`) so they never show up in the process list.
//! - Windows: mstsc. The password is stored as a session-only `TERMSRV/<host>`
//!   credential (what `cmdkey` does) and mstsc gets a temporary .rdp file.

use serde::Serialize;

use crate::error::Result;
use crate::model::Resolved;

/// Sent to the main window when an RDP client exits with an error.
#[cfg_attr(windows, allow(dead_code))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RdpExit {
    pub name: String,
    pub message: String,
}

/// Can we launch without asking for anything? On Linux FreeRDP has nowhere to
/// prompt, so a missing username or password has to be asked for up front.
/// mstsc on Windows shows its own login prompt.
pub fn needs_credentials(c: &Resolved) -> bool {
    cfg!(not(windows)) && (c.username.is_empty() || c.password.is_none())
}

/// `host:port`, with IPv6 addresses bracketed.
fn address(c: &Resolved) -> String {
    if c.host.contains(':') && !c.host.starts_with('[') {
        format!("[{}]:{}", c.host, c.port)
    } else {
        format!("{}:{}", c.host, c.port)
    }
}

#[cfg(not(windows))]
pub use linux::launch;
#[cfg(windows)]
pub use windows_impl::launch;

#[cfg(not(windows))]
mod linux {
    use std::io::{BufRead, BufReader, Write};
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    use tauri::{AppHandle, Emitter};

    use super::{RdpExit, Result, Resolved, address};
    use crate::error::msg;
    use crate::model::RdpScreen;

    /// FreeRDP binary names, best first. Distros name them differently.
    const CLIENTS: &[&str] = &[
        "xfreerdp3",
        "xfreerdp",
        "sdl-freerdp3",
        "sdl-freerdp",
        "wlfreerdp3",
        "wlfreerdp",
    ];

    fn find_client() -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        CLIENTS.iter().find_map(|name| {
            std::env::split_paths(&path)
                .map(|dir| dir.join(name))
                .find(|p| p.is_file())
        })
    }

    fn arguments(c: &Resolved) -> Vec<String> {
        let mut args = vec![
            format!("/v:{}", address(c)),
            format!("/t:{}", c.name),
            // Trust the certificate the first time, refuse if it later changes.
            "/cert:tofu".into(),
            "+clipboard".into(),
            "/auto-reconnect".into(),
        ];
        if !c.username.is_empty() {
            args.push(format!("/u:{}", c.username));
        }
        if !c.domain.is_empty() {
            args.push(format!("/d:{}", c.domain));
        }
        if let Some(p) = &c.password {
            args.push(format!("/p:{p}"));
        }
        match c.rdp_screen {
            RdpScreen::Fullscreen => args.push("/f".into()),
            RdpScreen::Window => {
                args.push("/size:1600x900".into());
                args.push("+dynamic-resolution".into());
            }
        }
        args
    }

    pub fn launch(app: &AppHandle, c: &Resolved) -> Result<()> {
        let exe = find_client().ok_or_else(|| {
            msg("FreeRDP isn't installed. On Fedora run: sudo dnf install freerdp")
        })?;
        let args = arguments(c);
        if args.iter().any(|a| a.contains('\n')) {
            return Err(msg("Connection settings can't contain line breaks."));
        }

        let mut child = Command::new(&exe)
            .arg("/args-from:stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| msg(format!("Couldn't start {}: {e}", exe.display())))?;

        let mut stdin = child.stdin.take().expect("stdin is piped");
        stdin.write_all(args.join("\n").as_bytes())?;
        stdin.write_all(b"\n")?;
        drop(stdin);

        // Watch the client so a failed login is reported instead of vanishing silently.
        let stderr = child.stderr.take().expect("stderr is piped");
        let app = app.clone();
        let name = c.name.clone();
        std::thread::spawn(move || {
            let mut last_error = None;
            for line in BufReader::new(stderr).lines().map_while(|l| l.ok()) {
                if line.contains("[ERROR]") {
                    // "[time] [pid:tid] [ERROR][tag] - [function]: message" -> message
                    let text = line.rsplit_once("]: ").map(|(_, m)| m).unwrap_or(&line);
                    last_error = Some(text.trim().to_string());
                }
            }
            if let Ok(status) = child.wait() {
                if !status.success() {
                    if let Some(message) = last_error {
                        let _ = app.emit("rdp-exit", RdpExit { name, message });
                    }
                }
            }
        });
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::model::Protocol;

        #[test]
        fn password_only_goes_through_stdin_args() {
            let c = Resolved {
                name: "Web 1".into(),
                protocol: Protocol::Rdp,
                host: "fe80::1".into(),
                port: 3389,
                username: "admin".into(),
                domain: "CORP".into(),
                password: Some("p@ss word".into()),
                ssh_key_path: String::new(),
                ssh_key_passphrase: None,
                rdp_screen: RdpScreen::Window,
            };
            let args = arguments(&c);
            assert!(args.contains(&"/v:[fe80::1]:3389".to_string()));
            assert!(args.contains(&"/p:p@ss word".to_string()));
            assert!(args.contains(&"/d:CORP".to_string()));
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    use tauri::AppHandle;
    use windows::Win32::Security::Credentials::{
        CRED_PERSIST_SESSION, CRED_TYPE_GENERIC, CREDENTIALW, CredWriteW,
    };
    use windows::core::PWSTR;

    use super::{Result, Resolved, address};
    use crate::error::msg;
    use crate::model::RdpScreen;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Same as `cmdkey /generic:TERMSRV/<host> /user:<user> /pass:<password>`,
    /// but only kept until you sign out of Windows.
    fn store_credential(host: &str, user: &str, password: &str) -> Result<()> {
        let mut target = wide(&format!("TERMSRV/{host}"));
        let mut user = wide(user);
        let blob: Vec<u8> = password.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let cred = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target.as_mut_ptr()),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_SESSION,
            UserName: PWSTR(user.as_mut_ptr()),
            ..Default::default()
        };
        // SAFETY: every pointer in `cred` points into a buffer that outlives the call.
        unsafe { CredWriteW(&cred, 0) }
            .map_err(|e| msg(format!("Couldn't save the RDP password for mstsc: {e}")))
    }

    fn rdp_file(c: &Resolved, user: &str) -> String {
        let screen = match c.rdp_screen {
            RdpScreen::Fullscreen => 2,
            RdpScreen::Window => 1,
        };
        let mut lines = vec![
            format!("full address:s:{}", address(c)),
            format!("screen mode id:i:{screen}"),
            "dynamic resolution:i:1".into(),
            "redirectclipboard:i:1".into(),
            "autoreconnection enabled:i:1".into(),
            "authentication level:i:2".into(),
        ];
        if !user.is_empty() {
            lines.push(format!("username:s:{user}"));
        }
        if c.password.is_some() {
            lines.push("prompt for credentials:i:0".into());
        }
        lines.join("\r\n") + "\r\n"
    }

    pub fn launch(_app: &AppHandle, c: &Resolved) -> Result<()> {
        let user = match (c.domain.is_empty(), c.username.is_empty()) {
            (false, false) => format!("{}\\{}", c.domain, c.username),
            _ => c.username.clone(),
        };
        if let Some(password) = &c.password {
            store_credential(&c.host, &user, password)?;
        }

        // mstsc expects UTF-16LE with a BOM, like the files it saves itself.
        let text = rdp_file(c, &user);
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        let path = std::env::temp_dir().join(format!("corestart-reach-{}.rdp", uuid::Uuid::new_v4()));
        std::fs::write(&path, bytes)?;

        Command::new("mstsc.exe")
            .arg(&path)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| msg(format!("Couldn't start mstsc: {e}")))?;

        // mstsc reads the file on start-up; tidy it away afterwards.
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(20));
            let _ = std::fs::remove_file(path);
        });
        Ok(())
    }
}
