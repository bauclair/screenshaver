{ pkgs ? import <nixpkgs> {} }:

with pkgs;

mkShell {
  nativeBuildInputs = [
    pkg-config
    llvmPackages.llvm
    llvmPackages.libclang
    cmake
    rustc
    cargo
  ];

  buildInputs = [
    SDL2
    SDL2_ttf
    libGL
    libglvnd
    wayland
    libpulseaudio
    libxkbcommon
    pam
    xorg.libX11
    xorg.libXScrnSaver
    qt6.qtbase
    qt6.qtdeclarative
    libdecor
  ];

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
  LLVM_CONFIG_PATH = "${llvmPackages.llvm}/bin/llvm-config";

  BINDGEN_EXTRA_CLANG_ARGS =
    "-I${pam}/include -I${stdenv.cc.libc.dev}/include";
}