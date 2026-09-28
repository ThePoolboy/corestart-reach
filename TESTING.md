# Test checklist

Run through this on each system: Windows 11, Fedora 44 GNOME, Fedora 44 KDE Plasma and
Ubuntu 26.04 LTS. Note the system and the build (commit or version) with anything that fails,
plus the exact error text or a screenshot.

Get the packages from the latest green run under **Actions → Build → Artifacts**
(see the README's Install section).

## 1. Install

- [ ] **Windows:** run `Corestart-Reach_…_x64-setup.exe`. SmartScreen warns (unsigned test
      build): **More info → Run anyway**. Reach is in the Start menu.
- [ ] **Fedora:** `sudo dnf install ./Corestart-Reach-…x86_64.rpm`. dnf also installs `freerdp`.
- [ ] **Ubuntu:** `sudo apt install ./Corestart-Reach_…_amd64.deb`. apt also installs `freerdp3-x11`.
- [ ] Reach shows up in the app menu / Activities search when you type "RDP" or "SSH",
      with the Corestart icon.
- [ ] Opening it shows the Reach icon in the taskbar / dock (not a generic one).

## 2. Vault

- [ ] First start asks you to create a master password (8+ characters, typed twice).
- [ ] Lock (Ctrl+L), then unlock with the right password. A wrong password is refused.
- [ ] Quit and restart: your connections are still there.
- [ ] Linux: `ls -l ~/.local/share/network.corestart.reach/` shows `vault.json` as `-rw-------`.
- [ ] Settings → change the master password. The old one no longer unlocks; the new one does.
      A wrong "current password" is refused.
- [ ] Settings → auto-lock after 5 minutes. Leave Reach alone for 5 minutes: it locks and says
      why. An open SSH window keeps working.
- [ ] Put the computer to sleep for longer than the auto-lock time: Reach is locked when it wakes.
- [ ] Auto-lock "Never": it stays unlocked. The setting survives a restart.

## 3. Organising

- [ ] Create folders and subfolders; add RDP and SSH connections into them.
- [ ] Drag a connection onto a folder, onto another connection (joins its folder), and onto
      empty space (top level). Drag a folder into another folder.
- [ ] Right-click → **Move to…**, **Rename…**, **Duplicate**, **Delete** all work.
- [ ] A folder can't be dragged or moved into itself or its own subfolder.
- [ ] Search (Ctrl+F) finds by name, host and folder; Enter connects to the first result.
- [ ] Saved credentials: create one, use it on two connections, change its password once,
      and both connections use the new one.
- [ ] Right-clicking anywhere other than the sidebar shows **no** browser menu
      (Back / Reload / Inspect).

## 4. RDP

- [ ] Connection with saved username, domain and password: opens already logged in.
- [ ] Connection with no saved password: the login box asks, then connects. Try
      username + Domain field, `CORP\user`, and `user@corp.example.com`.
- [ ] Quick connect (Ctrl+K) → RDP → host name, FQDN and IP (also `host:3390` if you
      have a non-standard port).
- [ ] **Window** mode opens at about 80% of the screen; resizing it resizes the remote
      desktop. **Full screen** mode fills the screen.
- [ ] Copy and paste text both ways between the remote desktop and your machine.
- [ ] Sign out of Windows, and Start → Disconnect: no error appears in Reach.
- [ ] A wrong password shows a readable error in Reach (Linux) or mstsc's own message
      (Windows).
- [ ] **Windows only:** no "unknown remote connection" / publisher warning appears.
      `cmdkey /list` shows a `TERMSRV/<host>` entry while you're signed in.

## 5. SSH

- [ ] First connection to a server asks you to trust its key (type `yes`); the second
      doesn't ask again.
- [ ] Password login, saved and typed in the terminal.
- [ ] Key file set on the connection (with and without a passphrase).
- [ ] No key set, but you have `~/.ssh/id_ed25519` or keys in ssh-agent: logs in without
      a password, like the `ssh` command. (Windows: the "OpenSSH Authentication Agent"
      service, if you use it.)
- [ ] A server that only accepts keys, with no key available: a clear "only accepts SSH
      keys" message, no password prompt.
- [ ] Take more than 20 seconds to answer the host-key question: still connects.
- [ ] `htop`, `vim` or `nano` draw correctly; Backspace, arrows and Ctrl+R (history search)
      work; non-English characters type correctly.
- [ ] Resize the window: the remote side follows (`htop` redraws to fit).
- [ ] Ctrl+Shift+C / Ctrl+Shift+V copy and paste.
- [ ] `exit` closes the window. Rebooting the server instead leaves it open with
      "Connection lost. Press Enter to reconnect", and Enter reconnects afterwards.

## 6. Desktop behaviour

- [ ] Start Reach again while it's open: the existing window comes to the front; no second
      copy starts.
- [ ] Close the main window while an SSH window is open, then start Reach again: the main
      window comes back, still unlocked.
- [ ] F5 and Ctrl+R in the main window do nothing (they must not reload the app).
- [ ] Light and dark system themes both look right.
- [ ] A small screen or high display scaling: the window fits, and the unlock screen
      scrolls instead of being cut off.

## 7. Uninstall

- [ ] **Windows:** Settings → Apps → Corestart Reach → Uninstall.
- [ ] **Fedora:** `sudo dnf remove corestart-reach` · **Ubuntu:** `sudo apt remove corestart-reach`
- [ ] Your vault stays in place (see the README for where), so reinstalling keeps your
      connections. Delete that folder to remove it.

## System notes

- **Fedora GNOME / Ubuntu:** GNOME has no system tray; Reach doesn't use one.
- **Fedora KDE with NVIDIA:** the Wayland crash fix is built in; if the window ever fails
  to open, run `corestart-reach` from a terminal and send the output.
- **Ubuntu:** if the window is blank or the app won't start, run `corestart-reach` from a
  terminal and send the output (Ubuntu restricts some sandboxing that WebKitGTK uses).
