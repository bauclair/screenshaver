{ lib
, rustPlatform
, stdenv
, pkg-config
, llvmPackages
, SDL2
, SDL2_ttf
, libGL
, libglvnd
, libdecor
, wayland
, libpulseaudio
, libxkbcommon
, pam
, xorg
, cmake
, qt6
, kdePackages
}:

rustPlatform.buildRustPackage rec {
  pname = "screenshaver";
  version = "0.5.5";

  src = ./.;

  buildFeatures = [ "libdecor-bridge" ];

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    llvmPackages.llvm
    llvmPackages.libclang
    cmake
    qt6.wrapQtAppsHook
  ];

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
  LLVM_CONFIG_PATH = "${llvmPackages.llvm}/bin/llvm-config";

  BINDGEN_EXTRA_CLANG_ARGS = "-I${pam}/include -I${stdenv.cc.libc.dev}/include";

  buildInputs = [
    SDL2
    SDL2_ttf
    libGL
    libglvnd
    libdecor
    wayland
    wayland.dev
    libpulseaudio
    libxkbcommon
    pam
    xorg.libX11
    xorg.libXScrnSaver
    qt6.qtbase
    qt6.qtdeclarative
    kdePackages.libkscreen
  ];



preBuild = ''
  echo "=== WAYLAND PKG-CONFIG DIAGNOSTICS ==="
  echo "PKG_CONFIG_PATH=$PKG_CONFIG_PATH"
  find ${wayland.dev} -name 'wayland-client.pc' -print
  pkg-config --modversion wayland-client
  pkg-config --libs wayland-client
  echo "=== END DIAGNOSTICS ==="
'';


  postBuild = ''
    echo "=== Building KDE FrameRenderEngine bridge ==="

    # Nix vendors crates from the repository-root Cargo.lock.  Do not allow
    # kde-renderer's development lock file to request versions outside that
    # vendor set during the production package build.
    rm -f kde-renderer/Cargo.lock

    cargo build \
      --offline \
      --manifest-path kde-renderer/Cargo.toml \
      --release

    echo "=== Building independent KDE wallpaper producer ==="
    # The root lockfile is the Nix-vendored dependency set.
    rm -f kde-wallpaper-producer/Cargo.lock
    cargo build \
      --offline \
      --manifest-path kde-wallpaper-producer/Cargo.toml \
      --release

    echo "=== Building KDE Qt Quick native host ==="
    cmake \
      -S kde-host \
      -B kde-host/build-release \
      -DCMAKE_BUILD_TYPE=Release \
      -DSCREENSHAVER_RENDERER_LIBRARY="$PWD/kde-renderer/target/release/libscreenshaver.so"

    cmake --build kde-host/build-release --parallel "$NIX_BUILD_CORES"
  '';


  postInstall = ''
    install -Dm755 \
      kde-wallpaper-producer/target/release/screenshaver-plasma-producer \
      $out/bin/screenshaver-plasma-producer

    # Keep the discovery executable beside its shared KScreen bridge.
    install -Dm755 \
      kde-host/build-release/libScreenshaverKScreen.so \
      $out/lib/screenshaver/kde/libScreenshaverKScreen.so

    install -Dm755 \
      kde-host/build-release/screenshaver-kscreen-discover \
      $out/lib/screenshaver/kde/screenshaver-kscreen-discover

    # The producer uses the same immutable package as the main executable.
    install -Dm755 \
      kde-renderer/target/release/libscreenshaver.so \
      $out/lib/screenshaver/kde/libscreenshaver.so

    # Install the complete Plasma wallpaper package, including a locally
    # importable QML module. The native plugin's $ORIGIN resolves its renderer.
    wallpaper_root="$out/share/plasma/wallpapers/org.screenshaver.wallpaper"
    qml_root="$wallpaper_root/contents/ui/ScreenshaverNativeGL"

    install -Dm644 \
      kde-host/screenshaver-plasma-wallpaper/metadata.json \
      "$wallpaper_root/metadata.json"

    install -Dm644 \
      kde-host/screenshaver-plasma-wallpaper/contents/ui/main.qml \
      "$wallpaper_root/contents/ui/main.qml"

    install -Dm755 \
      kde-host/build-release/qml/ScreenshaverNativeGL/libScreenshaverNativeGLPlugin.so \
      "$qml_root/libScreenshaverNativeGLPlugin.so"

    install -Dm755 \
      kde-renderer/target/release/libscreenshaver.so \
      "$qml_root/libscreenshaver.so"

    install -Dm644 \
      kde-host/build-release/qml/ScreenshaverNativeGL/qmldir \
      "$qml_root/qmldir"

    # Preserve the private library layout used by the existing KDE lock path.
    install -Dm755 \
      kde-host/build-release/qml/ScreenshaverNativeGL/libScreenshaverNativeGLPlugin.so \
      $out/lib/screenshaver/kde/libScreenshaverNativeGLPlugin.so

    install -Dm644 \
      kde-host/build-release/qml/ScreenshaverNativeGL/qmldir \
      $out/lib/screenshaver/kde/qmldir

    install -Dm644 \
      assets/screenshaver.desktop \
      $out/share/applications/screenshaver.desktop

    for size in 16 22 24 32 48 64 96 128 192 256 512; do
      install -Dm644 \
        assets/icons/hicolor/''${size}x''${size}/apps/screenshaver.png \
        $out/share/icons/hicolor/''${size}x''${size}/apps/screenshaver.png
    done

    install -Dm644 \
      assets/icons/hicolor/scalable/apps/screenshaver.svg \
      $out/share/icons/hicolor/scalable/apps/screenshaver.svg
  '';

  meta = with lib; {
    description = "Feature-packed GLSL screensaver and viewer";
    license = licenses.gpl3Plus;
    platforms = platforms.linux;
    # homepage = "https://github.com/bauclair/screenshaver";  (later)
    # mainProgram = "screenshaver";                           (later)
  };
}
