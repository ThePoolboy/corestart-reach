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
- [ ] On a dark taskbar the icon's blue rim shows, so it doesn't fade into the panel; on a light
      taskbar it's a clear dark tile.
- [ ] GNOME Software / KDE Discover show Reach with its description (from the metainfo).
- [ ] Nothing else needs installing: RDP works without a system FreeRDP.

## 2. Master password and settings

- [ ] First start asks you to create a master password (8+ characters, typed twice).
- [ ] Lock (the padlock at the end of the search row, or Ctrl+L), then unlock with the right
      password. A wrong password is refused.
- [ ] Quit and restart: your connections are still there.
- [ ] Linux: `ls -l ~/.var/app/io.github.thepoolboy.corestart-reach/data/io.github.thepoolboy.corestart-reach/` shows
      `vault.json` as `-rw-------`.
- [ ] Settings → **Change master password…** opens the form; **Cancel** closes it and clears
      what you typed. Change the password: the old one no longer unlocks; the new one does.
      A wrong "current password" is refused.
- [ ] Settings → Security → **Lock Reach when idle**: after 5 minutes. Leave Reach alone for 5
      minutes: it locks and says why. An open SSH window keeps working.
- [ ] Put the computer to sleep for longer than the auto-lock time: Reach is locked when it wakes.
- [ ] Auto-lock "Never": it stays unlocked. The setting survives a restart.
- [ ] Settings → **Source code on GitHub**, **Sponsor on GitHub** and **Support on Ko-fi** open
      in the web browser (in the Flatpak too). **Donate with crypto** shows the addresses: the
      copy buttons copy each one; **Show QR codes** shows both.
- [ ] Settings → About shows where the data file is, matching the table in the README.
- [ ] Settings → Keyboard shortcuts: click Quick connect's keys and press Ctrl+J. Ctrl+J now
      opens Quick connect and Ctrl+K doesn't; the ⚡ button's tooltip says Ctrl+J. It survives a
      restart. **Reset** brings back Ctrl+K. Ctrl+C, a plain letter and a combination another
      action uses are refused with a reason; Escape cancels.
- [ ] Settings → Appearance: Light and Dark switch the look at once. System follows the
      desktop. The choice survives a restart and shows on the lock screen.

## 3. Organising

- [ ] Create folders and subfolders; add RDP and SSH connections into them.
- [ ] Drag a connection onto a folder, onto another connection (joins its folder), and onto
      empty space (top level). Drag a folder into another folder.
- [ ] Connections show a monitor (RDP) or terminal (SSH) icon. Closed folders show how many
      connections they hold; open ones don't.
- [ ] Right-click → **Move to…**, **Rename…**, **Duplicate**, **Delete** all work.
- [ ] In a connection, **⋯** → **Duplicate** and **Delete** work. Pressing **⋯** again, Escape
      or clicking elsewhere closes the menu.
- [ ] Changing **Protocol** in a connection swaps the RDP and SSH fields and the default port.
- [ ] A folder can't be dragged or moved into itself or its own subfolder.
- [ ] From a connection, folder, Credentials or Settings, **Home** (bottom left) returns to the
      start screen. With unsaved edits it asks first. Home shows how many connections there are,
      or "No connections yet" before any are added, and its **Quick connect** and **New
      connection** buttons work.
- [ ] Search (Ctrl+F) finds by name, host and folder; Enter connects to the first result.
- [ ] Saved credentials: create one, use it on two connections, change its password once,
      and both connections use the new one. **Edit** next to the credential in a connection
      opens it under Credentials.
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
- [ ] **Window** mode on a laptop at 125% or 150% display scaling (Windows): the window
      fits on the screen with no scroll bars, and maximising it resizes the remote desktop.
- [ ] **Fixed size** (try 1280 × 720 and a custom size): the remote desktop has exactly
      that resolution (check in the remote Settings → Display). Linux: resizing or
      maximising the window scales the picture; no scroll bars, and on a screen smaller than
      the size the window still fits. Windows: the picture is never stretched; a window
      smaller than the size gets scroll bars.
- [ ] **Window** mode (Windows): resize the window a few times, maximise it, then restore it.
      The remote desktop follows every time. Disconnect while maximised, then connect again:
      it opens as a normal window. Same after a session that used **Fullscreen**.
- [ ] Switch a connection from Fixed size back to Window: it follows the window again
      (Windows: Reach resets mstsc's settings in `Documents\Default.rdp` each time).
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

- [ ] An SSH window opens centred at about 80% of the screen, the same size as an RDP window
      in **Window** mode, with no visible jump in size. Also on a laptop at 125% or 150% scaling.
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
- [ ] **Linux:** `flatpak uninstall io.github.thepoolboy.corestart-reach`
- [ ] Your data file stays in place (see the README for where), so reinstalling keeps your
      connections. On Linux, `flatpak uninstall --delete-data io.github.thepoolboy.corestart-reach` removes
      it too.

## System notes

- **Fedora GNOME / Ubuntu:** GNOME has no system tray; Reach doesn't use one.
- **Any Linux:** if the window doesn't open or is blank, run
  `flatpak run io.github.thepoolboy.corestart-reach` from a terminal and send the output.
- **NVIDIA:** the Wayland crash fix is built in. Flatpak also needs its NVIDIA driver
  add-on to match your driver; `flatpak update` installs it.
