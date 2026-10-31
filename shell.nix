# Development shell for the Screenshaver application and KDE native renderer.
# Run from the Screenshaver repository root:
#   nix-shell --file shell.nix
{ pkgs ? import <nixpkgs> {} }:
let
  screenshaver = pkgs.callPackage ./default.nix {};
in
pkgs.mkShell {
  inputsFrom = [ screenshaver ];

  # libdbus-sys requires dbus-1.pc. The main package's declared inputs
  # do not currently include dbus explicitly.
  packages = [ pkgs.dbus ];

  shellHook = ''
    export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
    export LLVM_CONFIG_PATH="${pkgs.llvmPackages.llvm}/bin/llvm-config"
    export BINDGEN_EXTRA_CLANG_ARGS="-I${pkgs.pam}/include -I${pkgs.stdenv.cc.libc.dev}/include"

    echo "Screenshaver KDE development environment ready."
    echo "Check native dependencies: pkg-config --modversion wayland-client dbus-1"
  '';
}
