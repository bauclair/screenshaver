{ lib
, rustPlatform
, stdenv
, pkg-config
, llvmPackages
, SDL2
, SDL2_ttf
, libGL
, libglvnd
, wayland
, libpulseaudio
, libxkbcommon
, pam
, xorg
, cmake
, qt6
}:

rustPlatform.buildRustPackage rec {
  pname = "screenshaver";
  version = "0.5.3";

  src = ./.;

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
    wayland
    wayland.dev
    libpulseaudio
    libxkbcommon
    pam
    xorg.libX11
    xorg.libXScrnSaver
    qt6.qtbase
    qt6.qtdeclarative
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
      kde-host/build-release/qml/ScreenshaverNativeGL/libScreenshaverNativeGLPlugin.so \
      $out/lib/screenshaver/kde/libScreenshaverNativeGLPlugin.so

    install -Dm755 \
      kde-renderer/target/release/libscreenshaver.so \
      $out/lib/screenshaver/kde/libscreenshaver.so

    install -Dm644 \
      kde-host/qmldir \
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
