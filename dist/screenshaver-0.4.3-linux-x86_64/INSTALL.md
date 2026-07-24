# Installing Screenshaver on Linux

Screenshaver is distributed as source code and built locally on the target Linux system. The repository includes an installation script that detects the Linux distribution, installs the required build dependencies, installs Rust when necessary, builds Screenshaver, and installs the program.

## Supported systems

The installer currently supports:

- Debian and Debian-derived distributions, including Ubuntu, Linux Mint, Pop!_OS, elementary OS, and Zorin OS
- Fedora
- CentOS Stream, Red Hat Enterprise Linux, Rocky Linux, and AlmaLinux major versions 8, 9, and 10
- Arch Linux, Manjaro, and EndeavourOS
- openSUSE Leap, openSUSE Tumbleweed, and SUSE Linux Enterprise Server
- Void Linux
- NixOS

Other Linux distributions may still be able to build Screenshaver after the required dependencies are installed manually.

## Requirements

Before cloning the repository, install Git if it is not already available.

Examples:

```bash
# Debian and Ubuntu
sudo apt update
sudo apt install git

# Fedora, CentOS, RHEL, Rocky Linux, or AlmaLinux
sudo dnf install git

# Arch Linux and derivatives
sudo pacman -S git

# openSUSE and SLES
sudo zypper install git

# Void Linux
sudo xbps-install -S git

# NixOS temporary shell
nix-shell -p git
```

The automated installer handles the remaining build dependencies on supported conventional Linux distributions. On NixOS, it uses the repository's Nix flake.

## 1. Clone the public GitHub repository

Choose a directory in which to keep the source code, then clone the repository.

```bash
git clone https://github.com/<repository-owner>/screenshaver.git
cd screenshaver
```

Replace `<repository-owner>` with the GitHub account or organization that owns the public Screenshaver repository.

HTTPS cloning does not require a GitHub account or SSH key for a public repository.

## 2. Review the installer

Before running any downloaded installation script, review it:

```bash
less scripts/install-linux.sh
```

You can also display its command-line options:

```bash
./scripts/install-linux.sh --help
```

If the script is not executable after cloning, restore its executable permission:

```bash
chmod +x scripts/install-linux.sh scripts/uninstall-linux.sh
```

## 3. Run the installer

Run the installer as your normal user:

```bash
./scripts/install-linux.sh
```

Do **not** run the entire script with `sudo`. On conventional Linux distributions, the installer requests `sudo` only when it needs to install system packages or copy files under `/usr/local`. Rust is installed for the current user rather than for the root account.

For an unattended installation that accepts the installer's confirmation prompts:

```bash
./scripts/install-linux.sh --yes
```

The `--yes` option answers the Screenshaver installer's prompts automatically. The operating system's package manager may still display messages or require authorization according to the system's configuration.

## What the installer does

### Conventional Linux distributions

On Debian, Ubuntu, Fedora, Enterprise Linux, Arch Linux, openSUSE, Void Linux, and their supported derivatives, the installer performs these operations:

1. Detects the distribution using `/etc/os-release`.
2. Installs or verifies the native build dependencies.
3. Installs `rustup`, Cargo, and the stable Rust toolchain for the current user when Rust is not already available.
4. Builds Screenshaver in release mode with:

   ```bash
   cargo build --release --locked
   ```

5. Installs the executable as:

   ```text
   /usr/local/bin/screenshaver
   ```

6. Installs the desktop launcher as:

   ```text
   /usr/local/share/applications/screenshaver.desktop
   ```

7. Installs the application icons under:

   ```text
   /usr/local/share/icons/hicolor
   ```

8. Refreshes the desktop application and icon caches when the corresponding utilities are available.
9. Validates the desktop launcher and checks the installed executable for unresolved shared libraries.

### CentOS, RHEL, Rocky Linux, and AlmaLinux

Enterprise Linux systems require additional repositories for some Screenshaver development dependencies. The installer configures these automatically before installing the packages.

Depending on the operating system and release, it may:

- Install `dnf-plugins-core`
- Enable PowerTools on Enterprise Linux 8
- Enable CRB on Enterprise Linux 9 or 10
- Enable the matching RHEL CodeReady Builder repository on registered RHEL systems
- Install the appropriate EPEL repository definition
- Install EPEL Next on CentOS Stream 9
- Refresh DNF metadata
- Verify that `SDL2_ttf-devel` and `libXScrnSaver-devel` are available

A registered RHEL system must have working subscription repositories. If repository configuration fails, inspect the enabled repositories with:

```bash
dnf repolist
```

Package visibility can be checked with:

```bash
dnf repoquery --available SDL2_ttf-devel
dnf repoquery --available libXScrnSaver-devel
```

