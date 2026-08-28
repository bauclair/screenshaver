{
  description = "Screenshaver — a shader-based screensaver for Linux";

  inputs = {
    # The exact nixpkgs revision is recorded in flake.lock.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      # Read the package version from Cargo.toml so release versions are
      # maintained in one authoritative location.
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      version = cargoToml.package.version;

      mkScreenshaver = pkgs:
        pkgs.rustPlatform.buildRustPackage {
          pname = "screenshaver";
          inherit version;

          # The flake source is the Cargo source tree.
          src = self;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            # SDL/OpenGL rendering
            SDL2
            SDL2_ttf
            libGL
            libglvnd

            # Native Wayland backend
            wayland
            wayland.dev

            # Native X11/GLX backend and SDL X11 support
            libx11
            libxcursor
            libxext
            libxi
            libxrandr
            libxscrnsaver
          ];

          # Cargo installs the executable. Install the Linux desktop entry and
          # icon theme files alongside it.
          postInstall = ''
            install -Dm644 \
              assets/screenshaver.desktop \
              "$out/share/applications/screenshaver.desktop"

            if [ -d assets/icons/hicolor ]; then
              mkdir -p "$out/share/icons/hicolor"
              cp -r assets/icons/hicolor/. "$out/share/icons/hicolor/"
            fi
          '';

          meta = with pkgs.lib; {
            description = "Feature-packed GLSL screensaver and viewer";
            longDescription = ''
              Screenshaver is a Linux screensaver written in Rust. It renders
              GLSL shaders through SDL2 and OpenGL and supports native Wayland
              and X11 wallpaper backends, multiple shader formats, and generated
              textures.
            '';
            homepage = "https://github.com/bauclair/screenshaver";
            license = licenses.gpl3Plus;
            mainProgram = "screenshaver";
            platforms = platforms.linux;
          };
        };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          screenshaver = mkScreenshaver pkgs;
        in
        {
          inherit screenshaver;
          default = screenshaver;
        }
      );

      apps = forAllSystems (
        system:
        let
          package = self.packages.${system}.screenshaver;
          app = {
            type = "app";
            program = "${package}/bin/screenshaver";
          };
        in
        {
          screenshaver = app;
          default = app;
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.screenshaver ];

            packages = with pkgs; [
              cargo
              rustc
              rustfmt
              clippy
              pkg-config

              # Required by clang-sys/bindgen when building the KDE renderer
              # secondary crate.
              llvmPackages.llvm
              llvmPackages.libclang

              # Required by pam-sys when building the KDE renderer
              # secondary crate. Provides PAM headers and libraries.
              linux-pam

              # Required when building the KDE renderer secondary crate.
              libpulseaudio
            ];

            LLVM_CONFIG_PATH = "${pkgs.llvmPackages.llvm}/bin/llvm-config";
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

            BINDGEN_EXTRA_CLANG_ARGS = ''
              -I${pkgs.lib.getDev pkgs.linux-pam}/include
              -I${pkgs.lib.getDev pkgs.glibc}/include
            '';

            shellHook = ''
              echo "Screenshaver development environment"
              echo "  cargo build"
              echo "  cargo test"
              echo "  cargo clippy"
              echo "  cargo fmt"
            '';
          };
        }
      );

      # `nix flake check` builds the same package exposed by `nix build`.
      checks = forAllSystems (system: {
        package = self.packages.${system}.screenshaver;
      });

      # Makes the package available as pkgs.screenshaver to flakes that import
      # this overlay.
      overlays.default = final: _prev: {
        screenshaver = mkScreenshaver final;
      };

      formatter = forAllSystems (
        system: nixpkgs.legacyPackages.${system}.nixfmt-rfc-style
      );
    };
}
