{ lib
, rustPlatform
, pkg-config
, SDL2
, SDL2_ttf
, libGL
, libglvnd
, wayland
, xorg
}:

rustPlatform.buildRustPackage rec {
  pname = "screenshaver";
  version = "0.4.5";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    SDL2
    SDL2_ttf
    libGL
    libglvnd
    wayland
    wayland.dev
    xorg.libX11
    xorg.libXScrnSaver
  ];



preBuild = ''
  echo "=== WAYLAND PKG-CONFIG DIAGNOSTICS ==="
  echo "PKG_CONFIG_PATH=$PKG_CONFIG_PATH"
  find ${wayland.dev} -name 'wayland-client.pc' -print
  pkg-config --modversion wayland-client
  pkg-config --libs wayland-client
  echo "=== END DIAGNOSTICS ==="
'';



  postInstall = ''
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
