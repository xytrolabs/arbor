//! Detection and balancing of multi-line bash control-flow blocks typed in
//! the REPL.
//!
//! Ported from Waash (Arbor's predecessor). Arbor already parses its own
//! language; the gap this fills is *bash* syntax: `if`/`for`/`while`/`case`,
//! `function` definitions and `{ ... }` groups. Those are collected as a
//! multi-line block and handed to the real `bash` interpreter, so muscle
//! memory from bash/zsh/fish keeps working instead of producing a Nu parse
//! error.
//!
//! This deliberately only *detects*; execution lives in the REPL, which has
//! access to the editor for continuation prompts.

/// Keywords that OPEN a bash control-flow block (increase nesting depth).
///
/// `then`/`do`/`else`/`elif` are NOT counted: they are continuations *inside*
/// an `if`/`for` block and do not need their own closer.
const BLOCK_OPENERS: &[&str] = &["if", "for", "while", "until", "case", "select"];

/// Keywords that CLOSE a bash control-flow block (decrease nesting depth).
const BLOCK_CLOSERS: &[&str] = &["fi", "done", "esac"];

/// Whether a line looks like a bash function definition: `name() {` or
/// `function name`.
fn looks_like_function_def(trimmed: &str) -> bool {
    if trimmed.starts_with("function ") {
        return true;
    }
    // name() { ... }
    if let Some(open) = trimmed.find("()") {
        let name = trimmed[..open].trim();
        // The part before `()` must be a valid identifier and a body must follow.
        return !name.is_empty()
            && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            && trimmed[open + 2..].contains('{');
    }
    false
}

/// Does `line` start a bash control-flow block that needs continuation?
///
/// Comments and blank input never start a block. A line that is only
/// whitespace is treated as "no block" so the REPL falls through to its
/// normal (no-op) path.
pub fn starts_bash_block(line: &str) -> bool {
    let t = line.trim_start();
    if t.is_empty() || t.starts_with('#') {
        return false;
    }
    if looks_like_function_def(t) {
        return true;
    }
    if t.starts_with("function ") || t.starts_with('{') {
        return true;
    }
    let first = t.split_whitespace().next().unwrap_or("");
    BLOCK_OPENERS.contains(&first)
}

/// Compute the control-flow nesting depth of a single line using a lightweight
/// word scan. Positive means more openers than closers (block still open).
///
/// This is intentionally a heuristic that over-counts rather than
/// under-counts: a real block is never split prematurely. Collecting one
/// extra line is harmless (bash just runs it); splitting a block early would
/// break it.
fn line_depth(line: &str) -> i32 {
    // Ignore trailing comments so `echo hi # done` doesn't close a block.
    let code = match line.find(" #") {
        Some(idx) => &line[..idx],
        None => line,
    };
    let mut depth = 0i32;
    for tok in code.split_whitespace() {
        match tok {
            t if BLOCK_OPENERS.contains(&t) => depth += 1,
            t if BLOCK_CLOSERS.contains(&t) => depth -= 1,
            "{" => depth += 1,
            "}" => depth -= 1,
            "[[" => depth += 1,
            "]]" => depth -= 1,
            _ => {}
        }
    }
    depth
}

