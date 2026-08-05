use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use nucleo::{Config, Nucleo};
use nucleo_matcher::pattern::{CaseMatching, Normalization};
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use crate::model::{self, Entry, Kind};
use crate::settings::Settings;
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
    QuitToDir(PathBuf),
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
    pub scroll_offset: usize,
    pub last_click: Option<(Instant, usize)>,
    pub last_area: Rect,
    pub settings: Settings,
    pub nav_back: Vec<PathBuf>,
    pub nav_forward: Vec<PathBuf>,
}

fn fresh_nucleo() -> Nucleo<Entry> {
    Nucleo::new(Config::DEFAULT.match_paths(), Arc::new(|| {}), None, 1)
}

impl App {
    pub fn new(cwd: PathBuf, settings: Settings) -> Self {
        let mut app = App {
            browse_entries: Vec::new(),
            nucleo: fresh_nucleo(),
            walker: None,
            show_hidden: settings.show_hidden,
            settings,
            mode: Mode::Browse,
            selected: 0,
            scroll_offset: 0,
            last_click: None,
            last_area: Rect::default(),
            query: String::new(),
            cwd,
            nav_back: Vec::new(),
            nav_forward: Vec::new(),
        };
        app.browse_entries = app.reload_entries();
        app
    }

    fn reload_entries(&self) -> Vec<Entry> {
        if self.settings.recursive {
            model::browse_recursive(
                &self.cwd,
                self.show_hidden,
                self.settings.ignore_gitignore,
                self.settings.follow_links,
                self.settings.sort_by,
                self.settings.dirs_first,
                self.settings.reverse,
            )
        } else {
            model::browse(
                &self.cwd,
                self.show_hidden,
                self.settings.sort_by,
                self.settings.dirs_first,
                self.settings.reverse,
            )
        }
    }

