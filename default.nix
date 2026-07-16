{ lib
, rustPlatform
, pkg-config
, SDL2
, SDL2_ttf
, libGL
}:

rustPlatform.buildRustPackage rec {
  pname = "screenshaver";
  version = "0.2.0";

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
  ];

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
    description = "ShaderToy-style GLSL screensaver";
    license = licenses.gpl3Plus;
    platforms = platforms.linux;
    # homepage = "https://github.com/bauclair/screenshaver";  (later)
    # mainProgram = "screenshaver";                           (later)
  };
}
