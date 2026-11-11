//! Indent runtime integration for Arbor.
//!
//! Ported from Waash. Arbor's own language (Nu) remains the default; this
//! module adds support for scripts written in **Indent**, a simpler scripting
//! language (`.ind` / `.arbor` files), plus a small helper library that lets
//! Indent scripts run shell commands and read/write files concisely.
//!
//! Design notes:
//! * Arbor has no `dirs` dependency, so paths are derived from `std` +
//!   `$HOME`/`$XDG_DATA_HOME`. This mirrors `nu-config`'s resolution closely
//!   enough for the helper library, without introducing a new dependency.
//! * Every failure path returns an error instead of panicking (Arbor's
//!   AGENTS.md forbids `unwrap()` outside tests).

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Env var overriding the Indent binary location.
pub const INDENT_BINARY_ENV: &str = "ARBOR_INDENT_BINARY";
/// Env var overriding the helper library directory.
pub const INDENT_LIB_ENV: &str = "ARBOR_INDENT_LIB";

/// User's home directory, from `$HOME` (Linux) or `%USERPROFILE%`.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// The user data directory (`$XDG_DATA_HOME` or `~/.local/share`).
pub fn data_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        if xdg.is_absolute() {
            return Some(xdg);
        }
    }
    home_dir().map(|h| h.join(".local/share"))
}

/// Candidate helper-library directory names, newest first.
///
/// `arbor` is preferred; `waash`/`nushell` are checked so an existing Waash
/// install keeps working after the rename (same migration story as the config
/// directories).
const LIB_DIR_NAMES: &[&str] = &["arbor", "waash"];

/// Find the Indent runtime binary.
///
/// Search order:
/// 1. `$ARBOR_INDENT_BINARY`
/// 2. the `indent` found on `PATH`
/// 3. `~/.local/bin/indent`
/// 4. `~/.local/share/indent/bin/indent` (the upstream installer's location)
pub fn find_indent_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(INDENT_BINARY_ENV) {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Some(p);
        }
    }

    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join("indent");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    let home = home_dir()?;
    for rel in [".local/bin/indent", ".local/share/indent/bin/indent"] {
        let candidate = home.join(rel);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

/// Find the directory containing the Arbor helper library (`arbor.ind`).
///
/// Checked in order: `$ARBOR_INDENT_LIB`, next to the executable
/// (`../share/arbor`), the current directory (`share/arbor`, dev mode), then
/// the installed data directory (`~/.local/share/arbor/lib`). A legacy
/// `waash` directory is accepted as a fallback.
pub fn find_helper_lib() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(INDENT_LIB_ENV) {
        let p = PathBuf::from(dir);
        if p.join("arbor.ind").is_file() || p.join("waash.ind").is_file() {
            return Some(p);
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1. Alongside the executable: `<prefix>/bin/arbor` -> `<prefix>/share/arbor`.
    if let Ok(exe) = std::env::current_exe() {
        let exe = exe.canonicalize().unwrap_or(exe);
        if let Some(bin_dir) = exe.parent() {
            if let Some(prefix) = bin_dir.parent() {
                candidates.push(prefix.join("share/arbor"));
            }
        }
    }

    // 2. Dev checkout: walk up from the current directory looking for
    //    `share/arbor`, so `cargo run` and `./target/debug/arbor` both work.
    if let Ok(cwd) = std::env::current_dir() {
        for dir in cwd.ancestors() {
            candidates.push(dir.join("share/arbor"));
        }
    }

    // 3. Installed data directory (`~/.local/share/arbor[/lib]`), plus the
    //    legacy Waash location for existing installs.
    if let Some(data) = data_dir() {
        for name in LIB_DIR_NAMES {
            candidates.push(data.join(name).join("lib"));
            candidates.push(data.join(name));
        }
    }

    candidates.into_iter().find(|dir| {
        dir.join("arbor.ind").is_file() || dir.join("waash.ind").is_file()
    })
}

/// How a script should be run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    /// Hand to the Indent runtime (`.ind`, `.arbor`, `.waash`).
    Indent,
    /// Hand to `bash` (`.sh`, `.bash`, executable scripts with a shebang).
    Bash,
    /// Let Arbor parse it itself (everything else).
    Native,
}

/// Classify a script path.
pub fn script_kind(path: &Path) -> ScriptKind {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ind" | "arbor" | "waash") => ScriptKind::Indent,
        Some("sh" | "bash" | "zsh" | "fish") => ScriptKind::Bash,
        _ => ScriptKind::Native,
    }
}

