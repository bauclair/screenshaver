#!/usr/bin/env bash

set -Eeuo pipefail

# -----------------------------------------------------------------------------
# Screenshaver Linux source installer
#
# This script:
#   1. Installs distribution-specific build dependencies.
#   2. Installs rustup, Cargo, and stable Rust when necessary.
#   3. Builds Screenshaver in release mode.
#   4. Installs the executable, desktop launcher, and icons.
#
# Run this script as a normal user:
#
#   ./scripts/install-linux.sh
#
# Do not run the entire script with sudo. The script invokes sudo only for
# operations that modify the operating system.
# -----------------------------------------------------------------------------

APP_NAME="Screenshaver"
BINARY_NAME="screenshaver"
INSTALL_PREFIX="/usr/local"
ASSUME_YES=false

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

DESKTOP_FILE="${PROJECT_DIR}/assets/screenshaver.desktop"
ICON_SOURCE_DIR="${PROJECT_DIR}/assets/icons/hicolor"
BUILD_BINARY="${PROJECT_DIR}/target/release/${BINARY_NAME}"

usage() {
    cat <<USAGE
Usage: $(basename "$0") [OPTIONS]

Build and install ${APP_NAME} from a cloned source repository.

Options:
  -y, --yes       Do not ask before installing system packages or Rust
  -h, --help      Show this help text

This script may install:

  * C/C++ compiler and build tools
  * pkg-config
  * SDL2 and SDL2_ttf development libraries
  * X11 and XScreenSaver development libraries
  * OpenGL development libraries
  * rustup, Cargo, and the stable Rust toolchain

Application files are installed under:

  ${INSTALL_PREFIX}
USAGE
}

log() {
    printf '\n==> %s\n' "$*"
}

warn() {
    printf '\nWarning: %s\n' "$*" >&2
}

die() {
    printf '\nError: %s\n' "$*" >&2
    exit 1
}

on_error() {
    local exit_code=$?

    printf \
        '\nInstallation stopped at line %s (exit code %s).\n' \
        "${BASH_LINENO[0]}" \
        "$exit_code" >&2

    exit "$exit_code"
}

trap on_error ERR

for arg in "$@"; do
    case "$arg" in
        -y|--yes)
            ASSUME_YES=true
            ;;

        -h|--help)
            usage
            exit 0
            ;;

        *)
            usage
            die "Unknown option: $arg"
            ;;
    esac
done

[[ "$(uname -s)" == "Linux" ]] ||
    die "This installer supports Linux only."

[[ "$EUID" -ne 0 ]] ||
    die "Do not run this entire script with sudo. Run it as your normal user."

[[ -f "${PROJECT_DIR}/Cargo.toml" ]] ||
    die "Cargo.toml was not found. Keep this script in the repository's scripts directory."

[[ -f "$DESKTOP_FILE" ]] ||
    die "Desktop file not found: $DESKTOP_FILE"

[[ -d "$ICON_SOURCE_DIR" ]] ||
    die "Icon directory not found: $ICON_SOURCE_DIR"

if command -v sudo >/dev/null 2>&1; then
    SUDO=(sudo)
else
    die "sudo is required to install packages and files under ${INSTALL_PREFIX}."
fi

confirm() {
    local prompt="$1"

    if $ASSUME_YES; then
        return 0
    fi

    local reply

    read -r -p "$prompt [Y/n] " reply

    case "${reply:-Y}" in
        y|Y|yes|YES)
            return 0
            ;;

        *)
            return 1
            ;;
    esac
}

load_os_release() {
    [[ -r /etc/os-release ]] ||
        die "/etc/os-release was not found; the Linux distribution cannot be identified."

    # Defines ID, ID_LIKE, PRETTY_NAME, VERSION_ID, and related values.
    # shellcheck disable=SC1091
    source /etc/os-release

    DISTRO_ID="${ID:-unknown}"
    DISTRO_LIKE="${ID_LIKE:-}"
    DISTRO_NAME="${PRETTY_NAME:-$DISTRO_ID}"
}

is_like() {
    local family="$1"

    [[ " ${DISTRO_LIKE} " == *" ${family} "* ]]
}

