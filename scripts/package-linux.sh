#!/usr/bin/env bash

# Build and package Screenshaver as a versioned Linux archive.
#
# Output:
#   dist/screenshaver-<version>-linux-<architecture>.tar.gz
#   dist/screenshaver-<version>-linux-<architecture>.tar.gz.sha256
#
# Run from anywhere inside the repository:
#   ./scripts/package-linux.sh

set -Eeuo pipefail

PROGRAM_NAME="screenshaver"
DISPLAY_NAME="Screenshaver"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

CARGO_TOML="${PROJECT_DIR}/Cargo.toml"
TARGET_DIR="${PROJECT_DIR}/target"
DIST_DIR="${PROJECT_DIR}/dist"

readonly PROGRAM_NAME
readonly DISPLAY_NAME
readonly SCRIPT_DIR
readonly PROJECT_DIR
readonly CARGO_TOML
readonly TARGET_DIR
readonly DIST_DIR

log() {
    printf '\n\033[1;32m==>\033[0m %s\n' "$*"
}

warn() {
    printf '\n\033[1;33mWarning:\033[0m %s\n' "$*" >&2
}

fail() {
    printf '\n\033[1;31mError:\033[0m %s\n' "$*" >&2
    exit 1
}

cleanup_on_error() {
    local exit_code=$?

    if (( exit_code != 0 )); then
        printf '\n\033[1;31mPackaging failed with exit code %d.\033[0m\n' \
            "${exit_code}" >&2
    fi
}

trap cleanup_on_error EXIT

require_command() {
    local command_name="$1"

    command -v "${command_name}" >/dev/null 2>&1 ||
        fail "Required command not found: ${command_name}"
}

copy_required_file() {
    local source_path="$1"
    local destination_path="$2"

    [[ -f "${source_path}" ]] ||
        fail "Required file not found: ${source_path}"

    install -Dm644 "${source_path}" "${destination_path}"
}

copy_optional_file() {
    local source_path="$1"
    local destination_path="$2"

    if [[ -f "${source_path}" ]]; then
        install -Dm644 "${source_path}" "${destination_path}"
    else
        warn "Optional file not found: ${source_path}"
    fi
}

read_cargo_version() {
    # Reads the first package version declared in Cargo.toml.
    #
    # This avoids requiring an additional TOML parser. If Cargo.toml later
    # contains multiple package declarations or uses workspace inheritance,
    # replace this function with a cargo metadata query.

    local version

    version="$(
        awk '
            /^\[package\]/ {
                in_package = 1
                next
            }

            /^\[/ && in_package {
                exit
            }

            in_package && /^[[:space:]]*version[[:space:]]*=/ {
                line = $0
                sub(/^[^=]*=[[:space:]]*"/, "", line)
                sub(/".*$/, "", line)
                print line
                exit
            }
        ' "${CARGO_TOML}"
    )"

    [[ -n "${version}" ]] ||
        fail "Could not determine package version from Cargo.toml."

    printf '%s\n' "${version}"
}

detect_architecture() {
    case "$(uname -m)" in
        x86_64 | amd64)
            printf 'x86_64\n'
            ;;
        aarch64 | arm64)
            printf 'aarch64\n'
            ;;
        *)
            fail "Unsupported packaging architecture: $(uname -m)"
            ;;
    esac
}

