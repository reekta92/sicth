use std::path::{Path, PathBuf};

use crate::settings::SortKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Dir,
    File,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub rel: String,
    pub abs: PathBuf,
    pub kind: Kind,
    pub size: u64,
    pub mtime: Option<std::time::SystemTime>,
}

fn from_dir_entry(de: &std::fs::DirEntry) -> Entry {
    let name = de.file_name().to_string_lossy().to_string();
    let path = de.path();
    let kind = if path.is_dir() { Kind::Dir } else { Kind::File };
    let meta = de.metadata().ok();
    Entry {
        rel: name,
        abs: path,
        kind,
        size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
        mtime: meta.and_then(|m| m.modified().ok()),
    }
}

fn sort(entries: &mut [Entry], sort_by: SortKey, dirs_first: bool, reverse: bool) {
    entries.sort_by(|a, b| {
        let pa = a.kind == Kind::Dir;
        let pb = b.kind == Kind::Dir;
        let pa = if dirs_first { !pa } else { pa };
        let pb = if dirs_first { !pb } else { pb };
        pa.cmp(&pb)
            .then_with(|| match sort_by {
                SortKey::Name => a.rel.to_lowercase().cmp(&b.rel.to_lowercase()),
                SortKey::Size => b.size.cmp(&a.size),
                SortKey::Mtime => b.mtime.cmp(&a.mtime),
            })
            .then_with(|| a.rel.to_lowercase().cmp(&b.rel.to_lowercase()))
    });
    if reverse {
        entries.reverse();
    }
}

/// One-level listing of `dir`, dirs first then case-insensitive name.
/// Dotfiles skipped unless show_hidden.
/// Unreadable entries are skipped silently; unreadable dir -> empty Vec.
pub fn browse(
    dir: &Path,
    show_hidden: bool,
    sort_by: SortKey,
    dirs_first: bool,
    reverse: bool,
) -> Vec<Entry> {
    let Ok(iter) = dir.read_dir() else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = iter
        .filter_map(|r| r.ok())
        .filter_map(|de| {
            let name_str = de.file_name().to_string_lossy().to_string();
            if !show_hidden && name_str.starts_with('.') {
                return None;
            }
            Some(from_dir_entry(&de))
        })
        .collect();
    sort(&mut entries, sort_by, dirs_first, reverse);
    entries
}

/// Recursive listing under `dir` via `ignore::WalkBuilder`.
/// Capped at 100k entries to avoid pathological hangs.
pub fn browse_recursive(
    dir: &Path,
    show_hidden: bool,
    ignore_gitignore: bool,
    follow_links: bool,
    sort_by: SortKey,
    dirs_first: bool,
    reverse: bool,
) -> Vec<Entry> {
    let mut b = ignore::WalkBuilder::new(dir);
    b.hidden(!show_hidden)
        .git_ignore(!ignore_gitignore)
        .git_global(!ignore_gitignore)
        .git_exclude(!ignore_gitignore)
        .parents(true)
        .follow_links(follow_links)
        .filter_entry(|e| e.file_name() != ".git");
    let mut out: Vec<Entry> = Vec::new();
    for r in b.build() {
        if out.len() >= 100_000 {
            break;
        }
        let Ok(de) = r else {
            continue;
        };
        let abs = de.path().to_path_buf();
        if abs == dir {
            continue;
        }
        let rel = abs
            .strip_prefix(dir)
            .unwrap_or(&abs)
            .to_string_lossy()
            .to_string();
        let kind = if abs.is_dir() { Kind::Dir } else { Kind::File };
        let meta = de.metadata().ok();
        out.push(Entry {
            rel,
            abs,
            kind,
            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            mtime: meta.and_then(|m| m.modified().ok()),
        });
    }
    sort(&mut out, sort_by, dirs_first, reverse);
    out
}
