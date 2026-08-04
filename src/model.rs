use std::path::{Path, PathBuf};

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
}

/// One-level listing of `dir`, dirs first then case-insensitive name.
/// Dotfiles skipped unless show_hidden.
/// Unreadable entries are skipped silently; unreadable dir -> empty Vec.
pub fn browse(dir: &Path, show_hidden: bool) -> Vec<Entry> {
    let Ok(iter) = dir.read_dir() else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = iter
        .filter_map(|r| r.ok())
        .filter_map(|de| {
            let name = de.file_name();
            let name_str = name.to_string_lossy().to_string();
            if !show_hidden && name_str.starts_with('.') {
                return None;
            }
            let path = de.path();
            let kind = if path.is_dir() { Kind::Dir } else { Kind::File };
            Some(Entry {
                rel: name_str,
                abs: path,
                kind,
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        (b.kind == Kind::Dir)
            .cmp(&(a.kind == Kind::Dir))
            .then_with(|| a.rel.to_lowercase().cmp(&b.rel.to_lowercase()))
    });
    entries
}
