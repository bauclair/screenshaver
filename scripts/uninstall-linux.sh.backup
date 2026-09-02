#!/usr/bin/env bash

set -Eeuo pipefail

# -----------------------------------------------------------------------------
# Screenshaver Linux uninstaller
#
# This script removes files installed specifically for Screenshaver.
#
# Conventional Linux distributions:
#   * Removes /usr/local/bin/screenshaver
#   * Removes the Screenshaver desktop launcher
#   * Removes Screenshaver icons from the hicolor icon theme
#   * Refreshes desktop and icon caches
#
# NixOS:
#   * Removes Screenshaver from the current user's Nix profile
#
# Shared dependencies are deliberately retained. The script does not remove:
#
#   * Rust
#   * Cargo
#   * rustup
#   * C/C++ compilers
#   * SDL2
#   * SDL2_ttf
#   * X11 libraries
#   * XScreenSaver libraries
#   * OpenGL libraries
#
# These components may have existed before Screenshaver was installed or may
# be required by other software. Removing them without reliable installation
# provenance could damage unrelated applications.
#
# Run as a normal user:
#
#   ./scripts/uninstall-linux.sh
#
# Do not run the entire script with sudo.
# -----------------------------------------------------------------------------

APP_NAME="Screenshaver"
BINARY_NAME="screenshaver"
FLAKE_PACKAGE_NAME="screenshaver"

INSTALL_PREFIX="/usr/local"
ASSUME_YES=false

DISTRO_ID=""
DISTRO_LIKE=""
DISTRO_NAME=""

SUDO=()

usage() {
    cat <<USAGE
Usage: $(basename "$0") [OPTIONS]

Uninstall ${APP_NAME}.

Options:
  -y, --yes       Do not ask for confirmation
  -h, --help      Show this help text

The uninstaller removes only files belonging specifically to ${APP_NAME}.

It does not remove shared development or runtime dependencies because those
packages may be used by other software.
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
        '\nUninstallation stopped at line %s (exit code %s).\n' \
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

    read -r -p "$prompt [y/N] " reply

    case "${reply:-N}" in
        y|Y|yes|YES)
            return 0
            ;;

        *)
            return 1
            ;;
    esac
}

require_linux() {
    [[ "$(uname -s)" == "Linux" ]] ||
        die "This uninstaller supports Linux only."
}

require_normal_user() {
    [[ "$EUID" -ne 0 ]] ||
        die \
            "Do not run this entire script with sudo.

Run it as your normal user:

  ./scripts/uninstall-linux.sh

The script will request sudo only when removing files from /usr/local."
}

