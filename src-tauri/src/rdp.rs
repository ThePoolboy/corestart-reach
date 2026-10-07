//! Launch RDP sessions in the platform's own client.
//!
//! - Linux: FreeRDP. All arguments, including the password, are written to its stdin
//!   (`/args-from:stdin`) so they never show up in the process list.
//! - Windows: mstsc. The password is stored as a session-only `TERMSRV/<host>`
//!   credential (what `cmdkey` does) and mstsc is started with `/v:host`. No .rdp
//!   file: Windows 11 warns about every unsigned .rdp file that gets opened.

use serde::Serialize;

use crate::error::Result;
use crate::model::{RdpScreen, RdpSize, Resolved};

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

/// `size`, shrunk to fit in 80% of `screen` without changing its shape.
#[cfg_attr(windows, allow(dead_code))]
fn fit(size: RdpSize, screen: (u32, u32)) -> (u32, u32) {
    let (w, h) = (u64::from(size.width), u64::from(size.height));
    let (max_w, max_h) = (u64::from(screen.0) * 8 / 10, u64::from(screen.1) * 8 / 10);
    if w <= max_w && h <= max_h {
        return (size.width, size.height);
    }
    // Scale by whichever side is tighter: w/max_w or h/max_h.
    let (w, h) = if w * max_h >= h * max_w { (max_w, h * max_w / w) } else { (w * max_h / h, max_h) };
    (w as u32, h as u32)
}

/// The mstsc settings, kept in `Default.rdp`, for a screen mode: either the remote
/// desktop follows the window (dynamic resolution), or it keeps its size. Smart
/// sizing (scaling the picture) is always off: it stretches the remote desktop out
/// of shape, and mstsc's window menu can still turn it on for a session.
///
/// Out of full screen, also undo what an earlier session left behind: a full
/// screen or all-monitors setting (dynamic resolution doesn't work across
/// monitors), and the saved window position, which reopens a maximised window.
/// `None` removes the setting.
#[cfg_attr(not(windows), allow(dead_code))]
fn mstsc_options(screen: RdpScreen) -> Vec<(&'static str, Option<&'static str>)> {
    let dynamic = if screen == RdpScreen::Fixed { "0" } else { "1" };
    let mut options = vec![("dynamic resolution:i:", Some(dynamic)), ("smart sizing:i:", Some("0"))];
    if screen != RdpScreen::Fullscreen {
        options.extend([("screen mode id:i:", Some("1")), ("use multimon:i:", Some("0")), ("winposstr:s:", None)]);
    }
    options
}