install_debian_dependencies() {
    "${SUDO[@]}" apt-get update

    "${SUDO[@]}" apt-get install -y \
        build-essential \
        pkg-config \
        curl \
        ca-certificates \
        libsdl2-dev \
        libsdl2-ttf-dev \
        libx11-dev \
        libxss-dev \
        libgl1-mesa-dev
}

install_fedora_dependencies() {
    "${SUDO[@]}" dnf install -y \
        gcc \
        gcc-c++ \
        make \
        pkgconf-pkg-config \
        curl \
        ca-certificates \
        SDL2-devel \
        SDL2_ttf-devel \
        libX11-devel \
        libXScrnSaver-devel \
        libglvnd-devel \
        mesa-libGL-devel
}

install_arch_dependencies() {
    "${SUDO[@]}" pacman -S --needed --noconfirm \
        base-devel \
        pkgconf \
        curl \
        ca-certificates \
        sdl2 \
        sdl2_ttf \
        libx11 \
        libxss \
        libglvnd
}

install_suse_dependencies() {
    "${SUDO[@]}" zypper --non-interactive install \
        -t pattern \
        devel_basis

    "${SUDO[@]}" zypper --non-interactive install \
        pkg-config \
        curl \
        ca-certificates \
        libSDL2-devel \
        libSDL2_ttf-devel \
        libX11-devel \
        libXss-devel \
        Mesa-libGL-devel
}

install_build_dependencies() {
    load_os_release

    log "Detected ${DISTRO_NAME}"

    if ! confirm "Install or verify the required system build dependencies?"; then
        die "System dependency installation was declined."
    fi

    case "$DISTRO_ID" in
        ubuntu|debian|linuxmint|pop|elementary|zorin)
            install_debian_dependencies
            ;;

        fedora)
            install_fedora_dependencies
            ;;

        rhel|centos|rocky|almalinux)
            warn \
                "SDL2 development packages may require EPEL or another enabled repository."

            install_fedora_dependencies
            ;;

        arch|manjaro|endeavouros)
            install_arch_dependencies
            ;;

        opensuse-leap|opensuse-tumbleweed|sles)
            install_suse_dependencies
            ;;

        nixos)
            die \
                "NixOS should build Screenshaver through its Nix expression rather than this mutable system installer."
            ;;

        *)
            if is_like debian; then
                install_debian_dependencies

            elif is_like fedora || is_like rhel; then
                install_fedora_dependencies

            elif is_like arch; then
                install_arch_dependencies

            elif is_like suse; then
                install_suse_dependencies

            else
                die \
                    "Unsupported distribution: ${DISTRO_NAME}.

Install these components manually and run the installer again:

  * C compiler and make
  * pkg-config
  * curl and CA certificates
  * SDL2 development files
  * SDL2_ttf development files
  * X11 development files
  * XScreenSaver extension development files
  * OpenGL development files"
            fi
            ;;
    esac
}

load_cargo_environment() {
    if [[ -f "$HOME/.cargo/env" ]]; then
        # shellcheck disable=SC1090
        source "$HOME/.cargo/env"
    fi
}

install_rust_toolchain() {
    load_cargo_environment

    if command -v rustup >/dev/null 2>&1 &&
       command -v cargo >/dev/null 2>&1 &&
       command -v rustc >/dev/null 2>&1; then

        log "Rust toolchain already available"

        rustup toolchain install stable --profile minimal
        rustup default stable

        return
    fi

    if ! confirm \
        "Rust was not found. Install rustup, Cargo, and stable Rust for user '$USER'?"; then

        die "Rust installation was declined."
    fi

    command -v curl >/dev/null 2>&1 ||
        die "curl is required to install rustup."

    log "Installing Rust with rustup"

    curl \
        --proto '=https' \
        --tlsv1.2 \
        -sSf \
        https://sh.rustup.rs |
        sh -s -- \
            -y \
            --profile minimal \
            --default-toolchain stable

    [[ -f "$HOME/.cargo/env" ]] ||
        die "rustup completed, but $HOME/.cargo/env was not created."

    load_cargo_environment

    command -v cargo >/dev/null 2>&1 ||
        die "Cargo was not found after installing rustup."

    command -v rustc >/dev/null 2>&1 ||
        die "rustc was not found after installing rustup."
}

build_application() {
    log "Building ${APP_NAME} in release mode"

    cd "$PROJECT_DIR"

    cargo build --release --locked

    [[ -x "$BUILD_BINARY" ]] ||
        die "Expected executable was not produced: $BUILD_BINARY"
}

