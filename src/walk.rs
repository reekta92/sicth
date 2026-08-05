use std::cmp::Ordering;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::thread::JoinHandle;

use ignore::WalkBuilder;
use nucleo::Injector;

use crate::model::{Entry, Kind};

pub struct WalkHandle {
    pub stop: Arc<AtomicBool>,
    #[allow(dead_code)]
    pub thread: JoinHandle<()>,
}

/// Streams every entry under `root` into `injector` until exhausted or `stop` is set.
pub fn spawn_walker(
    root: PathBuf,
    show_hidden: bool,
    ignore_gitignore: bool,
    follow_links: bool,
    injector: Injector<Entry>,
) -> WalkHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    let thread = std::thread::spawn(move || {
        let mut b = WalkBuilder::new(&root);
        b.hidden(!show_hidden)
            .git_ignore(!ignore_gitignore)
            .git_global(!ignore_gitignore)
            .git_exclude(!ignore_gitignore)
            .parents(true)
            .follow_links(follow_links)
            .sort_by_file_name(|_, _| Ordering::Equal)
            .filter_entry(|entry| entry.file_name() != ".git");

        let walker = b.build();

        for result in walker {
            if stop_clone.load(AtomicOrdering::Relaxed) {
                break;
            }
            match result {
                Ok(entry) => {
                    let meta = entry.metadata().ok();
                    let abs = entry.into_path();
                    let rel = abs
                        .strip_prefix(&root)
                        .unwrap_or(&abs)
                        .to_string_lossy()
                        .to_string();
                    let kind = if abs.is_dir() { Kind::Dir } else { Kind::File };
                    let e = Entry {
                        rel,
                        abs,
                        kind,
                        size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                        mtime: meta.and_then(|m| m.modified().ok()),
                    };
                    injector.push(e, |entry, cols| {
                        cols[0] = entry.rel.clone().into();
                    });
                }
                Err(_) => {
                    // Permission errors etc. — skip silently
                }
            }
        }
        // injector dropped here → nucleo signals end-of-stream
    });

    WalkHandle { stop, thread }
}
