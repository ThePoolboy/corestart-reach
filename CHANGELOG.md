# Changelog

## Unreleased

- **Backups** in Settings: **Export backup…** saves everything in Reach (folders, connections,
  saved credentials with their passwords, settings, theme and open folders) to one
  `.reachbackup` file, encrypted with your master password. **Import backup…** shows the
  backup's date and what's in it, then replaces everything, master password included, and
  keeps what you had as `vault.before-import.json`. A new install can **Restore from a
  backup…** instead of setting a master password. Backups from a newer Reach are refused

## 0.2.0 (2026-10-04)

- RDP connections can have a **Fixed size** screen: pick a common size or type your own
  (640 × 480 to 8192 × 8192). The remote desktop keeps that resolution, and resizing the
  window scales the picture to fit. On Linux the window opens small enough to fit the screen
- Windows: an RDP window in **Window** mode no longer opens bigger than the screen, with
  scroll bars, on laptops with display scaling (125%, 150%…) or a bigger second monitor.
  It starts at 80% of the smallest screen, after scaling
- Windows: mstsc has no switches for dynamic resolution or smart sizing, so before every
  connection Reach sets them in mstsc's `Documents\Default.rdp` to match the screen mode (as
  0.1.2 did, now also for Fixed size)
- SSH windows open centred at 80% of the screen, the same size as RDP windows in **Window**
  mode (they were 960×600)
- **Keyboard shortcuts** in Settings: see them all and click one to change it. **Reset** goes
  back to the default, and tooltips and the search box show your keys
- A calmer interface with the same features. The sidebar shows a small monitor (RDP) or
  terminal (SSH) icon instead of a text tag, counts only on closed folders, and no repeated
  logo and name; the lock button sits at the end of the search row
- Connections: Duplicate and Delete are in the **⋯** menu next to Connect (and still on
  right-click). Protocol sits next to Host and Port, a saved credential has an **Edit** button,
  and hints became tooltips or grey text in empty boxes
- Home shows how many connections you have, with **Quick connect** and **New connection**
- The lock screen keeps its look with less on it: the logo, the master password and
  **Unlock**. Reach now talks about your connections rather than a "vault"
- Settings: a clearer order, with auto-lock and the master password together under Security.
  Changing the master password and the crypto addresses each open from a button, and About
  shows where Reach keeps its data file. The sun/moon button is gone; choose Light or Dark
  under Appearance
- Settings → Support Reach links to [GitHub Sponsors](https://github.com/sponsors/ThePoolboy)
  as well as Ko-fi
- A clearer app icon: the charcoal tile has a blue rim and a bolder symbol, so it no longer
  fades into dark taskbars
- Once a connection uses **Fixed size**, Reach 0.1.x can't open your saved connections any
  more, so don't go back to an older version after using it

## 0.1.2 (2026-10-02)

- Windows: an RDP window always resizes the remote desktop to fit when you resize it, even
  if mstsc's own settings (`Documents\Default.rdp`) had dynamic resolution off or smart
  sizing on. Reach turns those two settings back on before it starts mstsc

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
