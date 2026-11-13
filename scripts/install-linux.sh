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

DESKTOP_ENVIRONMENT=""
XFCE_INTEGRATION=false
KDE_INTEGRATION=false
KDE_QML_SESSION_REFRESH_NEEDED=false
MATE_INTEGRATION=false
XFCE_SAVER_DIR=""

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
  * Wayland, EGL, and libdecor development libraries
  * Clang, PAM, and OpenSSL development libraries
  * Qt6, KDE Frameworks 6 KScreen, and CMake dependencies on KDE Plasma
  * Ayatana indicator integration on Debian-family MATE desktops
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
# Desktop-environment detection
# -----------------------------------------------------------------------------

detect_desktop_environment() {
    # Session detection is informational only. Installation capability is
    # determined independently so changing desktops does not require reinstall.
    DESKTOP_ENVIRONMENT="${XDG_CURRENT_DESKTOP:-${XDG_SESSION_DESKTOP:-${DESKTOP_SESSION:-}}}"
    KDE_INTEGRATION=false
    XFCE_INTEGRATION=false
    MATE_INTEGRATION=false

    if command -v plasmashell >/dev/null 2>&1; then
        KDE_INTEGRATION=true
    fi
    if [[ -x /usr/bin/xfce4-screensaver ]]; then
        XFCE_INTEGRATION=true
    fi
    if [[ -x /usr/bin/mate-screensaver ]]; then
        MATE_INTEGRATION=true
    fi
}

is_debian_family() {
    case "$DISTRO_ID" in
        ubuntu|debian|linuxmint|pop|elementary|zorin)
            return 0
            ;;
    esac

    is_like debian
}

# -----------------------------------------------------------------------------
# Conventional Linux dependency installation
# -----------------------------------------------------------------------------

