use ratatui::crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};
use std::io::{self, Stdout};

use crate::settings::Settings;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Height as percent of terminal rows, clamped 10-90%.
pub fn popup_height(term_rows: u16, percent: u16) -> u16 {
    let p = percent.clamp(10, 90);
    if term_rows < 8 {
        return term_rows.max(1);
    }
    ((term_rows as u32 * p as u32 / 100) as u16).clamp(6, term_rows - 2)
}

pub fn setup(sett: &Settings) -> io::Result<(Tui, TerminalGuard)> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    if sett.mouse {
        execute!(out, EnableMouseCapture)?;
    }
    let viewport = if sett.fullscreen {
        Viewport::Fullscreen
    } else {
        let rows = ratatui::crossterm::terminal::size()?.1;
        Viewport::Inline(popup_height(rows, sett.popup_percent))
    };
    let term = Terminal::with_options(CrosstermBackend::new(out), TerminalOptions { viewport })?;
    Ok((term, TerminalGuard { mouse: sett.mouse }))
}

/// Normal-path teardown. Clear erases the popup corpse — inline viewport does NOT auto-erase.
pub fn teardown(term: &mut Tui, mouse: bool) {
    let _ = term.clear();
    let _ = term.show_cursor();
    let _ = disable_raw_mode();
    if mouse {
        let _ = execute!(io::stdout(), DisableMouseCapture);
    }
}

/// Panic path: raw mode + mouse capture must not survive a panic.
pub struct TerminalGuard {
    pub mouse: bool,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        if self.mouse {
            let _ = execute!(io::stdout(), DisableMouseCapture);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_height_50_rows_40pct() {
        assert_eq!(popup_height(50, 40), 20);
    }

    #[test]
    fn popup_height_10_rows_clamped_to_6() {
        assert_eq!(popup_height(10, 40), 6);
    }

    #[test]
    fn popup_height_5_rows_passthrough() {
        assert_eq!(popup_height(5, 40), 5);
    }

    #[test]
    fn popup_height_8_rows_returns_6() {
        assert_eq!(popup_height(8, 40), 6);
    }

    #[test]
    fn popup_height_50_rows_70pct() {
        assert_eq!(popup_height(50, 70), 35);
    }
}