The installer does not disable or remove CRB, CodeReady Builder, EPEL, or EPEL Next afterward because they are system-wide repositories that may be useful to other software.

### NixOS

On NixOS, the installer does not install conventional development packages or copy files into `/usr/local`. Instead, it:

1. Verifies that the `nix` command is available.
2. Verifies that `flake.nix` exists in the repository root.
3. Displays and validates the flake outputs.
4. Builds the `screenshaver` flake package.
5. Adds Screenshaver to the current user's Nix profile.

The installer invokes Nix with the `nix-command` and `flakes` experimental features enabled for each command.

## 4. Launch Screenshaver

After a conventional Linux installation, launch Screenshaver from the desktop application menu or run:

```bash
screenshaver
```

The installed command should resolve to:

```bash
command -v screenshaver
```

Expected conventional installation path:

```text
/usr/local/bin/screenshaver
```

On NixOS, the command is installed through the current user's Nix profile. If it is not immediately visible in `PATH`, open a new terminal session and try again.

If Rust was installed during installation and Cargo is not visible in the current shell, load its environment with:

```bash
source "$HOME/.cargo/env"
```

## Updating Screenshaver

Return to the cloned repository and download the latest source changes:

```bash
cd /path/to/screenshaver
git pull --ff-only
```

Then rerun the installer:

```bash
./scripts/install-linux.sh
```

The new release build replaces the previously installed Screenshaver executable, desktop launcher, and icons.

Before updating, preserve any uncommitted changes in the repository. Check the working tree with:

```bash
git status
```

## Uninstalling Screenshaver

From the cloned repository, run the uninstaller as your normal user:

```bash
./scripts/uninstall-linux.sh
```

To skip its confirmation prompt:

```bash
./scripts/uninstall-linux.sh --yes
```

On conventional Linux distributions, the uninstaller removes:

- `/usr/local/bin/screenshaver`
- `/usr/local/share/applications/screenshaver.desktop`
- Screenshaver icons under `/usr/local/share/icons/hicolor`

On NixOS, it removes Screenshaver from the current user's Nix profile.

The uninstaller intentionally preserves:

- Rust, Cargo, and rustup
- Compilers and build tools
- SDL2 and SDL2_ttf
- X11 and XScreenSaver libraries
- OpenGL libraries
- CRB, CodeReady Builder, EPEL, and EPEL Next repository configuration
- User configuration, shaders, and logs

These resources may have existed before Screenshaver was installed or may be required by other applications.

## Manual build without system installation

Developers who already have Rust and all required native development libraries can build Screenshaver without running the installer:

```bash
git clone https://github.com/<repository-owner>/screenshaver.git
cd screenshaver
cargo build --release --locked
```

The resulting executable is:

```text
target/release/screenshaver
```

Run it directly with:

```bash
./target/release/screenshaver
```

This does not install the executable, desktop launcher, or icons under `/usr/local`.

## Troubleshooting

### The installer says not to run it with sudo

Exit the root shell or remove `sudo` from the command. Run:

```bash
./scripts/install-linux.sh
```

The script invokes `sudo` internally only when required.

### `Cargo.toml` was not found

The installer must remain in the repository's `scripts` directory and must be run from a complete clone of the Screenshaver repository. Do not copy the script by itself into another directory.

### Rust was installed, but Cargo is unavailable

Run:

```bash
source "$HOME/.cargo/env"
```

Then verify:

```bash
rustup --version
cargo --version
rustc --version
```

### A required package cannot be found

First refresh the distribution's package metadata and confirm that the standard development repositories are enabled.

For CentOS, RHEL, Rocky Linux, and AlmaLinux:

```bash
dnf repolist
dnf repoquery --available SDL2-devel
dnf repoquery --available SDL2_ttf-devel
dnf repoquery --available libXScrnSaver-devel
```

For openSUSE:

```bash
zypper refresh
zypper search -s SDL2-devel
zypper search -s SDL2_ttf-devel
```

Package availability can vary between distribution releases and enabled repositories.

### The desktop menu does not update immediately

Log out and back in, restart the desktop shell, or manually refresh the application database when supported:

```bash
sudo update-desktop-database /usr/local/share/applications
```

The desktop environment may refresh its menus automatically.

### Shared libraries are reported as missing

Inspect the installed executable with:

```bash
ldd /usr/local/bin/screenshaver
```

Any line ending in `not found` identifies a missing runtime library. Reinstall the corresponding distribution package, then rerun the Screenshaver installer.

## Installation security notes

- Review installation scripts before running them.
- Clone the project from its official public GitHub repository.
- Do not run the entire installer or uninstaller as root.
- The installer downloads Rust through the official rustup installation endpoint when Rust is unavailable.
- Enterprise Linux installations may download EPEL repository-release RPMs from Fedora infrastructure.
- The Cargo lock file is honored by building with `--locked`.

