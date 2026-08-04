# sicth

[![CI](https://github.com/reekta92/sicth/actions/workflows/ci.yml/badge.svg)](https://github.com/reekta92/sicth/actions/workflows/ci.yml)
[![Crates.io Downloads](https://img.shields.io/crates/d/sicth)](https://crates.io/crates/sicth)
[![License: GPL-3.0-only](https://img.shields.io/badge/License-GPL_3.0_only-blue.svg)](LICENSE)
[![AUR Package](https://img.shields.io/aur/version/sicth-bin)](https://aur.archlinux.org/packages/sicth-bin)

`sicth` has a one goal and one goal only, **to navigate through filesystem** interactively and blazingly fast. Can be considered as an alternative to [broot](https://github.com/canop/broot) though **it's as simple as possible and non-disruptive to the terminal window by design**.

<img width="948" height="614" alt="image" src="https://github.com/user-attachments/assets/73890107-2949-4e86-887e-32eb20829d90" />

## Features

* Fuzzy search powered by `nucleo`
* Mouse support (click, double-click, scroll)
* Shell integration — `cd` on exit via `sc` wrapper, `!command` execution
* `.gitignore`-aware recursive search
* Back/forward navigation history

## Installation

### Cargo
```sh
cargo install sicth
```

### Pre-built binaries
Available on the [latest GitHub release page](https://github.com/reekta92/sicth/releases/latest).

| Platform | Artifact |
|----------|----------|
| Linux x86_64 | `sicth-x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 | `sicth-aarch64-unknown-linux-gnu.tar.gz` |
| macOS ARM | `sicth-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `sicth-x86_64-pc-windows-msvc.zip` |
| AppImage | `sicth-0.1.0-x86_64.AppImage` |

### Arch Linux (AUR)
```sh
yay -S sicth-bin
```

### Nix
```sh
nix run github:reekta92/sicth
nix profile install github:reekta92/sicth
```

### Debian/Ubuntu
```sh
sudo dpkg -i sicth_0.1.0_amd64.deb
```

### Fedora/RHEL
```sh
sudo rpm -i sicth-0.1.0-1.x86_64.rpm
```

### Build from source
```sh
git clone https://github.com/reekta92/sicth
cd sicth
cargo build --release
cp target/release/sicth ~/.local/bin/
```

## Shell setup

sicth provides a shell wrapper to enable "cd on exit". Without it, the shell process that runs sicth cannot be navigated by sicth (as child processes cannot change parent's working directory).

Run this to install the wrapper:
```sh
sicth --setup
```

This adds an alias or function named `sc` to your shell profile.
Running `sc` opens sicth, and quitting will `cd` your shell to the last browsed directory.

## Keybinds

| Key | Action |
|-----|--------|
| `Down` / `Ctrl+j` | Move selection down |
| `Up` / `Ctrl+k` | Move selection up |
| `Ctrl+d` | Half-page down |
| `Ctrl+u` | Half-page up |
| `Enter` / `Ctrl+l` | Enter directory / Open file |
| `Right` | Enter directory (dirs only) |
| `Left` / `Ctrl+h` | Go to parent directory |
| `Ctrl+q` | Navigate back in history |
| `Ctrl+e` | Navigate forward in history |
| `Ctrl+.` | Toggle hidden files |
| `!command` | Quit and run command in the current directory (needs sc wrapper) |
| `Esc` | Clear search / Quit (when query empty) |
| `Ctrl+c` | Quit without writing out-file |
| `!` (prefix) | Prefix search to enter command mode |
| `[text]` | Filter entries by name |
| `Backspace` | Delete last char of query, or go to parent directory (if empty) |
| `Scroll` | Move selection |
| `Left Click` | Select entry |
| `Double Click`| Enter directory / Open file |
| `←` / `→` / `↑` buttons | Back, forward, parent directory |
## License

GPL-3.0-only. See [LICENSE](LICENSE).