/// mstsc's `Default.rdp` with `options` set, or `None` if it already has them.
/// mstsc saves the file as UTF-16 with a byte order mark; it's written back the
/// same way it was read, and a new file is written the way mstsc writes it.
#[cfg_attr(not(windows), allow(dead_code))]
fn with_options(file: Option<&[u8]>, options: &[(&str, Option<&str>)]) -> Option<Vec<u8>> {
    let file = file.unwrap_or(&[0xFF, 0xFE]);
    let utf16 = file.starts_with(&[0xFF, 0xFE]);
    let text = if utf16 {
        let units: Vec<u16> = file[2..].chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(file.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(file)).into_owned()
    };

    let mut out = String::with_capacity(text.len() + 64);
    let mut missing = options.to_vec();
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        match options.iter().find(|(key, _)| body.starts_with(key)) {
            Some((key, value)) => {
                missing.retain(|(k, _)| k != key);
                if let Some(value) = value {
                    out.push_str(key);
                    out.push_str(value);
                    out.push_str(&line[body.len()..]);
                }
            }
            None => out.push_str(line),
        }
    }
    for (key, value) in missing {
        let Some(value) = value else { continue };
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
    use super::*;

    #[test]
    fn fixed_sizes_fit_the_screen() {
        let size = |width, height| RdpSize { width, height };
        // Fits in 80% of the screen: unchanged.
        assert_eq!(fit(size(1280, 720), (1920, 1080)), (1280, 720));
        // Too wide or too tall: shrunk to 80%, keeping its shape.
        assert_eq!(fit(size(1920, 1080), (1920, 1080)), (1536, 864));
        assert_eq!(fit(size(1920, 1080), (1366, 768)), (1091, 614));
        assert_eq!(fit(size(1024, 768), (2560, 900)), (960, 720));
    }

    #[test]
    fn default_rdp_gets_the_screen_mode() {
        let utf16 = |s: &str| -> Vec<u8> {
            [0xFF, 0xFE].into_iter().chain(s.encode_utf16().flat_map(u16::to_le_bytes)).collect()
        };
        let follow = mstsc_options(RdpScreen::Window);
        let fixed = mstsc_options(RdpScreen::Fixed);
        let full = mstsc_options(RdpScreen::Fullscreen);

        // What an earlier session leaves behind: full screen across all monitors,
        // smart sizing on, and a maximised window position.
        let left = "screen mode id:i:2\r\nuse multimon:i:1\r\nsmart sizing:i:1\r\ndynamic resolution:i:0\r\n\
                    winposstr:s:0,3,0,0,800,600\r\nfull address:s:web1\r\n";
        let window = "screen mode id:i:1\r\nuse multimon:i:0\r\nsmart sizing:i:0\r\ndynamic resolution:i:1\r\n\
                      full address:s:web1\r\n";
        assert_eq!(with_options(Some(&utf16(left)), &follow), Some(utf16(window)));
        assert_eq!(with_options(Some(&utf16(window)), &follow), None);
        assert_eq!(
            with_options(Some(&utf16(window)), &fixed),
            Some(utf16(&window.replace("dynamic resolution:i:1", "dynamic resolution:i:0")))
        );
        // Full screen only gets dynamic resolution and no smart sizing.
        assert_eq!(
            with_options(Some(&utf16(left)), &full),
            Some(utf16(&left.replace("smart sizing:i:1", "smart sizing:i:0").replace("resolution:i:0", "resolution:i:1")))
        );

        // Missing settings are added; plain UTF-8 stays UTF-8; no file gets a new one.
        assert_eq!(
            with_options(Some(b"full address:s:web1"), &full).as_deref(),
            Some(&b"full address:s:web1\r\ndynamic resolution:i:1\r\nsmart sizing:i:0\r\n"[..])
        );
        assert_eq!(
            with_options(None, &fixed),
            Some(utf16("dynamic resolution:i:0\r\nsmart sizing:i:0\r\nscreen mode id:i:1\r\nuse multimon:i:0\r\n"))
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

    use super::{RdpExit, RdpScreen, Result, Resolved, address, fit};
    use crate::error::msg;

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

    /// `screen` is the primary monitor in pixels, if known.
    fn arguments(c: &Resolved, screen: Option<(u32, u32)>) -> Vec<String> {
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
            RdpScreen::Fixed => {
                // A window that fits the screen, showing the fixed-size desktop scaled.
                let size = c.rdp_size;
                let (w, h) = screen.map_or((size.width, size.height), |s| fit(size, s));
                args.push(format!("/size:{w}x{h}"));
                args.push(format!("/smart-sizing:{}x{}", size.width, size.height));
            }
        }
        args
    }

    pub fn launch(app: &AppHandle, c: &Resolved) -> Result<()> {
        let exe = find_client()?;
        let screen = app.primary_monitor().ok().flatten().map(|m| (m.size().width, m.size().height));
        let args = arguments(c, screen);
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
        use crate::model::{Protocol, RdpSize};

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
                rdp_size: RdpSize::default(),
            };
            let args = arguments(&c, None);
            assert!(args.contains(&"/v:[fe80::1]:3389".to_string()));
            assert!(args.contains(&"/p:p@ss word".to_string()));
            assert!(args.contains(&"/d:CORP".to_string()));

            // A username that already names its domain wins over the Domain field.
            let c = Resolved { username: "CORP\\admin".into(), domain: "OTHER".into(), ..c };
            let args = arguments(&c, None);
            assert!(args.contains(&"/u:CORP\\admin".to_string()));
            assert!(!args.iter().any(|a| a.starts_with("/d:")));

            // Window: follows the window. Fixed: scaled into a window that fits the screen.
            assert!(args.contains(&"+dynamic-resolution".to_string()));
            let c = Resolved { rdp_screen: RdpScreen::Fixed, rdp_size: RdpSize { width: 1920, height: 1080 }, ..c };
            let args = arguments(&c, Some((1366, 768)));
            assert!(args.ends_with(&["/size:1091x614".to_string(), "/smart-sizing:1920x1080".to_string()]));
            assert!(!args.contains(&"+dynamic-resolution".to_string()));
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

    use super::{RdpScreen, Result, Resolved, address, mstsc_options, with_options};
    use crate::error::msg;

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

    /// `/v:host:port`, then full screen, a window at 80% of `screen` that the
    /// remote desktop follows, or the fixed size.
    fn arguments(c: &Resolved, screen: Option<(u32, u32)>) -> Vec<String> {
        let mut args = vec![format!("/v:{}", address(c))];
        let size = match (c.rdp_screen, screen) {
            (RdpScreen::Fullscreen, _) => {
                args.push("/f".into());
                None
            }
            (RdpScreen::Fixed, _) => Some((c.rdp_size.width, c.rdp_size.height)),
            (RdpScreen::Window, screen) => screen.map(|(w, h)| (w * 8 / 10, h * 8 / 10)),
        };
        if let Some((w, h)) = size {
            args.push(format!("/w:{w}"));
            args.push(format!("/h:{h}"));
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

    /// Set mstsc up for this connection's screen mode ([`mstsc_options`]).
    /// mstsc has no switches for them; `/v` connections take them from
    /// `Default.rdp`. That file is mstsc's "last used settings", which it rewrites
    /// itself after connecting, so they're set again before every launch.
    fn set_screen_mode(screen: RdpScreen) -> std::io::Result<()> {
        let Some(file) = default_rdp() else { return Ok(()) };
        let old = match std::fs::read(&file) {
            Ok(old) => Some(old),
            // No file means mstsc's defaults, which already follow the window.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && screen != RdpScreen::Fixed => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        let Some(new) = with_options(old.as_deref(), &mstsc_options(screen)) else { return Ok(()) };
        // Rewrite the file in place (no truncate): Windows refuses to replace a
        // hidden file, and mstsc hides this one.
        let mut f = OpenOptions::new().write(true).create(true).truncate(false).open(&file)?;
        f.write_all(&new)?;
        f.set_len(new.len() as u64)
    }

    /// The usable area of the smallest screen, before display scaling: mstsc scales
    /// its window up by the scaling (125%, 150%…), so on a scaled laptop screen
    /// physical pixels make it taller than the screen, with scroll bars. The
    /// smallest, because mstsc may open on any of them.
    fn smallest_screen(app: &AppHandle) -> Option<(u32, u32)> {
        let screens: Vec<(u32, u32)> = app
            .available_monitors()
            .ok()?
            .iter()
            .map(|m| {
                let area = m.work_area().size;
                let area = if area.width > 0 && area.height > 0 { area } else { *m.size() };
                let unscaled = |px: u32| (f64::from(px) / m.scale_factor().max(1.0)) as u32;
                (unscaled(area.width), unscaled(area.height))
            })
            .collect();
        Some((screens.iter().map(|s| s.0).min()?, screens.iter().map(|s| s.1).min()?))
    }

    pub fn launch(app: &AppHandle, c: &Resolved) -> Result<()> {
        if let Err(e) = set_screen_mode(c.rdp_screen) {
            // A fixed size can't stay fixed without it; a window just might not resize.
            if c.rdp_screen == RdpScreen::Fixed {
                return Err(msg(format!("Couldn't set mstsc to the fixed screen size (Documents\\Default.rdp): {e}")));
            }
        }

        let user = match c.separate_domain() {
            Some(domain) if !c.username.is_empty() => format!("{domain}\\{}", c.username),
            _ => c.username.clone(),
        };
        // `needs_credentials` makes the UI ask for these before we get here.
        let password = c.password.as_deref().ok_or_else(|| msg("There's no password to log in with."))?;
        store_credential(&c.host, &user, password)?;

        Command::new("mstsc.exe")
            .args(arguments(c, smallest_screen(app)))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| msg(format!("Couldn't start mstsc: {e}")))?;
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::model::{Protocol, RdpSize};

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
                rdp_size: RdpSize { width: 1280, height: 1024 },
            };
            assert_eq!(
                arguments(&c, Some((2560, 1440))),
                ["/v:web1.corp.example.com:3390", "/w:2048", "/h:1152"]
            );
            c.rdp_screen = RdpScreen::Fixed;
            assert_eq!(
                arguments(&c, Some((2560, 1440))),
                ["/v:web1.corp.example.com:3390", "/w:1280", "/h:1024"]
            );
            c.rdp_screen = RdpScreen::Fullscreen;
            assert_eq!(arguments(&c, None), ["/v:web1.corp.example.com:3390", "/f"]);
        }
    }
}
