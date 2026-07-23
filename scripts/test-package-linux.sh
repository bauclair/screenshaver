#!/usr/bin/env bash

# Screenshaver Linux release-package test script
#
# This script packages the version of Screenshaver installed in the current
# NixOS system profile. It does not compile Screenshaver directly with Cargo.
#
# Expected workflow:
#
#   1. Update Cargo.toml and default.nix to the new version.
#   2. Rebuild NixOS:
#
#        sudo nixos-rebuild switch --flake /etc/nixos
#
#   3. Run this script:
#
#        ./test-package-linux.sh
#
# Generated files are placed under:
#
#   dist/
#
# Exit immediately when a command fails, an undefined variable is used,
# or a command in a pipeline fails.
set -Eeuo pipefail

# Use predictable word splitting.
IFS=$'\n\t'


# -----------------------------------------------------------------------------
# Output helpers
# -----------------------------------------------------------------------------

info() {
    printf '%s\n' "$*"
}

success() {
    printf '  OK: %s\n' "$*"
}

warning() {
    printf '  WARNING: %s\n' "$*" >&2
}

error() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}


# -----------------------------------------------------------------------------
# Copy helpers
# -----------------------------------------------------------------------------

copy_required_file() {
    local source_path="$1"
    local destination_path="$2"

    if [[ ! -f "$source_path" ]]; then
        error "Required file not found: $source_path"
    fi

    mkdir -p "$(dirname "$destination_path")"
    cp -p "$source_path" "$destination_path"

    success "Copied $(basename "$source_path")"
}

copy_optional_file() {
    local source_path="$1"
    local destination_path="$2"

    if [[ ! -f "$source_path" ]]; then
        warning "Optional file not found; skipping: $source_path"
        return 0
    fi

    mkdir -p "$(dirname "$destination_path")"
    cp -p "$source_path" "$destination_path"

    success "Copied $(basename "$source_path")"
}

copy_required_directory() {
    local source_path="$1"
    local destination_path="$2"

    if [[ ! -d "$source_path" ]]; then
        error "Required directory not found: $source_path"
    fi

    mkdir -p "$(dirname "$destination_path")"
    cp -a "$source_path" "$destination_path"

    success "Copied directory $(basename "$source_path")"
}

copy_optional_directory() {
    local source_path="$1"
    local destination_path="$2"

    if [[ ! -d "$source_path" ]]; then
        warning "Optional directory not found; skipping: $source_path"
        return 0
    fi

    mkdir -p "$(dirname "$destination_path")"
    cp -a "$source_path" "$destination_path"

    success "Copied directory $(basename "$source_path")"
}


# -----------------------------------------------------------------------------
# Determine project location
# -----------------------------------------------------------------------------

# This allows the script to be run from any current working directory.
SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
    pwd
)"

PROJECT_ROOT="$SCRIPT_DIR"
CARGO_TOML="$PROJECT_ROOT/Cargo.toml"

info "Project root: $PROJECT_ROOT"

if [[ ! -f "$CARGO_TOML" ]]; then
    error "Cargo.toml was not found at: $CARGO_TOML"
fi


# -----------------------------------------------------------------------------
# Read the Screenshaver version from Cargo.toml
# -----------------------------------------------------------------------------

# This reads the first version assignment in Cargo.toml.
VERSION="$(
    awk '
        /^[[:space:]]*version[[:space:]]*=/ {
            line = $0
            sub(/^[^"]*"/, "", line)
            sub(/".*$/, "", line)
            print line
            exit
        }
    ' "$CARGO_TOML"
)"

if [[ -z "$VERSION" ]]; then
    error "Could not determine the Screenshaver version from Cargo.toml."
fi

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]]; then
    error "Cargo.toml contains an unexpected version value: $VERSION"
fi

info "Screenshaver version: $VERSION"


# -----------------------------------------------------------------------------
# Package naming
# -----------------------------------------------------------------------------

MACHINE_ARCH="$(uname -m)"

case "$MACHINE_ARCH" in
    x86_64)
        PACKAGE_ARCH="x86_64"
        ;;

    aarch64 | arm64)
        PACKAGE_ARCH="aarch64"
        ;;

    *)
        error "Unsupported package architecture: $MACHINE_ARCH"
        ;;
esac

PACKAGE_NAME="screenshaver-${VERSION}-linux-${PACKAGE_ARCH}"

