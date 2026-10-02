# Flatpak

Corestart Reach ships on Linux only as a Flatpak, with FreeRDP built in.

| File | What it is |
| --- | --- |
| `io.github.thepoolboy.corestart-reach.yml` | Build recipe: FreeRDP 3 (SDL3 client), then Reach |
| `cargo-sources.json`, `node-sources.json` | Every Rust crate and npm package, so the build runs offline |
| `io.github.thepoolboy.corestart-reach.desktop` | App menu entry |
| `io.github.thepoolboy.corestart-reach.metainfo.xml` | Store listing for Flathub, GNOME Software and KDE Discover |
| `corestart-reach.gpg` | Public key of the update repository (see [Update repository](#update-repository)) |
| `site/index.html` | Install page of the update repository |

The recipe uses the GNOME 51 runtime. FreeRDP is built with only its **SDL3 client**: it
runs natively on Wayland and falls back to X11 on X11 desktops, so RDP windows are sharp on
scaled displays.

## Build and run locally

One-time setup (per user, no root needed):

```bash
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.flatpak.Builder org.gnome.Sdk//51 \
  org.freedesktop.Sdk.Extension.rust-stable//26.08 org.freedesktop.Sdk.Extension.node24//26.08
```

Build, install and run (or use the VS Code task **Reach: Build and install Flatpak**):

```bash
flatpak run org.flatpak.Builder --user --install --force-clean --disable-rofiles-fuse \
  build-flatpak flatpak/io.github.thepoolboy.corestart-reach.yml
flatpak run io.github.thepoolboy.corestart-reach
```

`--disable-rofiles-fuse` is needed when flatpak-builder itself runs as a Flatpak.
The build folders (`build-flatpak/`, `.flatpak-builder/`) are git-ignored.

Make a single installable file:

```bash
flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
  ~/.local/share/flatpak/repo corestart-reach.flatpak io.github.thepoolboy.corestart-reach
```

GitHub builds the same bundle on every push (Actions → Build → Artifacts →
`corestart-reach-flatpak`).

## After changing dependencies

The build has no network access, so after any change to `src-tauri/Cargo.lock` or
`package-lock.json`, regenerate the source lists with
[flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools):

```bash
python3 -m venv /tmp/fbt && /tmp/fbt/bin/pip install aiohttp tomlkit \
  "git+https://github.com/flatpak/flatpak-builder-tools.git#subdirectory=node"
curl -fsSLO https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
/tmp/fbt/bin/python flatpak-cargo-generator.py src-tauri/Cargo.lock -o flatpak/cargo-sources.json
/tmp/fbt/bin/flatpak-node-generator npm package-lock.json -o flatpak/node-sources.json
```

If you forget, the Flatpak build fails with a missing-package error, so it can't slip
through unnoticed.

## Sandbox permissions

| Permission | Why |
| --- | --- |
| `--share=network` | RDP and SSH connections |
| `--socket=wayland`, `--socket=fallback-x11`, `--device=dri` | Reach's windows and the RDP window |
| `--socket=pulseaudio` | Sound from the remote desktop |
| `--socket=ssh-auth` | Use your ssh-agent |
| `--filesystem=~/.ssh` | SSH keys and `known_hosts`, shared with OpenSSH |

Key files outside `~/.ssh` are picked with the **Browse…** button, which goes through
the desktop's file-chooser portal.

The vault lives inside the sandbox, at
`~/.var/app/io.github.thepoolboy.corestart-reach/data/io.github.thepoolboy.corestart-reach/vault.json`.

## Updating FreeRDP

Change the URL and `sha256` of the `freerdp` module. FreeRDP publishes the checksum next to
each release (`freerdp-<version>.tar.gz.sha256`).

## Update repository

Linux copies of Reach update from the project's own Flatpak repository on GitHub Pages,
https://thepoolboy.github.io/corestart-reach/, rather than from Flathub.

- Publishing a release runs [`flatpak-repo.yml`](../.github/workflows/flatpak-repo.yml). It
  takes the release's `.flatpak`, puts it in a fresh repository signed with the project's
  GPG key, and deploys it with an install page, `corestart-reach.flatpakref` and
  `corestart-reach.flatpakrepo`. Installed copies see the update within about 10 minutes.
- Release bundles are built with the repository's address and `corestart-reach.gpg` inside
  them (in `build.yml`), so installing the file also sets up updates.
- `site/index.html` is the install page. `corestart-reach.gpg` is the public key; its
  private half and passphrase are the `FLATPAK_GPG_PRIVATE_KEY` and
  `FLATPAK_GPG_PASSPHRASE` repository secrets.
- To publish the latest release again (after changing the page, say), run the workflow by
  hand: Actions → Flatpak repository → Run workflow.

## Flathub

This recipe was written with an AI assistant, and Flathub doesn't accept manifests with
AI-generated or AI-assisted content
([requirements](https://docs.flathub.org/docs/for-app-authors/requirements)). A Flathub
submission needs its own manifest written by the maintainer, and the submission PR has to
disclose the AI-written parts of the app.
