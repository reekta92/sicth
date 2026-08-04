use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    Editor,
    System,
}

pub fn classify(path: &Path) -> How {
    match infer::get_from_path(path) {
        Ok(Some(_)) => How::System,
        Ok(None) => {
            if has_nul(path) {
                How::System
            } else {
                How::Editor
            }
        }
        Err(_) => How::System,
    }
}

fn has_nul(path: &Path) -> bool {
    let Ok(mut f) = fs::File::open(path) else {
        return true;
    };
    let mut buf = [0u8; 8192];
    match f.read(&mut buf) {
        Ok(n) => buf[..n].contains(&0),
        Err(_) => true,
    }
}

pub fn resolve_editor() -> (String, Vec<String>) {
    for var in ["VISUAL", "EDITOR"] {
        if let Ok(val) = env::var(var) {
            if !val.is_empty() {
                let mut parts = val.split_whitespace();
                let prog = parts.next().unwrap_or("vi").to_string();
                let args: Vec<String> = parts.map(String::from).collect();
                return (prog, args);
            }
        }
    }
    ("vi".to_string(), Vec::new())
}

pub fn write_out_file(out: &Option<PathBuf>, dir: &Path) {
    if let Some(ref path) = out {
        let _ = fs::write(path, dir.to_string_lossy().as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn tmp_file(content: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir();
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = dir.join(format!("sicth_test_{}_{}", std::process::id(), id));
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    #[test]
    fn classify_text_as_editor() {
        let p = tmp_file(b"hello world\n");
        assert_eq!(classify(&p), How::Editor);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn classify_png_as_system() {
        let p = tmp_file(b"\x89PNG\r\n\x1a\nsome data");
        assert_eq!(classify(&p), How::System);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn classify_nul_as_system() {
        let p = tmp_file(b"a\0b");
        assert_eq!(classify(&p), How::System);
        let _ = fs::remove_file(&p);
    }
}
