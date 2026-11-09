#!/usr/bin/env bash
set -euo pipefail

PROFILE="${1:-debug}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd -- "${2:-${SCRIPT_DIR}/..}" && pwd)"

case "$PROFILE" in
    debug)
        CMAKE_BUILD_TYPE=Debug
        ;;
    release)
        CMAKE_BUILD_TYPE=Release
        ;;
    *)
        echo "Usage: $0 [debug|release] [screenshaver-project-root]" >&2
        exit 2
        ;;
esac

RENDERER_LIBRARY="${PROJECT_ROOT}/kde-renderer/target/${PROFILE}/libscreenshaver.so"
# Never build into build-debug/build-release: Plasma may have those
# shared libraries mapped. This staging tree is not used by the live QML import.
BUILD_DIR="${SCRIPT_DIR}/build-staging-${PROFILE}"

if [[ ! -f "$RENDERER_LIBRARY" ]]; then
    echo "ERROR: Screenshaver KDE renderer library not found:" >&2
    echo "       $RENDERER_LIBRARY" >&2
    echo >&2
    echo "Build it first with:" >&2
    if [[ "$PROFILE" == "release" ]]; then
        echo "  cargo build --release --manifest-path ${PROJECT_ROOT}/kde-renderer/Cargo.toml" >&2
    else
        echo "  cargo build --manifest-path ${PROJECT_ROOT}/kde-renderer/Cargo.toml" >&2
    fi
    exit 1
fi

cmake -S "$SCRIPT_DIR" -B "$BUILD_DIR" \
    -DCMAKE_BUILD_TYPE="$CMAKE_BUILD_TYPE" \
    -DSCREENSHAVER_RENDERER_LIBRARY="$RENDERER_LIBRARY"

cmake --build "$BUILD_DIR" --parallel

PLUGIN_DIR="${BUILD_DIR}/qml/ScreenshaverNativeGL"
PLUGIN="${PLUGIN_DIR}/libScreenshaverNativeGLPlugin.so"

if [[ ! -f "$PLUGIN" ]]; then
    echo "ERROR: KDE host build did not produce $PLUGIN" >&2
    exit 1
fi

# Keep the renderer beside the plugin so $ORIGIN resolves the runtime library.
# The staging tree is never used by live Plasma processes.
cp -- "$RENDERER_LIBRARY" "${PLUGIN_DIR}/libscreenshaver.so"

# Publish a lock-screen-only runtime generation. Do not change the live
# plasmashell QML import or overwrite any mapped shared library. The Rust
# lock-screen overlay installer reads the 'current' generation on startup.
# A new directory and atomic symlink switch make deployment deterministic.
DATA_HOME="${XDG_DATA_HOME:-${HOME}/.local/share}"
LOCK_RUNTIME_ROOT="${DATA_HOME}/screenshaver/kde-lock-runtime"
mkdir -p -- "$LOCK_RUNTIME_ROOT"
GENERATION_DIR="$(mktemp -d "${LOCK_RUNTIME_ROOT}/.generation-XXXXXXXX")"
NEW_LINK="${LOCK_RUNTIME_ROOT}/.current-$$"
cleanup_deployment() {
    rm -f -- "$NEW_LINK"
    if [[ -n "${GENERATION_DIR:-}" && -d "$GENERATION_DIR" ]]; then
        rm -rf -- "$GENERATION_DIR"
    fi
}
trap cleanup_deployment EXIT
for filename in libScreenshaverNativeGLPlugin.so libscreenshaver.so qmldir; do
    if [[ ! -f "${PLUGIN_DIR}/${filename}" ]]; then
        echo "ERROR: Missing KDE lock runtime asset: ${PLUGIN_DIR}/${filename}" >&2
        exit 1
    fi
    cp -- "${PLUGIN_DIR}/${filename}" "${GENERATION_DIR}/${filename}"
done
printf '%s\n' "$PROFILE" > "${GENERATION_DIR}/build-profile"
ln -s -- "$(basename -- "$GENERATION_DIR")" "$NEW_LINK"
mv -Tf -- "$NEW_LINK" "${LOCK_RUNTIME_ROOT}/current"
# Preserve the published generation; older generations remain safe for any
# already-running greeter and can be removed during a later maintenance pass.
PUBLISHED_GENERATION="$GENERATION_DIR"
GENERATION_DIR=""

echo
echo "Screenshaver KDE host staging build and lock-screen deployment completed:"
echo "  KScreen discovery: ${BUILD_DIR}/libScreenshaverKScreen.so"
echo "  ${PLUGIN_DIR}"
echo
echo "Contents:"
ls -lh "$PLUGIN_DIR"

echo
echo "The active build-${PROFILE} directory was not modified."
echo "Do not repoint the live QML import or copy staged libraries into"
echo "a directory currently mapped by plasmashell."

echo
echo "Lock-screen runtime generation: ${PUBLISHED_GENERATION}"
echo "Lock-screen runtime selection: ${LOCK_RUNTIME_ROOT}/current"
echo "The next Screenshaver start will copy this generation into its temporary lock overlay."
