# Configuration Reference

Every flag listed below can be passed on the command line or set permanently in
a config file. CLI flags override the config file; the config file overrides
built-in defaults.

## Config file

sicth reads a plain `key = value` file at:

- `$XDG_CONFIG_HOME/sicth/config` (e.g. `~/.config/sicth/config`), or
- `--config <path>` to use an explicit path.

The file is line-oriented: blank lines and `#`-comments are ignored. Unknown
keys and unparseable values are silently skipped — the field keeps its prior
value (default or CLI-override). A missing or unreadable file is non-fatal:
sicth starts with defaults.

**Example** (`~/.config/sicth/config`):

```
# Always search the home directory, not the current one
home_scope = true

# Monochrome, no icons, no mouse
colors = false
icons = false
mouse = false

# Sort by modification time, newest first
sort_by = mtime

# Use neovim as editor
editor = nvim

# Quit on Enter when a search match is selected
quit_on_match = true
```

## Flag reference

Every flag has a corresponding config key (same name). The "default" column
shows what sicth uses when neither a config file nor a CLI flag says otherwise.

Flags can be clustered: `sicth -nmc` disables icons, mouse, and colors in one
go. Value flags (`-p`, `-e`) consume the remainder of the cluster or the next
argv element — they terminate the cluster.

| Flag | Config key | Type | Default | What it does |
|------|-----------|------|---------|--------------|
| `-a` | `show_hidden` | bool | `false` | Show dotfiles in Browse mode. |
| `-r` | `recursive` | bool | `false` | Recursive directory listing in Browse mode (flat listing otherwise). Capped at 100k entries. |
| `-x` | `exact` | bool | `false` | Exact (contiguous substring) matching instead of fuzzy. Each whitespace-delimited query token becomes a substring atom — `sic` matches `sicth` but not `sith`. |
| `-i` | `case_insensitive` | bool | `false` | Make search case-insensitive. Default is smart-case: lowercase queries match both cases, uppercase queries are case-sensitive. |
| `-z` | `quit_on_match` | bool | `false` | Pressing Enter on a Search-mode match quits sicth and `cd`s the shell to the matched directory (or opens the file). Without `-z`, Enter navigates into the directory inside the popup. |
| `-g` | `ignore_gitignore` | bool | `false` | Ignore `.gitignore` rules — the recursive walker includes git-ignored files. |
| `-L` | `follow_links` | bool | `false` | Follow symbolic links during recursive walk. Use with care — symlink cycles are not detected and may cause hangs. |
| `-A` | `show_all` | bool | `false` | Convenience combo: enables both `show_hidden` and `ignore_gitignore` (same as `-a -g`). |
| `-s` | `sort_by` | enum | `name` | Sort Browse entries by **size** (largest first). Name is used as the tiebreaker. |
| `-t` | `sort_by` | enum | `name` | Sort Browse entries by **modification time** (newest first). Entries with unknown mtime sort last. |
| `-d` | `dirs_first` | bool | `true` | Put directories **last** (default is directories first). |
| `-v` | `reverse` | bool | `false` | Reverse the sort order. Combined with `-s` gives smallest-first; with `-t` gives oldest-first. |
| `-n` | `icons` | bool | `true` | Disable Nerd Font icons in the file list. Entries are shown as plain names. |
| `-c` | `colors` | bool | `true` | Disable all color output. Highlights switch to reverse-video; icons and bold are suppressed. |
| `-b` | `bold_dirs` | bool | `true` | Disable bold styling on directory names. |
| `-l` | `slash_dirs` | bool | `true` | Disable the trailing `/` appended to directory names. |
| `-q` | `show_cwd` | bool | `true` | Hide the current-working-directory path shown when the query is empty. |
| `-m` | `mouse` | bool | `true` | Disable mouse capture. The `← → ↑` nav buttons in the query bar are also hidden (they are mouse-only affordances; keyboard equivalents Ctrl+q/e/h always work). |
| `-F` | `fullscreen` | bool | `false` | Run in full-screen mode instead of the default inline popup. The popup-height flag (`-p`) is ignored. |
| `-w` | `wrap_selection` | bool | `false` | Wrap selection at list boundaries: pressing Down on the last entry jumps to the first; pressing Up on the first jumps to the last. |
| `-H` | `home_scope` | bool | `false` | Start sicth in the user's home directory (`$HOME`) instead of the current working directory. All browsing and searching is scoped to home. |
| `-k` | `keep_open` | bool | `false` | Keep sicth open after opening a file: return to the popup when the editor exits (or immediately after the system opener launches) instead of exiting the app. |
| `-o` | `open_system` | bool | `false` | Always open files with the system opener (xdg-open / open) instead of sicth's content-type detection. |
| `-p N` | `popup_percent` | u16 | `40` | Popup height as a percentage of terminal rows. Clamped to 10–90. Ignored when `-F` (fullscreen) is set. |
| `-e CMD` | `editor` | string | — | Override the editor command. Default is `$VISUAL`, then `$EDITOR`, then `vi`. `CMD` is split on whitespace: `-e "nvim -R"` passes `-R` as an argument. |

### Meta flags

These short-circuit and exit immediately — they are not config keys.

| Flag | What it does |
|------|-------------|
| `--setup` | Install the `sc` shell function (bash/zsh/fish) for cd-on-exit integration. |
| `--keybinds` | Print the full keybinding reference and exit. |
| `--help`, `-h` | Print usage and flag reference and exit. |
| `--out-file <path>` | Write the exit-directory script to `<path>` (used internally by the `sc` shell wrapper). |
| `--config <path>` | Load config from `<path>` instead of the default location. |

## Precedence

```
built-in defaults  <  config file  <  CLI flags
```

1. sicth starts with the hard-coded defaults in the table above.
2. If a config file exists, each key it contains overwrites the corresponding
   default.
3. CLI flags are applied last and always win — a flag explicitly passed on the
   command line cannot be undone by the config file.

**Example:** config has `mouse = false`, CLI has `-m`. Both say "disable
mouse", so the result is `mouse = false`. Had the config said `mouse = true`,
the CLI `-m` would still win and mouse would be off.

## Shell wrapper integration

When you run `sicth --setup`, the installed `sc` shell function passes all
arguments through to sicth:

```sh
sc -n -m -H     # no icons, no mouse, home-directory scope
```

The `sc` wrapper sources sicth's exit script, so the shell lands in the
directory sicth was navigating when it quit — even with `-H` (home scope) or
`-z` (quit-on-match), the cd target is the directory of the matched entry.
