<div align='center'>

<h2>Hyprlauncher <img src='https://raw.githubusercontent.com/hyprutils/.github/refs/heads/main/hyprutils_transparent.png'width='18' height='18'></h2>

<img src='hyprlauncher.png' width='200' height='200'><br>

[![Grind Compliant](https://img.shields.io/badge/Grind-Compliant-blue)](https://github.com/The-Grindhouse/guidelines)<br>
An unofficial [daemon-like](https://en.wikipedia.org/wiki/Daemon_(computing)) GUI for launching applications, built with GTK4 and Rust. 🚀🦀<br>

## Preview
![Preview](.github/preview.png)

</div>

## Usage

> [!TIP]
> For optimal performance, bind Hyprlauncher to a keyboard shortcut instead of launching it from a terminal. While the initial launch takes a moment to daemonize, subsequent launches are near-instant (~28-30ms).

Example Hyprland config bind:
```conf
bind = $mainMod_SHIFT, E, exec, hyprlauncher
```

## Configuration
For configuration options and customization, see [configuration.md](configuration.md).

The configuration file is located at `~/.config/hyprlauncher/config.json`

Changes to the configuration file are applied instantly without needing to restart.

## Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/hyprlauncher.svg)](https://repology.org/project/hyprlauncher/versions)

### Requirements
- GTK4
- GTK4-Layer-Shell
- Pango

### GitHub Releases
See Hyprlauncher's [releases page](https://github.com/hyprutils/hyprlauncher/releases) for downloadable binaries.

### Arch Linux
There are 2 different [AUR](https://aur.archlinux.org) packages available:

- [hyprlauncher](https://aur.archlinux.org/packages/hyprlauncher) - Latest release built from source
- [hyprlauncher-bin](https://aur.archlinux.org/packages/hyprlauncher-bin) - Latest release in binary form

Install the preferred package with:
```bash
git clone https://aur.archlinux.org/<package>.git
cd <package>
makepkg -si
```

Or, if you're using an [AUR Helper](https://wiki.archlinux.org/title/AUR_helpers), it's even simpler (using [paru](https://github.com/Morganamilo/paru) as an example):
```bash
paru -S <package>
```

## Building from source
1. Install Rust (preferably `rustup`) through your distro's package or [the official script](https://www.rust-lang.org/tools/install)
2. Clone this repository:
`git clone https://github.com/hyprutils/hyprlauncher && cd hyprlauncher`
3. Compile the app with `cargo build --release` or run it directly with `cargo run --release`

## Credits:
- [Nyx](https://github.com/nnyyxxxx) - Implementing the GUI, and maintaining the project
- [Adam](https://github.com/adamperkowski) - Code improvements, and maintaining the project
- [Vaxry](https://github.com/vaxerski) - Hyprland
- [rust-gtk](https://github.com/gtk-rs/gtk4-rs) - The GTK4 library
- [Hyprland](https://github.com/hyprwm/Hyprland) - The wayland compositor

<h6 align='center'>Copyright (C) 2024 HyprUtils<h6>
