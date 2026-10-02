//! Launch RDP sessions in the platform's own client.
//!
//! - Linux: FreeRDP. All arguments, including the password, are written to its stdin
//!   (`/args-from:stdin`) so they never show up in the process list.
//! - Windows: mstsc. The password is stored as a session-only `TERMSRV/<host>`
//!   credential (what `cmdkey` does) and mstsc is started with `/v:host`. No .rdp
//!   file: Windows 11 warns about every unsigned .rdp file that gets opened.

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

/// Can we launch without asking for anything? Both clients get the login up
/// front: FreeRDP has nowhere to prompt, and mstsc started with `/v` reads it
/// from the stored credential.
pub fn needs_credentials(c: &Resolved) -> bool {
    c.username.is_empty() || c.password.is_none()
}

/// `host:port`, with IPv6 addresses bracketed.
fn address(c: &Resolved) -> String {
    if c.host.contains(':') && !c.host.starts_with('[') {
        format!("[{}]:{}", c.host, c.port)
    } else {
        format!("{}:{}", c.host, c.port)
    }
}

/// The mstsc settings that make the remote desktop follow its window: change the
/// remote resolution when the window is resized, instead of stretching the picture
/// (smart sizing) or adding scroll bars.
#[cfg_attr(not(windows), allow(dead_code))]
const FOLLOW_WINDOW: &[(&str, &str)] = &[("dynamic resolution:i:", "1"), ("smart sizing:i:", "0")];

/// mstsc's `Default.rdp` with [`FOLLOW_WINDOW`] set, or `None` if it already is.
/// mstsc saves the file as UTF-16 with a byte order mark; it's written back the
/// same way it was read.
#[cfg_attr(not(windows), allow(dead_code))]
fn follow_window(file: &[u8]) -> Option<Vec<u8>> {
    let utf16 = file.starts_with(&[0xFF, 0xFE]);
    let text = if utf16 {
        let units: Vec<u16> = file[2..].chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(file.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(file)).into_owned()
    };

    let mut out = String::with_capacity(text.len() + 64);
    let mut missing = FOLLOW_WINDOW.to_vec();
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        match FOLLOW_WINDOW.iter().find(|(key, _)| body.starts_with(key)) {
            Some((key, value)) => {
                missing.retain(|(k, _)| k != key);
                out.push_str(key);
                out.push_str(value);
                out.push_str(&line[body.len()..]);
            }
            None => out.push_str(line),
        }
    }
    for (key, value) in missing {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str("\r\n");
        }
        out.push_str(&format!("{key}{value}\r\n"));
    }
    if out == text {
        return None;
    }

    Some(if utf16 {
        [0xFF, 0xFE].into_iter().chain(out.encode_utf16().flat_map(u16::to_le_bytes)).collect()
    } else {
        out.into_bytes()
    })
}

#[cfg(test)]
mod tests {
    use super::follow_window;

    #[test]
    fn default_rdp_is_made_to_follow_the_window() {
        let utf16 = |s: &str| -> Vec<u8> {
            [0xFF, 0xFE].into_iter().chain(s.encode_utf16().flat_map(u16::to_le_bytes)).collect()
        };
        let before = "screen mode id:i:1\r\nsmart sizing:i:1\r\ndynamic resolution:i:0\r\nfull address:s:web1\r\n";
        let after = "screen mode id:i:1\r\nsmart sizing:i:0\r\ndynamic resolution:i:1\r\nfull address:s:web1\r\n";
        assert_eq!(follow_window(&utf16(before)), Some(utf16(after)));
        assert_eq!(follow_window(&utf16(after)), None);

        // Missing settings are added; plain UTF-8 stays UTF-8.
        assert_eq!(
            follow_window(b"full address:s:web1").as_deref(),
            Some(&b"full address:s:web1\r\ndynamic resolution:i:1\r\nsmart sizing:i:0\r\n"[..])
        );
    }
}

