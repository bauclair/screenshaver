# NixOS Installation Guide for Screenshaver

This guide explains how to install, build, run, and develop
**Screenshaver** on **NixOS** using the project's `flake.nix`.

Repository:

``` text
https://github.com/bauclair/screenshaver
```

## Prerequisites

-   NixOS 26.05 or newer recommended
-   Internet access
-   Git installed

Install Git if needed:

``` bash
sudo nix-shell -p git
```

## Enable Flakes

Edit `/etc/nixos/configuration.nix`:

``` nix
{
  nix.settings.experimental-features = [ "nix-command" "flakes" ];
}
```

Rebuild:

``` bash
sudo nixos-rebuild switch
```

Verify:

``` bash
nix --version
```

## Clone the repository

``` bash
git clone https://github.com/bauclair/screenshaver.git
cd screenshaver
```

## Build

Create/update the lock file:

``` bash
nix flake lock
```

Compile:

``` bash
nix build
```

The executable will appear at:

``` text
./result/bin/screenshaver
```

## Run

``` bash
nix run
```

or

``` bash
./result/bin/screenshaver
```

## Development Shell

``` bash
nix develop
```

This provides Cargo, Rust, pkg-config, SDL2, SDL2_ttf, OpenGL, and X11
development libraries.

Build inside the shell:

``` bash
cargo build
cargo run
```

## Verify the Flake

``` bash
nix flake check
```

## Install into your Profile

``` bash
nix profile install .
```

or directly from GitHub:

``` bash
nix profile install github:bauclair/screenshaver
```

## Build Directly from GitHub

Without cloning:

``` bash
nix build github:bauclair/screenshaver
```

Run:

``` bash
nix run github:bauclair/screenshaver
```

## Updating

``` bash
git pull
nix flake update
nix build
```

## Adding to a NixOS Configuration

You may reference the repository as a flake input:

``` nix
inputs.screenshaver.url = "github:bauclair/screenshaver";
```

## Uninstall

``` bash
nix profile remove screenshaver
```

or remove the generated `result` symlink:

``` bash
rm result
```

## Troubleshooting

-   Ensure flakes are enabled.
-   Run `nix flake check` to validate the package.
-   If dependencies change, regenerate `flake.lock` with
    `nix flake update`.

## Recommended Workflow

1.  `git pull`
2.  `nix develop`
3.  `cargo build`
4.  `nix flake check`
5.  `git commit`
6.  `git push`
