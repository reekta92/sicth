# sicth

[![CI](https://github.com/reekta92/sicth/actions/workflows/ci.yml/badge.svg)](https://github.com/reekta92/sicth/actions/workflows/ci.yml)
[![Crates.io Version](https://img.shields.io/crates/v/sicth)](https://crates.io/crates/sicth)
[![Crates.io Downloads](https://img.shields.io/crates/d/sicth)](https://crates.io/crates/sicth)
[![License: GPL-3.0-only](https://img.shields.io/badge/License-GPL_3.0_only-blue.svg)](LICENSE)
[![AUR Package](https://img.shields.io/aur/version/sicth-bin)](https://aur.archlinux.org/packages/sicth-bin)

A minimal TUI file navigator with fuzzy search, Nerd Font icons, and shell integration.

<!-- TODO: Add screenshot or GIF here -->

## Features

* Fuzzy search powered by nucleo
* Nerd Font file-type icons with syntax-aware coloring
* Inline popup viewport (doesn't take over full terminal)
* Mouse support (click, double-click, scroll)
* Shell integration — `cd` on exit via `sc` wrapper, `!command` execution
* `.gitignore`-aware recursive search
* Vim-style keybinds (Ctrl+j/k/d/u/h/l)
* Toggle hidden files with `.`
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
| **Navigation** | |
| `Down` / `Ctrl+j` | Move selection down |
| `Up` / `Ctrl+k` | Move selection up |
| `Ctrl+d` | Half-page down |
| `Ctrl+u` | Half-page up |
| `Enter` / `Ctrl+l` | Enter directory / Open file |
| `Right` | Enter directory (dirs only) |
| `Left` / `Ctrl+h` | Go to parent directory |
| `.` | Toggle hidden files |
| **Shell** | |
| `!command` | Quit and run command in the current directory (needs sc wrapper) |
| **Actions** | |
| `Esc` | Clear search / Quit (when query empty) |
| `Ctrl+c` | Quit without writing out-file |
| **Typing** | |
| `!` (prefix) | Prefix search to enter command mode |
| `[text]` | Filter entries by name |
| `Backspace` | Delete last char of query, or go to parent directory (if empty) |
| **Mouse** | |
| `Scroll` | Move selection |
| `Left Click` | Select entry |
| `Double Click`| Enter directory / Open file |
| `←` / `→` / `↑` buttons | Back, forward, parent directory |
## License

GPL-3.0-only. See [LICENSE](LICENSE).