#[cfg(not(windows))]
pub use linux::launch;
#[cfg(windows)]
pub use windows_impl::launch;

#[cfg(not(windows))]
mod linux {
    use std::io::{BufRead, BufReader, Write};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    use tauri::{AppHandle, Emitter};

    use super::{RdpExit, Result, Resolved, address};
    use crate::error::msg;
    use crate::model::RdpScreen;

    /// FreeRDP binary names, best first. Distros name them differently; the
    /// Flatpak ships only the SDL3 client (native Wayland).
    const CLIENTS: &[&str] = &[
        "xfreerdp3",
        "xfreerdp",
        "sdl3-freerdp",
        "sdl-freerdp3",
        "sdl-freerdp",
        "wlfreerdp3",
        "wlfreerdp",
    ];

    /// What to do about a missing FreeRDP. The Flatpak ships its own, so there
    /// it means a broken build; from source it's the distro package.
    fn install_hint() -> &'static str {
        if std::env::var_os("FLATPAK_ID").is_some() {
            "FreeRDP should be included in the Flatpak. Please report this as a bug."
        } else {
            "Fedora: sudo dnf install freerdp\nUbuntu / Debian: sudo apt install freerdp3-x11"
        }
    }

    /// The first FreeRDP 3 client on PATH. FreeRDP 2 can't be used: it has no
    /// `/args-from`, so the password would have to go on the command line.
    fn find_client() -> Result<PathBuf> {
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut too_old = None;
        for name in CLIENTS {
            let Some(exe) = std::env::split_paths(&path).map(|dir| dir.join(name)).find(|p| p.is_file())
            else {
                continue;
            };
            match major_version(&exe) {
                Some(major) if major < 3 => {
                    too_old.get_or_insert((exe, major));
                }
                // 3 or newer, or a version we couldn't read: use it.
                _ => return Ok(exe),
            }
        }
        Err(match too_old {
            Some((exe, major)) => msg(format!(
                "Corestart Reach needs FreeRDP 3, but {} is FreeRDP {major}.\n{}",
                exe.display(),
                install_hint()
            )),
            None => msg(format!("FreeRDP isn't installed.\n{}", install_hint())),
        })
    }

    /// Major version from "This is FreeRDP version 3.31.1 (n/a)".
    fn major_version(exe: &Path) -> Option<u32> {
        let out = Command::new(exe).arg("--version").stdin(Stdio::null()).output().ok()?;
        parse_major(&String::from_utf8_lossy(&out.stdout))
    }

    fn parse_major(text: &str) -> Option<u32> {
        text.split("version ").nth(1)?.split('.').next()?.trim().parse().ok()
    }

    fn arguments(c: &Resolved) -> Vec<String> {
        let mut args = vec![
            format!("/v:{}", address(c)),
            format!("/t:{}", c.name),
            // Trust the certificate the first time, refuse if it later changes.
            "/cert:tofu".into(),
            "+clipboard".into(),
            "+auto-reconnect".into(),
        ];
        if !c.username.is_empty() {
            args.push(format!("/u:{}", c.username));
        }
        if let Some(domain) = c.separate_domain() {
            args.push(format!("/d:{domain}"));
        }
        if let Some(p) = &c.password {
            args.push(format!("/p:{p}"));
        }
        match c.rdp_screen {
            RdpScreen::Fullscreen => args.push("/f".into()),
            RdpScreen::Window => {
                // 80% of the screen, whatever its size; then follows the window.
                args.push("/size:80%".into());
                args.push("+dynamic-resolution".into());
            }
        }
        args
    }

    pub fn launch(app: &AppHandle, c: &Resolved) -> Result<()> {
        let exe = find_client()?;
        let args = arguments(c);
        if args.iter().any(|a| a.contains('\n')) {
            return Err(msg("Connection settings can't contain line breaks."));
        }

        let mut child = Command::new(&exe)
            .arg("/args-from:stdin")
            // FreeRDP's SDL client: show Reach's icon and group with its windows.
            .env("SDL_APP_ID", "io.github.thepoolboy.corestart-reach")
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
            let mut watch = ExitWatch::default();
            for line in BufReader::new(stderr).lines().map_while(|l| l.ok()) {
                watch.line(&line);
            }
            if let Ok(status) = child.wait() {
                if let Some(message) = watch.problem(status.success()) {
                    let _ = app.emit("rdp-exit", RdpExit { name, message });
                }
            }
        });
        Ok(())
    }

    /// Session endings the user chose: signing out of Windows, or Disconnect in
    /// the Start menu. FreeRDP logs them as errors and exits non-zero anyway.
    const USER_ENDED: &[&str] = &["ERRINFO_LOGOFF_BY_USER", "ERRINFO_RPC_INITIATED_DISCONNECT_BY_USER"];

    /// Reads FreeRDP's log to decide whether its exit is worth reporting.
    #[derive(Default)]
    struct ExitWatch {
        user_ended: bool,
        last_error: Option<String>,
    }

    impl ExitWatch {
        fn line(&mut self, line: &str) {
            if USER_ENDED.iter().any(|m| line.contains(m)) {
                self.user_ended = true;
            }
            if line.contains("[ERROR]") {
                // "[time] [pid:tid] [ERROR][tag] - [function]: message" -> message
                let text = line.rsplit_once("]: ").map_or(line, |(_, m)| m).trim();
                // "ERRINFO_IDLE_TIMEOUT (0x00000003):The idle session limit…" -> the sentence
                let text = match text.split_once("):") {
                    Some((code, sentence)) if code.starts_with("ERRINFO_") => sentence.trim(),
                    _ => text,
                };
                self.last_error = Some(text.to_string());
            }
        }

        /// The message to show, or `None` if the session ended normally.
        fn problem(self, success: bool) -> Option<String> {
            if success || self.user_ended { None } else { self.last_error }
        }
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

            // A username that already names its domain wins over the Domain field.
            let c = Resolved { username: "CORP\\admin".into(), domain: "OTHER".into(), ..c };
            let args = arguments(&c);
            assert!(args.contains(&"/u:CORP\\admin".to_string()));
            assert!(!args.iter().any(|a| a.starts_with("/d:")));
        }

        #[test]
        fn reads_freerdp_version() {
            assert_eq!(parse_major("This is FreeRDP version 3.31.1 (n/a)\n"), Some(3));
            assert_eq!(parse_major("This is FreeRDP version 2.11.7 (2.11.7)"), Some(2));
            assert_eq!(parse_major("something else"), None);
        }

        #[test]
        fn signing_out_is_not_an_error() {
            let mut w = ExitWatch::default();
            w.line("[08:40:01:123] [4242:4243] [ERROR][com.freerdp.core] - [rdp_print_errinfo]: ERRINFO_LOGOFF_BY_USER (0x0000000C):The disconnection was initiated by the user logging off their session on the server.");
            assert_eq!(w.problem(false), None);
        }

        #[test]
        fn real_failures_are_reported_readably() {
            let mut w = ExitWatch::default();
            w.line("[08:40:01:123] [4242:4243] [ERROR][com.freerdp.core] - [rdp_print_errinfo]: ERRINFO_IDLE_TIMEOUT (0x00000003):The idle session limit timer on the server has elapsed.");
            assert_eq!(
                w.problem(false).as_deref(),
                Some("The idle session limit timer on the server has elapsed.")
            );

            let mut w = ExitWatch::default();
            w.line("[08:40:01:123] [4242:4243] [ERROR][com.freerdp.core.transport] - [transport_connect]: ERRCONNECT_LOGON_FAILURE [0x00020014]");
            assert_eq!(w.problem(false).as_deref(), Some("ERRCONNECT_LOGON_FAILURE [0x00020014]"));
            assert_eq!(ExitWatch::default().problem(true), None);
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::Command;

    use tauri::AppHandle;
    use windows::Win32::Security::Credentials::{
        CRED_PERSIST_SESSION, CRED_TYPE_GENERIC, CREDENTIALW, CredWriteW,
    };
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Documents, KNOWN_FOLDER_FLAG, SHGetKnownFolderPath};
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

    /// `/v:host:port`, then full screen, or a window at 80% of the screen
    /// (mstsc resizes the remote desktop to match when you resize the window).
    fn arguments(c: &Resolved, screen: Option<(u32, u32)>) -> Vec<String> {
        let mut args = vec![format!("/v:{}", address(c))];
        match (c.rdp_screen, screen) {
            (RdpScreen::Fullscreen, _) => args.push("/f".into()),
            (RdpScreen::Window, Some((w, h))) => {
                args.push(format!("/w:{}", w * 8 / 10));
                args.push(format!("/h:{}", h * 8 / 10));
            }
            (RdpScreen::Window, None) => {}
        }
        args
    }

    /// mstsc's `Documents\Default.rdp`, where it keeps the settings it uses for
    /// `/v` connections. Documents may be redirected (OneDrive), so ask Windows.
    fn default_rdp() -> Option<PathBuf> {
        // SAFETY: the returned string is copied, then freed exactly once.
        unsafe {
            let path = SHGetKnownFolderPath(&FOLDERID_Documents, KNOWN_FOLDER_FLAG(0), None).ok()?;
            let documents = path.to_string();
            CoTaskMemFree(Some(path.0 as *const _));
            Some(PathBuf::from(documents.ok()?).join("Default.rdp"))
        }
    }

    /// Make the remote desktop resize with the mstsc window. There's no switch for
    /// it, so it's set in `Default.rdp`, the file mstsc takes `/v` connections'
    /// settings from. No file means mstsc's defaults, which already resize.
    fn follow_window() -> std::io::Result<()> {
        let Some(file) = default_rdp() else { return Ok(()) };
        let old = match std::fs::read(&file) {
            Ok(old) => old,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        let Some(new) = super::follow_window(&old) else { return Ok(()) };
        // Rewrite the file in place: Windows refuses to replace a hidden file, and
        // mstsc hides this one.
        let mut f = OpenOptions::new().write(true).open(&file)?;
        f.write_all(&new)?;
        f.set_len(new.len() as u64)
    }

    pub fn launch(app: &AppHandle, c: &Resolved) -> Result<()> {
        // Not worth failing the connection over: at worst the window doesn't resize.
        let _ = follow_window();

        let user = match c.separate_domain() {
            Some(domain) if !c.username.is_empty() => format!("{domain}\\{}", c.username),
            _ => c.username.clone(),
        };
        // `needs_credentials` makes the UI ask for these before we get here.
        let password = c.password.as_deref().ok_or_else(|| msg("There's no password to log in with."))?;
        store_credential(&c.host, &user, password)?;

        let screen = app.primary_monitor().ok().flatten().map(|m| (m.size().width, m.size().height));
        Command::new("mstsc.exe")
            .args(arguments(c, screen))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| msg(format!("Couldn't start mstsc: {e}")))?;
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::model::Protocol;

        #[test]
        fn mstsc_arguments() {
            let mut c = Resolved {
                name: "Web 1".into(),
                protocol: Protocol::Rdp,
                host: "web1.corp.example.com".into(),
                port: 3390,
                username: "admin".into(),
                domain: "CORP".into(),
                password: Some("pw".into()),
                ssh_key_path: String::new(),
                ssh_key_passphrase: None,
                rdp_screen: RdpScreen::Window,
            };
            assert_eq!(
                arguments(&c, Some((2560, 1440))),
                ["/v:web1.corp.example.com:3390", "/w:2048", "/h:1152"]
            );
            c.rdp_screen = RdpScreen::Fullscreen;
            assert_eq!(arguments(&c, None), ["/v:web1.corp.example.com:3390", "/f"]);
        }
    }
}