load_os_release() {
    if [[ ! -r /etc/os-release ]]; then
        DISTRO_ID="unknown"
        DISTRO_LIKE=""
        DISTRO_NAME="Unknown Linux distribution"
        return
    fi

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

is_nixos() {
    [[ "$DISTRO_ID" == "nixos" ]] || is_like nixos
}

initialize_sudo() {
    if command -v sudo >/dev/null 2>&1; then
        SUDO=(sudo)
    else
        die \
            "sudo is required to remove Screenshaver files from:

  ${INSTALL_PREFIX}"
    fi
}

remove_file_if_present() {
    local path="$1"

    if [[ -e "$path" || -L "$path" ]]; then
        log "Removing $path"
        "${SUDO[@]}" rm -f -- "$path"
    fi
}

remove_empty_directory() {
    local directory="$1"

    if [[ -d "$directory" ]]; then
        "${SUDO[@]}" rmdir --ignore-fail-on-non-empty "$directory" 2>/dev/null ||
            true
    fi
}

# -----------------------------------------------------------------------------
# Conventional Linux uninstall
# -----------------------------------------------------------------------------

remove_installed_binary() {
    remove_file_if_present \
        "${INSTALL_PREFIX}/bin/${BINARY_NAME}"
}

remove_desktop_launcher() {
    remove_file_if_present \
        "${INSTALL_PREFIX}/share/applications/screenshaver.desktop"
}

remove_screenshaver_icons() {
    local icon_root
    local icon_file
    local removed_count=0

    icon_root="${INSTALL_PREFIX}/share/icons/hicolor"

    if [[ ! -d "$icon_root" ]]; then
        return
    fi

    log "Searching for installed Screenshaver icons"

    while IFS= read -r -d '' icon_file; do
        printf 'Removing %s\n' "$icon_file"

        "${SUDO[@]}" rm -f -- "$icon_file"

        ((removed_count += 1))
    done < <(
        find "$icon_root" \
            -type f \
            \( \
                -iname 'screenshaver.png' \
                -o -iname 'screenshaver.svg' \
                -o -iname 'screenshaver.xpm' \
            \) \
            -print0
    )

    if [[ "$removed_count" -eq 0 ]]; then
        printf 'No installed Screenshaver icons were found.\n'
    fi

    # Remove only directories that became empty. Directories containing icons
    # belonging to other applications are preserved.
    while IFS= read -r directory; do
        remove_empty_directory "$directory"
    done < <(
        find "$icon_root" \
            -depth \
            -type d \
            \( \
                -path '*/apps' \
                -o -path '*/scalable' \
                -o -path '*/16x16' \
                -o -path '*/22x22' \
                -o -path '*/24x24' \
                -o -path '*/32x32' \
                -o -path '*/48x48' \
                -o -path '*/64x64' \
                -o -path '*/96x96' \
                -o -path '*/128x128' \
                -o -path '*/256x256' \
                -o -path '*/512x512' \
            \) \
            -print
    )
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
refresh automatically after the next login."
    fi

    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        "${SUDO[@]}" gtk-update-icon-cache \
            -f \
            -t \
            "${INSTALL_PREFIX}/share/icons/hicolor" ||
            warn "The icon cache could not be refreshed."
    else
        warn \
            "gtk-update-icon-cache was not found. The desktop may refresh its
icon cache automatically."
    fi
}

verify_conventional_removal() {
    local remaining=false

    log "Verifying Screenshaver file removal"

    if [[ -e "${INSTALL_PREFIX}/bin/${BINARY_NAME}" ||
          -L "${INSTALL_PREFIX}/bin/${BINARY_NAME}" ]]; then

        warn \
            "The installed executable still exists:

  ${INSTALL_PREFIX}/bin/${BINARY_NAME}"

        remaining=true
    fi

    if [[ -e "${INSTALL_PREFIX}/share/applications/screenshaver.desktop" ]]; then
        warn \
            "The desktop launcher still exists:

  ${INSTALL_PREFIX}/share/applications/screenshaver.desktop"

        remaining=true
    fi

    if find "${INSTALL_PREFIX}/share/icons/hicolor" \
        -type f \
        \( \
            -iname 'screenshaver.png' \
            -o -iname 'screenshaver.svg' \
            -o -iname 'screenshaver.xpm' \
        \) \
        -print -quit 2>/dev/null |
        grep -q .; then

        warn "One or more Screenshaver icons remain installed."
        remaining=true
    fi

    if $remaining; then
        die "One or more Screenshaver files could not be removed."
    fi
}

print_dependency_retention_notice() {
    cat <<NOTICE

Shared dependencies were retained intentionally.

The uninstaller did not remove Rust, Cargo, rustup, build tools, SDL2,
SDL2_ttf, X11, XScreenSaver, or OpenGL packages.

Those components may:

  * Have been installed before Screenshaver
  * Be used by another program
  * Be needed for other Rust development projects
  * Be required by the desktop environment

Your distribution's package manager can identify genuinely unused packages,
but a Screenshaver-specific uninstaller cannot safely prove that these shared
packages are no longer needed.

NOTICE
}

print_conventional_success() {
    cat <<SUCCESS

${APP_NAME} was uninstalled successfully.

Removed:

  ${INSTALL_PREFIX}/bin/${BINARY_NAME}
  ${INSTALL_PREFIX}/share/applications/screenshaver.desktop
  Screenshaver icons under ${INSTALL_PREFIX}/share/icons/hicolor

User configuration, shaders, logs, and shared dependencies were preserved.

SUCCESS

    print_dependency_retention_notice
}

uninstall_conventional_linux() {
    initialize_sudo

    remove_installed_binary
    remove_desktop_launcher
    remove_screenshaver_icons
    refresh_desktop_caches
    verify_conventional_removal
    print_conventional_success
}

# -----------------------------------------------------------------------------
# NixOS uninstall
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

nix_profile_contains_screenshaver() {
    nix_command profile list 2>/dev/null |
        grep -i -q "$FLAKE_PACKAGE_NAME"
}

remove_nix_profile_entry_by_name() {
    local candidate

    # Profile element names may differ depending on the Nix version and how the
    # package was installed. Try likely names without treating a failed attempt
    # as a fatal script error.
    for candidate in \
        "$FLAKE_PACKAGE_NAME" \
        "$APP_NAME"; do

        if nix_command profile remove "$candidate" >/dev/null 2>&1; then
            printf 'Removed Nix profile entry: %s\n' "$candidate"
            return 0
        fi
    done

    return 1
}

remove_nix_profile_entry_by_index() {
    local matching_indexes
    local index
    local removed=false

    matching_indexes="$(
        nix_command profile list 2>/dev/null |
            awk '
                BEGIN {
                    IGNORECASE = 1
                }

                /screenshaver/ {
                    if ($1 ~ /^[0-9]+$/) {
                        print $1
                    }
                }
            '
    )"

    if [[ -z "$matching_indexes" ]]; then
        return 1
    fi

    # Remove in reverse numeric order so deleting one index does not alter an
    # index that still needs to be removed.
    while IFS= read -r index; do
        [[ -n "$index" ]] || continue

        log "Removing Screenshaver Nix profile entry $index"

        nix_command profile remove "$index"
        removed=true
    done < <(
        printf '%s\n' "$matching_indexes" |
            sort -rn
    )

    $removed
}