/// Keywords and helper names that mark a line as Indent rather than a bare
/// shell command.
///
/// Kept aligned with Indent 2.4.x. Used only for *classification*, never to
/// rewrite code, so a wrong guess degrades politely.
pub const INDENT_KEYWORDS: &[&str] = &[
    "fun ", "var ", "if ", "or ", "otherwise", "repeat ", "for ", "say ", "give ", "get ",
    "import ", "class ", "match ", "do:", "catch ", "lastly:", "flag ", "while ", "stop", "next",
    "reset", "include ", "assert ", "#!", "set ", "contains ", "process_exit ", "os_system ",
    "os_exists ", "os_getenv ", "os_setenv ", "os_remove ", "file_read_text ", "file_write_text ",
    "file_append_text ", "time_perf_counter ", "time_now ", "time_sleep ", "json_loads ",
    "json_dumps ", "keys ", "has_key ", "slice ", "trim ", "starts_with ", "ends_with ",
    "string ", "int ", "float ", "boolean ", "call_func ",
    // Arbor helper functions from arbor.ind (auto-imported by `preprocess`).
    // These must be listed here or a bare `header "x"` line is mistaken for a
    // shell command and wrapped in `sh(...)`.
    "header ", "success ", "error ", "info ", "sh ", "sh_capture ", "has_command ",
    "env ", "set_env ", "exists ", "read_file ", "write_file ", "append_file ",
    "require_ok ",
];

/// Whether a single trimmed line is an Indent construct.
pub fn is_indent_line(trimmed: &str) -> bool {
    INDENT_KEYWORDS
        .iter()
        .any(|kw| trimmed.starts_with(kw))
        || is_comprehension(trimmed)
        || is_assignment(trimmed)
}

/// Whether a line is an Indent assignment/reassignment: `x = 1`, `x is 1`.
///
/// Indented assignments inside a `for`/`if` body are extremely common
/// (`total = total + n`) and would otherwise be wrapped as shell commands.
/// A shell command never starts with `<identifier> = `, so this is safe.
fn is_assignment(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    let Some(first) = parts.next() else {
        return false;
    };
    // The target must be a plain identifier (optionally indexed).
    if first.is_empty()
        || !first
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !first
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '[' || c == ']')
    {
        return false;
    }
    matches!(parts.next(), Some("=") | Some("is"))
}

/// Indent list/dict comprehensions: `[x * 2 for x in nums]`.
fn is_comprehension(line: &str) -> bool {
    let l = line.trim_start();
    (l.starts_with('[') || l.starts_with('{')) && l.contains(" for ") && l.contains(" in ")
}

/// Does this source text look like an Indent script rather than a shell script
/// or Nu code?
///
/// Used for the `-c`/stdin path, where there is no file extension to consult.
/// Requires a *strong* signal to avoid hijacking ordinary shell one-liners.
pub fn looks_like_indent(source: &str) -> bool {
    let mut indent_lines = 0usize;
    let mut other_lines = 0usize;

    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("#!") {
            continue;
        }
        if is_indent_line(line) {
            indent_lines += 1;
        } else {
            other_lines += 1;
        }
    }

    indent_lines > 0 && indent_lines >= other_lines
}

/// Preprocess an Indent script before handing it to the runtime.
///
/// Adds the helper-library import (when found) and wraps bare shell-command
/// lines in `sh("...")` so a script can freely mix Indent and shell commands.
///
/// Indentation is preserved: a bare command inside a `fun` body becomes
/// `    sh("...")` at the same depth, so it stays inside the function.
/// Lines that already look like Indent, or that call a function defined in the
/// script, are left untouched.
pub fn preprocess(source: &str) -> String {
    let func_names = collect_function_names(source);

    let mut out = String::with_capacity(source.len() + 256);
    out.push_str("#! Auto-imported by Arbor\n");
    for helper in ["sh", "sh_capture", "has_command", "header", "success", "error", "info"] {
        out.push_str(&format!("get {helper} from arbor\n"));
    }
    out.push('\n');

    for raw in source.lines() {
        let line = raw.trim();

        if line.is_empty() {
            out.push('\n');
            continue;
        }

        let is_indent = is_indent_line(line)
            || func_names
                .iter()
                .any(|name| line == name || line.starts_with(&format!("{name} ")));

        if is_indent {
            out.push_str(raw);
            out.push('\n');
        } else {
            let indent = &raw[..raw.len() - raw.trim_start().len()];
            let escaped = line.replace('\\', "\\\\").replace('"', "\\\"");
            out.push_str(&format!("{indent}sh(\"{escaped}\")\n"));
        }
    }

    out
}

