use std::path::{Path, PathBuf};
use std::process;

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Mtime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub show_hidden: bool,
    pub recursive: bool,
    pub exact: bool,
    pub case_insensitive: bool,
    pub quit_on_match: bool,
    pub ignore_gitignore: bool,
    pub follow_links: bool,
    pub show_all: bool,
    pub sort_by: SortKey,
    pub dirs_first: bool,
    pub reverse: bool,
    pub icons: bool,
    pub colors: bool,
    pub bold_dirs: bool,
    pub slash_dirs: bool,
    pub show_cwd: bool,
    pub mouse: bool,
    pub fullscreen: bool,
    pub popup_percent: u16,
    pub editor: Option<String>,
    pub open_system: bool,
    pub wrap_selection: bool,
    pub home_scope: bool,
    pub keep_open: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_hidden: false,
            recursive: false,
            exact: false,
            case_insensitive: false,
            quit_on_match: false,
            ignore_gitignore: false,
            follow_links: false,
            show_all: false,
            sort_by: SortKey::Name,
            dirs_first: true,
            reverse: false,
            icons: true,
            colors: true,
            bold_dirs: true,
            slash_dirs: true,
            show_cwd: true,
            mouse: true,
            fullscreen: false,
            popup_percent: 40,
            editor: None,
            open_system: false,
            wrap_selection: false,
            home_scope: false,
            keep_open: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Resolves the default config path ($XDG_CONFIG_HOME/sicth/config or
/// ~/.config/sicth/config). Returns `None` when neither env var provides a
/// usable path.
pub fn config_path() -> Option<PathBuf> {
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        if x.starts_with('/') {
            return Some(PathBuf::from(x).join("sicth/config"));
        }
    }
    if let Ok(h) = std::env::var("HOME") {
        if !h.is_empty() {
            return Some(PathBuf::from(h).join(".config/sicth/config"));
        }
    }
    None
}

impl Settings {
    /// Apply a single `key=value` from a config line. Unknown keys and
    /// unparseable values are silently skipped.
    pub fn apply_config(&mut self, key: &str, value: &str) {
        match key {
            "show_hidden" => self.show_hidden = parse_bool(value, self.show_hidden),
            "recursive" => self.recursive = parse_bool(value, self.recursive),
            "exact" => self.exact = parse_bool(value, self.exact),
            "case_insensitive" => self.case_insensitive = parse_bool(value, self.case_insensitive),
            "quit_on_match" => self.quit_on_match = parse_bool(value, self.quit_on_match),
            "ignore_gitignore" => self.ignore_gitignore = parse_bool(value, self.ignore_gitignore),
            "follow_links" => self.follow_links = parse_bool(value, self.follow_links),
            "show_all" => self.show_all = parse_bool(value, self.show_all),
            "sort_by" => {
                if let Some(sk) = parse_sort_key(value) {
                    self.sort_by = sk;
                }
            }
            "dirs_first" => self.dirs_first = parse_bool(value, self.dirs_first),
            "reverse" => self.reverse = parse_bool(value, self.reverse),
            "icons" => self.icons = parse_bool(value, self.icons),
            "colors" => self.colors = parse_bool(value, self.colors),
            "bold_dirs" => self.bold_dirs = parse_bool(value, self.bold_dirs),
            "slash_dirs" => self.slash_dirs = parse_bool(value, self.slash_dirs),
            "show_cwd" => self.show_cwd = parse_bool(value, self.show_cwd),
            "mouse" => self.mouse = parse_bool(value, self.mouse),
            "fullscreen" => self.fullscreen = parse_bool(value, self.fullscreen),
            "popup_percent" => {
                if let Ok(n) = value.parse::<u16>() {
                    self.popup_percent = n.clamp(10, 90);
                }
            }
            "editor" => {
                let v = value.trim_matches('"');
                self.editor = Some(v.to_string());
            }
            "open_system" => self.open_system = parse_bool(value, self.open_system),
            "wrap_selection" => self.wrap_selection = parse_bool(value, self.wrap_selection),
            "home_scope" => self.home_scope = parse_bool(value, self.home_scope),
            "keep_open" => self.keep_open = parse_bool(value, self.keep_open),
            _ => { /* unknown key — ignore */ }
        }
    }
}