    fn search_text(&self) -> String {
        if self.settings.exact {
            self.query
                .split_whitespace()
                .map(|tok| {
                    if tok.starts_with('\'') {
                        tok.to_string()
                    } else {
                        format!("'{tok}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            self.query.clone()
        }
    }

    fn case_matching(&self) -> CaseMatching {
        if self.settings.case_insensitive {
            CaseMatching::Ignore
        } else {
            CaseMatching::Smart
        }
    }

    fn resolve_entry(&self, idx: usize, force_quit: bool) -> Cmd {
        let entry = match self.mode {
            Mode::Browse | Mode::Command => self.browse_entries.get(idx).cloned(),
            Mode::Search => self
                .nucleo
                .snapshot()
                .get_matched_item(idx as u32)
                .map(|m| m.data.clone()),
        };
        let Some(e) = entry else {
            return Cmd::None;
        };
        let quit = force_quit || self.settings.quit_on_match;
        match e.kind {
            Kind::Dir if quit => Cmd::QuitToDir(e.abs.clone()),
            Kind::Dir => Cmd::EnterDir(e.abs.clone()),
            Kind::File => Cmd::OpenFile(e.abs.clone()),
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
            self.settings.ignore_gitignore,
            self.settings.follow_links,
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
            self.browse_entries = self.reload_entries();
        } else if self.query.starts_with('!') {
            self.stop_walker();
            self.mode = Mode::Command;
        } else {
            if self.mode != Mode::Search {
                self.mode = Mode::Search;
                self.start_walker();
            }
            let text = self.search_text();
            self.nucleo.pattern.reparse(
                0,
                &text,
                self.case_matching(),
                Normalization::Smart,
                append,
            );
        }
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn set_dir(&mut self, dir: PathBuf) {
        self.nav_back.push(self.cwd.clone());
        self.nav_forward.clear();
        self.set_dir_raw(dir);
    }

    fn set_dir_raw(&mut self, dir: PathBuf) {
        self.cwd = dir;
        self.query.clear();
        self.stop_walker();
        self.mode = Mode::Browse;
        self.browse_entries = self.reload_entries();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn reload(&mut self) {
        self.set_dir_raw(self.cwd.clone());
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.reload();
    }

    pub fn navigate_back(&mut self) {
        if let Some(dir) = self.nav_back.pop() {
            self.nav_forward.push(self.cwd.clone());
            self.set_dir_raw(dir);
        }
    }

    pub fn navigate_forward(&mut self) {
        if let Some(dir) = self.nav_forward.pop() {
            self.nav_back.push(self.cwd.clone());
            self.set_dir_raw(dir);
        }
    }

    /// Visible items: Browse → slice; Search → nucleo snapshot.
    pub fn visible(&self, max: u32) -> Vec<&Entry> {
        let off = self.scroll_offset;
        match self.mode {
            Mode::Browse | Mode::Command => self
                .browse_entries
                .iter()
                .skip(off)
                .take(max as usize)
                .collect(),
            Mode::Search => {
                let snap = self.nucleo.snapshot();
                let total = snap.matched_item_count() as usize;
                (off..total.min(off + max as usize))
                    .filter_map(|i| snap.get_matched_item(i as u32).map(|m| m.data))
                    .collect()
            }
        }
    }

    fn item_count(&self) -> usize {
        match self.mode {
            Mode::Browse | Mode::Command => self.browse_entries.len(),
            Mode::Search => self.nucleo.snapshot().matched_item_count() as usize,
        }
    }

    fn list_rows(&self) -> usize {
        (self.last_area.height as usize).saturating_sub(1)
    }

    fn selectable_count(&self) -> usize {
        self.item_count()
    }
    pub(crate) fn ensure_selection_visible(&mut self) {
        let rows = self.list_rows();
        let count = self.item_count();
        if count == 0 {
            self.scroll_offset = 0;
            return;
        }
        if self.selected >= count {
            self.selected = count - 1;
        }
        if self.scroll_offset >= count {
            self.scroll_offset = count.saturating_sub(1);
        }
        if rows == 0 {
            return;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + rows {
            self.scroll_offset = self.selected - rows + 1;
        }
    }

    pub fn on_key(&mut self, k: KeyEvent) -> Cmd {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
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
            KeyCode::Enter if alt => {
                if self.mode == Mode::Command {
                    return Cmd::None;
                }
                let count = self.selectable_count();
                if count == 0 || self.selected >= count {
                    return Cmd::None;
                }
                self.resolve_entry(self.selected, true)
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
                self.resolve_entry(self.selected, false)
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
                let cmd = self.resolve_entry(self.selected, false);
                match cmd {
                    Cmd::OpenFile(_) => Cmd::None,
                    other => other,
                }
            }
            KeyCode::Up => {
                let count = self.selectable_count();
                if count > 0 {
                    self.selected = if self.settings.wrap_selection && self.selected == 0 {
                        count - 1
                    } else {
                        self.selected.saturating_sub(1)
                    };
                }
                Cmd::None
            }
            KeyCode::Char('k') if ctrl => {
                let count = self.selectable_count();
                if count > 0 {
                    self.selected = if self.settings.wrap_selection && self.selected == 0 {
                        count - 1
                    } else {
                        self.selected.saturating_sub(1)
                    };
                }
                Cmd::None
            }
            KeyCode::Down => {
                let count = self.selectable_count();
                if count > 0 {
                    self.selected = if self.settings.wrap_selection && self.selected + 1 >= count {
                        0
                    } else {
                        (self.selected + 1).min(count - 1)
                    };
                }
                Cmd::None
            }
            KeyCode::Char('j') if ctrl => {
                let count = self.selectable_count();
                if count > 0 {
                    self.selected = if self.settings.wrap_selection && self.selected + 1 >= count {
                        0
                    } else {
                        (self.selected + 1).min(count - 1)
                    };
                }
                Cmd::None
            }
            KeyCode::Char('l') if ctrl => self.resolve_entry(self.selected, false),

            KeyCode::Char('d') if ctrl => {
                let half = self.list_rows() / 2;
                let count = self.selectable_count();
                if count > 0 && half > 0 {
                    self.selected = (self.selected + half).min(count - 1);
                }
                Cmd::None
            }
            KeyCode::Char('u') if ctrl => {
                let half = self.list_rows() / 2;
                self.selected = self.selected.saturating_sub(half);
                Cmd::None
            }
            KeyCode::Char('h') if ctrl => Cmd::ParentDir,
            KeyCode::Char('q') if ctrl => {
                self.navigate_back();
                Cmd::None
            }
            KeyCode::Char('e') if ctrl => {
                self.navigate_forward();
                Cmd::None
            }
            KeyCode::Char('.') if ctrl => {
                self.toggle_hidden();
                Cmd::None
            }
            KeyCode::Char(c)
                if !k
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
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
        let list_rows = if self.mode == Mode::Command {
            area.height.saturating_sub(2) as usize
        } else {
            area.height.saturating_sub(1) as usize
        };
        // Query bar row (buttons): row == area.y
        if m.row == area.y {
            if matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) {
                if m.column == area.x && !self.nav_back.is_empty() {
                    self.navigate_back();
                } else if m.column == area.x + 1 && !self.nav_forward.is_empty() {
                    self.navigate_forward();
                } else if m.column == area.x + 2 && self.cwd.parent().is_some() {
                    return Cmd::ParentDir;
                }
            }
            return Cmd::None;
        }

        let row = m.row as usize;
        let y = area.y as usize;
        let list_start = if self.mode == Mode::Command {
            y + 2
        } else {
            y + 1
        };
        if row < list_start || row >= list_start + list_rows {
            return Cmd::None;
        }
        let rel = row - list_start;
        let idx = self.scroll_offset + rel;
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
        self.resolve_entry(idx, false)
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
        let dir =
            std::env::temp_dir().join(format!("sicth_app_test_{}_{}", std::process::id(), id));
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
        let app = App::new(dir.clone(), Settings::default());
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
    fn selection_scroll_past_viewport() {
        let (mut app, _dir) = app_with_entries(15);
        app.last_area = Rect::new(0, 0, 80, 5); // 4 list rows
        assert_eq!(app.selectable_count(), 15);

        for _ in 0..10 {
            let ev = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
            app.on_key(ev);
        }
        assert_eq!(app.selected, 10, "selection reaches past viewport");

        app.ensure_selection_visible();
        assert_eq!(app.scroll_offset, 7, "scroll_offset = selected - rows + 1");
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

    #[test]
    fn ctrl_d_u_half_page_jump() {
        let (mut app, _dir) = app_with_entries(20);
        app.last_area = Rect::new(0, 0, 80, 11); // 10 list rows, half = 5
        let ctrl = KeyModifiers::CONTROL;
        app.on_key(KeyEvent::new(KeyCode::Char('d'), ctrl));
        assert_eq!(app.selected, 5);
        app.on_key(KeyEvent::new(KeyCode::Char('d'), ctrl));
        assert_eq!(app.selected, 10);
        app.on_key(KeyEvent::new(KeyCode::Char('u'), ctrl));
        assert_eq!(app.selected, 5);
    }

    #[test]
    fn nav_back_forward_buttons() {
        let (mut app, dir) = app_with_entries(5);
        app.last_area = Rect::new(0, 0, 80, 20);
        let sub = dir.join("dir_0");
        assert!(sub.exists());

        // Enter subdirectory — records history
        app.set_dir(sub.clone());
        assert_eq!(app.cwd, sub);
        assert_eq!(app.nav_back.len(), 1);
        assert!(app.nav_forward.is_empty());

        // Go back
        app.navigate_back();
        assert_eq!(app.cwd, dir);
        assert!(app.nav_back.is_empty());
        assert_eq!(app.nav_forward.len(), 1);

        // Go forward
        app.navigate_forward();
        assert_eq!(app.cwd, sub);
        assert_eq!(app.nav_back.len(), 1);
        assert!(app.nav_forward.is_empty());
    }

    #[test]
    fn command_mode_shows_list_and_navigates() {
        let (mut app, _dir) = app_with_entries(5);
        app.last_area = Rect::new(0, 0, 80, 20);

        // Enter Command mode
        app.set_query("!".into());
        assert_eq!(app.mode, Mode::Command);

        // Ensure item count is 5 (like Browse mode)
        assert_eq!(app.item_count(), 5);

        // Ensure visible items is not empty
        assert_eq!(app.visible(10).len(), 5);

        // Move selection down in Command mode
        let ev = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        app.on_key(ev);
        assert_eq!(
            app.selected, 1,
            "Down in Command mode should move selection to 1"
        );

        // KeyCode::Right in Command mode shouldn't panic
        let ev = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        let cmd = app.on_key(ev);
        match cmd {
            Cmd::EnterDir(_) | Cmd::None => {}
            _ => panic!(
                "Unexpected command on Right arrow in Command mode: {:?}",
                cmd
            ),
        }
    }

    #[test]
    fn alt_enter_quits_search_match_without_z() {
        let (mut app, _dir) = app_with_entries(5);
        app.last_area = Rect::new(0, 0, 80, 20);
        app.set_query("dir".into());
        // Spin nucleo until snapshot is non-empty (walker thread pushes async)
        for _ in 0..500 {
            app.nucleo.tick(10);
            if app.nucleo.snapshot().matched_item_count() > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let count = app.nucleo.snapshot().matched_item_count();
        assert!(count > 0, "nucleo should have matched items, got {count}");
        app.selected = 0;
        assert!(matches!(
            app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT)),
            Cmd::QuitToDir(_)
        ));
        assert!(matches!(
            app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Cmd::EnterDir(_)
        ));
    }

    #[test]
    fn z_flag_quits_on_plain_enter() {
        let s = Settings {
            quit_on_match: true,
            ..Settings::default()
        };
        let dir = fixture_dir(5);
        let mut app = App::new(dir, s);
        app.last_area = Rect::new(0, 0, 80, 20);
        app.set_query("dir".into());
        for _ in 0..500 {
            app.nucleo.tick(10);
            if app.nucleo.snapshot().matched_item_count() > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let count = app.nucleo.snapshot().matched_item_count();
        assert!(count > 0, "nucleo should have matched items, got {count}");
        app.selected = 0;
        assert!(matches!(
            app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Cmd::QuitToDir(_)
        ));
    }
}
