# Changelog

## 0.1.1 (2026-10-02)

- Windows: the installer can install Reach for everyone on the computer (as an admin), or
  just for you as before. Silent install for everyone: `/S /AllUsers`
- Linux: Reach's own Flatpak repository at
  [thepoolboy.github.io/corestart-reach](https://thepoolboy.github.io/corestart-reach/).
  Installing from there, or from a release's `.flatpak` file, gets updates through the
  software center or `flatpak update`

## 0.1.0 (2026-10-02)

First public test release.

- One encrypted vault for connections, folders, saved credentials, notes and settings
  (Argon2id + XChaCha20-Poly1305)
- RDP through FreeRDP 3 on Linux (built into the Flatpak) and mstsc on Windows, logged in
  with the saved password; asks for a login when none is saved
- SSH in its own terminal window: saved password, key file, ssh-agent, the usual
  `~/.ssh` keys, and keyboard-interactive / 2FA prompts; host keys checked against
  `known_hosts`
- Quick connect to any host without saving it
- Folders with drag and drop, right-click menus, search and keyboard shortcuts
- Home button (and the Reach name in the sidebar) returns to the start screen
- Auto-lock when idle (default 15 minutes) and changing the master password
- Light and dark mode: follows the system, or pick one in Settings → Appearance or with the
  sun/moon button next to the lock button
- Settings → About links to the source code, and an optional Support Reach section
  (Ko-fi, Bitcoin and Ethereum)
- Flatpak for Linux, installer for Windows
- Updates on Windows: Reach checks GitHub Releases after unlocking and every 12 hours and
  offers "Update and restart" (Settings → Updates turns the check off)
