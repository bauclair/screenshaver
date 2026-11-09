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
# Copy only into the staging tree. Never overwrite a live mapped library.
cp -- "$RENDERER_LIBRARY" "${PLUGIN_DIR}/libscreenshaver.so"

echo
echo "Screenshaver KDE host STAGING build completed (not deployed):"
echo "  KScreen discovery: ${BUILD_DIR}/libScreenshaverKScreen.so"
echo "  ${PLUGIN_DIR}"
echo
echo "Contents:"
ls -lh "$PLUGIN_DIR"

echo
echo "The active build-${PROFILE} directory was not modified."
echo "Do not repoint the live QML import or copy staged libraries into"
echo "a directory currently mapped by plasmashell."