/// Is the collected block of lines balanced (complete)?
///
/// `lines` must be non-empty. A single self-contained line (e.g.
/// `if x; then y; fi`) is complete; a block missing its terminator is not.
pub fn bash_block_done(lines: &[String]) -> bool {
    if lines.is_empty() {
        return false;
    }
    let mut depth = 0i32;
    for l in lines {
        depth += line_depth(l);
    }
    depth <= 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_openers() {
        assert!(starts_bash_block("if [ -f x ]; then"));
        assert!(starts_bash_block("for i in 1 2 3; do"));
        assert!(starts_bash_block("while true; do"));
        assert!(starts_bash_block("until false; do"));
        assert!(starts_bash_block("case $x in"));
        assert!(starts_bash_block("select x in a b; do"));
        assert!(starts_bash_block("function foo {"));
        assert!(starts_bash_block("myfunc() {"));
        assert!(starts_bash_block("my-func() {"));
        assert!(starts_bash_block("{ echo hi;"));
    }

    #[test]
    fn non_openers_do_not_start_blocks() {
        assert!(!starts_bash_block("echo hello"));
        assert!(!starts_bash_block("ls -la"));
        assert!(!starts_bash_block("git status"));
        // `[[ ]]` is a single-line test, not a block.
        assert!(!starts_bash_block("[[ -f x ]]"));
        // Comments and blank lines.
        assert!(!starts_bash_block("# if this were code"));
        assert!(!starts_bash_block("   "));
        assert!(!starts_bash_block(""));
        // A Nu command that merely begins with a similar word.
        assert!(!starts_bash_block("ifmt"));
    }

    #[test]
    fn one_line_complete_blocks_are_done() {
        assert!(bash_block_done(&["if true; then echo hi; fi".to_string()]));
        assert!(bash_block_done(&["for i in 1 2 3; do echo $i; done".to_string()]));
    }

    #[test]
    fn multi_line_needs_terminator() {
        assert!(!bash_block_done(&["if [ -f x ]; then".to_string()]));

        assert!(bash_block_done(&[
            "if [ -f x ]; then".to_string(),
            "  echo hi".to_string(),
            "fi".to_string()
        ]));

        assert!(bash_block_done(&[
            "for i in 1 2 3; do".to_string(),
            "  echo $i".to_string(),
            "done".to_string()
        ]));

        // Nested: open 2, close 2.
        assert!(bash_block_done(&[
            "if true; then".to_string(),
            "  for i in 1 2; do".to_string(),
            "    echo $i".to_string(),
            "  done".to_string(),
            "fi".to_string()
        ]));

        // Nested but missing the outer `fi`.
        assert!(!bash_block_done(&[
            "if true; then".to_string(),
            "  for i in 1 2; do".to_string(),
            "    echo $i".to_string(),
            "  done".to_string(),
        ]));
    }

    #[test]
    fn function_definition() {
        assert!(bash_block_done(&[
            "greet() {".to_string(),
            "  echo hi".to_string(),
            "}".to_string()
        ]));
        assert!(!bash_block_done(&["greet() {".to_string()]));
        assert!(starts_bash_block("greet() {"));
    }

    #[test]
    fn trailing_comment_does_not_close_block() {
        // The word `done` inside a comment must not close the block.
        assert!(!bash_block_done(&[
            "for i in 1 2; do".to_string(),
            "  echo $i # done later".to_string(),
        ]));
    }
}

// ── Execution ────────────────────────────────────────────────────────────

/// The continuation prompt shown while collecting a multi-line block.
pub const CONTINUATION_PROMPT: &str = "… ";

/// Run `code` through `bash -c`, inheriting stdio, and return the exit code.
///
/// Failure to spawn bash is reported and treated as exit code 1 rather than
/// surfacing as a Nu parse error.
pub fn run_bash(code: &str) -> i32 {
    match std::process::Command::new("bash").arg("-c").arg(code).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!("arbor: failed to run bash: {err}");
            1
        }
    }
}

/// Function definitions collected from previously run blocks.
///
/// Each block runs in a fresh `bash -c`, so a `name() { ... }` defined in one
/// submission would otherwise be gone by the next. Definitions are remembered
/// here and prepended to every later block so they behave like a persistent
/// shell session.
static DEFINITIONS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Run `code` in a bash shell that also has every previously collected function
/// definition available. Returns bash's exit code.
pub fn run_bash_with_definitions(code: &str) -> i32 {
    let defs = {
        let guard = match DEFINITIONS.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.join("\n")
    };

    let full = if defs.is_empty() {
        code.to_string()
    } else {
        format!("{defs}\n{code}")
    };
    run_bash(&full)
}