remove_nixos_profile_entry() {
    if ! nix_profile_contains_screenshaver; then
        warn \
            "No Screenshaver entry was found in the current user's Nix profile."
        return
    fi

    log "Removing Screenshaver from the current user's Nix profile"

    if remove_nix_profile_entry_by_name; then
        return
    fi

    if remove_nix_profile_entry_by_index; then
        return
    fi

    die \
        "Screenshaver appears in the Nix profile, but its entry could not be
removed automatically.

Review the profile manually:

  nix profile list

Then remove the corresponding entry:

  nix profile remove <index-or-name>"
}

verify_nixos_removal() {
    log "Verifying the Nix profile removal"

    if nix_profile_contains_screenshaver; then
        warn \
            "A Screenshaver-related entry still appears in the current user's
Nix profile."

        nix_command profile list || true

        die "Screenshaver could not be fully removed from the Nix profile."
    fi
}

print_nixos_success() {
    cat <<SUCCESS

${APP_NAME} was removed from the current user's Nix profile.

Nix store paths and shared dependencies were not removed directly.

Nix automatically preserves store objects that are still referenced by other
profiles, generations, packages, or system configurations.

Unused Nix store objects may later be collected through the user's normal
garbage-collection policy.

User configuration, shaders, and logs were preserved.

SUCCESS
}

uninstall_nixos() {
    verify_nix_available
    remove_nixos_profile_entry
    verify_nixos_removal
    print_nixos_success
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

main() {
    parse_arguments "$@"

    require_linux
    require_normal_user
    load_os_release

    printf '%s uninstaller\n' "$APP_NAME"
    printf 'Detected system: %s\n' "$DISTRO_NAME"

    if ! confirm "Uninstall ${APP_NAME}?"; then
        printf '\nUninstallation cancelled.\n'
        exit 0
    fi

    if is_nixos; then
        uninstall_nixos
    else
        uninstall_conventional_linux
    fi
}

main "$@"
