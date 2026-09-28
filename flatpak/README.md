# Flatpak

Corestart Reach ships on Linux only as a Flatpak, with FreeRDP built in.

| File | What it is |
| --- | --- |
| `network.corestart.reach.yml` | Build recipe: libxkbfile, FreeRDP 3, then Reach |
| `network.corestart.reach.desktop` | App menu entry |
| `network.corestart.reach.metainfo.xml` | Store listing for Flathub, GNOME Software and KDE Discover |

## Build and run locally

One-time setup (per user, no root needed):

```bash
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.flatpak.Builder org.gnome.Sdk//50 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08 org.freedesktop.Sdk.Extension.node24//25.08
```

Build, install and run (or use the VS Code task **Reach: Build and install Flatpak**):

```bash
flatpak run org.flatpak.Builder --user --install --force-clean --disable-rofiles-fuse \
  build-flatpak flatpak/network.corestart.reach.yml
flatpak run network.corestart.reach
```

`--disable-rofiles-fuse` is needed when flatpak-builder itself runs as a Flatpak.
The build folders (`build-flatpak/`, `.flatpak-builder/`) are git-ignored.

Make a single installable file:

```bash
flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
  ~/.local/share/flatpak/repo corestart-reach.flatpak network.corestart.reach
```

GitHub builds the same bundle on every push (Actions → Build → Artifacts →
`corestart-reach-flatpak`).

## Sandbox permissions

| Permission | Why |
| --- | --- |
| `--share=network` | RDP and SSH connections |
| `--socket=wayland`, `--socket=x11`, `--device=dri` | Reach's windows, and FreeRDP's RDP window (an X11 client) |
| `--socket=pulseaudio` | Sound from the remote desktop |
| `--socket=ssh-auth` | Use your ssh-agent |
| `--filesystem=~/.ssh` | SSH keys and `known_hosts`, shared with OpenSSH |

Key files outside `~/.ssh` are picked with the **Browse…** button, which goes through
the desktop's file-chooser portal.

The vault lives inside the sandbox, at
`~/.var/app/network.corestart.reach/data/network.corestart.reach/vault.json`.

## Updating FreeRDP

Change the URL and `sha256` of the `freerdp` module. FreeRDP publishes the checksum next to
each release (`freerdp-<version>.tar.gz.sha256`). The `x-checker-data` blocks let
Flathub's bot propose these updates automatically once the app is on Flathub.

## Before submitting to Flathub

- **Offline build.** Flathub builds without network access, so the `--share=network` in the
  `corestart-reach` module has to go. Generate source lists with
  [flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools)
  (`flatpak-cargo-generator.py src-tauri/Cargo.lock` and
  `flatpak-node-generator npm package-lock.json`) and add them as sources.
- **Screenshots** in the metainfo (Flathub requires at least one).
- **Verified badge.** Serve the token Flathub gives you at
  `https://corestart.network/.well-known/org.flathub.VerifiedApps.txt`.
- **Source from a tag.** Build from a tagged git release instead of the local folder.
