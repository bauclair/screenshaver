#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
    pwd
)"

PROJECT_ROOT="$(
    cd -- "$SCRIPT_DIR/.."
    pwd
)"

BINARY="$PROJECT_ROOT/target/release/screenshaver"
DESKTOP_FILE="$PROJECT_ROOT/assets/screenshaver.desktop"
ICON_ROOT="$PROJECT_ROOT/assets/icons/hicolor"

if [[ ! -x "$BINARY" ]]; then
    echo "Error: release binary not found:"
    echo "  $BINARY"
    echo
    echo "Run this first:"
    echo "  cargo build --release"
    exit 1
fi

if [[ ! -f "$DESKTOP_FILE" ]]; then
    echo "Error: desktop file not found:"
    echo "  $DESKTOP_FILE"
    exit 1
fi

if [[ ! -d "$ICON_ROOT" ]]; then
    echo "Error: icon directory not found:"
    echo "  $ICON_ROOT"
    exit 1
fi

echo "Installing Screenshaver..."

sudo install -Dm755 \
    "$BINARY" \
    /usr/local/bin/screenshaver

sudo install -Dm644 \
    "$DESKTOP_FILE" \
    /usr/local/share/applications/screenshaver.desktop

while IFS= read -r -d '' icon; do
    relative_path="${icon#"$ICON_ROOT"/}"

    sudo install -Dm644 \
        "$icon" \
        "/usr/local/share/icons/hicolor/$relative_path"
done < <(
    find "$ICON_ROOT" -type f \
        \( -name 'screenshaver.png' -o -name 'screenshaver.svg' \) \
        -print0
)

if command -v update-desktop-database >/dev/null 2>&1; then
    sudo update-desktop-database /usr/local/share/applications
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    sudo gtk-update-icon-cache \
        --force \
        --ignore-theme-index \
        /usr/local/share/icons/hicolor
fi

echo
echo "Screenshaver installation complete."
echo "Executable: /usr/local/bin/screenshaver"
echo "Launcher:   /usr/local/share/applications/screenshaver.desktop"