create_installer() {
    local installer_path="$1"

    cat >"${installer_path}" <<'INSTALLER'
#!/usr/bin/env bash

# Install Screenshaver for the current user by default.
#
# Default installation:
#   ~/.local/bin/screenshaver
#   ~/.local/share/applications/screenshaver.desktop
#   ~/.local/share/icons/hicolor/...
#   ~/.local/share/doc/screenshaver/...
#
# System-wide installation:
#   sudo ./install.sh --prefix /usr/local

set -Eeuo pipefail

PREFIX="${HOME}/.local"

usage() {
    cat <<'EOF'
Usage:
  ./install.sh [--prefix DIRECTORY]

Options:
  --prefix DIRECTORY   Installation prefix.
                       Default: ~/.local

Examples:
  ./install.sh
  sudo ./install.sh --prefix /usr/local
EOF
}

while (($# > 0)); do
    case "$1" in
        --prefix)
            (($# >= 2)) || {
                printf 'Error: --prefix requires a directory.\n' >&2
                exit 2
            }

            PREFIX="$2"
            shift 2
            ;;

        --help | -h)
            usage
            exit 0
            ;;

        *)
            printf 'Error: unknown option: %s\n' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

PACKAGE_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

install -Dm755 \
    "${PACKAGE_DIR}/bin/screenshaver" \
    "${PREFIX}/bin/screenshaver"

if [[ -f "${PACKAGE_DIR}/share/applications/screenshaver.desktop" ]]; then
    install -Dm644 \
        "${PACKAGE_DIR}/share/applications/screenshaver.desktop" \
        "${PREFIX}/share/applications/screenshaver.desktop"
fi

if [[ -d "${PACKAGE_DIR}/share/icons" ]]; then
    mkdir -p "${PREFIX}/share/icons"
    cp -a \
        "${PACKAGE_DIR}/share/icons/." \
        "${PREFIX}/share/icons/"
fi

if [[ -d "${PACKAGE_DIR}/share/doc/screenshaver" ]]; then
    mkdir -p "${PREFIX}/share/doc/screenshaver"
    cp -a \
        "${PACKAGE_DIR}/share/doc/screenshaver/." \
        "${PREFIX}/share/doc/screenshaver/"
fi

printf '\nScreenshaver was installed under:\n  %s\n' "${PREFIX}"

if [[ ":${PATH}:" != *":${PREFIX}/bin:"* ]]; then
    printf '\nAdd this directory to PATH if necessary:\n  %s/bin\n' "${PREFIX}"
fi
INSTALLER

    chmod 755 "${installer_path}"
}

create_uninstaller() {
    local uninstaller_path="$1"

    cat >"${uninstaller_path}" <<'UNINSTALLER'
#!/usr/bin/env bash

# Remove files installed by this Screenshaver archive.
#
# Default:
#   ./uninstall.sh
#
# System-wide:
#   sudo ./uninstall.sh --prefix /usr/local

set -Eeuo pipefail

PREFIX="${HOME}/.local"

usage() {
    cat <<'EOF'
Usage:
  ./uninstall.sh [--prefix DIRECTORY]
EOF
}

while (($# > 0)); do
    case "$1" in
        --prefix)
            (($# >= 2)) || {
                printf 'Error: --prefix requires a directory.\n' >&2
                exit 2
            }

            PREFIX="$2"
            shift 2
            ;;

        --help | -h)
            usage
            exit 0
            ;;

        *)
            printf 'Error: unknown option: %s\n' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

rm -f "${PREFIX}/bin/screenshaver"
rm -f "${PREFIX}/share/applications/screenshaver.desktop"
rm -rf "${PREFIX}/share/doc/screenshaver"

for size in 16x16 22x22 24x24 32x32 48x48 64x64 128x128 256x256 512x512; do
    rm -f \
        "${PREFIX}/share/icons/hicolor/${size}/apps/screenshaver.png"
done

rm -f \
    "${PREFIX}/share/icons/hicolor/scalable/apps/screenshaver.svg"

printf 'Screenshaver was removed from %s.\n' "${PREFIX}"
UNINSTALLER

    chmod 755 "${uninstaller_path}"
}

