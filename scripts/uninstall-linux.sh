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
XFCE_TRUSTED_PRESENTER="/usr/libexec/xfce4-screensaver/screenshaver"
MATE_TRUSTED_PRESENTER="/usr/libexec/mate-screensaver/screenshaver"
XFCE_SAVER_DESKTOP="$HOME/.local/share/applications/screensavers/screenshaver.desktop"
LIGHT_LOCKER_OVERRIDE="$HOME/.config/autostart/light-locker.desktop"
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
# Xfce lock-screen integration removal
# -----------------------------------------------------------------------------

remove_user_file_if_exact() {
    local path="$1"
    local expected_content="$2"

    [[ -f "$path" ]] || return

    if [[ "$(cat "$path")" == "$expected_content" ]]; then
        log "Removing Screenshaver-owned user configuration: $path"
        rm -f -- "$path"
    else
        warn "Preserving modified user configuration: $path"
    fi
}

remove_xfce_lock_integration() {
    local saver_desktop_content
    local light_locker_content

    saver_desktop_content='[Desktop Entry]
Name=Screenshaver
Comment=Screenshaver shader presentation for the Xfce lock screen
Exec=/usr/libexec/xfce4-screensaver/screenshaver
TryExec=/usr/libexec/xfce4-screensaver/screenshaver
StartupNotify=false
Terminal=false
Type=Application
Categories=Screensaver;
OnlyShowIn=XFCE;'

    light_locker_content='[Desktop Entry]
Type=Application
Name=Screen Locker
Hidden=true'

    # Remove only the trusted executable installed specifically for Screenshaver.
    remove_file_if_present "$XFCE_TRUSTED_PRESENTER"

    # User configuration is removed only when it still exactly matches the
    # content generated by Screenshaver. Customized files are preserved.
    remove_user_file_if_exact "$XFCE_SAVER_DESKTOP" "$saver_desktop_content"
    remove_user_file_if_exact "$LIGHT_LOCKER_OVERRIDE" "$light_locker_content"

    # Do not remove xfce4-screensaver itself. It is a shared desktop component.
}

# -----------------------------------------------------------------------------
# MATE native lock-screen integration removal
# -----------------------------------------------------------------------------

remove_mate_lock_integration() {
    local expected_presenter expected_theme theme_path data_home

    # Match the launcher produced by install-linux.sh. Do not delete an
    # unrelated or modified executable under MATE's system libexec directory.
    expected_presenter="$(cat <<EOF
#!/bin/sh

export SCREENSHAVER_MATE_PRESENTATION=1

if [ -z "\${XDG_RUNTIME_DIR:-}" ]; then
    export XDG_RUNTIME_DIR="/run/user/\$(id -u)"
fi

exec ${INSTALL_PREFIX}/bin/${BINARY_NAME}
EOF
)"

    if [[ -f "$MATE_TRUSTED_PRESENTER" && ! -L "$MATE_TRUSTED_PRESENTER" ]]; then
        if [[ "$(cat -- "$MATE_TRUSTED_PRESENTER")" == "$expected_presenter" ]]; then
            remove_file_if_present "$MATE_TRUSTED_PRESENTER"
        else
            warn "Preserving nonmatching MATE presenter: $MATE_TRUSTED_PRESENTER"
        fi
    elif [[ -e "$MATE_TRUSTED_PRESENTER" || -L "$MATE_TRUSTED_PRESENTER" ]]; then
        warn "Preserving non-regular MATE presenter: $MATE_TRUSTED_PRESENTER"
    fi

    data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
    if [[ "$data_home" != /* ]]; then
        warn "Cannot safely resolve relative XDG_DATA_HOME; MATE theme left unchanged"
        return 0
    fi
    theme_path="$data_home/applications/screensavers/screenshaver-mate.desktop"
    expected_theme='[Desktop Entry]
Name=Screenshaver
Comment=Screenshaver shader presentation for the native MATE screensaver
Exec=/usr/libexec/mate-screensaver/screenshaver
TryExec=/usr/libexec/mate-screensaver/screenshaver
StartupNotify=false
Terminal=false
Type=Application
Categories=Screensaver;
OnlyShowIn=MATE;'
    remove_user_file_if_exact "$theme_path" "$expected_theme"

    # MATE owns authentication. Never uninstall mate-screensaver or change
    # the selected theme here; the resident runtime must be stopped first.
}

# -----------------------------------------------------------------------------
# KDE native integration cleanup
# -----------------------------------------------------------------------------

screenshaver_process_running() {
    local proc executable name
    for proc in /proc/[0-9]*; do
        [[ -d "$proc" ]] || continue
        executable="$(readlink -- "$proc/exe" 2>/dev/null || true)"
        name="${executable##*/}"
        case "$name" in
            screenshaver|.screenshaver-wrapped|screenshaver\ \(deleted\)|.screenshaver-wrapped\ \(deleted\))
                return 0 ;;
        esac
    done
    return 1
}

