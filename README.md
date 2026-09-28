# Corestart Reach

A simple, modern RDP and SSH connection manager for Windows and Linux, in the spirit of
mRemoteNG and Royal TS. Free and open source under GPL-3.0.

- **One encrypted vault** for every host, username, password and note. Nothing is readable
  without your master password.
- **RDP** opens in your system's own client: `mstsc` on Windows, FreeRDP on Linux.
  Saved passwords are passed along, so you're logged straight in.
- **SSH** opens in its own terminal window, with password, private-key and
  keyboard-interactive (2FA) login. Host keys are checked against `~/.ssh/known_hosts`,
  the same file OpenSSH uses.
- **Saved credentials**: store a login once (e.g. a domain admin) and use it on many
  connections. Change the password in one place.
- Folders, search, keyboard shortcuts, light and dark themes.

## Status

Early development (v0.1). RDP and SSH each open in a separate window. Tabs, embedded RDP,
import from mRemoteNG, auto-update and installers are planned.

## How your data is stored

| | |
| --- | --- |
| Vault file (Linux) | `~/.local/share/network.corestart.reach/vault.json` |
| Vault file (Windows) | `%APPDATA%\network.corestart.reach\vault.json` |
| Key derivation | Argon2id, 64 MiB memory, 3 passes, random 16-byte salt |
| Encryption | XChaCha20-Poly1305, fresh random nonce on every save |
| Backup | The previous version is kept as `vault.json.bak` on every save |

Passwords never reach the interface: it only knows whether one is saved. On Linux the
RDP password goes to FreeRDP through a pipe (`/args-from:stdin`), so it never appears in
the process list. On Windows it's stored as a session-only `TERMSRV/<host>` credential
for mstsc, and Windows clears it when you sign out.

**There is no password recovery.** If you forget the master password, the vault can't be
opened.

## Requirements

- **Linux:** FreeRDP 3 for RDP (`sudo dnf install freerdp` or `sudo apt install freerdp3-x11`).
- **Windows 10/11:** nothing extra. Uses the built-in Remote Desktop client and WebView2.

## Building from source

You need Rust (via [rustup](https://rustup.rs)) and Node.js 20+.

Fedora packages:

```bash
sudo dnf install nodejs webkit2gtk4.1-devel openssl-devel librsvg2-devel \
  libappindicator-gtk3-devel libxdo-devel
```

Then:

```bash
npm install
npm run tauri dev      # run with live reload
npm run tauri build    # build installers into src-tauri/target/release/bundle/
```

Checks:

```bash
npm run check                          # TypeScript / Svelte
cd src-tauri && cargo test && cargo clippy
```

## Project layout

```
src/                     interface (Svelte 5 + TypeScript)
  main.ts                picks the main window or an SSH window from the URL
  lib/api.ts             typed wrappers for every Rust command
  views/Workspace.svelte sidebar tree, search and panes
  views/SshSession.svelte terminal window (xterm.js)
src-tauri/               Rust core (Tauri 2)
  src/vault.rs           encrypted vault file
  src/model.rs           connections, folders, credentials
  src/rdp.rs             launch FreeRDP / mstsc
  src/ssh.rs             SSH sessions (russh)
  src/commands.rs        commands the interface calls
```

## Known issues

- **NVIDIA on Wayland:** WebKitGTK's DMA-BUF renderer crashes with
  `Error 71 (Protocol error) dispatching to Wayland display`. Reach turns it off at
  start-up (`WEBKIT_DISABLE_DMABUF_RENDERER=1`). Set the variable to `0` yourself to
  override.

## License

Corestart Reach is free software under the [GNU General Public License v3.0](LICENSE).
Anyone may use, study, share and change it. Copies you distribute must stay under the
same license, with source code.

"Corestart", "Corestart Reach" and the Corestart logo are names and marks of Corestart
Networks. Forks are welcome under the GPL, but please give them a different name and logo
so nobody confuses them with the official project.