install_icons() {
    log "Installing application icons"

    local source_file
    local relative_file
    local target_file
    local icon_count=0

    while IFS= read -r -d '' source_file; do
        relative_file="${source_file#${ICON_SOURCE_DIR}/}"

        target_file="${INSTALL_PREFIX}/share/icons/hicolor/${relative_file}"

        "${SUDO[@]}" install \
            -Dm644 \
            "$source_file" \
            "$target_file"

        ((icon_count += 1))
    done < <(
        find "$ICON_SOURCE_DIR" \
            -type f \
            \( \
                -name '*.png' \
                -o -name '*.svg' \
                -o -name '*.xpm' \
            \) \
            -print0
    )

    [[ "$icon_count" -gt 0 ]] ||
        die "No PNG, SVG, or XPM icon files were found under $ICON_SOURCE_DIR."
}

install_application() {
    log "Installing ${APP_NAME}"

    "${SUDO[@]}" install \
        -Dm755 \
        "$BUILD_BINARY" \
        "${INSTALL_PREFIX}/bin/${BINARY_NAME}"

    "${SUDO[@]}" install \
        -Dm644 \
        "$DESKTOP_FILE" \
        "${INSTALL_PREFIX}/share/applications/screenshaver.desktop"

    install_icons
}

refresh_desktop_caches() {
    log "Refreshing desktop application and icon caches"

    if command -v update-desktop-database >/dev/null 2>&1; then
        "${SUDO[@]}" update-desktop-database \
            "${INSTALL_PREFIX}/share/applications" ||
            warn "The desktop application database could not be refreshed."
    else
        warn \
            "update-desktop-database was not found. The application menu may refresh automatically after login."
    fi

    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        "${SUDO[@]}" gtk-update-icon-cache \
            -f \
            -t \
            "${INSTALL_PREFIX}/share/icons/hicolor" ||
            warn "The icon cache could not be refreshed."
    else
        warn \
            "gtk-update-icon-cache was not found. The desktop may refresh the icon cache automatically."
    fi
}

verify_desktop_file() {
    local installed_desktop_file

    installed_desktop_file="${INSTALL_PREFIX}/share/applications/screenshaver.desktop"

    [[ -f "$installed_desktop_file" ]] ||
        die "Installed desktop launcher not found: $installed_desktop_file"

    if command -v desktop-file-validate >/dev/null 2>&1; then
        desktop-file-validate "$installed_desktop_file" ||
            warn "The desktop launcher reported validation warnings."
    fi
}

verify_shared_libraries() {
    local installed_binary
    local missing

    installed_binary="${INSTALL_PREFIX}/bin/${BINARY_NAME}"

    [[ -x "$installed_binary" ]] ||
        die "Installed executable not found: $installed_binary"

    if ! command -v ldd >/dev/null 2>&1; then
        warn "ldd was not found; shared-library verification was skipped."
        return
    fi

    missing="$(
        ldd "$installed_binary" 2>/dev/null |
            awk '/not found/ { print $1 }'
    )"

    if [[ -n "$missing" ]]; then
        printf '\nThe following shared libraries are unresolved:\n' >&2
        printf '  %s\n' $missing >&2

        die "Installation completed, but runtime libraries are missing."
    fi
}

verify_installation() {
    log "Verifying the installation"

    verify_desktop_file
    verify_shared_libraries
}

main() {
    printf '%s source installer\n' "$APP_NAME"
    printf 'Project directory: %s\n' "$PROJECT_DIR"
    printf 'Installation prefix: %s\n' "$INSTALL_PREFIX"

    install_build_dependencies
    install_rust_toolchain
    build_application
    install_application
    refresh_desktop_caches
    verify_installation

    cat <<SUCCESS

${APP_NAME} was installed successfully.

Executable:

  ${INSTALL_PREFIX}/bin/${BINARY_NAME}

Desktop launcher:

  ${INSTALL_PREFIX}/share/applications/screenshaver.desktop

You can launch ${APP_NAME} from the desktop application menu or by running:

  ${BINARY_NAME}

Rust was installed only for the current user.

If Cargo is not available in another terminal session, run:

  source "\$HOME/.cargo/env"

SUCCESS
}

main