remove_kde_native_integration() {
    local root="${INSTALL_PREFIX}/lib/screenshaver/kde/qml/ScreenshaverNativeGL"
    local name
    for name in qmldir libScreenshaverNativeGLPlugin.so libscreenshaver.so; do
        remove_file_if_present "${root}/${name}"
    done
    remove_file_if_present "${INSTALL_PREFIX}/bin/screenshaver-plasma-producer"
    remove_file_if_present "${INSTALL_PREFIX}/lib/screenshaver/kde/libscreenshaver.so"
    remove_file_if_present "${INSTALL_PREFIX}/lib/screenshaver/kde/libScreenshaverKScreen.so"
    local data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
    if [[ "$data_home" == /* ]]; then
        local package="${data_home}/plasma/wallpapers/org.screenshaver.wallpaper"
        if [[ -f "$package/.screenshaver-installed" ]] &&
           [[ "$(cat "$package/.screenshaver-installed")" == screenshaver-installer-v1 ]]; then
            # Only remove the known installed files. Leave modified/extra package
            # content in place rather than recursively deleting user data.
            rm -f -- "$package/metadata.json" "$package/contents/ui/main.qml" "$package/.screenshaver-installed"
            rmdir -- "$package/contents/ui" "$package/contents" "$package" 2>/dev/null || true
        else
            [[ ! -e "$package" ]] || warn "Preserving unmarked KDE wallpaper package: $package"
        fi
    fi
    remove_empty_directory "$root"
    remove_empty_directory "${INSTALL_PREFIX}/lib/screenshaver/kde/qml"
    remove_empty_directory "${INSTALL_PREFIX}/lib/screenshaver/kde"
    remove_empty_directory "${INSTALL_PREFIX}/lib/screenshaver"
}

remove_kde_development_runtime() {
    local data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
    local root="${data_home}/screenshaver/kde-lock-runtime"
    local link target generation entry
    [[ "$data_home" == /* ]] || { warn "Relative XDG_DATA_HOME: preserving KDE development runtime"; return; }
    [[ -d "$root" && ! -L "$root" ]] || return 0

    # Remove only the runtime layout published by build_kde_host.sh. Do not
    # follow symlinks, remove unrelated files, or touch Plasma's shell overlay.
    link="$root/current"
    if [[ -L "$link" ]]; then
        target="$(readlink -- "$link")"
        if [[ "$target" == .generation-* && "$target" != */* ]]; then
            rm -f -- "$link"
        else
            warn "Preserving unexpected KDE runtime selection: $link"
            return 0
        fi
    elif [[ -e "$link" ]]; then
        warn "Preserving unexpected KDE runtime selection: $link"
        return 0
    fi

    shopt -s nullglob
    for generation in "$root"/.generation-*; do
        [[ -d "$generation" && ! -L "$generation" ]] || continue
        # Avoid removing a directory that contains anything other than the
        # exact files created by Screenshaver's runtime publisher.
        local safe=true
        for entry in "$generation"/* "$generation"/.[!.]* "$generation"/..?*; do
            [[ -e "$entry" || -L "$entry" ]] || continue
            case "${entry##*/}" in
                qmldir|libScreenshaverNativeGLPlugin.so|libscreenshaver.so|build-profile) ;;
                *) safe=false ;;
            esac
        done
        if $safe; then
            rm -f -- "$generation/qmldir" "$generation/libScreenshaverNativeGLPlugin.so" \
                "$generation/libscreenshaver.so" "$generation/build-profile"
            rmdir -- "$generation" 2>/dev/null || true
        else
            warn "Preserving modified KDE runtime generation: $generation"
        fi
    done
    shopt -u nullglob
    rmdir -- "$root" 2>/dev/null || true
    rmdir -- "$data_home/screenshaver" 2>/dev/null || true
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

    if [[ -e "$XFCE_TRUSTED_PRESENTER" || -L "$XFCE_TRUSTED_PRESENTER" ]]; then
        warn "The Screenshaver Xfce trusted presenter still exists: $XFCE_TRUSTED_PRESENTER"
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

    if [[ -e "${INSTALL_PREFIX}/bin/screenshaver-plasma-producer" ||
          -L "${INSTALL_PREFIX}/bin/screenshaver-plasma-producer" ]]; then
        warn "Screenshaver KDE frame producer remains installed."
        remaining=true
    fi
    local kde_root="${INSTALL_PREFIX}/lib/screenshaver/kde/qml/ScreenshaverNativeGL"
    if [[ -e "$kde_root/qmldir" || -e "$kde_root/libScreenshaverNativeGLPlugin.so" ||
          -e "$kde_root/libscreenshaver.so" ]]; then
        warn "One or more Screenshaver KDE native runtime assets remain installed."
        remaining=true
    fi
    if [[ -e "${INSTALL_PREFIX}/lib/screenshaver/kde/libscreenshaver.so" ||
          -e "${INSTALL_PREFIX}/lib/screenshaver/kde/libScreenshaverKScreen.so" ]]; then
        warn "KDE producer/KScreen libraries remain installed."
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
SDL2_ttf, X11, XScreenSaver, OpenGL, or xfce4-screensaver packages.

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
  Screenshaver Xfce trusted presenter, when present
  Unmodified Screenshaver-owned Xfce user integration files, when present

Customized user configuration, shaders, logs, and shared dependencies were preserved.

SUCCESS

    print_dependency_retention_notice
}

uninstall_conventional_linux() {
    # KDE's temporary shell overlay is restored by Screenshaver on exit.
    # Never remove its files while a renderer or greeter may still use them.
    if screenshaver_process_running; then
        die "Screenshaver is running. Stop it and verify KDE wallpaper/lock-screen restoration before uninstalling."
    fi
    initialize_sudo

    remove_kde_native_integration
    remove_kde_development_runtime
    remove_xfce_lock_integration
    remove_mate_lock_integration
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
    if screenshaver_process_running; then
        die "Screenshaver is running. Stop it and verify KDE wallpaper/lock-screen restoration before uninstalling."
    fi
    verify_nix_available
    remove_kde_development_runtime
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
