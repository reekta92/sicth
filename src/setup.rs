use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

const BASH_ZSH_TEMPLATE: &str = "\
# >>> sicth wrapper v2 >>>
{NAME}() {
    local tmp_file
    tmp_file=\"$(mktemp -t sicth.XXXXXX)\" || return 1
    {BIN} --out-file \"$tmp_file\" \"$@\"
    local ret=$?
    if [[ -s \"$tmp_file\" ]]; then
        . \"$tmp_file\"
    fi
    rm -f \"$tmp_file\"
    return $ret
}
# <<< sicth wrapper v2 <<<
";

const FISH_TEMPLATE: &str = "\
function {NAME}
    set -l tmp_file (mktemp -t sicth.XXXXXX)
    {BIN} --out-file \"$tmp_file\" $argv
    set -l ret $status
    if test -s \"$tmp_file\"
        source \"$tmp_file\"
    end
    rm -f \"$tmp_file\"
    return $ret
end
";

const MARKER_START: &str = "# >>> sicth wrapper v2 >>>";
const MARKER_END: &str = "# <<< sicth wrapper v2 <<<";
const LEGACY_START: &str = "# >>> sicth wrapper >>>";
const LEGACY_END: &str = "# <<< sicth wrapper <<<";

fn render_template(tmpl: &str, bin: &str, name: &str) -> String {
    tmpl.replace("{BIN}", bin).replace("{NAME}", name)
}

/// Byte range of the marker block (v2 preferred, legacy v1 accepted) for replace/detection.
fn find_marker_block(existing: &str) -> Option<(usize, usize)> {
    for (start_m, end_m) in [(MARKER_START, MARKER_END), (LEGACY_START, LEGACY_END)] {
        if let Some(start) = existing.find(start_m) {
            let end = existing[start..]
                .find(end_m)
                .map(|i| start + i + end_m.len())
                .unwrap_or(start);
            return Some((start, end));
        }
    }
    None
}

fn read_line() -> String {
    let mut s = String::new();
    let _ = io::stdin().read_line(&mut s);
    s.trim().to_string()
}

pub fn run() {
    let shell_path = env::var("SHELL").unwrap_or_default();
    let shell_name = Path::new(&shell_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let home = env::var("HOME").unwrap_or_else(|_| {
        eprintln!("sicth: HOME not set");
        process::exit(1);
    });

    let bin_path = match env::current_exe().and_then(|p| p.canonicalize()) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(e) => {
            eprintln!("sicth: cannot resolve binary path: {e}");
            process::exit(1);
        }
    };

    let (template, rc_path, is_fish) = match shell_name {
        "bash" => (
            BASH_ZSH_TEMPLATE,
            PathBuf::from(&home).join(".bashrc"),
            false,
        ),
        "zsh" => (
            BASH_ZSH_TEMPLATE,
            PathBuf::from(&home).join(".zshrc"),
            false,
        ),
        "fish" => (
            FISH_TEMPLATE,
            PathBuf::from(&home)
                .join(".config")
                .join("fish")
                .join("functions"),
            true,
        ),
        _ => {
            eprintln!("sicth: unsupported shell: {}", shell_path);
            eprintln!();
            eprintln!("Add the following function to your shell's rc file:");
            eprintln!();
            eprintln!("{}", render_template(BASH_ZSH_TEMPLATE, &bin_path, "sc"));
            eprintln!("Or for fish:");
            eprintln!("{}", render_template(FISH_TEMPLATE, &bin_path, "sc"));
            process::exit(1);
        }
    };

    // Resolve function name
    let mut func_name = "sc".to_string();
    let marker_present = if is_fish {
        let fish_file = rc_path.join(&func_name).with_extension("fish");
        fish_file.exists()
    } else {
        if let Ok(content) = fs::read_to_string(&rc_path) {
            find_marker_block(&content).is_some()
        } else {
            false
        }
    };

    if marker_present {
        let yn = prompt_confirm(&format!(
            "Existing sicth integration found in {}. Replace?",
            if is_fish {
                rc_path.join(&func_name).with_extension("fish").display().to_string()
            } else {
                rc_path.display().to_string()
            }
        ));
        if !yn {
            process::exit(0);
        }
    } else {
        // Check for name conflict
        let conflict = check_name_conflict(&func_name, &rc_path, is_fish);
        if let Some(what) = conflict {
            let choice = prompt_name_conflict(&func_name, &what);
            match choice {
                NameChoice::Shadow => { /* keep sc */ }
                NameChoice::Sicth => func_name = "sicth".to_string(),
                NameChoice::Abort => {
                    println!("Aborted.");
                    process::exit(1);
                }
            }
        }
    }

    // Confirm
    let rendered = render_template(template, &bin_path, &func_name);
    let target_display = if is_fish {
        rc_path.join(&func_name).with_extension("fish")
    } else {
        rc_path.clone()
    };

    println!("sicth setup — shell integration\n");
    println!("Detected shell:  {}", shell_name);
    println!("Target file:     {}", target_display.display());
    println!("Function name:   {}\n", func_name);
    println!("The following function will be appended:\n");
    println!("{}", rendered);
    println!("Why: a child process cannot change its parent shell's directory.");
    println!("The wrapper runs sicth and sources what it writes on exit: a cd into the last browsed directory, plus any !command you ran.\n");

    let yn = prompt_confirm("Proceed?");
    if !yn {
        println!("Aborted.");
        process::exit(1);
    }

    // Write
    if is_fish {
        if let Some(parent) = target_display.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&target_display, rendered.as_bytes()) {
            eprintln!("sicth: failed to write {}: {e}", target_display.display());
            process::exit(1);
        }
        println!("Done. Open a new shell to use {}.", func_name);
    } else {
        // bash/zsh: either replace marker block or append
        let existing = fs::read_to_string(&rc_path).unwrap_or_default();
        let new_content = if find_marker_block(&existing).is_some() {
            replace_marker_block(&existing, &rendered)
        } else {
            if !existing.is_empty() && !existing.ends_with('\n') {
                format!("{}\n{}", existing, rendered)
            } else {
                format!("{}{}", existing, rendered)
            }
        };
        if let Err(e) = fs::write(&rc_path, new_content.as_bytes()) {
            eprintln!("sicth: failed to write {}: {e}", rc_path.display());
            process::exit(1);
        }
        println!(
            "Done. Restart your shell or run: source {}",
            rc_path.display()
        );
    }
}

fn prompt_confirm(prompt: &str) -> bool {
    print!("{} [y/N] ", prompt);
    let _ = io::stdout().flush();
    let answer = read_line();
    answer == "y" || answer == "Y"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NameChoice {
    Shadow,
    Sicth,
    Abort,
}

fn check_name_conflict(name: &str, rc_path: &Path, is_fish: bool) -> Option<String> {
    // Check PATH for executable
    if let Ok(path_var) = env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = Path::new(dir).join(name);
            if candidate.is_file() {
                return Some(format!("executable at {}", candidate.display()));
            }
        }
    }
    // Check rc for alias/function
    if !is_fish {
        if let Ok(content) = fs::read_to_string(rc_path) {
            if content.contains(&format!("alias {}", name))
                || content.contains(&format!("{}()", name))
                || content.contains(&format!("function {}", name))
            {
                return Some(format!("function/alias in {}", rc_path.display()));
            }
        }
    }
    None
}