DIST_DIR="$PROJECT_ROOT/dist"
STAGING_DIR="$DIST_DIR/$PACKAGE_NAME"

ARCHIVE_NAME="${PACKAGE_NAME}.tar.gz"
ARCHIVE_PATH="$DIST_DIR/$ARCHIVE_NAME"
CHECKSUM_PATH="${ARCHIVE_PATH}.sha256"


# -----------------------------------------------------------------------------
# Check commands used by this script
# -----------------------------------------------------------------------------

info
info "Checking required commands..."

REQUIRED_COMMANDS=(
    awk
    basename
    cat
    cp
    date
    dirname
    find
    grep
    mkdir
    pwd
    readlink
    rm
    sha256sum
    sort
    tar
    uname
)

for command_name in "${REQUIRED_COMMANDS[@]}"; do
    if ! command -v "$command_name" >/dev/null 2>&1; then
        error "Required command not found: $command_name"
    fi

    success "FOUND: $command_name"
done

info
info "All preliminary checks passed."


# -----------------------------------------------------------------------------
# Locate and validate the installed binary
# -----------------------------------------------------------------------------

info
info "Locating installed Screenshaver binary..."

BINARY_PATH="$(command -v screenshaver || true)"

if [[ -z "$BINARY_PATH" ]]; then
    error "Screenshaver is not installed or is not available in PATH.

Run:

  sudo nixos-rebuild switch --flake /etc/nixos

and then run this script again."
fi

if [[ ! -x "$BINARY_PATH" ]]; then
    error "Installed Screenshaver binary is not executable: $BINARY_PATH"
fi

INSTALLED_TARGET="$(readlink -f "$BINARY_PATH")"

if [[ ! -f "$INSTALLED_TARGET" ]]; then
    error "Could not resolve the installed Screenshaver executable."
fi

if [[ ! -x "$INSTALLED_TARGET" ]]; then
    error "Resolved Screenshaver binary is not executable: $INSTALLED_TARGET"
fi

info
info "Installed Screenshaver binary found."
info "Binary: $BINARY_PATH"
ls -l "$BINARY_PATH"

INSTALLED_VERSION=""

