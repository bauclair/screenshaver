#!/usr/bin/env bash

set -Eeuo pipefail

# -----------------------------------------------------------------------------
# Screenshaver Linux source installer
#
# Conventional Linux distributions:
#   1. Install distribution-specific build dependencies.
#   2. Install rustup, Cargo, and stable Rust when necessary.
#   3. Build Screenshaver with Cargo.
#   4. Install the executable, desktop launcher, and icons under /usr/local.
#
# NixOS:
#   1. Validate the repository's flake.
#   2. Build the Screenshaver flake package.
#   3. Install the package into the current user's Nix profile.
#
# Run as a normal user:
#
#   ./scripts/install-linux.sh
#
# Do not run the entire script with sudo. The script invokes sudo only when
# conventional distributions require system package or /usr/local access.
# -----------------------------------------------------------------------------

APP_NAME="Screenshaver"
BINARY_NAME="screenshaver"
FLAKE_PACKAGE_NAME="screenshaver"

INSTALL_PREFIX="/usr/local"
ASSUME_YES=false

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

DESKTOP_FILE="${PROJECT_DIR}/assets/screenshaver.desktop"
ICON_SOURCE_DIR="${PROJECT_DIR}/assets/icons/hicolor"
BUILD_BINARY="${PROJECT_DIR}/target/release/${BINARY_NAME}"

DISTRO_ID=""
DISTRO_LIKE=""
DISTRO_NAME=""

SUDO=()

usage() {
    cat <<USAGE
Usage: $(basename "$0") [OPTIONS]

Build and install ${APP_NAME} from a cloned source repository.

Options:
  -y, --yes       Do not ask before installing system packages or Rust
  -h, --help      Show this help text

Conventional Linux installation may install:

  * C/C++ compiler and build tools
  * pkg-config
  * curl and CA certificates
  * SDL2 and SDL2_ttf development libraries
  * X11 and XScreenSaver development libraries
  * OpenGL development libraries
  * rustup, Cargo, and the stable Rust toolchain

On conventional Linux distributions, application files are installed under:

  ${INSTALL_PREFIX}

On NixOS, ${APP_NAME} is built from flake.nix and installed into the
current user's Nix profile.
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
    local line_number="${BASH_LINENO[0]:-unknown}"

    printf \
        '\nInstallation stopped at line %s (exit code %s).\n' \
        "$line_number" \
        "$exit_code" >&2

    exit "$exit_code"
}

trap on_error ERR

parse_arguments() {
    local arg

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
}

confirm() {
    local prompt="$1"
    local reply

    if $ASSUME_YES; then
        return 0
    fi

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

require_normal_user() {
    [[ "$EUID" -ne 0 ]] ||
        die \
            "Do not run this entire script with sudo.

Run it as your normal user:

  ./scripts/install-linux.sh

The script will request sudo only when system access is required."
}

initialize_sudo() {
    if command -v sudo >/dev/null 2>&1; then
        SUDO=(sudo)
    else
        die \
            "sudo is required to install system packages and files under
${INSTALL_PREFIX}."
    fi
}

validate_repository_layout() {
    [[ -f "${PROJECT_DIR}/Cargo.toml" ]] ||
        die \
            "Cargo.toml was not found in:

  ${PROJECT_DIR}

This script must remain in the repository's scripts directory."
}

# -----------------------------------------------------------------------------
# Conventional Linux dependency installation
# -----------------------------------------------------------------------------

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
        libgl1-mesa-dev \
        desktop-file-utils
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
        mesa-libGL-devel \
        desktop-file-utils
}


install_rhel_dependencies() {
    local major_version
    local crb_repository
    local epel_release_url

    major_version="${VERSION_ID%%.*}"

    case "$major_version" in
        8)
            crb_repository="powertools"
            epel_release_url="https://dl.fedoraproject.org/pub/epel/epel-release-latest-8.noarch.rpm"
            ;;

        9)
            crb_repository="crb"
            epel_release_url="https://dl.fedoraproject.org/pub/epel/epel-release-latest-9.noarch.rpm"
            ;;

        10)
            crb_repository="crb"
            epel_release_url="https://dl.fedoraproject.org/pub/epel/epel-release-latest-10.noarch.rpm"
            ;;

        *)
            die \
                "Unsupported Enterprise Linux major version: ${VERSION_ID:-unknown}

Screenshaver currently supports CentOS, RHEL, Rocky Linux, and AlmaLinux
major versions 8, 9, and 10."
            ;;
    esac

    log "Installing DNF repository-management tools"

    "${SUDO[@]}" dnf install -y \
        dnf-plugins-core

    if [[ "$DISTRO_ID" == "rhel" ]]; then
        command -v subscription-manager >/dev/null 2>&1 ||
            die \
                "subscription-manager is required to enable the RHEL