# Installation-media repositories are often left enabled after a fresh Debian
# installation. APT cannot refresh those entries without the original media.
# Never change system repositories without an explicit confirmation, including
# when --yes was supplied for package installation.
repair_debian_cdrom_sources() {
    local source_file temp_file backup_file
    local -a affected=()
    local found=false

    for source_file in /etc/apt/sources.list /etc/apt/sources.list.d/*.list /etc/apt/sources.list.d/*.sources; do
        [[ -f "$source_file" ]] || continue
        if [[ "$source_file" == *.sources ]]; then
            if awk 'BEGIN { RS=""; IGNORECASE=1 }
                { if ($0 ~ /(^|\n)URIs:[^\n]*cdrom:/ && $0 !~ /(^|\n)Enabled:[[:space:]]*no([[:space:]]|$)/) found=1 }
                END { exit !found }' "$source_file"; then
                affected+=("$source_file")
            fi
        elif grep -Eiq '^[[:space:]]*deb([[:space:]]+\[[^]]*\])?[[:space:]]+cdrom:' "$source_file"; then
            affected+=("$source_file")
        fi
    done

    (( ${#affected[@]} > 0 )) || return 0

    warn "Enabled installation-media (cdrom:) APT repositories were found:"
    printf '  %s\n' "${affected[@]}" >&2
    printf '%s\n' 'APT updates can fail when the original DVD is unavailable.' >&2

    # Deliberately do not use confirm(): --yes must not authorize editing
    # system package sources.
    if [[ ! -t 0 ]]; then
        die "Disable the obsolete cdrom: APT source manually, then rerun the installer."
    fi
    local reply
    read -r -p "Back up these files and disable only their cdrom: entries? [y/N] " reply
    case "$reply" in y|Y|yes|YES) ;; *) die "APT source configuration was left unchanged." ;; esac

    for source_file in "${affected[@]}"; do
        backup_file="${source_file}.screenshaver-backup-$(date +%Y%m%d%H%M%S)"
        "${SUDO[@]}" cp -p -- "$source_file" "$backup_file"
        temp_file="$(mktemp)"
        if [[ "$source_file" == *.sources ]]; then
            # A deb822 stanza can include multiple URIs. Disable the whole
            # stanza containing installation media; preserve other stanzas.
            awk 'BEGIN { RS=""; ORS="\n\n" }
                {
                    if ($0 ~ /(^|\n)URIs:[^\n]*cdrom:/ && $0 !~ /(^|\n)Enabled:[[:space:]]*no([[:space:]]|$)/) {
                        if ($0 ~ /(^|\n)Enabled:/) {
                            n=split($0, a, "\n"); $0="";
                            for (i=1;i<=n;i++) {
                                if (a[i] ~ /^Enabled:/) a[i]="Enabled: no";
                                $0=$0 (i==1 ? "" : "\n") a[i];
                            }
                        } else $0=$0 "\nEnabled: no";
                    }
                    print
                }' "$source_file" > "$temp_file"
        else
            sed -E '/^[[:space:]]*deb([[:space:]]+\[[^]]*\])?[[:space:]]+cdrom:/ s/^/# Disabled by Screenshaver installer: /' \
                "$source_file" > "$temp_file"
        fi
        "${SUDO[@]}" cp -- "$temp_file" "$source_file"
        rm -f -- "$temp_file"
        log "Disabled installation-media source in $source_file (backup: $backup_file)"
    done
}

install_debian_dependencies() {
    repair_debian_cdrom_sources
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
        libegl-dev \
        libwayland-dev \
        libdecor-0-dev \
        libclang-dev \
        libpam0g-dev \
        libssl-dev \
        desktop-file-utils

    if $KDE_INTEGRATION; then
        log "Installing Debian KDE-native screen-lock build dependencies"
        "${SUDO[@]}" apt-get install -y \
            cmake \
            ninja-build \
            qt6-base-dev \
            qt6-declarative-dev \
            libkscreen-dev
    fi
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
        wayland-devel \
        libdecor-devel \
        clang-devel \
        pam-devel \
        openssl-devel \
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
        wayland-devel \
        libdecor-devel \
        clang-devel \
        pam-devel \
        openssl-devel \
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
        wayland \
        libdecor \
        clang \
        pam \
        openssl \
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
        wayland-devel \
        libdecor-devel \
        clang-devel \
        pam-devel \
        libopenssl-devel \
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
        libglvnd-devel \
        wayland-devel \
        libdecor-devel \
        clang \
        pam-devel \
        openssl-devel \
        desktop-file-utils
}

install_xfce_runtime_dependency() {
    [[ "${DESKTOP_ENVIRONMENT,,}" == *xfce* ]] || return 0

    if ! is_debian_family; then
        warn \
            "Xfce was detected, but automatic xfce4-screensaver provisioning is
currently verified only for Debian-family distributions. Core Screenshaver
installation will continue without modifying the native screen locker."
        return 0
    fi

    log "Installing the Xfce secure-lock runtime dependency"

    "${SUDO[@]}" apt-get install -y \
        xfce4-screensaver
}

install_mate_indicator_runtime_dependency() {
    [[ "${DESKTOP_ENVIRONMENT,,}" == *mate* ]] || return 0

    if ! is_debian_family; then
        warn "MATE was detected, but automatic Ayatana provisioning is currently supported only for Debian-family distributions. Screenshaver installation will continue."
        return 0
    fi

    local package
    local -a packages=(ayatana-indicator-application mate-indicator-applet)

    for package in "${packages[@]}"; do
        if ! apt-cache show "$package" >/dev/null 2>&1; then
            warn "MATE indicator package '$package' is unavailable in the configured APT repositories. Screenshaver installation will continue; tray integration may require manual setup."
            return 0
        fi
    done

    log "Installing MATE Ayatana indicator runtime support"
    if ! "${SUDO[@]}" apt-get install -y "${packages[@]}"; then
        warn "MATE indicator support could not be installed. Screenshaver installation will continue, but its tray icon may not appear until a compatible StatusNotifier host is configured."
    fi
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

        arch|manjaro|endeavouros|omarchy)
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

    install_xfce_runtime_dependency
    install_mate_indicator_runtime_dependency
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

verify_libdecor_build_dependencies() {
    local component
    command -v pkg-config >/dev/null 2>&1 ||
        die "pkg-config is required to build the native libdecor bridge."

    for component in libdecor-0 wayland-client egl wayland-egl; do
        pkg-config --exists "$component" ||
            die "Missing native libdecor build dependency: $component (pkg-config)."
    done
}

build_application() {
    log "Building ${APP_NAME} in release mode"

    cd "$PROJECT_DIR"

    verify_libdecor_build_dependencies
    cargo build --release --locked --features libdecor-bridge

    [[ -x "$BUILD_BINARY" ]] ||
        die "Expected executable was not produced: $BUILD_BINARY"
}

# -----------------------------------------------------------------------------
# KDE Plasma native lock-screen presenter (conventional distributions)
# -----------------------------------------------------------------------------

# Resolve KDE KScreen development package names against enabled repositories.
# Different distro releases may use different names; never silently omit KF6Screen.
install_kde_kscreen_package() {
    local manager="$1"; shift
    local candidate
    case "$manager" in
        dnf)
            for candidate in "$@"; do
                if dnf -q repoquery --available --qf '%{name}' "$candidate" 2>/dev/null | grep -Fxq "$candidate" ||
                   rpm -q "$candidate" >/dev/null 2>&1; then
                    "${SUDO[@]}" dnf install -y "$candidate"
                    return 0
                fi
            done
            ;;
        pacman)
            for candidate in "$@"; do
                if pacman -Si "$candidate" >/dev/null 2>&1 || pacman -Q "$candidate" >/dev/null 2>&1; then
                    "${SUDO[@]}" pacman -S --needed --noconfirm "$candidate"
                    return 0
                fi
            done
            ;;
        zypper)
            for candidate in "$@"; do
                if rpm -q "$candidate" >/dev/null 2>&1 ||
                   zypper --non-interactive search --match-exact --type package "$candidate" 2>/dev/null | grep -Eq "^[[:space:]]*[i |]*[|][[:space:]]*${candidate}[[:space:]]*[|]"; then
                    "${SUDO[@]}" zypper --non-interactive install "$candidate"
                    return 0
                fi
            done
            ;;
        xbps)
            for candidate in "$@"; do
                if xbps-query -Rs "^${candidate}$" 2>/dev/null | grep -Fq "$candidate" ||
                   xbps-query -p pkgver "$candidate" >/dev/null 2>&1; then
                    "${SUDO[@]}" xbps-install -y "$candidate"
                    return 0
                fi
            done
            ;;
    esac
    die "KDE Plasma was detected, but no KF6Screen development package was found in enabled ${DISTRO_NAME} repositories. Tried: $*. Verify that KDE Frameworks 6 development repositories are enabled."
}

# Fail before the expensive Rust build if the KDE CMake dependency is absent.
verify_kde_kscreen_cmake() {
    $KDE_INTEGRATION || return 0
    local probe_dir
    probe_dir="$(mktemp -d)"
    cat > "${probe_dir}/CMakeLists.txt" <<'CMAKE_EOF'
cmake_minimum_required(VERSION 3.21)
project(ScreenshaverKScreenDependencyCheck LANGUAGES CXX)
find_package(Qt6 REQUIRED COMPONENTS Core DBus Gui Qml Quick)
find_package(KF6Screen REQUIRED)
CMAKE_EOF
    if ! cmake -S "$probe_dir" -B "${probe_dir}/build"; then
        rm -rf -- "$probe_dir"
        die "KDE build dependency preflight failed. Qt6 and KF6Screen must be discoverable by CMake before compiling Screenshaver."
    fi
    rm -rf -- "$probe_dir"
}

install_kde_build_dependencies() {
    $KDE_INTEGRATION || return 0
    # Debian-family packages are installed with the base dependencies above.
    if is_debian_family; then return 0; fi

    log "Installing KDE-native Qt6/CMake build dependencies"
    if [[ "$DISTRO_ID" == "fedora" || "$DISTRO_ID" == "rhel" ||
          "$DISTRO_ID" == "centos" || "$DISTRO_ID" == "rocky" ||
          "$DISTRO_ID" == "almalinux" ]] || is_like fedora || is_like rhel; then
        "${SUDO[@]}" dnf install -y cmake ninja-build qt6-qtbase-devel qt6-qtdeclarative-devel
        install_kde_kscreen_package dnf libkscreen-qt6-devel libkscreen-devel
    elif [[ "$DISTRO_ID" == "arch" || "$DISTRO_ID" == "manjaro" ||
            "$DISTRO_ID" == "endeavouros" || "$DISTRO_ID" == "omarchy" ]] || is_like arch; then
        "${SUDO[@]}" pacman -S --needed --noconfirm cmake ninja qt6-base qt6-declarative
        install_kde_kscreen_package pacman libkscreen
    elif [[ "$DISTRO_ID" == opensuse-* || "$DISTRO_ID" == "sles" ]] || is_like suse; then
        "${SUDO[@]}" zypper --non-interactive install cmake ninja libQt6Core6-devel libQt6DBus6-devel libQt6Gui6-devel libQt6Qml6-devel libQt6Quick6-devel
        install_kde_kscreen_package zypper libkscreen6-devel libkscreen-qt6-devel
    elif [[ "$DISTRO_ID" == "void" ]] || is_like void; then
        "${SUDO[@]}" xbps-install -y cmake ninja qt6-base-devel qt6-declarative-devel
        install_kde_kscreen_package xbps libkscreen-devel libkscreen-qt6-devel
    else
        die "KDE-native build dependency package mapping is not verified for ${DISTRO_NAME}."
    fi
}

build_kde_native_integration() {
    $KDE_INTEGRATION || return 0
    [[ -f "${PROJECT_DIR}/kde-renderer/Cargo.toml" ]] ||
        die "KDE renderer manifest missing: ${PROJECT_DIR}/kde-renderer/Cargo.toml"
    [[ -f "${PROJECT_DIR}/kde-host/CMakeLists.txt" ]] ||
        die "KDE host CMakeLists.txt is missing."
    [[ -f "${PROJECT_DIR}/kde-host/build_kde_host.sh" ]] ||
        die "KDE host build script is missing."

    command -v cmake >/dev/null 2>&1 || die "CMake unavailable after KDE dependency installation."

    log "Building the KDE native Rust renderer (release)"
    cargo build --release --manifest-path "${PROJECT_DIR}/kde-renderer/Cargo.toml"
    [[ -f "${PROJECT_DIR}/kde-renderer/target/release/libscreenshaver.so" ]] ||
        die "KDE renderer shared library missing after build."

    # Independent production frame producer: separate Cargo crate, not part of
    # the main binary or the KDE renderer library crate.
    local producer_manifest="${PROJECT_DIR}/kde-wallpaper-producer/Cargo.toml"
    local producer_binary="${PROJECT_DIR}/kde-wallpaper-producer/target/release/screenshaver-plasma-producer"
    [[ -f "$producer_manifest" ]] || die "KDE wallpaper producer manifest missing: $producer_manifest"
    log "Building the KDE wallpaper frame producer (release)"
    cargo build --release --locked --manifest-path "$producer_manifest"
    [[ -x "$producer_binary" ]] || die "KDE wallpaper producer executable missing after build: $producer_binary"

    log "Building the KDE Qt6 native host plugin (release)"
    # The KDE host build script also publishes a user-level runtime generation
    # for live development. An installer must NOT modify that selection: it may
    # be in use by a running KDE greeter or development session. Redirect that
    # side effect into a disposable build-only XDG data home instead.
    local isolated_data_home
    isolated_data_home="$(mktemp -d)"
    if ! XDG_DATA_HOME="$isolated_data_home" bash "${PROJECT_DIR}/kde-host/build_kde_host.sh" release "$PROJECT_DIR"; then
        rm -rf -- "$isolated_data_home"
        die "KDE host release build failed."
    fi
    rm -rf -- "$isolated_data_home"

    local module="${PROJECT_DIR}/kde-host/build-staging-release/qml/ScreenshaverNativeGL"
    local name
    for name in qmldir libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
        [[ -f "${module}/${name}" ]] || die "KDE native build output missing: ${module}/${name}"
    done
    [[ -f "${PROJECT_DIR}/kde-host/build-staging-release/libScreenshaverKScreen.so" ]] ||
        die "KDE KScreen discovery library was not produced by the release build."
    [[ -x "${PROJECT_DIR}/kde-host/build-staging-release/screenshaver-kscreen-discover" ]] ||
        die "KDE KScreen discovery helper was not produced by the release build."
}

kde_qt_qml_root() {
    # Qt distributions do not agree on the qtpaths executable name.
    # Prefer Qt's own queried installation path over a hard-coded distro path.
    local tool root version candidate
    for tool in qtpaths6 qtpaths qmake6; do
        command -v "$tool" >/dev/null 2>&1 || continue
        if [[ "$tool" == qmake6 ]]; then
            version="$(qmake6 -query QT_VERSION 2>/dev/null || true)"
            [[ "$version" == 6.* ]] || continue
            root="$(qmake6 -query QT_INSTALL_QML 2>/dev/null || true)"
        else
            # qtpaths on some systems belongs to Qt5. Do not accept it
            # without a positive Qt6 version check.
            version="$("$tool" --qt-version 2>/dev/null || true)"
            [[ "$version" == 6.* ]] || continue
            root="$("$tool" --query QT_INSTALL_QML 2>/dev/null || true)"
        fi
        if [[ "$root" == /* && "$root" == */qml && -d "$root" ]]; then
            printf '%s\n' "$root"
            return 0
        fi
    done

    # Arch/CachyOS and some other distributions install Qt6 QML modules
    # here even when Qt's path-query utility is not installed. Only accept
    # a directory containing existing Qt6 module namespaces.
    for candidate in /usr/lib/qt6/qml /usr/lib64/qt6/qml; do
        if [[ -d "$candidate/QtQuick" && -d "$candidate/QtQml" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done

    die "Cannot identify a Qt6 QML import directory. Install the Qt6 qtpaths/qmake utility or inspect the system's Qt6 QML module installation."
}

install_kde_native_integration() {
    $KDE_INTEGRATION || return 0
    local source="${PROJECT_DIR}/kde-host/build-staging-release/qml/ScreenshaverNativeGL"
    local destination="${INSTALL_PREFIX}/lib/screenshaver/kde/qml/ScreenshaverNativeGL"
    local name mode
    log "Installing KDE-native production runtime assets (upgrade-safe)"
    # The installed path remains stable across the old build-release and new
    # build-staging-release layouts. Never install from an obsolete build tree.
    for name in qmldir libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
        mode=644
        [[ "$name" == *.so ]] && mode=755
        "${SUDO[@]}" install -Dm"$mode" "${source}/${name}" "${destination}/${name}"
    done
    # Plasma's QML engine searches Qt's standard module paths, not the
    # Screenshaver-private installation directory or a local qmldir plugin
    # imported as a JavaScript namespace.
    local qt_module="$(kde_qt_qml_root)/ScreenshaverNativeGL"
    log "Installing Plasma QML module at $qt_module"
    # A running plasmashell may retain a failed QML import lookup. Detect a
    # changed module before replacing it, so installation cannot silently
    # imply that an existing Plasma session has refreshed its QML engine.
    if pgrep -u "$(id -u)" -x plasmashell >/dev/null 2>&1; then
        for name in qmldir libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
            if [[ ! -f "${qt_module}/${name}" ]] || ! cmp -s -- "${source}/${name}" "${qt_module}/${name}"; then
                KDE_QML_SESSION_REFRESH_NEEDED=true
                break
            fi
        done
    fi
    if [[ -L "$qt_module" ]]; then
        die "Refusing to replace symlinked Qt module: $qt_module"
    fi
    for name in qmldir libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
        mode=644
        [[ "$name" == *.so ]] && mode=755
        "${SUDO[@]}" install -Dm"$mode" "${source}/${name}" "${qt_module}/${name}"
    done
    # The independent frame producer loads this library from the KDE root,
    # not from the QML import directory.
    "${SUDO[@]}" install -Dm755 "${source}/libscreenshaver.so" \
        "${INSTALL_PREFIX}/lib/screenshaver/kde/libscreenshaver.so"

    # Install the independent producer at the exact path resolved by
    # ProducerSettings::from_environment() for /usr/local/bin/screenshaver.
    "${SUDO[@]}" install -Dm755 \
        "${PROJECT_DIR}/kde-wallpaper-producer/target/release/screenshaver-plasma-producer" \
        "${INSTALL_PREFIX}/bin/screenshaver-plasma-producer"

    # The temporary wallpaper selection guard requires a user-level package.
    # Install the production package only; never select it or edit appletsrc.
    local package_source="${PROJECT_DIR}/kde-host/screenshaver-plasma-wallpaper"
    local data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
    [[ "$data_home" == /* ]] || die "XDG_DATA_HOME must be absolute for KDE wallpaper installation."
    local package_dest="${data_home}/plasma/wallpapers/org.screenshaver.wallpaper"
    [[ -f "${package_source}/metadata.json" && -f "${package_source}/contents/ui/main.qml" ]] ||
        die "Production KDE wallpaper package sources are missing."
    if [[ -e "$package_dest" && ! -f "${package_dest}/.screenshaver-installed" ]]; then
        die "Production wallpaper package exists but is not marked as installer-owned: $package_dest"
    fi
    mkdir -p "${package_dest}/contents/ui"
    install -m644 "${package_source}/metadata.json" "${package_dest}/metadata.json"
    install -m644 "${package_source}/contents/ui/main.qml" "${package_dest}/contents/ui/main.qml"
    # Bundle the QML native plugin beside main.qml. Plasma does not automatically
    # search the private /usr/local/lib/screenshaver/kde/qml module directory.
    # The wallpaper's main.qml uses a named import; the Qt QML root copy above
    # is required for discovery. The bundle is retained for packaging diagnostics.
    local bundled_module="${package_dest}/contents/ui/ScreenshaverNativeGL"
    if [[ -L "$bundled_module" ]]; then
        die "Refusing to install KDE QML module through a symlink: $bundled_module"
    fi
    mkdir -p "$bundled_module"
    for name in qmldir libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
        mode=644
        [[ "$name" == *.so ]] && mode=755
        install -m"$mode" "${source}/${name}" "${bundled_module}/${name}"
    done
    printf '%s\n' 'screenshaver-installer-v1' > "${package_dest}/.screenshaver-installed"
    # KScreen discovery is a separate native bridge, built at the CMake root.
    "${SUDO[@]}" install -Dm755 \
        "${PROJECT_DIR}/kde-host/build-staging-release/libScreenshaverKScreen.so" \
        "${INSTALL_PREFIX}/lib/screenshaver/kde/libScreenshaverKScreen.so"
    "${SUDO[@]}" install -Dm755 \
        "${PROJECT_DIR}/kde-host/build-staging-release/screenshaver-kscreen-discover" \
        "${INSTALL_PREFIX}/lib/screenshaver/kde/screenshaver-kscreen-discover"
}

verify_kde_native_integration() {
    $KDE_INTEGRATION || return 0
    local destination="${INSTALL_PREFIX}/lib/screenshaver/kde/qml/ScreenshaverNativeGL"
    local name missing
    for name in qmldir libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
        [[ -f "${destination}/${name}" ]] || die "Installed KDE runtime asset missing: ${destination}/${name}"
    done
    local qt_module="$(kde_qt_qml_root)/ScreenshaverNativeGL"
    log "Verifying Plasma QML module at $qt_module"
    grep -Fxq "module ScreenshaverNativeGL" "${qt_module}/qmldir" ||
        die "Qt QML module declaration is invalid: ${qt_module}/qmldir"
    for name in qmldir libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
        [[ -f "${qt_module}/${name}" ]] || die "Qt QML module asset missing: ${qt_module}/${name}"
        cmp -s -- "${PROJECT_DIR}/kde-host/build-staging-release/qml/ScreenshaverNativeGL/${name}" "${qt_module}/${name}" ||
            die "Qt QML module asset differs from release build: ${name}"
    done
    if command -v ldd >/dev/null 2>&1; then
        for name in libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
            missing="$(ldd "${qt_module}/${name}" 2>/dev/null | awk '/not found/ { print $1 }')"
            [[ -z "$missing" ]] || die "Unresolved Qt QML dependencies for ${name}: ${missing}"
        done
    fi
    # Exercise the installed QML module with an actual Qt engine. This verifies
    # discovery and plugin loading; it does NOT certify an already-running
    # plasmashell QML engine or its cached failed imports.
    if command -v qml6 >/dev/null 2>&1; then
        local qml_probe_dir qml_probe_output
        qml_probe_dir="$(mktemp -d)"
        cat > "${qml_probe_dir}/probe.qml" <<'QML_EOF'
import QtQuick 2.15
import ScreenshaverNativeGL 1.0
Item {
    Component.onCompleted: {
        console.log("SCREENSHAVER_QML_IMPORT_PASS")
        Qt.quit()
    }
}
QML_EOF
        if ! qml_probe_output="$(QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software timeout 15s qml6 -I "$(kde_qt_qml_root)" "${qml_probe_dir}/probe.qml" 2>&1)"; then
            rm -rf -- "$qml_probe_dir"
            die "Qt6 could not import the installed Screenshaver module: $qml_probe_output"
        fi
        rm -rf -- "$qml_probe_dir"
        [[ "$qml_probe_output" == *SCREENSHAVER_QML_IMPORT_PASS* ]] ||
            die "Qt6 import probe did not complete: $qml_probe_output"
        log "Qt6 native QML import probe passed (fresh engine only)"
    else
        warn "qml6 is unavailable; the installed native QML module could not be import-tested."
    fi
    local producer_source="${PROJECT_DIR}/kde-wallpaper-producer/target/release/screenshaver-plasma-producer"
    local producer_installed="${INSTALL_PREFIX}/bin/screenshaver-plasma-producer"
    [[ -x "$producer_installed" ]] || die "KDE frame producer executable not installed: $producer_installed"
    cmp -s -- "$producer_source" "$producer_installed" ||
        die "Installed KDE frame producer differs from the release build."
    local root_library="${INSTALL_PREFIX}/lib/screenshaver/kde/libscreenshaver.so"
    [[ -f "$root_library" ]] || die "KDE producer renderer library missing: $root_library"
    cmp -s -- "${PROJECT_DIR}/kde-host/build-staging-release/qml/ScreenshaverNativeGL/libscreenshaver.so" "$root_library" ||
        die "KDE producer renderer library differs from release build."
    local data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
    local package_dest="${data_home}/plasma/wallpapers/org.screenshaver.wallpaper"
    [[ -f "${package_dest}/.screenshaver-installed" ]] || die "Production KDE wallpaper ownership marker missing."
    cmp -s -- "${PROJECT_DIR}/kde-host/screenshaver-plasma-wallpaper/metadata.json" "${package_dest}/metadata.json" ||
        die "Production KDE wallpaper metadata differs from source."
    cmp -s -- "${PROJECT_DIR}/kde-host/screenshaver-plasma-wallpaper/contents/ui/main.qml" "${package_dest}/contents/ui/main.qml" ||
        die "Production KDE wallpaper QML differs from source."
    local bundled_module="${package_dest}/contents/ui/ScreenshaverNativeGL"
    for name in qmldir libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
        [[ -f "${bundled_module}/${name}" ]] ||
            die "Bundled KDE wallpaper QML module asset missing: ${bundled_module}/${name}"
        cmp -s -- "${PROJECT_DIR}/kde-host/build-staging-release/qml/ScreenshaverNativeGL/${name}" "${bundled_module}/${name}" ||
            die "Bundled KDE wallpaper QML module differs from release build: ${name}"
    done
    if command -v ldd >/dev/null 2>&1; then
        for name in libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
            missing="$(ldd "${bundled_module}/${name}" 2>/dev/null | awk '/not found/ { print $1 }')"
            [[ -z "$missing" ]] || die "Unresolved bundled KDE QML libraries for ${name}: ${missing}"
        done
    fi
    local kscreen_source="${PROJECT_DIR}/kde-host/build-staging-release/libScreenshaverKScreen.so"
    local kscreen_destination="${INSTALL_PREFIX}/lib/screenshaver/kde/libScreenshaverKScreen.so"
    [[ -f "$kscreen_destination" ]] || die "Installed KScreen discovery library missing: $kscreen_destination"
    cmp -s -- "$kscreen_source" "$kscreen_destination" ||
        die "Installed KScreen discovery library differs from the current release build."
    local kscreen_helper_source="${PROJECT_DIR}/kde-host/build-staging-release/screenshaver-kscreen-discover"
    local kscreen_helper_dest="${INSTALL_PREFIX}/lib/screenshaver/kde/screenshaver-kscreen-discover"
    [[ -x "$kscreen_helper_dest" ]] || die "Installed KScreen discovery helper missing: $kscreen_helper_dest"
    cmp -s -- "$kscreen_helper_source" "$kscreen_helper_dest" ||
        die "Installed KScreen discovery helper differs from the current release build."
    # Compare the installed generation to the release build; an old plugin
    # must never silently survive an upgrade.
    local source="${PROJECT_DIR}/kde-host/build-staging-release/qml/ScreenshaverNativeGL"
    for name in qmldir libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
        cmp -s -- "${source}/${name}" "${destination}/${name}" ||
            die "Installed KDE asset differs from the current release build: ${name}"
    done
    if command -v ldd >/dev/null 2>&1; then
        for name in libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
            missing="$(ldd "${destination}/${name}" 2>/dev/null | awk '/not found/ { print $1 }')"
            [[ -z "$missing" ]] || die "Unresolved KDE runtime libraries for ${name}: ${missing}"
        done
        missing="$(ldd "$kscreen_destination" 2>/dev/null | awk '/not found/ { print $1 }')"
        [[ -z "$missing" ]] || die "Unresolved KScreen discovery libraries: ${missing}"
        missing="$(ldd "$kscreen_helper_dest" 2>/dev/null | awk '/not found/ { print $1 }')"
        [[ -z "$missing" ]] || die "Unresolved KScreen discovery helper libraries: ${missing}"
    fi
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

resolve_xfce_saver_directory() {
    XFCE_SAVER_DIR=""
    $XFCE_INTEGRATION || return 0
    # This is Screenshaver's stable launcher location, not a claim about
    # the distribution's xfce4-screensaver executable directory.
    XFCE_SAVER_DIR="/usr/libexec/xfce4-screensaver"
}


# -----------------------------------------------------------------------------
# MATE native screen-lock presenter (conventional distributions)
# -----------------------------------------------------------------------------

install_mate_lock_integration() {
    local presenter="/usr/libexec/mate-screensaver/screenshaver"
    local installed_binary="${INSTALL_PREFIX}/bin/${BINARY_NAME}"
    local expected existing temp_file

    $MATE_INTEGRATION || return 0

    if [[ ! -x /usr/bin/mate-screensaver ]]; then
        warn "MATE native screensaver is unavailable; MATE lock integration was not installed."
        return 0
    fi

    [[ -x "$installed_binary" ]] || die "Installed Screenshaver binary is missing: $installed_binary"

    # Keep this contract synchronized with construct_lock_screen_mate.rs.
    # Do not permanently select a MATE theme; the resident runtime owns that.
    expected="$(cat <<EOF
#!/bin/sh

export SCREENSHAVER_MATE_PRESENTATION=1

if [ -z "\${XDG_RUNTIME_DIR:-}" ]; then
    export XDG_RUNTIME_DIR="/run/user/\$(id -u)"
fi

exec ${installed_binary}
EOF
)"

    if [[ -e "$presenter" || -L "$presenter" ]]; then
        if [[ ! -f "$presenter" || -L "$presenter" ]]; then
            die "Refusing to replace an existing non-regular MATE presenter: $presenter"
        fi
        existing="$(cat -- "$presenter")"
        if [[ "$existing" != "$expected" ]]; then
            die "Refusing to overwrite a different MATE presenter: $presenter. Inspect and reconcile it manually."
        fi
        log "Existing MATE presenter matches the expected launcher"
    else
        log "Installing the trusted MATE Screenshaver presenter"
        temp_file="$(mktemp)"
        printf '%s\n' "$expected" > "$temp_file"
        if ! "${SUDO[@]}" install -Dm755 -- "$temp_file" "$presenter"; then
            rm -f -- "$temp_file"
            die "Unable to install MATE presenter: $presenter"
        fi
        rm -f -- "$temp_file"
    fi

    [[ -x "$presenter" ]] || die "MATE presenter is not executable: $presenter"
    log "Registering the MATE Screenshaver theme for the current user"
    "$installed_binary" --construct-lock-screen-mate
}

install_omarchy_pam_service() {
    # Omarchy uses a dedicated PAM service for its native lock-password stack.
    # Screenshaver keeps its own stable PAM service name ("screenshaver") and
    # delegates to Omarchy here, rather than hard-coding distro-specific PAM
    # service names in the Rust authentication code.
    #
    # Screenshaver calls both pam_authenticate() and pam_acct_mgmt().  Both PAM
    # management groups therefore must be defined.  Omitting the account stack
    # causes PAM to fall through to pam_warn(screenshaver:account); the password
    # can authenticate successfully, but Screenshaver will still reject the
    # unlock and desktop access will not be restored.
    [[ "$DISTRO_ID" == "omarchy" ]] || return 0

    [[ -r /etc/pam.d/omarchy-lock-password ]] ||
        die "/etc/pam.d/omarchy-lock-password was not found on this Omarchy system."

    log "Installing the Screenshaver PAM service for Omarchy"

    cat <<'PAM_EOF' | "${SUDO[@]}" tee /etc/pam.d/screenshaver >/dev/null
#%PAM-1.0

# Screenshaver authentication on Omarchy.
#
# Delegate both password authentication and account validation to Omarchy's
# native lock-password PAM service. Screenshaver calls both pam_authenticate()
# and pam_acct_mgmt(), so both PAM management groups must be defined here.
auth       include    omarchy-lock-password
account    include    omarchy-lock-password
PAM_EOF

    "${SUDO[@]}" chmod 0644 /etc/pam.d/screenshaver
}

install_xfce_lock_integration() {
    local trusted_presenter
    local installed_binary

    $XFCE_INTEGRATION || return 0

    if ! resolve_xfce_saver_directory; then
        warn \
            "Xfce was detected, but the trusted xfce4-screensaver engine directory
is not yet verified for ${DISTRO_NAME}. Xfce lock-screen integration was not
installed."
        return 0
    fi

    [[ -x /usr/bin/xfce4-screensaver ]] ||
        die "xfce4-screensaver was not found after installing the Xfce runtime dependency."

    installed_binary="${INSTALL_PREFIX}/bin/${BINARY_NAME}"
    trusted_presenter="${XFCE_SAVER_DIR}/${BINARY_NAME}"

    log "Installing the trusted Xfce Screenshaver presenter"

    "${SUDO[@]}" install \
        -Dm755 \
        "$installed_binary" \
        "$trusted_presenter"

    [[ -x "$trusted_presenter" ]] ||
        die "Trusted Xfce presenter was not installed: $trusted_presenter"

    log "Configuring Screenshaver for the current Xfce user"

    "$installed_binary" --construct-lock-screen-xfce
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

verify_libdecor_linkage() {
    local installed_binary="${INSTALL_PREFIX}/bin/${BINARY_NAME}"
    local dependencies

    command -v ldd >/dev/null 2>&1 || return 0
    dependencies="$(ldd "$installed_binary")"
    grep -Eq '^[[:space:]]*libdecor-0\.so\.0[[:space:]]*=>' <<< "$dependencies" ||
        die "Installed Screenshaver is missing the native libdecor shared-library dependency."
}

verify_conventional_installation() {
    log "Verifying the installation"

    verify_desktop_file
    verify_shared_libraries
    verify_libdecor_linkage
    verify_kde_native_integration
    if $KDE_INTEGRATION; then
        log "KDE production runtime verified under ${INSTALL_PREFIX}/lib/screenshaver/kde"
    fi

    if $XFCE_INTEGRATION; then
        resolve_xfce_saver_directory
        [[ -x "${XFCE_SAVER_DIR}/${BINARY_NAME}" ]] ||
            die "Trusted Xfce presenter is missing after installation."
    fi
    if $MATE_INTEGRATION; then
        [[ -x /usr/libexec/mate-screensaver/screenshaver ]] ||
            die "Trusted MATE presenter is missing after installation."
    fi
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

verify_gnome_extension_prerequisite() {
    case "${DESKTOP_ENVIRONMENT,,}" in
        *gnome*|*zorin*) ;;
        *) return 0 ;;
    esac

    log "Verifying GNOME Shell extension management prerequisite"
    command -v gnome-extensions >/dev/null 2>&1 ||
        die "GNOME was detected, but 'gnome-extensions' is unavailable. Install the distribution's GNOME Shell extension management tools and rerun the installer."
}

install_conventional_linux() {
    initialize_sudo
    validate_runtime_assets
    install_build_dependencies
    detect_desktop_environment
    verify_gnome_extension_prerequisite
    if $KDE_INTEGRATION; then
        install_kde_build_dependencies
        verify_kde_kscreen_cmake
    fi
    install_rust_toolchain
    build_application
    build_kde_native_integration
    install_application
    install_kde_native_integration
    install_omarchy_pam_service
    install_xfce_lock_integration
    install_mate_lock_integration
    refresh_desktop_caches
    verify_conventional_installation
    if $KDE_QML_SESSION_REFRESH_NEEDED; then
        warn "The KDE QML module changed while plasmashell was running."
        warn "The existing Plasma session may retain a failed import or old plugin instance."
        warn "A complete logout/login may be required before KDE wallpaper activation works."
        warn "Do not interpret the fresh-engine QML probe as live Plasma readiness."
    fi
    print_conventional_success
    printf 'Desktop integration inventory: KDE=%s Xfce=%s MATE=%s\n' \
        "$KDE_INTEGRATION" "$XFCE_INTEGRATION" "$MATE_INTEGRATION"
    if $MATE_INTEGRATION; then
        cat <<'MATE_NOTICE'
MATE system tray: If the Screenshaver icon is not visible, add
"Indicator Applet Complete" to your MATE panel and restart Screenshaver.
MATE_NOTICE
    fi
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
    log "Building ${APP_NAME} with Nix (standalone project flake)"

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
    detect_desktop_environment

    printf '%s source installer\n' "$APP_NAME"
    printf 'Detected system: %s\n' "$DISTRO_NAME"
    printf 'Detected desktop: %s\n' "${DESKTOP_ENVIRONMENT:-unknown}"
    printf 'Project directory: %s\n' "$PROJECT_DIR"

    if [[ "$DISTRO_ID" == "nixos" ]] || is_like nixos; then
        install_nixos
        exit 0
    fi

    printf 'Installation prefix: %s\n' "$INSTALL_PREFIX"

    install_conventional_linux
}

main "$@"