fn parse_bool(raw: &str, fallback: bool) -> bool {
    match raw {
        "true" => true,
        "false" => false,
        _ => fallback,
    }
}

fn parse_sort_key(raw: &str) -> Option<SortKey> {
    match raw {
        "name" => Some(SortKey::Name),
        "size" => Some(SortKey::Size),
        "mtime" => Some(SortKey::Mtime),
        _ => None,
    }
}

pub fn parse_config(path: &Path) -> Settings {
    let mut s = Settings::default();
    let Ok(text) = std::fs::read_to_string(path) else {
        return s;
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if k.is_empty() || v.is_empty() {
            continue;
        }
        s.apply_config(k, v);
    }
    s
}

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum Parsed {
    Run {
        settings: Settings,
        out_file: Option<PathBuf>,
    },
    Setup,
    Keybinds,
    Help,
}

#[derive(Debug)]
pub enum ParseError {
    Msg(String),
}

pub fn parse_args() -> Parsed {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match parse_args_inner(&argv) {
        Ok(p) => p,
        Err(ParseError::Msg(m)) => {
            eprintln!("sicth: {m}\n");
            eprint!("{}", crate::usage());
            process::exit(2);
        }
    }
}

/// Single-pass algorithm:
/// 1. Scan for --config <path> + meta-flag short-circuit.
/// 2. Load config (explicit or default).
/// 3. Second scan applies CLI flags over config.
fn parse_args_inner(argv: &[String]) -> Result<Parsed, ParseError> {
    // --- first scan: config path + meta short-circuit ---
    let mut config_override: Option<PathBuf> = None;
    let mut i = 0;
    while i < argv.len() {
        let a = argv[i].as_str();
        // meta flags short-circuit (first-encountered wins)
        if a == "--setup" {
            return Ok(Parsed::Setup);
        }
        if a == "--keybinds" {
            return Ok(Parsed::Keybinds);
        }
        if a == "--help" || a == "-h" {
            return Ok(Parsed::Help);
        }
        if a == "--config" {
            i += 1;
            if i < argv.len() {
                config_override = Some(PathBuf::from(&argv[i]));
            }
        } else if let Some(val) = a.strip_prefix("--config=") {
            config_override = Some(PathBuf::from(val));
        }
        i += 1;
    }

    // --- load config ---
    let mut settings = if let Some(ref cp) = config_override {
        parse_config(cp)
    } else if let Some(cp) = config_path() {
        if cp.exists() {
            parse_config(&cp)
        } else {
            Settings::default()
        }
    } else {
        Settings::default()
    };

    let mut out_file: Option<PathBuf> = None;

    // --- second scan: apply CLI flags (skip --config + metas) ---
    let mut i = 0;
    while i < argv.len() {
        let a = argv[i].as_str();

        // skip --config path element (handled above)
        if a == "--config" {
            i += 2;
            continue;
        }
        if a.starts_with("--config=") {
            i += 1;
            continue;
        }
        // skip metas (already short-circuited, but handle if they somehow appear again)
        if a == "--setup" || a == "--keybinds" || a == "--help" || a == "-h" {
            i += 1;
            continue;
        }
        // --out-file
        if a == "--out-file" {
            i += 1;
            if i >= argv.len() {
                return Err(ParseError::Msg("--out-file requires a value".into()));
            }
            out_file = Some(PathBuf::from(&argv[i]));
            i += 1;
            continue;
        }
        if let Some(val) = a.strip_prefix("--out-file=") {
            out_file = Some(PathBuf::from(val));
            i += 1;
            continue;
        }
        if a == "--keep-open" {
            settings.keep_open = true;
            i += 1;
            continue;
        }
        if a.starts_with('-') && a.len() >= 2 && !a.starts_with("--") {
            let chars: Vec<char> = a[1..].chars().collect();
            let mut ci = 0;
            while ci < chars.len() {
                let ch = chars[ci];
                ci += 1;
                match ch {
                    'a' => settings.show_hidden = true,
                    'r' => settings.recursive = true,
                    'x' => settings.exact = true,
                    'i' => settings.case_insensitive = true,
                    'z' => settings.quit_on_match = true,
                    'g' => settings.ignore_gitignore = true,
                    'L' => settings.follow_links = true,
                    'A' => settings.show_all = true,
                    's' => settings.sort_by = SortKey::Size,
                    't' => settings.sort_by = SortKey::Mtime,
                    'd' => settings.dirs_first = false,
                    'v' => settings.reverse = true,
                    'n' => settings.icons = false,
                    'c' => settings.colors = false,
                    'b' => settings.bold_dirs = false,
                    'l' => settings.slash_dirs = false,
                    'q' => settings.show_cwd = false,
                    'm' => settings.mouse = false,
                    'F' => settings.fullscreen = true,
                    'o' => settings.open_system = true,
                    'w' => settings.wrap_selection = true,
                    'H' => settings.home_scope = true,
                    'k' => settings.keep_open = true,
                    'h' => return Ok(Parsed::Help),
                    'p' => {
                        let val: String = if ci < chars.len() {
                            // remainder of cluster
                            chars[ci..].iter().collect()
                        } else {
                            // next argv element
                            i += 1;
                            if i >= argv.len() {
                                return Err(ParseError::Msg("-p requires a number 10..=90".into()));
                            }
                            argv[i].clone()
                        };
                        match val.parse::<u16>() {
                            Ok(n) => settings.popup_percent = n.clamp(10, 90),
                            Err(_) => {
                                return Err(ParseError::Msg("-p requires a number 10..=90".into()));
                            }
                        }
                        break; // stop cluster iteration
                    }
                    'e' => {
                        let val: String = if ci < chars.len() {
                            chars[ci..].iter().collect()
                        } else {
                            i += 1;
                            if i >= argv.len() {
                                return Err(ParseError::Msg("-e requires a value".into()));
                            }
                            argv[i].clone()
                        };
                        settings.editor = Some(val);
                        break; // stop cluster iteration
                    }
                    _ => {
                        return Err(ParseError::Msg(format!("-{ch}: unknown flag")));
                    }
                }
            }
            i += 1;
            continue;
        }
        // long flag (--xxx) not handled above
        if a.starts_with("--") {
            return Err(ParseError::Msg(format!("{a}: unknown flag")));
        }
        // bare argument
        return Err(ParseError::Msg(format!("unexpected argument: {a}")));
    }

    // post-process: show_all forces show_hidden + ignore_gitignore
    if settings.show_all {
        settings.show_hidden = true;
        settings.ignore_gitignore = true;
    }

    Ok(Parsed::Run { settings, out_file })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_config(content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sicth_cfg_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("config");
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    // --- config parsing ---

    #[test]
    fn parse_config_round_trips_all_keys() {
        let content = "\
show_hidden = true
recursive = true
exact = true
case_insensitive = true
quit_on_match = true
ignore_gitignore = true
follow_links = true
show_all = true
sort_by = size
dirs_first = false
reverse = true
icons = false
colors = false
bold_dirs = false
slash_dirs = false
show_cwd = false
mouse = false
fullscreen = true
popup_percent = 70
editor = nvim
open_system = true
wrap_selection = true
keep_open = true
";
        let p = write_config(content);
        let s = parse_config(&p);
        assert!(s.show_hidden);
        assert!(s.recursive);
        assert!(s.exact);
        assert!(s.case_insensitive);
        assert!(s.quit_on_match);
        assert!(s.ignore_gitignore);
        assert!(s.follow_links);
        assert!(s.show_all);
        assert_eq!(s.sort_by, SortKey::Size);
        assert!(!s.dirs_first);
        assert!(s.reverse);
        assert!(!s.icons);
        assert!(!s.colors);
        assert!(!s.bold_dirs);
        assert!(!s.slash_dirs);
        assert!(!s.show_cwd);
        assert!(!s.mouse);
        assert!(s.fullscreen);
        assert_eq!(s.popup_percent, 70);
        assert_eq!(s.editor.as_deref(), Some("nvim"));
        assert!(s.open_system);
        assert!(s.wrap_selection);
        assert!(s.keep_open);
    }

    #[test]
    fn apply_config_rejects_bad_bool() {
        let mut s = Settings {
            show_hidden: true,
            ..Settings::default()
        };
        s.apply_config("show_hidden", "nope");
        assert!(s.show_hidden, "bad bool leaves field unchanged");
    }

    #[test]
    fn apply_config_rejects_bad_sort_key() {
        let mut s = Settings {
            sort_by: SortKey::Size,
            ..Settings::default()
        };
        s.apply_config("sort_by", "banana");
        assert_eq!(
            s.sort_by,
            SortKey::Size,
            "unknown sort word leaves field unchanged"
        );
    }

    #[test]
    fn apply_config_unknown_key_is_noop() {
        let mut s = Settings::default();
        let snap = s.clone();
        s.apply_config("nonexistent", "true");
        assert_eq!(s, snap);
    }

    #[test]
    fn missing_config_file_returns_defaults() {
        let s = parse_config(&PathBuf::from("/tmp/sicth_nonexistent_config_xyz"));
        assert_eq!(s, Settings::default());
    }

    // --- arg parsing ---

    #[test]
    fn cli_overrides_config() {
        let p = write_config("show_hidden = false\n");
        let argv = [
            "--config".to_string(),
            p.to_string_lossy().to_string(),
            "-a".to_string(),
        ];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => {
                assert!(
                    settings.show_hidden,
                    "CLI -a overrides config show_hidden=false"
                );
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn cluster_minus_nmc() {
        let argv = ["-nmc".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => {
                assert!(!settings.icons);
                assert!(!settings.mouse);
                assert!(!settings.colors);
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn flag_p_glued() {
        let argv = ["-p50".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => assert_eq!(settings.popup_percent, 50),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn flag_p_separate() {
        let argv = ["-p".to_string(), "50".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => assert_eq!(settings.popup_percent, 50),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn flag_p_clamps_low() {
        let argv = ["-p1".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => assert_eq!(settings.popup_percent, 10),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn flag_p_clamps_high() {
        let argv = ["-p99".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => assert_eq!(settings.popup_percent, 90),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn flag_e_glued() {
        let argv = ["-evim".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => assert_eq!(settings.editor.as_deref(), Some("vim")),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn flag_e_separate() {
        let argv = ["-e".to_string(), "vim".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => assert_eq!(settings.editor.as_deref(), Some("vim")),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn unknown_short_flag_is_error() {
        let argv = ["-Z".to_string()];
        let err = parse_args_inner(&argv).unwrap_err();
        assert!(
            format!("{err:?}").contains("-Z"),
            "error should mention -Z: {err:?}"
        );
    }

    #[test]
    fn show_all_forces_hidden_and_ignore_gitignore() {
        let argv = ["-A".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => {
                assert!(settings.show_hidden);
                assert!(settings.ignore_gitignore);
                assert!(settings.show_all);
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn flag_k_keep_open_short() {
        let argv = ["-k".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => assert!(settings.keep_open),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn flag_keep_open_long() {
        let argv = ["--keep-open".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => assert!(settings.keep_open),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn keep_open_defaults_off() {
        let parsed = parse_args_inner(&[]).unwrap();
        match parsed {
            Parsed::Run { settings, .. } => assert!(!settings.keep_open),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn meta_setup_short_circuits() {
        let argv = ["--setup".to_string(), "-n".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        assert!(matches!(parsed, Parsed::Setup));
    }

    #[test]
    fn meta_keybinds_short_circuits() {
        let argv = ["--keybinds".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        assert!(matches!(parsed, Parsed::Keybinds));
    }

    #[test]
    fn meta_help_short_circuits() {
        let argv = ["-h".to_string()];
        let parsed = parse_args_inner(&argv).unwrap();
        assert!(matches!(parsed, Parsed::Help));
    }

    #[test]
    fn default_settings_all_match_plan() {
        let s = Settings::default();
        assert!(!s.show_hidden);
        assert!(!s.recursive);
        assert!(!s.exact);
        assert!(s.icons);
        assert!(s.colors);
        assert!(s.mouse);
        assert!(s.dirs_first);
        assert_eq!(s.popup_percent, 40);
        assert_eq!(s.sort_by, SortKey::Name);
        assert!(!s.keep_open);
    }
}
