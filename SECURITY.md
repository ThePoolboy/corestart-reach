# Security

Corestart Reach stores passwords, so security reports are welcome and taken seriously.

## Reporting a vulnerability

Please **don't open a public issue** for a security problem. Use GitHub's private
reporting instead: the repository's **Security** tab → **Report a vulnerability**. Only the
maintainer sees it. You'll get a reply within a week, and credit in the release notes if you
want it.

Please include the version, your operating system, and the steps to reproduce.

## Supported versions

Only the latest release gets security fixes. The Flatpak and the Windows installer both
update to it.

## How Reach protects your data

- The vault is one file encrypted with your master password: Argon2id (64 MiB, 3 passes)
  derives the key, and XChaCha20-Poly1305 encrypts everything, including host names and
  notes.
- Passwords never reach the interface; it only learns whether one is saved.
- The RDP password goes to FreeRDP through a pipe (Linux) or a session-only Windows
  credential (Windows), never on a command line.
- SSH host keys are checked against `~/.ssh/known_hosts`, and a changed key stops the
  connection.
- Reach sends nothing anywhere except the RDP and SSH connections you open. There is no
  telemetry.

## Known, accepted advisories

`cargo audit` runs on every build. Advisories accepted for a documented reason are listed
in [`src-tauri/.cargo/audit.toml`](src-tauri/.cargo/audit.toml), currently:

- **RUSTSEC-2023-0071** (RSA timing side channel, no fix released yet). Reach uses RSA
  only for SSH: it signs with your own RSA key once per login and verifies servers' RSA
  host keys. It never decrypts with RSA, the operation the attack mainly targets.