if [[ "$INSTALLED_TARGET" =~ screenshaver-([0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?)/bin/screenshaver$ ]]; then
    INSTALLED_VERSION="${BASH_REMATCH[1]}"
fi

if [[ -z "$INSTALLED_VERSION" ]]; then
    error "Could not determine the installed version from:

  $INSTALLED_TARGET"
fi

info "Installed version: $INSTALLED_VERSION"

if [[ "$INSTALLED_VERSION" != "$VERSION" ]]; then
    error "Version mismatch.

Cargo.toml version:  $VERSION
Installed version:   $INSTALLED_VERSION
Installed executable: $INSTALLED_TARGET

Run:

  sudo nixos-rebuild switch --flake /etc/nixos

and then run this script again."
fi

success "Installed version matches Cargo.toml"


# -----------------------------------------------------------------------------
# Prepare the staging directory
# -----------------------------------------------------------------------------

info
info "Creating package staging directory..."

# Remove only the directory for this exact package version and architecture.
rm -rf -- "$STAGING_DIR"

mkdir -p "$STAGING_DIR/bin"
mkdir -p "$STAGING_DIR/share/applications"
mkdir -p "$STAGING_DIR/share/icons"
mkdir -p "$STAGING_DIR/docs"
mkdir -p "$STAGING_DIR/examples"

success "Created $STAGING_DIR"


# -----------------------------------------------------------------------------
# Copy the executable
# -----------------------------------------------------------------------------

info
info "Copying Screenshaver executable..."

# -L dereferences /run/current-system/sw/bin/screenshaver so that the package
# contains a regular executable rather than a symlink into /nix/store.
cp -Lp "$BINARY_PATH" "$STAGING_DIR/bin/screenshaver"

if [[ ! -f "$STAGING_DIR/bin/screenshaver" ]]; then
    error "Screenshaver binary was not copied into the staging directory."
fi

if [[ -L "$STAGING_DIR/bin/screenshaver" ]]; then
    error "Staged Screenshaver executable is still a symbolic link."
fi

if [[ ! -x "$STAGING_DIR/bin/screenshaver" ]]; then
    error "Staged Screenshaver binary is not executable."
fi

success "Copied executable"
ls -lh "$STAGING_DIR/bin/screenshaver"


# -----------------------------------------------------------------------------
# Create the generated VERSION file
# -----------------------------------------------------------------------------

info
info "Creating VERSION file..."

VERSION_FILE="$STAGING_DIR/VERSION"
BUILD_DATE="$(date -u +%Y-%m-%d)"
BUILD_TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

cat > "$VERSION_FILE" <<EOF
Screenshaver $VERSION
Build date: $BUILD_DATE
Build timestamp: $BUILD_TIMESTAMP
Operating system: Linux
Architecture: $PACKAGE_ARCH
Package: $PACKAGE_NAME
EOF

if [[ ! -s "$VERSION_FILE" ]]; then
    error "VERSION file was not created or is empty."
fi

success "Created VERSION"
cat "$VERSION_FILE"


# -----------------------------------------------------------------------------
# Copy top-level release files
# -----------------------------------------------------------------------------

info
info "Copying top-level package files..."

# Mark files as required only when every release must contain them.
copy_required_file \
    "$PROJECT_ROOT/README.md" \
    "$STAGING_DIR/README.md"

copy_required_file \
    "$PROJECT_ROOT/LICENSE" \
    "$STAGING_DIR/LICENSE"

# These are examples of optional top-level files. Delete or add entries as
# appropriate for the Screenshaver repository.
copy_optional_file \
    "$PROJECT_ROOT/CHANGELOG.md" \
    "$STAGING_DIR/CHANGELOG.md"

copy_optional_file \
    "$PROJECT_ROOT/CONTRIBUTING.md" \
    "$STAGING_DIR/CONTRIBUTING.md"

copy_optional_file \
    "$PROJECT_ROOT/SCREENSHAVER_USER_MANUAL.md" \
    "$STAGING_DIR/SCREENSHAVER_USER_MANUAL.md"

copy_optional_file \
    "$PROJECT_ROOT/INSTALL.md" \
    "$STAGING_DIR/INSTALL.md"


# -----------------------------------------------------------------------------
# Copy desktop integration files
# -----------------------------------------------------------------------------

info
info "Copying desktop integration files..."

# The desktop file is placed in the conventional share/applications location.
copy_optional_file \
    "$PROJECT_ROOT/assets/screenshaver.desktop" \
    "$STAGING_DIR/share/applications/screenshaver.desktop"

# Preserve the complete packaged icon hierarchy beneath share/icons.
#
# For example:
#
#   assets/icons/hicolor/16x16/apps/screenshaver.png
#   assets/icons/hicolor/32x32/apps/screenshaver.png
#   assets/icons/hicolor/scalable/apps/screenshaver.svg
#
copy_optional_directory \
    "$PROJECT_ROOT/assets/icons" \
    "$STAGING_DIR/share/icons/screenshaver"


# -----------------------------------------------------------------------------
# Copy other runtime assets
# -----------------------------------------------------------------------------

info
info "Copying additional runtime assets..."

# Copy individual runtime assets here when they are needed by the installed
# application or useful to end users.
copy_optional_file \
    "$PROJECT_ROOT/assets/screenshaver-splash.png" \
    "$STAGING_DIR/share/screenshaver/screenshaver-splash.png"

# Uncomment this block if the entire assets directory should be distributed
# exactly as it appears in the repository.
#
# copy_optional_directory \
#     "$PROJECT_ROOT/assets" \
#     "$STAGING_DIR/assets"


# -----------------------------------------------------------------------------
# Copy documentation
# -----------------------------------------------------------------------------

info
info "Copying documentation..."

# This copies the entire docs directory. You can replace this with individual
# copy_optional_file calls if the release should contain only user-facing
# documentation.
copy_optional_directory \
    "$PROJECT_ROOT/docs" \
    "$STAGING_DIR/docs/screenshaver"


# -----------------------------------------------------------------------------
# Copy example shaders or configurations
# -----------------------------------------------------------------------------

info
info "Copying examples and configuration templates..."

# Adjust these paths to match files that actually exist in the repository.
# Missing optional files produce warnings but do not stop packaging.

copy_optional_directory \
    "$PROJECT_ROOT/examples" \
    "$STAGING_DIR/examples/screenshaver"

copy_optional_directory \
    "$PROJECT_ROOT/shaders" \
    "$STAGING_DIR/examples/shaders"

copy_optional_directory \
    "$PROJECT_ROOT/config" \
    "$STAGING_DIR/examples/config"

# Individual examples can be copied instead:
#
# copy_optional_file \
#     "$PROJECT_ROOT/default.glsl" \
#     "$STAGING_DIR/examples/shaders/default.glsl"
#
# copy_optional_file \
#     "$PROJECT_ROOT/screenshaver.conf.example" \
#     "$STAGING_DIR/examples/config/screenshaver.conf"


# -----------------------------------------------------------------------------
# Remove empty optional directories
# -----------------------------------------------------------------------------

info
info "Removing empty staging directories..."

find "$STAGING_DIR" -depth -type d -empty -delete

success "Removed empty directories"


# -----------------------------------------------------------------------------
# Display and validate the staging tree
# -----------------------------------------------------------------------------

info
info "Validating package staging directory..."

if [[ ! -d "$STAGING_DIR" ]]; then
    error "Staging directory does not exist."
fi

if [[ ! -x "$STAGING_DIR/bin/screenshaver" ]]; then
    error "Staged package does not contain an executable Screenshaver binary."
fi

if [[ ! -s "$STAGING_DIR/VERSION" ]]; then
    error "Staged package does not contain a valid VERSION file."
fi

if [[ ! -s "$STAGING_DIR/README.md" ]]; then
    error "Staged package does not contain README.md."
fi

if [[ ! -s "$STAGING_DIR/LICENSE" ]]; then
    error "Staged package does not contain LICENSE."
fi

success "Staging directory passed validation"

info
info "Package contents:"

(
    cd "$DIST_DIR"
    find "$PACKAGE_NAME" -print | sort
)


# -----------------------------------------------------------------------------
# Create the compressed archive
# -----------------------------------------------------------------------------

info
info "Creating release archive..."

rm -f -- "$ARCHIVE_PATH"
rm -f -- "$CHECKSUM_PATH"

# Running tar from DIST_DIR ensures that the archive contains one clean,
# top-level package directory rather than absolute filesystem paths.
tar \
    --create \
    --gzip \
    --file "$ARCHIVE_PATH" \
    --directory "$DIST_DIR" \
    "$PACKAGE_NAME"

if [[ ! -s "$ARCHIVE_PATH" ]]; then
    error "Release archive was not created or is empty."
fi

success "Created $ARCHIVE_NAME"
ls -lh "$ARCHIVE_PATH"


# -----------------------------------------------------------------------------
# Validate the archive
# -----------------------------------------------------------------------------

info
info "Validating release archive..."

if ! tar -tzf "$ARCHIVE_PATH" >/dev/null; then
    error "The generated release archive failed tar validation."
fi

if ! tar -tzf "$ARCHIVE_PATH" |
    grep -Fxq "$PACKAGE_NAME/bin/screenshaver"; then
    error "The generated archive does not contain the Screenshaver executable."
fi

if ! tar -tzf "$ARCHIVE_PATH" |
    grep -Fxq "$PACKAGE_NAME/VERSION"; then
    error "The generated archive does not contain the VERSION file."
fi

if ! tar -tzf "$ARCHIVE_PATH" |
    grep -Fxq "$PACKAGE_NAME/README.md"; then
    error "The generated archive does not contain README.md."
fi

if ! tar -tzf "$ARCHIVE_PATH" |
    grep -Fxq "$PACKAGE_NAME/LICENSE"; then
    error "The generated archive does not contain LICENSE."
fi

success "Archive passed validation"


# -----------------------------------------------------------------------------
# Generate the SHA-256 checksum
# -----------------------------------------------------------------------------

info
info "Generating SHA-256 checksum..."

(
    cd "$DIST_DIR"
    sha256sum "$ARCHIVE_NAME" > "$(basename "$CHECKSUM_PATH")"
)

if [[ ! -s "$CHECKSUM_PATH" ]]; then
    error "Checksum file was not created or is empty."
fi

success "Created $(basename "$CHECKSUM_PATH")"
cat "$CHECKSUM_PATH"


# -----------------------------------------------------------------------------
# Final result
# -----------------------------------------------------------------------------

info
info "Screenshaver Linux package created successfully."
info
info "Staging directory:"
info "  $STAGING_DIR"
info
info "Release archive:"
info "  $ARCHIVE_PATH"
info
info "SHA-256 checksum:"
info "  $CHECKSUM_PATH"
info
info "Package version:"
info "  $VERSION"
info
info "Installed source binary:"
info "  $INSTALLED_TARGET"
