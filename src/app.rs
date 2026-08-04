use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use nucleo::{Config, Nucleo};
use nucleo_matcher::pattern::{CaseMatching, Normalization};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::model::{self, Entry};
use crate::walk::{self, WalkHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Browse,
    Search,
    Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Cmd {
    None,
    QuitCd,
    QuitNoCd,
    OpenFile(PathBuf),
    EnterDir(PathBuf),
    ParentDir,
    RebuildList,
    RunCommand(String),
}

pub struct App {
    pub cwd: PathBuf,
    pub query: String,
    pub show_hidden: bool,
    pub mode: Mode,
    pub browse_entries: Vec<Entry>,
    pub nucleo: Nucleo<Entry>,
    pub walker: Option<WalkHandle>,
    pub selected: usize,
    pub last_click: Option<(Instant, usize)>,
    pub last_area: Rect,
}

fn fresh_nucleo() -> Nucleo<Entry> {
    Nucleo::new(Config::DEFAULT.match_paths(), Arc::new(|| {}), None, 1)
}

impl App {
    pub fn new(cwd: PathBuf) -> Self {
        App {
            browse_entries: model::browse(&cwd, false),
            nucleo: fresh_nucleo(),
            walker: None,
            show_hidden: false,
            mode: Mode::Browse,
            selected: 0,
            last_click: None,
            last_area: Rect::default(),
            query: String::new(),
            cwd,
        }
    }

    fn stop_walker(&mut self) {
        if let Some(ref h) = self.walker {
            h.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.walker = None;
    }

    fn start_walker(&mut self) {
        self.stop_walker();
        self.nucleo = fresh_nucleo();
        let injector = self.nucleo.injector();
        self.walker = Some(walk::spawn_walker(
            self.cwd.clone(),
            self.show_hidden,
            injector,
        ));
    }

    pub fn set_query(&mut self, new: String) {
        self.query = new;
        self.after_query_change(false);
    }

    pub fn set_query_append(&mut self, new: String, old: &str) {
        let append = !new.is_empty() && new.starts_with(old) && new.len() > old.len();
        self.query = new;
        self.after_query_change(append);
    }

    fn after_query_change(&mut self, append: bool) {
        if self.query.is_empty() {
            self.stop_walker();
            self.mode = Mode::Browse;
            self.browse_entries = model::browse(&self.cwd, self.show_hidden);
        } else if self.query.starts_with('!') {
            self.stop_walker();
            self.mode = Mode::Command;
        } else {
            if self.mode != Mode::Search {
                self.mode = Mode::Search;
                self.start_walker();
            }
            self.nucleo.pattern.reparse(
                0,
                &self.query,
                CaseMatching::Smart,
                Normalization::Smart,
                append,
            );
        }
        self.selected = 0;
    }

    pub fn set_dir(&mut self, dir: PathBuf) {
        self.cwd = dir;
        self.query.clear();
        self.stop_walker();
        self.mode = Mode::Browse;
        self.browse_entries = model::browse(&self.cwd, self.show_hidden);
        self.selected = 0;
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.set_dir(self.cwd.clone());
    }

    /// Visible items: Browse → slice; Search → nucleo snapshot.
    pub fn visible(&self, max: u32) -> Vec<&Entry> {
        match self.mode {
            Mode::Browse => self.browse_entries.iter().take(max as usize).collect(),
            Mode::Command => Vec::new(),
            Mode::Search => {
                let snap = self.nucleo.snapshot();
                (0..snap.matched_item_count().min(max))
                    .filter_map(|i| snap.get_matched_item(i).map(|m| m.data))
                    .collect()
            }
        }
    }

    fn item_count(&self) -> usize {
        match self.mode {
            Mode::Browse => self.browse_entries.len(),
            Mode::Command => 0,
            Mode::Search => self.nucleo.snapshot().matched_item_count() as usize,
        }
    }

    /// Entries the popup can actually show and select: min(total matches, list rows).
    /// last_area is set on every draw; the main loop draws before polling events,
    /// so it is never the zero-Rect in practice. If height is 0, this yields 0 and
    /// all selection keys safely no-op.
    fn selectable_count(&self) -> usize {
        let list_rows = (self.last_area.height as usize).saturating_sub(1);
        self.item_count().min(list_rows)
    }

    pub fn on_key(&mut self, k: KeyEvent) -> Cmd {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Char('c') if ctrl => Cmd::QuitNoCd,
            KeyCode::Esc => {
                if !self.query.is_empty() {
                    self.set_query(String::new());
                    Cmd::None
                } else {
                    Cmd::QuitCd
                }
            }
            KeyCode::Enter => {
                if self.mode == Mode::Command {
                    let cmd = self.query[1..].trim();
                    if cmd.is_empty() {
                        return Cmd::None;
                    }
                    return Cmd::RunCommand(cmd.to_string());
                }
                let count = self.selectable_count();
                if count == 0 || self.selected >= count {
                    return Cmd::None;
                }
                let entries = match self.mode {
                    Mode::Command => unreachable!(),
                    Mode::Browse => &self.browse_entries,
                    Mode::Search => {
                        // We need the entry at selected; rebuild
                        let snap = self.nucleo.snapshot();
                        // Can't borrow snap across await; collect
                        return snap
                            .get_matched_item(self.selected as u32)
                            .map(|m| {
                                let e = m.data;
                                match e.kind {
                                    crate::model::Kind::Dir => Cmd::EnterDir(e.abs.clone()),
                                    crate::model::Kind::File => Cmd::OpenFile(e.abs.clone()),
                                }
                            })
                            .unwrap_or(Cmd::None);
                    }
                };
                if self.selected >= entries.len() {
                    return Cmd::None;
                }
                let e = &entries[self.selected];
                match e.kind {
                    crate::model::Kind::Dir => Cmd::EnterDir(e.abs.clone()),
                    crate::model::Kind::File => Cmd::OpenFile(e.abs.clone()),
                }
            }
            KeyCode::Backspace => {
                if self.query.is_empty() {
                    Cmd::ParentDir
                } else {
                    let old = self.query.clone();
                    let mut new = old.clone();
                    new.pop();
                    self.set_query_append(new, &old);
                    Cmd::None
                }
            }
            KeyCode::Left => Cmd::ParentDir,
            KeyCode::Right => {
                let count = self.selectable_count();
                if count == 0 || self.selected >= count {
                    return Cmd::None;
                }
                let entries = match self.mode {
                    Mode::Command => unreachable!(),
                    Mode::Browse => &self.browse_entries,
                    Mode::Search => {
                        let snap = self.nucleo.snapshot();
                        return snap
                            .get_matched_item(self.selected as u32)
                            .map(|m| {
                                if m.data.kind == crate::model::Kind::Dir {
                                    Cmd::EnterDir(m.data.abs.clone())
                                } else {
                                    Cmd::None
                                }
                            })
                            .unwrap_or(Cmd::None);
                    }
                };
                if self.selected >= entries.len() {
                    return Cmd::None;
                }
                if entries[self.selected].kind == crate::model::Kind::Dir {
                    Cmd::EnterDir(entries[self.selected].abs.clone())
                } else {
                    Cmd::None
                }
            }
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                Cmd::None
            }
            KeyCode::Char('k') if ctrl => {
                self.selected = self.selected.saturating_sub(1);
                Cmd::None
            }
            KeyCode::Down => {
                let count = self.selectable_count();
                if count > 0 {
                    self.selected = (self.selected + 1).min(count - 1);
                }
                Cmd::None
            }
            KeyCode::Char('j') if ctrl => {
                let count = self.selectable_count();
                if count > 0 {
                    self.selected = (self.selected + 1).min(count - 1);
                }
                Cmd::None
            }
            KeyCode::Char('l') if ctrl => {
                let count = self.selectable_count();
                if count == 0 || self.selected >= count {
                    return Cmd::None;
                }
                self.activate_entry(self.selected)
            }
            KeyCode::Char('h') if ctrl => Cmd::ParentDir,
            KeyCode::Char('.') => {
                self.toggle_hidden();
                Cmd::None
            }
            KeyCode::Char(c) if !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                let mut new = self.query.clone();
                new.push(c);
                let old = self.query.clone();
                self.set_query_append(new, &old);
                Cmd::None
            }
            _ => Cmd::None,
        }
    }

    pub fn on_mouse(&mut self, m: MouseEvent, area: Rect) -> Cmd {
        let list_rows = area.height.saturating_sub(1); // query bar at top row
        if m.row <= area.y || m.row >= area.y + 1 + list_rows {
            return Cmd::None;
        }
        let idx = (m.row - area.y - 1) as usize;
        let count = self.selectable_count();
        if idx >= count {
            return Cmd::None;
        }

        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let now = Instant::now();
                if let Some((prev, prev_idx)) = self.last_click {
                    if prev_idx == idx && now.duration_since(prev).as_millis() < 300 {
                        self.last_click = None;
                        // Activate: get the entry at idx
                        return self.activate_entry(idx);
                    }
                }
                self.selected = idx;
                self.last_click = Some((now, idx));
                Cmd::None
            }
            MouseEventKind::ScrollUp => {
                self.selected = self.selected.saturating_sub(3);
                Cmd::None
            }
            MouseEventKind::ScrollDown => {
                let count = self.selectable_count();
                if count > 0 {
                    self.selected = (self.selected + 3).min(count - 1);
                }
                Cmd::None
            }
            _ => Cmd::None,
        }
    }

    fn activate_entry(&self, idx: usize) -> Cmd {
        match self.mode {
            Mode::Command => Cmd::None,
            Mode::Browse => {
                if idx >= self.browse_entries.len() {
                    return Cmd::None;
                }
                let e = &self.browse_entries[idx];
                match e.kind {
                    crate::model::Kind::Dir => Cmd::EnterDir(e.abs.clone()),
                    crate::model::Kind::File => Cmd::OpenFile(e.abs.clone()),
                }
            }
            Mode::Search => {
                let snap = self.nucleo.snapshot();
                snap.get_matched_item(idx as u32)
                    .map(|m| match m.data.kind {
                        crate::model::Kind::Dir => Cmd::EnterDir(m.data.abs.clone()),
                        crate::model::Kind::File => Cmd::OpenFile(m.data.abs.clone()),
                    })
                    .unwrap_or(Cmd::None)
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::layout::Rect;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn fixture_dir(entries: usize) -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir()
            .join(format!("sicth_app_test_{}_{}", std::process::id(), id));
        fs::create_dir_all(&dir).unwrap();
        for i in 0..entries {
            if i % 3 == 0 {
                fs::create_dir_all(dir.join(format!("dir_{i}"))).unwrap();
            } else {
                fs::write(dir.join(format!("file_{i}.txt")), b"test").unwrap();
            }
        }
        dir
    }

    fn app_with_entries(n: usize) -> (App, PathBuf) {
        let dir = fixture_dir(n);
        let app = App::new(dir.clone());
        (app, dir)
    }

    #[test]
    fn plain_arrows_move_selection() {
        let (mut app, _dir) = app_with_entries(5);
        app.last_area = Rect::new(0, 0, 80, 20); // 19 list rows

        // Down from 0 → 1
        let ev = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        app.on_key(ev);
        assert_eq!(app.selected, 1, "Down should move selection from 0 to 1");

        // Up from 1 → 0
        let ev = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        app.on_key(ev);
        assert_eq!(app.selected, 0, "Up should move selection from 1 to 0");
    }

    #[test]
    fn backspace_multibyte_does_not_panic() {
        let (mut app, _dir) = app_with_entries(1);
        app.set_query("hé".into());
        assert_eq!(app.query, "hé");

        let ev = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        app.on_key(ev);
        assert_eq!(app.query, "h");
    }

    #[test]
    fn selection_clamped_to_window() {
        let (mut app, _dir) = app_with_entries(15);
        app.last_area = Rect::new(0, 0, 80, 5); // 4 list rows (height 5, minus 1 for query)
        assert_eq!(app.selectable_count(), 4);

        for _ in 0..10 {
            let ev = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
            app.on_key(ev);
        }
        assert_eq!(app.selected, 3, "selection should be clamped to last visible row");
    }

    #[test]
    fn typing_enters_search_and_stays_alive() {
        let (mut app, _dir) = app_with_entries(1);
        app.last_area = Rect::new(0, 0, 80, 20);

        let ev = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let cmd = app.on_key(ev);
        assert_eq!(cmd, Cmd::None, "typing should not quit");
        assert_eq!(app.query, "a");
        assert_eq!(app.mode, Mode::Search);
    }

    #[test]
    fn bang_enters_command_mode_without_walker() {
        let (mut app, _dir) = app_with_entries(5);
        app.last_area = Rect::new(0, 0, 80, 20);
        let ev = KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE);
        app.on_key(ev);
        assert_eq!(app.mode, Mode::Command);
        assert_eq!(app.query, "!");
        assert!(app.walker.is_none());
    }

    #[test]
    fn bare_bang_enter_is_noop() {
        let (mut app, _dir) = app_with_entries(1);
        app.last_area = Rect::new(0, 0, 80, 20);
        app.set_query("!".into());
        assert_eq!(app.mode, Mode::Command);
        let ev = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let cmd = app.on_key(ev);
        assert_eq!(cmd, Cmd::None);
        assert_eq!(app.mode, Mode::Command);
    }

    #[test]
    fn command_enter_yields_run_command() {
        let (mut app, _dir) = app_with_entries(1);
        app.last_area = Rect::new(0, 0, 80, 20);
        app.set_query("!echo hi".into());
        let ev = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let cmd = app.on_key(ev);
        assert_eq!(cmd, Cmd::RunCommand("echo hi".to_string()));
    }

    #[test]
    fn command_to_search_restarts_walker() {
        let (mut app, _dir) = app_with_entries(5);
        app.last_area = Rect::new(0, 0, 80, 20);
        app.set_query("!x".into());
        assert_eq!(app.mode, Mode::Command);
        app.set_query("x".into());
        assert_eq!(app.mode, Mode::Search);
        assert!(app.walker.is_some());
    }

}
