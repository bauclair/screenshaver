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


install_runtime_dependencies() {
    if [[ ! -r /etc/os-release ]]; then
        echo "Error: /etc/os-release was not found."
        echo "Cannot determine the Linux distribution."
        return 1
    fi

    # Defines ID, ID_LIKE, NAME, VERSION_ID, and related fields.
    # shellcheck disable=SC1091
    source /etc/os-release

    local distro_id="${ID:-unknown}"
    local distro_like="${ID_LIKE:-}"
    local -a packages=()

    echo "Detected distribution: ${PRETTY_NAME:-$distro_id}"

    case "$distro_id" in
        ubuntu|debian|linuxmint|pop)
            packages=(
                libsdl2-2.0-0
                libsdl2-ttf-2.0-0
                libx11-6
                libxss1
                libgl1
            )

            sudo apt-get update
            sudo apt-get install -y "${packages[@]}"
            ;;

        fedora)
            packages=(
                SDL2
                SDL2_ttf
                libX11
                libXScrnSaver
                mesa-libGL
            )

            sudo dnf install -y "${packages[@]}"
            ;;

        rhel|centos|rocky|almalinux)
            packages=(
                SDL2
                SDL2_ttf
                libX11
                libXScrnSaver
                mesa-libGL
            )

            sudo dnf install -y "${packages[@]}"
            ;;

        arch|manjaro|endeavouros)
            packages=(
                sdl2
                sdl2_ttf
                libx11
                libxss
                libglvnd
            )

            sudo pacman -S --needed --noconfirm "${packages[@]}"
            ;;

        opensuse-leap|opensuse-tumbleweed|sles)
            packages=(
                libSDL2-2_0-0
                libSDL2_ttf-2_0-0
                libX11-6
                libXss1
                libglvnd
            )

            sudo zypper --non-interactive install "${packages[@]}"
            ;;

        *)
            # Try the broader distribution family where possible.
            if [[ " $distro_like " == *" debian "* ]]; then
                packages=(
                    libsdl2-2.0-0
                    libsdl2-ttf-2.0-0
                    libx11-6
                    libxss1
                    libgl1
                )

                sudo apt-get update
                sudo apt-get install -y "${packages[@]}"

            elif [[ " $distro_like " == *" fedora "* ||
                    " $distro_like " == *" rhel "* ]]; then
                packages=(
                    SDL2
                    SDL2_ttf
                    libX11
                    libXScrnSaver
                    mesa-libGL
                )

                sudo dnf install -y "${packages[@]}"

            elif [[ " $distro_like " == *" arch "* ]]; then
                packages=(
                    sdl2
                    sdl2_ttf
                    libx11
                    libxss
                    libglvnd
                )

                sudo pacman -S --needed --noconfirm "${packages[@]}"

            else
                echo
                echo "Unsupported distribution: ${PRETTY_NAME:-$distro_id}"
                echo
                echo "Screenshaver requires these runtime libraries:"
                echo "  SDL2"
                echo "  SDL2_ttf"
                echo "  Xlib"
                echo "  XScreenSaver extension library"
                echo "  OpenGL"
                echo
                echo "Install those libraries with your distribution's"
                echo "package manager, then run this installer again."
                return 1
            fi
            ;;
    esac
}


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