main() {
    require_command awk
    require_command cargo
    require_command install
    require_command tar
    require_command sha256sum
    require_command uname

    [[ -f "${CARGO_TOML}" ]] ||
        fail "Cargo.toml not found at ${CARGO_TOML}"

    local version
    local architecture
    local package_name
    local staging_root
    local package_root
    local executable_path
    local archive_path
    local checksum_path

    version="$(read_cargo_version)"
    architecture="$(detect_architecture)"

    package_name="${PROGRAM_NAME}-${version}-linux-${architecture}"
    staging_root="${DIST_DIR}/staging"
    package_root="${staging_root}/${package_name}"
    executable_path="${TARGET_DIR}/release/${PROGRAM_NAME}"
    archive_path="${DIST_DIR}/${package_name}.tar.gz"
    checksum_path="${archive_path}.sha256"

    log "Packaging ${DISPLAY_NAME} ${version}"
    printf 'Project:      %s\n' "${PROJECT_DIR}"
    printf 'Architecture: %s\n' "${architecture}"
    printf 'Package:      %s\n' "${package_name}"

    log "Checking Rust formatting"
    (
        cd "${PROJECT_DIR}"
        cargo fmt --all --check
    )

    log "Running Rust tests"
    (
        cd "${PROJECT_DIR}"
        cargo test --locked
    )

    log "Building release executable"
    (
        cd "${PROJECT_DIR}"
        cargo build --release --locked
    )

    [[ -x "${executable_path}" ]] ||
        fail "Release executable was not created: ${executable_path}"

    log "Preparing staging directory"
    rm -rf "${staging_root}"
    mkdir -p "${package_root}"

    install -Dm755 \
        "${executable_path}" \
        "${package_root}/bin/${PROGRAM_NAME}"

    log "Copying application metadata"

    copy_optional_file \
        "${PROJECT_DIR}/assets/screenshaver.desktop" \
        "${package_root}/share/applications/screenshaver.desktop"

    if [[ -d "${PROJECT_DIR}/assets/icons/hicolor" ]]; then
        mkdir -p "${package_root}/share/icons/hicolor"
        cp -a \
            "${PROJECT_DIR}/assets/icons/hicolor/." \
            "${package_root}/share/icons/hicolor/"
    else
        warn "Icon directory not found: assets/icons/hicolor"
    fi

    log "Copying documentation"

    copy_required_file \
        "${PROJECT_DIR}/README.md" \
        "${package_root}/share/doc/${PROGRAM_NAME}/README.md"

    copy_required_file \
        "${PROJECT_DIR}/LICENSE" \
        "${package_root}/share/doc/${PROGRAM_NAME}/LICENSE"

    copy_optional_file \
        "${PROJECT_DIR}/CHANGELOG.md" \
        "${package_root}/share/doc/${PROGRAM_NAME}/CHANGELOG.md"

    copy_optional_file \
        "${PROJECT_DIR}/SCREENSHAVER_USER_MANUAL.md" \
        "${package_root}/share/doc/${PROGRAM_NAME}/SCREENSHAVER_USER_MANUAL.md"

    copy_optional_file \
        "${PROJECT_DIR}/docs/user-manual/README.md" \
        "${package_root}/share/doc/${PROGRAM_NAME}/USER_MANUAL.md"

    create_installer "${package_root}/install.sh"
    create_uninstaller "${package_root}/uninstall.sh"

    log "Recording package information"

    {
        printf 'Name: %s\n' "${DISPLAY_NAME}"
        printf 'Version: %s\n' "${version}"
        printf 'Architecture: %s\n' "${architecture}"
        printf 'Built: %s\n' "$(date --utc '+%Y-%m-%dT%H:%M:%SZ')"
        printf 'Rust compiler: %s\n' "$(rustc --version)"
        printf 'Cargo: %s\n' "$(cargo --version)"
    } >"${package_root}/BUILD-INFO.txt"

    log "Reviewing runtime library dependencies"

    if command -v ldd >/dev/null 2>&1; then
        ldd "${package_root}/bin/${PROGRAM_NAME}" |
            tee "${package_root}/RUNTIME-DEPENDENCIES.txt"
    else
        warn "ldd is unavailable; runtime dependencies were not recorded."
    fi

    log "Creating compressed archive"

    rm -f "${archive_path}" "${checksum_path}"

    tar \
        --create \
        --gzip \
        --file "${archive_path}" \
        --directory "${staging_root}" \
        "${package_name}"

    (
        cd "${DIST_DIR}"
        sha256sum "$(basename "${archive_path}")" \
            >"$(basename "${checksum_path}")"
    )

    log "Verifying archive"

    tar --list --file "${archive_path}"

    log "Package completed successfully"

    printf '\nArchive:\n  %s\n' "${archive_path}"
    printf '\nChecksum:\n  %s\n' "${checksum_path}"
    printf '\nSHA-256:\n  '
    cut -d' ' -f1 "${checksum_path}"

    if [[ -f /etc/NIXOS ]]; then
        warn \
            "This archive was built on NixOS. Test the binary on clean " \
            "non-NixOS systems before advertising it as a generic Linux build."
    fi
}

main "$@"
