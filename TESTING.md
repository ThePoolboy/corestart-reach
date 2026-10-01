# Test checklist

Run through this on each system: Windows 11, Fedora 44 Workstation (GNOME), Fedora 44 KDE
Plasma Desktop, Ubuntu Desktop 26.04 LTS and Linux Mint 22.3 Cinnamon. Note the system and
the build (commit or version) with anything that fails, plus the exact error text or a
screenshot.

Get the packages from the latest green run under **Actions → Build → Artifacts**
(see the README's Install section).

## 1. Install

- [ ] **Windows:** run `Corestart-Reach_…_x64-setup.exe`. SmartScreen warns (unsigned test
      build): **More info → Run anyway**. Reach is in the Start menu.
- [ ] **Fedora (GNOME and KDE) and Linux Mint:** `flatpak install --user ./corestart-reach.flatpak`.
      It downloads the GNOME 51 runtime from Flathub the first time.
- [ ] **Ubuntu:** first `sudo apt install flatpak`, add Flathub
      (`flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo`),
      log out and back in, then install the file as above.
- [ ] Reach shows up in the app menu / Activities search when you type "RDP" or "SSH",
      with the Corestart icon.
- [ ] Opening it shows the Reach icon in the title bar and the taskbar / dock (not the generic
      Wayland "W" or a gear). RDP and SSH windows do too and group with Reach.
- [ ] GNOME Software / KDE Discover show Reach with its description (from the metainfo).
- [ ] Nothing else needs installing: RDP works without a system FreeRDP.

## 2. Vault

- [ ] First start asks you to create a master password (8+ characters, typed twice).
- [ ] Lock (Ctrl+L), then unlock with the right password. A wrong password is refused.
- [ ] Quit and restart: your connections are still there.
- [ ] Linux: `ls -l ~/.var/app/network.corestart.reach/data/network.corestart.reach/` shows
      `vault.json` as `-rw-------`.
- [ ] Settings → change the master password. The old one no longer unlocks; the new one does.
      A wrong "current password" is refused.
- [ ] Settings → auto-lock after 5 minutes. Leave Reach alone for 5 minutes: it locks and says
      why. An open SSH window keeps working.
- [ ] Put the computer to sleep for longer than the auto-lock time: Reach is locked when it wakes.
- [ ] Auto-lock "Never": it stays unlocked. The setting survives a restart.
- [ ] Settings → **Source code on GitHub** and **Support on Ko-fi** open in the web browser
      (in the Flatpak too). The copy buttons copy each address; **Show QR codes** shows both.
- [ ] Settings → Appearance: Light and Dark switch the look at once, and so does the sun/moon
      button by the lock. System follows the desktop. The choice survives a restart and shows
      on the lock screen.

## 3. Organising

- [ ] Create folders and subfolders; add RDP and SSH connections into them.
- [ ] Drag a connection onto a folder, onto another connection (joins its folder), and onto
      empty space (top level). Drag a folder into another folder.
- [ ] Right-click → **Move to…**, **Rename…**, **Duplicate**, **Delete** all work.
- [ ] A folder can't be dragged or moved into itself or its own subfolder.
- [ ] From a connection, folder, Credentials or Settings, **Home** (bottom left) or clicking the
      Corestart Reach name returns to the start screen. With unsaved edits it asks first.
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
- [ ] **Flatpak:** the RDP window is sharp on a scaled display (125%, 150%), keyboard
      shortcuts like Alt+Tab reach the remote desktop while it has focus (GNOME may ask
      once to allow this), and sound plays.
- [ ] Sign out of Windows, and Start → Disconnect: no error appears in Reach.
- [ ] A wrong password shows a readable error in Reach (Linux) or mstsc's own message
      (Windows).
- [ ] **Windows only:** no "unknown remote connection" / publisher warning appears.
      `cmdkey /list` shows a `TERMSRV/<host>` entry while you're signed in.

## 5. SSH

- [ ] First connection to a server asks you to trust its key (type `yes`); the second
      doesn't ask again.
- [ ] Password login, saved and typed in the terminal.
- [ ] Key file set on the connection (with and without a passphrase), typed in and picked with
      **Browse…**. On Linux, also try a key kept outside `~/.ssh` (the Flatpak can only see it
      when picked with Browse…).
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
- [ ] **Linux:** `flatpak uninstall network.corestart.reach`
- [ ] Your vault stays in place (see the README for where), so reinstalling keeps your
      connections. On Linux, `flatpak uninstall --delete-data network.corestart.reach` removes
      it too.

## System notes

- **Fedora GNOME / Ubuntu:** GNOME has no system tray; Reach doesn't use one.
- **Any Linux:** if the window doesn't open or is blank, run
  `flatpak run network.corestart.reach` from a terminal and send the output.
- **NVIDIA:** the Wayland crash fix is built in. Flatpak also needs its NVIDIA driver
  add-on to match your driver; `flatpak update` installs it.
