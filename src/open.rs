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

/// Protocol v2: the out-file is a shell script the wrapper sources.
/// Always starts with the cd so every quit path lands the shell in the browsed dir;
/// `command` (a `!command` payload) is appended verbatim for the interactive shell to run.
pub fn write_out_script(out: &Option<PathBuf>, dir: &Path, command: Option<&str>) {
    let Some(ref path) = out else { return };
    let mut content = format!("cd {}\n", shell_quote(&dir.to_string_lossy()));
    if let Some(cmd) = command {
        content.push_str(cmd);
        content.push('\n');
    }
    let _ = fs::write(path, content.as_bytes());
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
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

    #[test]
    fn script_dir_only_is_cd_line() {
        use std::io::Read;
        let dir = std::env::temp_dir();
        let c = COUNTER.fetch_add(1, Ordering::Relaxed);
        let out = Some(dir.join(format!("sicth_open_test_{}_{}", std::process::id(), c)));
        write_out_script(&out, Path::new("/tmp/xyz"), None);
        let content = fs::read_to_string(out.as_ref().unwrap()).unwrap();
        assert_eq!(content, "cd '/tmp/xyz'\n");
        let _ = fs::remove_file(out.as_ref().unwrap());
    }

    #[test]
    fn script_with_command_appends_verbatim() {
        let dir = std::env::temp_dir();
        let c = COUNTER.fetch_add(1, Ordering::Relaxed);
        let out = Some(dir.join(format!("sicth_open_test_{}_{}", std::process::id(), c)));
        write_out_script(&out, Path::new("/tmp/xyz"), Some("touch a b"));
        let content = fs::read_to_string(out.as_ref().unwrap()).unwrap();
        assert!(content.contains("touch a b"));
        let _ = fs::remove_file(out.as_ref().unwrap());
    }

    #[test]
    fn shell_quote_escapes_single_quotes() {
        let result = shell_quote("a'b");
        assert_eq!(result, "'a'\\''b'");
    }
}
