# Changelog

## 0.1.0 (unreleased)

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
