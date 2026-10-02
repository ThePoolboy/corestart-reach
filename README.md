# Corestart Reach

A simple, modern RDP and SSH connection manager for Windows and Linux, in the spirit of
mRemoteNG and Royal TS. Free and open source under GPL-3.0.

[![Support on Ko-fi](https://img.shields.io/badge/Support_on-Ko--fi-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/thepoolboy)

![Corestart Reach with connections in folders](screenshots/main.png)

<table>
  <tr>
    <td><img src="screenshots/ssh.png" alt="An SSH session in its own terminal window"></td>
    <td><img src="screenshots/rdp.png" alt="An RDP session to a Windows server"></td>
  </tr>
  <tr>
    <td><img src="screenshots/credentials.png" alt="Saved credentials shared by several connections"></td>
    <td><img src="screenshots/quick-connect.png" alt="Quick connect to a host without saving it"></td>
  </tr>
</table>

- **One encrypted vault** for every host, username, password and note. Nothing is readable
  without your master password.
- **RDP** opens in your system's own client: `mstsc` on Windows, FreeRDP on Linux.
  Saved passwords are passed along, so you're logged straight in.
- **SSH** opens in its own terminal window. Logs in with a saved password, a key file,
  your ssh-agent or your usual `~/.ssh` keys, or keyboard-interactive / 2FA prompts.
  Host keys are checked against `~/.ssh/known_hosts`, the same file OpenSSH uses.
- **Quick connect** to any host without saving it (Ctrl+K).
- **Saved credentials**: store a login once (e.g. a domain admin) and use it on many
  connections. Change the password in one place.
- Folders with drag and drop, right-click menus, search, keyboard shortcuts, light and dark
  themes.

## Status

Early development (v0.1), ready for testing. RDP and SSH each open in their own window.
Planned: auto-update, import from mRemoteNG, tabs and embedded RDP.

Tested on Windows 11, Fedora 44 Workstation, Fedora 44 KDE Plasma Desktop, Ubuntu Desktop
26.04 and Linux Mint 22.3 Cinnamon. The Flatpak should also run on other Linux
distributions.

The vault **locks itself after 15 minutes without use** (change it or turn it off in
Settings). Open RDP and SSH sessions keep running when it locks. The master password can be
changed in Settings; the vault and its backup are re-encrypted with the new one.

## Install

Two downloads cover everything: a **Flatpak** for every Linux distribution (FreeRDP
included) and an **installer** for Windows.

Test builds are made automatically for every change: open the repository's
[**Actions**](https://github.com/ThePoolboy/corestart-reach/actions) tab, pick the latest
green **Build** run, and download from **Artifacts** at the bottom (you need to be signed in
to GitHub). Unzip it to get the file.

| System | Download | Install |
| --- | --- | --- |
| Windows 10 / 11 | `corestart-reach-windows` → `Corestart-Reach_0.1.0_x64-setup.exe` | Run it. No admin rights needed. |
| Linux | `corestart-reach-flatpak` → `corestart-reach.flatpak` | `flatpak install --user ./corestart-reach.flatpak` |

- **Windows** shows "Windows protected your PC" the first time, because test builds aren't
  code-signed yet. Click **More info → Run anyway**.
- **Linux:** Fedora and Linux Mint have Flatpak and Flathub set up already. **Ubuntu** needs
  it once:
  ```bash
  sudo apt install flatpak
  flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
  ```
  then log out and back in. Installing the file downloads the GNOME runtime it needs from
  Flathub the first time. Start Reach from the app menu, or with `flatpak run io.github.thepoolboy.corestart-reach`.

The Flatpak's permissions and how it's built are described in [flatpak/README.md](flatpak/README.md).

## How your data is stored

| | |
| --- | --- |
| Vault file (Flatpak) | `~/.var/app/io.github.thepoolboy.corestart-reach/data/io.github.thepoolboy.corestart-reach/vault.json` (owner-only, 0600) |
| Vault file (Linux, built from source) | `~/.local/share/io.github.thepoolboy.corestart-reach/vault.json` |
| Vault file (Windows) | `%APPDATA%\io.github.thepoolboy.corestart-reach\vault.json` |
| Key derivation | Argon2id, 64 MiB memory, 3 passes, random 16-byte salt |
| Encryption | XChaCha20-Poly1305, fresh random nonce on every save |
| Backup | The previous version is kept as `vault.json.bak` on every save |
| Settings | Inside the encrypted vault |
| SSH host keys | `~/.ssh/known_hosts`, shared with OpenSSH |

Passwords never reach the interface: it only knows whether one is saved. On Linux the
RDP password goes to FreeRDP through a pipe (`/args-from:stdin`), so it never appears in
the process list. On Windows it's stored as a session-only `TERMSRV/<host>` credential
for mstsc (what `cmdkey` does), and Windows clears it when you sign out.

**There is no password recovery.** If you forget the master password, the vault can't be
opened.

Reach sends nothing anywhere except the RDP and SSH connections you open: no telemetry, no
accounts. To report a security problem privately, see [SECURITY.md](SECURITY.md). Changes
in each version are in [CHANGELOG.md](CHANGELOG.md).

Only one copy of Reach runs at a time; starting it again brings the open one forward.

## Keyboard shortcuts

| Keys | Action |
| --- | --- |
| Ctrl+K | Quick connect |
| Ctrl+N | New connection |
| Ctrl+F | Search (Enter connects to the first match, or quick connects if none) |
| Ctrl+L | Lock the vault |
| Ctrl+, | Settings |
| Ctrl+S | Save the open connection |
| ↑ ↓ ← → / Enter | Move through the tree / connect |
| Shift+F10 or Menu key | Right-click menu for the selected item |
| Ctrl+Shift+C / Ctrl+Shift+V | Copy / paste in an SSH window |

## Building from source

You need Rust (via [rustup](https://rustup.rs)), Node.js 20+ and WebKitGTK on Linux.
Running from source on Linux uses your system's FreeRDP 3 for RDP.

```bash
# Fedora
sudo dnf group install c-development && sudo dnf install nodejs webkit2gtk4.1-devel freerdp
# Ubuntu / Debian
sudo apt install build-essential nodejs npm libwebkit2gtk-4.1-dev freerdp3-x11
```

Windows needs the "Desktop development with C++" workload from Visual Studio Build Tools.

```bash
npm install
npm run tauri dev                          # run with live reload
npm run tauri build                        # Windows: setup.exe in src-tauri/target/release/bundle/nsis
npm run check                              # type-check the interface
cd src-tauri && cargo clippy && cargo test # lint and test the Rust core
```

For screenshots, `npm run demo-vault` creates a vault full of made-up connections (master
password `demo-password`). It won't overwrite an existing vault, so move yours aside first.
Set `REACH_DEMO_VAULT` to the vault path to fill another location, such as the Flatpak's.

The Flatpak is built with flatpak-builder; see [flatpak/README.md](flatpak/README.md).

Every push to `main` runs the checks and builds the Flatpak and the Windows installer on
GitHub ([`.github/workflows/build.yml`](.github/workflows/build.yml)).

## Project layout

```
src/                        interface (Svelte 5 + TypeScript)
  main.ts                   picks the main window or an SSH window from the URL
  lib/api.ts                typed wrappers for every Rust command
  lib/desktop.ts            turns off web-page behaviour (browser menu, F5 reload)
  views/Workspace.svelte    sidebar tree, drag and drop, menus, panes
  views/SshSession.svelte   terminal window (xterm.js)
src-tauri/                  Rust core (Tauri 2)
  src/vault.rs              encrypted vault file
  src/model.rs              connections, folders, credentials
  src/rdp.rs                launch FreeRDP / mstsc
  src/ssh.rs                SSH sessions (russh)
  src/commands.rs           commands the interface calls
flatpak/                    Flatpak recipe, app menu entry and store listing
```

## Known issues

- **NVIDIA on Wayland:** WebKitGTK's DMA-BUF renderer crashes with
  `Error 71 (Protocol error) dispatching to Wayland display`. Reach turns it off at
  start-up (`WEBKIT_DISABLE_DMABUF_RENDERER=1`). Set the variable to `0` yourself to
  override.
- **Windows with Credential Guard** (on by default on some Windows 11 Enterprise and
  Education machines) can refuse saved RDP credentials. mstsc then asks for the password
  itself.
- **FreeRDP 2 isn't supported** when running from source: Reach needs FreeRDP 3 to pass the
  password privately. The Flatpak includes FreeRDP 3.
- **RDP in the Flatpak** uses FreeRDP's SDL3 client, which runs natively on Wayland (X11 on
  X11 desktops). Running from source uses your distribution's FreeRDP client instead.

## Support Reach

Reach is free and always will be: no paid edition, no ads, no locked features. If it saves
you time, you can help keep it going. Donations are optional and don't unlock anything.

- **[Ko-fi](https://ko-fi.com/thepoolboy)**: one-off or monthly, by card or PayPal.
- **Crypto**: scan a code or copy the address. Check the address in your wallet before
  sending; crypto payments can't be reversed.

| Bitcoin (BTC) | Ethereum (ETH) |
| :---: | :---: |
| <img src="assets/donate/bitcoin.png" alt="Bitcoin QR code" width="160"> | <img src="assets/donate/ethereum.png" alt="Ethereum QR code" width="160"> |
| `bc1qrwfpl77k7zfgrea8suv78cxn8wp3lvn3mjumz0` | `0xB968531aa4f6EaE2c2c479B56b111c8B3B5c6C54` |

The same links are in Reach under **Settings → Support Reach**. Starring the repository,
reporting bugs and telling others about Reach help too.

## License

Corestart Reach is free software under the [GNU General Public License v3.0](LICENSE).
Anyone may use, study, share and change it. Copies you distribute must stay under the
same license, with source code.

"Corestart", "Corestart Reach" and the Corestart logo are names and marks of Corestart
Networks. Forks are welcome under the GPL, but please give them a different name and logo
so nobody confuses them with the official project.

## Contributors

- **[ThePoolboy](https://github.com/ThePoolboy)**: project owner
- **Claude** (Anthropic's AI assistant): co-author of code and documentation

Claude's work shows in the history either as the commit author or as a
`Co-Authored-By: Claude` line on the commit.
