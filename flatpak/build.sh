#!/bin/bash
set -e
cd "$(dirname "$0")/.."

# Build reqs
SHARED_MODULES_DIR="flatpak/shared-modules"
APPINDICATOR_MANIFEST="$SHARED_MODULES_DIR/libayatana-appindicator/libayatana-appindicator-gtk3.json"
if [ ! -f "$APPINDICATOR_MANIFEST" ]; then
    # Fetch the external manifest repository into an ignored working directory rather
    # than embedding the full Flathub shared-modules tree in this source repository.
    git clone --depth 1 https://github.com/flathub/shared-modules.git "$SHARED_MODULES_DIR"
fi
flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --user --noninteractive flathub org.flatpak.Builder

# Vendor deps
cargo vendor flatpak/cargo-vendor > flatpak/cargo-config.toml

# Build
flatpak run org.flatpak.Builder --user --repo=flatpak/repo --force-clean --install-deps-from=flathub --state-dir=flatpak/.flatpak-builder flatpak/build-dir flatpak/io.github.gxoled.GxOled.yml
flatpak build-bundle flatpak/repo flatpak/GxOled-"$(flatpak --default-arch)".flatpak io.github.gxoled.GxOled