CodeReady Builder repository."

        log "Enabling the RHEL CodeReady Builder repository"

        "${SUDO[@]}" subscription-manager repos \
            --enable "codeready-builder-for-rhel-${major_version}-$(uname -m)-rpms"
    else
        log "Enabling the ${crb_repository} repository"

        "${SUDO[@]}" dnf config-manager \
            --set-enabled "$crb_repository"
    fi

    log "Installing the EPEL repository definition"

    "${SUDO[@]}" dnf install -y \
        "$epel_release_url"

    # EPEL Next is specifically intended for CentOS Stream 9 packages that
    # build against packages newer than the corresponding RHEL release.
    if [[ "$DISTRO_ID" == "centos" && "$major_version" == "9" ]]; then
        log "Installing the EPEL Next repository definition"

        "${SUDO[@]}" dnf install -y \
            https://dl.fedoraproject.org/pub/epel/epel-next-release-latest-9.noarch.rpm
    fi

    log "Refreshing DNF package metadata"

    "${SUDO[@]}" dnf makecache

    if ! dnf -q repoquery --available --qf '%{name}' SDL2_ttf-devel 2>/dev/null |
        grep -Fxq SDL2_ttf-devel; then
        die \
            "SDL2_ttf-devel is still unavailable after enabling the required
Enterprise Linux repositories.

Verify that ${crb_repository}, EPEL, and any required vendor subscription
repositories are enabled:

  dnf repolist
  dnf repoquery --available SDL2_ttf-devel"
    fi

    if ! dnf -q repoquery --available --qf '%{name}' libXScrnSaver-devel 2>/dev/null |
        grep -Fxq libXScrnSaver-devel; then
        die \
            "libXScrnSaver-devel is still unavailable after enabling the required
Enterprise Linux repositories.

Verify that ${crb_repository}, EPEL, and any required vendor subscription
repositories are enabled:

  dnf repolist
  dnf repoquery --available libXScrnSaver-devel"
    fi

    log "Installing Enterprise Linux build dependencies"

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
        mesa-libGL-devel \
        desktop-file-utils
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
        libglvnd \
        desktop-file-utils
}

install_suse_dependencies() {
    "${SUDO[@]}" zypper --non-interactive install \
        -t pattern \
        devel_basis

    "${SUDO[@]}" zypper --non-interactive install \
        pkgconf \
        curl \
        ca-certificates \
        SDL2-devel \
        SDL2_ttf-devel \
        libX11-devel \
        libXss-devel \
        libglvnd-devel \
        desktop-file-utils
}

install_void_dependencies() {
    log "Synchronizing Void Linux package indexes"

    "${SUDO[@]}" xbps-install -S

    log "Installing Void Linux build dependencies"

    "${SUDO[@]}" xbps-install -y \
        base-devel \
        pkg-config \
        curl \
        ca-certificates \
        SDL2-devel \
        SDL2_ttf-devel \
        libX11-devel \
        libXScrnSaver-devel \
        MesaLib-devel \
        desktop-file-utils
}

install_build_dependencies() {
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
            install_rhel_dependencies
            ;;

        arch|manjaro|endeavouros)
            install_arch_dependencies
            ;;

        opensuse-leap|opensuse-tumbleweed|sles)
            install_suse_dependencies
            ;;

        void)
            install_void_dependencies
            ;;

        *)
            if is_like debian; then
                install_debian_dependencies

            elif is_like rhel; then
                install_rhel_dependencies

            elif is_like fedora; then
                install_fedora_dependencies

            elif is_like arch; then
                install_arch_dependencies

            elif is_like suse; then
                install_suse_dependencies

            elif is_like void; then
                install_void_dependencies

            else
                die \
                    "Unsupported distribution: ${DISTRO_NAME}

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

# -----------------------------------------------------------------------------
# Rust installation and Cargo build
# -----------------------------------------------------------------------------

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

# -----------------------------------------------------------------------------
# Conventional filesystem installation
# -----------------------------------------------------------------------------

validate_runtime_assets() {
    [[ -f "$DESKTOP_FILE" ]] ||
        die "Desktop file not found: $DESKTOP_FILE"

    [[ -d "$ICON_SOURCE_DIR" ]] ||
        die "Icon directory not found: $ICON_SOURCE_DIR"
}

install_icons() {
    local source_file
    local relative_file
    local target_file
    local icon_count=0

    log "Installing application icons"

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
        die \
            "No PNG, SVG, or XPM icon files were found under:

  ${ICON_SOURCE_DIR}"
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
            "update-desktop-database was not found. The application menu may
refresh automatically after login."
    fi

    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        "${SUDO[@]}" gtk-update-icon-cache \
            -f \
            -t \
            "${INSTALL_PREFIX}/share/icons/hicolor" ||
            warn "The icon cache could not be refreshed."
    else
        warn \
            "gtk-update-icon-cache was not found. The desktop may refresh the
icon cache automatically."
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

        while IFS= read -r library; do
            [[ -n "$library" ]] &&
                printf '  %s\n' "$library" >&2
        done <<< "$missing"

        die "Installation completed, but runtime libraries are missing."
    fi
}

