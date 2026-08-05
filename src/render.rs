use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListState},
    Frame,
};

use crate::app::App;
use crate::model::{Entry, Kind};

/// Nerd Font codepoint + color for an entry.
fn glyph(e: &Entry) -> (char, Color) {
    if e.kind == Kind::Dir {
        return ('\u{f07b}', Color::Blue);
    }
    let name = e.rel.to_lowercase();
    if let Some(ext) = name.rsplit('.').next() {
        // Code
        if matches!(
            ext,
            "rs" | "c"
                | "h"
                | "cpp"
                | "hpp"
                | "cc"
                | "py"
                | "js"
                | "ts"
                | "jsx"
                | "tsx"
                | "mjs"
                | "go"
                | "java"
                | "kt"
                | "rb"
                | "sh"
                | "bash"
                | "zsh"
                | "fish"
                | "lua"
                | "pl"
                | "hs"
                | "ml"
                | "clj"
                | "cljs"
                | "zig"
                | "v"
                | "swift"
        ) {
            return ('\u{f121}', Color::Yellow);
        }
        // Text
        if matches!(ext, "txt" | "md" | "org" | "rst" | "log" | "tex") {
            return ('\u{f15c}', Color::White);
        }
        // Config
        if matches!(
            ext,
            "toml" | "yaml" | "yml" | "json" | "ini" | "cfg" | "conf" | "xml" | "env" | "lock"
        ) {
            return ('\u{e615}', Color::Cyan);
        }
        // Images
        if matches!(
            ext,
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "avif"
        ) {
            return ('\u{f1c5}', Color::Magenta);
        }
        // Audio
        if matches!(ext, "mp3" | "flac" | "ogg" | "oga" | "wav" | "m4a" | "opus") {
            return ('\u{f1c7}', Color::Cyan);
        }
        // Video
        if matches!(ext, "mp4" | "mkv" | "avi" | "mov" | "webm" | "m4v") {
            return ('\u{f1c8}', Color::Magenta);
        }
        // Archives
        if matches!(
            ext,
            "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst"
        ) {
            return ('\u{f410}', Color::Red);
        }
        // PDF
        if ext == "pdf" {
            return ('\u{f1c1}', Color::Red);
        }
        // Executables
        if matches!(ext, "exe" | "bin" | "appimage") {
            return ('\u{f489}', Color::Green);
        }
    }
    // No extension + exec bit check
    if !name.contains('.') {
        #[cfg(unix)]
        if let Ok(meta) = std::fs::metadata(&e.abs) {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o111 != 0 {
                return ('\u{f489}', Color::Green);
            }
        }
    }
    // Fallback
    ('\u{f016}', Color::White)
}

fn entry_line(e: &Entry, s: &App) -> Line<'static> {
    let sett = &s.settings;
    let mut style = Style::default();
    if sett.colors {
        let (_g, fg) = glyph(e);
        style = style.fg(fg);
        if e.kind == Kind::Dir && sett.bold_dirs {
            style = style.add_modifier(Modifier::BOLD);
        }
    }
    let mut spans: Vec<Span> = Vec::new();
    if sett.icons {
        let (g, fg) = glyph(e);
        let mut ic = Style::default().fg(fg);
        if sett.bold_dirs && e.kind == Kind::Dir {
            ic = ic.add_modifier(Modifier::BOLD);
        }
        spans.push(Span::styled(format!("{} ", g), ic));
    }
    let display_name = if e.kind == Kind::Dir && sett.slash_dirs {
        format!("{}/", e.rel)
    } else {
        e.rel.clone()
    };
    spans.push(Span::styled(display_name, style));
    Line::from(spans)
}

pub fn draw(f: &mut Frame, app: &mut App) {
    app.last_area = f.area();
    app.ensure_selection_visible();
    let area = f.area();

    let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    let query_area = layout[0];
    let list_area = layout[1];

    // Compute visible items
    let list_height = if app.mode == crate::app::Mode::Command {
        list_area.height.saturating_sub(1)
    } else {
        list_area.height
    };
    let visible: Vec<&Entry> = app.visible(list_height as u32);

    let mut list_state = ListState::default();
    if !visible.is_empty() {
        let rel = app.selected.saturating_sub(app.scroll_offset);
        if rel < visible.len() {
            list_state.select(Some(rel));
        }
    }

    let items: Vec<Line> = visible.iter().map(|e| entry_line(e, app)).collect();
    let highlight = if app.settings.colors {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default().add_modifier(Modifier::REVERSED)
    };
    let list = List::new(items).highlight_style(highlight);

    if app.mode == crate::app::Mode::Command {
        let sub = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(list_area);
        let hint = Line::from(vec![
            Span::styled("run in ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.cwd.display().to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]);
        f.render_widget(hint, sub[0]);
        f.render_stateful_widget(list, sub[1], &mut list_state);
    } else {
        f.render_stateful_widget(list, list_area, &mut list_state);
    }

    // Query line with nav buttons: ← back, → forward, ↑ parent
    let show_btns = app.settings.mouse;
    let btn_w: u16 = if show_btns { 4 } else { 0 };

    let at_root = app.cwd.parent().is_none();

    let back_styled = if show_btns {
        if app.nav_back.is_empty() {
            Span::styled("\u{2190}", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(
                "\u{2190}",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        }
    } else {
        Span::raw("")
    };
    let fwd_styled = if show_btns {
        if app.nav_forward.is_empty() {
            Span::styled("\u{2192}", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(
                "\u{2192}",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        }
    } else {
        Span::raw("")
    };
    let up_styled = if show_btns {
        if at_root {
            Span::styled("\u{2191}", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(
                "\u{2191}",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        }
    } else {
        Span::raw("")
    };

    let query_spans: Vec<Span> = if app.query.is_empty() {
        let cwd_str = app.cwd.display().to_string();
        let max_len = (query_area.width as usize).saturating_sub(btn_w as usize);
        let display = if cwd_str.chars().count() > max_len && max_len > 0 {
            let truncated: String = cwd_str.chars().take(max_len.saturating_sub(1)).collect();
            format!("{truncated}\u{2026}")
        } else {
            cwd_str
        };
        let mut v = Vec::new();
        if show_btns {
            v.push(back_styled);
            v.push(fwd_styled);
            v.push(up_styled);
            v.push(Span::raw(" "));
        }
        if app.settings.show_cwd {
            v.push(Span::styled(
                display,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ));
        }
        v
    } else {
        let mut v = Vec::new();
        if show_btns {
            v.push(back_styled);
            v.push(fwd_styled);
            v.push(up_styled);
            v.push(Span::raw(" "));
        }
        v.push(Span::raw(&app.query));
        v
    };
    let query_line = Line::from(query_spans);
    f.render_widget(query_line, query_area);

    f.set_cursor_position((
        query_area.x + btn_w + app.query.chars().count() as u16,
        query_area.y,
    ));
}