fn prompt_name_conflict(name: &str, what: &str) -> NameChoice {
    println!(
        "The name '{}' is already used by: {}\n\
         [s] install as '{}' anyway (shell functions shadow PATH binaries)\n\
         [i] install as 'sicth' instead\n\
         [a] abort",
        name, what, name
    );
    print!("Choice [s/i/a]: ");
    let _ = io::stdout().flush();
    match read_line().as_str() {
        "s" | "S" => NameChoice::Shadow,
        "i" | "I" => NameChoice::Sicth,
        _ => NameChoice::Abort,
    }
}

fn replace_marker_block(existing: &str, rendered: &str) -> String {
    let Some((start, end)) = find_marker_block(existing) else {
        if !existing.is_empty() && !existing.ends_with('\n') {
            return format!("{}\n{}", existing, rendered);
        }
        return format!("{}{}", existing, rendered);
    };
    let mut s = String::with_capacity(existing.len());
    s.push_str(&existing[..start]);
    s.push_str(rendered);
    if end < existing.len() {
        let rest = &existing[end..];
        let rest = rest.strip_prefix('\n').unwrap_or(rest);
        s.push_str(rest);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_template_substitutes_bin_and_name() {
        let result = render_template(BASH_ZSH_TEMPLATE, "/usr/bin/sicth", "sc");
        assert!(result.contains("/usr/bin/sicth"));
        assert!(result.contains("sc"));
        assert!(result.contains("# >>> sicth wrapper v2 >>>"));
        assert!(result.contains("# <<< sicth wrapper v2 <<<"));
    }

    #[test]
    fn replace_marker_block_swaps_marked_section() {
        let original = "echo hello\n# >>> sicth wrapper v2 >>>\nold stuff\n# <<< sicth wrapper v2 <<<\necho bye\n";
        let rendered = "# >>> sicth wrapper v2 >>>\nsc() { stuff }\n# <<< sicth wrapper v2 <<<\n";
        let result = replace_marker_block(original, rendered);
        assert_eq!(result, "echo hello\n# >>> sicth wrapper v2 >>>\nsc() { stuff }\n# <<< sicth wrapper v2 <<<\necho bye\n");
    }

    #[test]
    fn replace_marker_block_no_marker_appends() {
        let original = "echo hello\n";
        let rendered = "# >>> sicth wrapper v2 >>>\nsc() { stuff }\n# <<< sicth wrapper v2 <<<\n";
        let result = replace_marker_block(original, rendered);
        assert_eq!(result, "echo hello\n# >>> sicth wrapper v2 >>>\nsc() { stuff }\n# <<< sicth wrapper v2 <<<\n");
    }

    #[test]
    fn legacy_v1_block_is_replaced() {
        let original = "echo hello\n# >>> sicth wrapper >>>\nold stuff\n# <<< sicth wrapper <<<\necho bye\n";
        let rendered = "# >>> sicth wrapper v2 >>>\nsc() { stuff }\n# <<< sicth wrapper v2 <<<\n";
        let result = replace_marker_block(original, rendered);
        assert!(result.contains("# >>> sicth wrapper v2 >>>"));
        assert!(!result.contains("# >>> sicth wrapper >>>"));
        assert!(result.contains("# <<< sicth wrapper v2 <<<"));
        assert!(!result.contains("# <<< sicth wrapper <<<"));
    }
}