verify_conventional_installation() {
    log "Verifying the installation"

    verify_desktop_file
    verify_shared_libraries
}

print_conventional_success() {
    cat <<SUCCESS

${APP_NAME} was installed successfully.

Executable:

  ${INSTALL_PREFIX}/bin/${BINARY_NAME}

Desktop launcher:

  ${INSTALL_PREFIX}/share/applications/screenshaver.desktop

Launch ${APP_NAME} from the desktop application menu or run:

  ${BINARY_NAME}

Rust was installed only for the current user.

If Cargo is not available in another terminal session, run:

  source "\$HOME/.cargo/env"

SUCCESS
}

install_conventional_linux() {
    initialize_sudo
    validate_runtime_assets
    install_build_dependencies
    install_rust_toolchain
    build_application
    install_application
    refresh_desktop_caches
    verify_conventional_installation
    print_conventional_success
}

# -----------------------------------------------------------------------------
# NixOS installation
# -----------------------------------------------------------------------------

nix_command() {
    nix \
        --extra-experimental-features "nix-command flakes" \
        "$@"
}

verify_nix_available() {
    command -v nix >/dev/null 2>&1 ||
        die \
            "The nix command was not found.

NixOS normally provides this command as part of the operating system."
}

verify_flake_files() {
    [[ -f "${PROJECT_DIR}/flake.nix" ]] ||
        die \
            "NixOS was detected, but flake.nix was not found in:

  ${PROJECT_DIR}

Add the Screenshaver flake to the repository root before running this
installer on NixOS."

    if [[ ! -f "${PROJECT_DIR}/flake.lock" ]]; then
        warn \
            "flake.lock was not found. Nix may resolve the flake inputs without
a committed lock file, but the build will not be pinned to the repository's
intended input revisions."
    fi
}

show_nix_flake_package() {
    log "Checking the available Screenshaver flake output"

    nix_command \
        flake show \
        "path:${PROJECT_DIR}" \
        --no-write-lock-file
}

check_nixos_flake() {
    log "Validating the Screenshaver flake"

    nix_command \
        flake check \
        "path:${PROJECT_DIR}" \
        --no-write-lock-file
}

build_nixos_flake() {
    log "Building ${APP_NAME} with Nix"

    nix_command \
        build \
        "path:${PROJECT_DIR}#${FLAKE_PACKAGE_NAME}" \
        --no-write-lock-file
}

install_nixos_profile() {
    log "Installing ${APP_NAME} into the current user's Nix profile"

    if nix_command \
        profile install \
        "path:${PROJECT_DIR}#${FLAKE_PACKAGE_NAME}" \
        --no-write-lock-file; then

        return
    fi

    die \
        "Nix could not add ${APP_NAME} to the current user's profile.

If Screenshaver is already installed, inspect the profile with:

  nix profile list

Remove the old profile entry if necessary, then run this installer again:

  nix profile remove <index>"
}

verify_nixos_installation() {
    log "Verifying the Nix profile installation"

    if command -v "$BINARY_NAME" >/dev/null 2>&1; then
        printf 'Installed executable: %s\n' \
            "$(command -v "$BINARY_NAME")"
        return
    fi

    warn \
        "${BINARY_NAME} was installed into the Nix profile, but it is not
currently visible through PATH.

Open a new terminal or make sure the user profile bin directory is included
in PATH."
}

print_nixos_success() {
    cat <<SUCCESS

${APP_NAME} was installed successfully through Nix.

The package was added to the current user's Nix profile.

Launch ${APP_NAME} with:

  ${BINARY_NAME}

Display the installed profile entries with:

  nix profile list

To remove Screenshaver later, identify its profile index and run:

  nix profile remove <index>

For a declarative system-wide installation, add the Screenshaver flake input
and package to your own NixOS configuration instead of relying on a user
profile installation.

SUCCESS
}

install_nixos() {
    log "NixOS installation path selected"

    verify_nix_available
    verify_flake_files
    show_nix_flake_package
    check_nixos_flake
    build_nixos_flake
    install_nixos_profile
    verify_nixos_installation
    print_nixos_success
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

main() {
    parse_arguments "$@"

    [[ "$(uname -s)" == "Linux" ]] ||
        die "This installer supports Linux only."

    require_normal_user
    validate_repository_layout
    load_os_release

    printf '%s source installer\n' "$APP_NAME"
    printf 'Detected system: %s\n' "$DISTRO_NAME"
    printf 'Project directory: %s\n' "$PROJECT_DIR"

    if [[ "$DISTRO_ID" == "nixos" ]] || is_like nixos; then
        install_nixos
        exit 0
    fi

    printf 'Installation prefix: %s\n' "$INSTALL_PREFIX"

    install_conventional_linux
}

main "$@"

