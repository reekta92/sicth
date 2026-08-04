use ratatui::crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};
use std::io::{self, Stdout};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// 40% of rows, min 6, always leave >=2 rows of terminal free; degenerate tiny terminals pass through.
pub fn popup_height(term_rows: u16) -> u16 {
    if term_rows < 8 {
        return term_rows.max(1);
    }
    ((term_rows as u32 * 2 / 5) as u16).clamp(6, term_rows - 2)
}

pub fn setup() -> io::Result<(Tui, TerminalGuard)> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnableMouseCapture)?;
    let rows = ratatui::crossterm::terminal::size()?.1;
    let height = popup_height(rows);
    let term = Terminal::with_options(
        CrosstermBackend::new(out),
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    )?;
    Ok((term, TerminalGuard))
}

/// Normal-path teardown. Clear erases the popup corpse — inline viewport does NOT auto-erase.
pub fn teardown(term: &mut Tui) {
    let _ = term.clear();
    let _ = term.show_cursor();
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), DisableMouseCapture);
}

/// Panic path: raw mode + mouse capture must not survive a panic.
pub struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableMouseCapture);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_height_50_rows() {
        assert_eq!(popup_height(50), 20);
    }

    #[test]
    fn popup_height_10_rows_clamped_to_6() {
        assert_eq!(popup_height(10), 6);
    }

    #[test]
    fn popup_height_5_rows_passthrough() {
        assert_eq!(popup_height(5), 5);
    }

    #[test]
    fn popup_height_8_rows_returns_6() {
        assert_eq!(popup_height(8), 6);
    }
}
