mod app;
mod model;
mod open;
mod render;
mod setup;
mod terminal;
mod walk;

use std::path::PathBuf;
use std::process;
use std::time::Duration;

use app::{App, Cmd};
use ratatui::crossterm::event::{self, Event, KeyEventKind};

fn usage() -> &'static str {
    "sicth — inline popup file navigator\n\
\n\
USAGE:\n    \
    sicth [--out-file <path>]\n    \
    sicth --setup\n    \
    sicth --keybinds\n    \
    sicth --help\n\
\n\
OPTIONS:\n    \
    --out-file <path>  write the last browsed directory here on exit (used by the sc shell function)\n    \
    --setup            install the sc shell function (bash/zsh/fish) that cds your shell on exit\n    \
    --keybinds         list all keyboard shortcuts\n"
}

fn keybinds() -> &'static str {
    "NAVIGATION\n  \
    Up, Ctrl+k          move selection up\n  \
    Down, Ctrl+j        move selection down\n  \
    Ctrl+d              half-page down (vim)\n  \
    Ctrl+u              half-page up (vim)\n  \
    Enter               enter directory or open file\n  \
    Right               enter directory (dirs only)\n  \
    Left, Ctrl+h        go to parent directory\n\n\
SHELL\n  \
    !command            quit and run command in the current directory (needs sc wrapper)\n\n\
ACTIONS\n  \
    Esc                 clear query / quit (when query is empty)\n  \
    Ctrl+c              quit without writing out-file\n  \
    Ctrl+.              toggle hidden files\n  \
    Ctrl+l              enter directory or open file (vim mode)\n\n\
TYPING\n  \
    !                   prefix query with ! to enter command mode\n  \
    a-z, 0-9, ...      filter entries by name\n  \
    Backspace           delete last character of query, or go to parent directory\n\n\
MOUSE\n  \
    Left click          select entry\n  \
    Double click        enter directory or open file\n  \
    Scroll              move selection\n  \
    \u{2190} / \u{2192} / \u{2191} buttons  back, forward, parent directory\n"
}

fn main() {
    let mut out_file: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out-file" => {
                let path = args.next().unwrap_or_else(|| {
                    eprintln!("sicth: --out-file requires a value");
                    process::exit(2);
                });
                out_file = Some(PathBuf::from(path));
            }
            "--setup" => {
                setup::run();
                return;
            }
            "--keybinds" => {
                print!("{}", keybinds());
                return;
            }
            "--help" | "-h" => {
                print!("{}", usage());
                return;
            }
            _ => {
                eprint!("{}", usage());
                process::exit(2);
            }
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let mut app = App::new(cwd);

    let (mut term, _guard) = terminal::setup().unwrap_or_else(|e| {
        eprintln!("sicth: failed to initialize terminal: {e}");
        process::exit(1);
    });

    let mut quit_action: Option<Cmd> = None;

    loop {
        let _ = app.nucleo.tick(10);

        if let Err(e) = term.draw(|f| render::draw(f, &mut app)) {
            eprintln!("sicth: draw error: {e}");
            break;
        }

        if event::poll(Duration::from_millis(16)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => {
                    let cmd = app.on_key(k);
                    quit_action = dispatch(&mut app, cmd);
                }
                Ok(Event::Mouse(m)) => {
                    let area = app.last_area;
                    let cmd = app.on_mouse(m, area);
                    quit_action = dispatch(&mut app, cmd);
                }
                Ok(Event::Resize(_, rows)) => {
                    // Inline viewport height is fixed at creation — nothing to do.
                    // Terminal::resize is auto-called by ratatui on Resize events.
                    let _ = rows;
                }
                Err(_) => break,
                _ => {}
            }
        }

        if quit_action.is_some() {
            break;
        }
    }

    let action = quit_action.unwrap_or(Cmd::QuitCd);

    terminal::teardown(&mut term);

    match action {
        Cmd::QuitCd => {
            open::write_out_script(&out_file, &app.cwd, None);
            process::exit(0);
        }
        Cmd::QuitNoCd => {
            process::exit(130);
        }
        Cmd::OpenFile(path) => {
            open::write_out_script(&out_file, &app.cwd, None);
            let how = open::classify(&path);
            match how {
                open::How::Editor => {
                    let (prog, args) = open::resolve_editor();
                    let status = process::Command::new(&prog).args(&args).arg(&path).status();
                    match status {
                        Ok(s) if s.success() => process::exit(0),
                        Ok(s) => process::exit(s.code().unwrap_or(1)),
                        Err(e) => {
                            eprintln!("sicth: failed to launch editor: {e}");
                            process::exit(1);
                        }
                    }
                }
                open::How::System => {
                    if let Err(e) = ::open::that_detached(&path) {
                        eprintln!("sicth: failed to open: {e}");
                        process::exit(1);
                    }
                    process::exit(0);
                }
            }
        }
        Cmd::RunCommand(cmd) => {
            if out_file.is_none() {
                eprintln!("sicth: !command requires the sc shell function (run: sicth --setup)");
                process::exit(1);
            }
            open::write_out_script(&out_file, &app.cwd, Some(&cmd));
            process::exit(0);
        }
        _ => unreachable!("dispatch only yields quit commands"),
    }
}

fn dispatch(app: &mut App, cmd: Cmd) -> Option<Cmd> {
    match cmd {
        Cmd::None => None,
        Cmd::EnterDir(dir) => {
            app.set_dir(dir);
            None
        }
        Cmd::ParentDir => {
            if let Some(parent) = app.cwd.parent().map(|p| p.to_path_buf()) {
                app.set_dir(parent);
            }
            None
        }
        Cmd::RebuildList => {
            app.reload();
            None
        }
        quit @ (Cmd::QuitCd | Cmd::QuitNoCd | Cmd::OpenFile(_) | Cmd::RunCommand(_)) => Some(quit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sicth_main_{}_{}", std::process::id(), label));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn dispatch_noop_is_not_quit() {
        let dir = fixture_dir("noop");
        let sub = dir.join("child");
        let _ = fs::create_dir_all(&sub);
        let mut app = App::new(dir);

        // Cmd::None is consumed, not propagated
        assert!(dispatch(&mut app, Cmd::None).is_none());

        // Cmd::QuitCd propagates through
        assert_eq!(dispatch(&mut app, Cmd::QuitCd), Some(Cmd::QuitCd));

        // Cmd::OpenFile propagates through
        let p = PathBuf::from("/tmp/test.txt");
        assert_eq!(
            dispatch(&mut app, Cmd::OpenFile(p.clone())),
            Some(Cmd::OpenFile(p))
        );

        // Cmd::ParentDir is consumed, cwd moves up
        let mut app2 = App::new(sub);
        let parent = app2.cwd.parent().unwrap().to_path_buf();
        assert!(dispatch(&mut app2, Cmd::ParentDir).is_none());
        assert_eq!(app2.cwd, parent);

        #[test]
        fn dispatch_propagates_run_command() {
            let dir = fixture_dir("run_cmd");
            let mut app = App::new(dir);
            assert_eq!(
                dispatch(&mut app, Cmd::RunCommand("ls".into())),
                Some(Cmd::RunCommand("ls".into()))
            );
        }
    }
}