/// Remember a function definition so later blocks can call it.
///
/// `block` is the collected definition text (e.g. `f() { ... }`). Only complete
/// definitions are stored; the text is appended verbatim.
pub fn remember_definition(block: &str) {
    let trimmed = block.trim();
    if !trimmed.contains('{') || !trimmed.ends_with('}') {
        return;
    }
    let mut guard = match DEFINITIONS.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    if !guard.iter().any(|d| d == trimmed) {
        guard.push(trimmed.to_string());
    }
}

/// Whether a collected block is a function definition (as opposed to a loop or
/// conditional, which must not be replayed on later submissions).
pub fn is_function_definition(first_line: &str) -> bool {
    let t = first_line.trim_start();
    t.starts_with("function ") || (t.contains("()") && t.contains('{'))
}

/// The short name of a remembered function definition, e.g. `f` for `f() { ... }`.
fn definition_name(def: &str) -> Option<&str> {
    let first = def.lines().next()?.trim_start();
    if let Some(rest) = first.strip_prefix("function ") {
        return rest.split_whitespace().next();
    }
    let open = first.find("()")?;
    let name = first[..open].trim();
    if name.is_empty() { None } else { Some(name) }
}

/// Whether `command` invokes a bash function defined earlier in the session.
///
/// Lets `f` typed at the prompt reach the remembered `f() { ... }` instead of
/// failing as an unknown Nu command.
pub fn is_remembered_function_call(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Only a bare command line: no pipes, redirections, or Nu syntax.
    if trimmed.contains(['|', ';', '<', '>']) {
        return false;
    }
    let name = trimmed.split_whitespace().next().unwrap_or("");

    let guard = match DEFINITIONS.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard
        .iter()
        .filter_map(|d| definition_name(d))
        .any(|n| n == name)
}

/// Handle one submitted REPL line that starts a bash control-flow block.
///
/// If the line is already complete (`if x; then y; fi`) it runs immediately.
/// Otherwise `next_line` is called to read continuation lines until the block
/// balances, then the whole block is handed to `bash`.
///
/// `next_line` returns `None` when the user aborted (Ctrl+C / Ctrl+D); in that
/// case the partially collected block is discarded and never executed.
///
/// Returns the exit code of the block (or 130 when aborted).
pub fn run_bash_line(
    first: &str,
    mut next_line: impl FnMut(&str) -> Option<String>,
) -> i32 {
    let mut block: Vec<String> = vec![first.to_string()];

    if !bash_block_done(&block) {
        loop {
            let Some(line) = next_line(CONTINUATION_PROMPT) else {
                // Aborted: never run a half-collected block.
                return 130;
            };
            if line.trim().is_empty() {
                continue;
            }
            block.push(line.trim_end().to_string());
            if bash_block_done(&block) {
                break;
            }
        }
    }

    let code = block.join("\n");
    let exit_code = run_bash_with_definitions(&code);

    // Keep function definitions so a later `f` call still finds them.
    if is_function_definition(first) {
        remember_definition(&code);
    }

    exit_code
}

/// Read one continuation line straight from the terminal.
///
/// Re-entering `reedline` from inside the REPL's command handler deadlocks,
/// because the outer loop already owns the editor for this input. Reading the
/// raw line works for the common cases (typing or pasting a block); there is no
/// completion or history on continuation lines.
///
/// Returns `None` on EOF, Ctrl+C, or a read error so the caller aborts the block
/// rather than executing something half-formed.
pub fn read_continuation_line(prompt: &str) -> Option<String> {
    use std::io::{BufRead, Write};

    // Show the continuation prompt so the user knows the block is incomplete.
    {
        let mut out = std::io::stdout();
        let _ = write!(out, "{prompt}");
        let _ = out.flush();
    }

    let stdin = std::io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None, // EOF / Ctrl+D
        Ok(_) => {
            let trimmed = line.trim_end_matches(['\n', '\r']);
            // A bare Ctrl+C arrives as ETX in the buffered line.
            if trimmed.contains('\u{3}') {
                return None;
            }
            Some(trimmed.to_string())
        }
        Err(_) => None,
    }
}