/// Collect names declared with `fun NAME ...`.
fn collect_function_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in source.lines() {
        if let Some(rest) = line.trim().strip_prefix("fun ") {
            if let Some(name) = rest.split_whitespace().next() {
                // Strip a trailing `:` from older `fun name:` syntax.
                let name = name.trim_end_matches(':');
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

/// Hand a source file to the Indent runtime, inheriting stdio.
///
/// `preprocess` is applied first so plain shell lines work inside Indent
/// scripts. Returns the runtime's exit code.
pub fn run_indent_file(indent_bin: &Path, script: &Path, lib_dir: Option<&Path>) -> std::io::Result<i32> {
    let source = std::fs::read_to_string(script)
        .map_err(|e| io::Error::other(format!("failed to read {}: {e}", script.display())))?;
    run_indent_source(indent_bin, &source, lib_dir)
}

/// Hand Indent source to the runtime, inheriting stdio.
pub fn run_indent_source(
    indent_bin: &Path,
    source: &str,
    lib_dir: Option<&Path>,
) -> std::io::Result<i32> {
    let processed = preprocess(source);

    let tmp_path = std::env::temp_dir().join(format!("arbor_indent_{}.ind", std::process::id()));
    std::fs::write(&tmp_path, &processed)
        .map_err(|e| io::Error::other(format!("failed to write temp script: {e}")))?;

    let mut cmd = Command::new(indent_bin);
    cmd.arg("run").arg(&tmp_path);

    if let Some(dir) = lib_dir {
        // Make `get ... from arbor` resolvable.
        let mut path = dir.to_string_lossy().to_string();
        if let Some(existing) = std::env::var_os("INDENT_PATH") {
            path.push(':');
            path.push_str(&existing.to_string_lossy());
        }
        cmd.env("INDENT_PATH", path);
    }

    let output = cmd
        .output()
        .map_err(|e| io::Error::other(format!("failed to run indent ({}): {e}", indent_bin.display())))?;

    let _ = std::fs::remove_file(&tmp_path);

    use std::io::Write;
    let _ = std::io::stdout().write_all(&output.stdout);
    let _ = std::io::stderr().write_all(&output.stderr);

    Ok(output.status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_scripts_by_extension() {
        assert_eq!(script_kind(Path::new("a.ind")), ScriptKind::Indent);
        assert_eq!(script_kind(Path::new("a.arbor")), ScriptKind::Indent);
        assert_eq!(script_kind(Path::new("a.waash")), ScriptKind::Indent);
        assert_eq!(script_kind(Path::new("a.sh")), ScriptKind::Bash);
        assert_eq!(script_kind(Path::new("a.bash")), ScriptKind::Bash);
        assert_eq!(script_kind(Path::new("a.nu")), ScriptKind::Native);
        assert_eq!(script_kind(Path::new("a")), ScriptKind::Native);
    }

    #[test]
    fn detects_indent_lines() {
        assert!(is_indent_line("fun main"));
        assert!(is_indent_line("var x = 42"));
        assert!(is_indent_line("if x > 1"));
        assert!(is_indent_line("say \"hi\""));
        assert!(is_indent_line("[x * 2 for x in nums]"));
    }

    #[test]
    fn shell_lines_are_not_indent() {
        assert!(!is_indent_line("cargo build --release"));
        assert!(!is_indent_line("ls -la"));
        assert!(!is_indent_line("git status"));
    }

    #[test]
    fn looks_like_indent_needs_a_signal() {
        assert!(looks_like_indent("fun main\n  say \"hi\"\n"));
        assert!(!looks_like_indent("ls -la\ngit status\n"));
        assert!(!looks_like_indent(""));
    }

    #[test]
    fn preprocess_wraps_bare_commands() {
        let out = preprocess("cargo build --release\n");
        assert!(out.contains(r#"sh("cargo build --release")"#));
        // The helper imports are always present.
        assert!(out.contains("get sh from arbor"));
    }

    #[test]
    fn preprocess_leaves_indent_alone() {
        let out = preprocess("fun greet\n  say \"hi\"\n");
        assert!(out.contains("fun greet"));
        assert!(!out.contains(r#"sh("fun greet")"#));
    }

    #[test]
    fn preprocess_respects_user_functions() {
        let out = preprocess("fun build\n  say \"b\"\nbuild arg\n");
        assert!(out.contains("\nbuild arg\n"));
    }

    #[test]
    fn preprocess_escapes_quotes() {
        let out = preprocess("echo \"hello world\"\n");
        assert!(out.contains(r#"sh("echo \"hello world\"")"#));
    }
}
