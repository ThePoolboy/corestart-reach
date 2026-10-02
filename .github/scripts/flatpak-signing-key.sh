#!/usr/bin/env bash
# Loads the Flatpak repository's signing key into a fresh GNUPGHOME, unlocks it
# for this job, and prints its fingerprint for flatpak's --gpg-sign.
#
# Needs GNUPGHOME (a directory that doesn't exist yet), FLATPAK_GPG_PRIVATE_KEY
# and FLATPAK_GPG_PASSPHRASE in the environment. The key must match the public
# key that installed copies trust: $1, default flatpak/corestart-reach.gpg.
set -euo pipefail

public_key="${1:-flatpak/corestart-reach.gpg}"
if [ -z "${FLATPAK_GPG_PRIVATE_KEY:-}" ]; then
  echo "::error::The FLATPAK_GPG_PRIVATE_KEY secret isn't set." >&2
  exit 1
fi

mkdir -m 700 "$GNUPGHOME"
echo allow-preset-passphrase > "$GNUPGHOME/gpg-agent.conf"
echo "$FLATPAK_GPG_PRIVATE_KEY" | gpg --batch --quiet --import

fpr=$(gpg --list-secret-keys --with-colons | awk -F: '/^fpr/ {print $10; exit}')
if ! gpg --show-keys --with-colons "$public_key" | grep -q "^fpr:.*:$fpr:$"; then
  echo "::error::FLATPAK_GPG_PRIVATE_KEY doesn't match $public_key." >&2
  exit 1
fi

# flatpak signs through gpg-agent, which can't ask for the passphrase here.
grip=$(gpg --list-secret-keys --with-keygrip --with-colons | awk -F: '/^grp/ {print $10; exit}')
gpgconf --launch gpg-agent
for preset in /usr/libexec/gpg-preset-passphrase /usr/lib/gnupg/gpg-preset-passphrase; do
  [ -x "$preset" ] && break
done
"$preset" --preset -P "${FLATPAK_GPG_PASSPHRASE:-}" "$grip"

echo "$fpr"
