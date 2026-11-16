//! Login-shell environment setup.
//!
//! A login shell is expected to read the system profile files (`/etc/profile`
//! and `~/.profile`) so the session starts with the environment the system and
//! the user configured: `PATH` additions from `/etc/profile.d`, `export`s such
//! as `EDITOR` or `JAVA_HOME`, and so on.
//!
//! Arbor cannot evaluate those files itself. They are POSIX shell scripts, and
//! Arbor's language is not POSIX — `/etc/profile` on Arch, Debian and Fedora is
//! a chain of `if`/`case`/function definitions that no Nushell-syntax parser
//! can run. `login.nu` does not help either: it only runs *after* the profiles
//! would have, so a `PATH` that depends on them is already missing.
//!
//! So the profiles are evaluated by the system's POSIX `sh`, and the resulting
//! environment is imported into this process before any Arbor configuration is
//! read. That mirrors what bash and zsh do when they are login shells, and it
//! means `chsh -s .../arbor` gives a working session with no extra setup —
//! including on a distribution that ships Arbor as its default shell.
//!
//! Without this, a login session starts with only the `PATH` that `login(1)`
//! or the display manager provides (typically `/usr/bin:/bin`), so
//! `~/.local/bin`, `~/.cargo/bin`, Flatpak exports and anything a profile adds
//! are all missing.

use std::env;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

#[cfg(unix)]
// `process_group`, used below, is a `CommandExt` method on unix.
use std::os::unix::process::CommandExt;
#[cfg(unix)]
use nix::sys::signal::{Signal, kill};
#[cfg(unix)]
use nix::unistd::Pid;

/// A login shell must never hang: there is no shell left to escape from, so a
/// profile that blocks on a `read`, a network mount or a prompt would lock the
/// user out of their account. The profile shell is given this long, then killed
/// and its result discarded.
#[cfg(unix)]
const PROFILE_TIMEOUT: Duration = Duration::from_secs(5);

/// The files a POSIX login shell sources, in order.
///
/// Bash and zsh read these two. Each one is checked for readability before it
/// is sourced: `.` is a *special builtin*, and in a non-interactive `sh` a
/// failed special builtin makes the shell **exit**, so sourcing a missing file
/// would skip every profile after it — and silently, since the profiles'
/// stderr is not useful to show.
#[cfg(unix)]
const PROFILE_FILES: [&str; 2] = ["/etc/profile", "$HOME/.profile"];

/// Variables the profile shell necessarily reports with values that belong to
/// *it*, not to us. Importing them would corrupt Arbor's own state.
///
/// - `PWD` / `OLDPWD`: Arbor tracks the working directory itself and keeps
///   `$env.PWD` in sync with it; a stale value from a subshell would desync them.
/// - `SHLVL`: describes the profile shell's own nesting, not this process.
/// - `_`: the last argument of the profile shell's last command.
const NOT_INHERITED: [&str; 4] = ["PWD", "OLDPWD", "SHLVL", "_"];

/// Source `/etc/profile` then `~/.profile` and import the resulting
/// environment into this process.
///
/// Failures are deliberately silent: a broken or missing profile must not stop
/// the shell from starting, because starting is the only way to fix it.
#[cfg(unix)]
pub fn source_login_profiles() {
    let Ok(dump_path) = temp_dump_path() else {
        return;
    };
    let quoted = shell_quote(&dump_path.to_string_lossy());
    let script = profile_script(&quoted);

    // `mut` is needed here: the waiter thread calls `wait()`, which takes
    // `&mut self` on the child it owns.
    let Ok(mut child) = Command::new("sh")
        .arg("-c")
        .arg(&script)
        // Anything the profiles `read` from stdin gets EOF immediately instead
        // of blocking forever on a terminal that is not there yet.
        .stdin(Stdio::null())
        // Its own process group, so a timeout can kill the profiles *and* any
        // command they started. Killing only the shell would leave a grandchild
        // like `sleep` holding the terminal open.
        .process_group(0)
        .spawn()
    else {
        return;
    };

    // Wait for the profiles with a deadline, but without polling: sleeping in
    // 20 ms steps added that much dead time to every single login. The waiter
    // thread reaps the child in every case, including after a kill.
    let pid = Pid::from_raw(child.id() as i32);
    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = done_tx.send(child.wait());
    });

    if done_rx.recv_timeout(PROFILE_TIMEOUT).is_err() {
        // Timed out: kill the whole group, and leave the environment alone
        // rather than importing a half-built one.
        let _ = kill(Pid::from_raw(-pid.as_raw()), Signal::SIGKILL);
        let _ = std::fs::remove_file(&dump_path);
        return;
    }

    if let Ok(dump) = std::fs::read(&dump_path) {
        apply_dump(&dump);
    }
    let _ = std::fs::remove_file(&dump_path);
}

/// Build the `sh` script that evaluates the profiles and dumps the environment.
///
/// `set -a` marks plain assignments (`FOO=bar`) for export, so profiles that do
/// not say `export` still contribute — this is what bash does for a login shell.
/// Profile output is left on stdout/stderr so it reaches the terminal, as it
/// would under bash; only the env dump is redirected.
#[cfg(unix)]
fn profile_script(quoted_dump_path: &str) -> String {
    let mut script = String::from("set -a\n");
    for file in PROFILE_FILES {
        // `-r` guard, not `2>/dev/null`: see PROFILE_FILES for why a missing
        // file must be skipped rather than allowed to fail. Quoted so a home
        // directory containing spaces still works.
        script.push_str(&format!("[ -r \"{file}\" ] && . \"{file}\"\n"));
    }
    script.push_str(&format!(
        "set +a\nenv -0 > {quoted_dump_path} 2>/dev/null\n"
    ));
    script
}

/// Windows has no `/etc/profile`; this is the one OS where nothing is needed.
#[cfg(not(unix))]
pub fn source_login_profiles() {}

/// Import a NUL-separated `env -0` dump into this process's environment.
#[cfg(unix)]
fn apply_dump(dump: &[u8]) {
    for (name, value) in parse_env_dump(dump) {
        if NOT_INHERITED.contains(&name.as_str()) {
            continue;
        }
        // `set_var` is `unsafe` in edition 2024 because writing the environment
        // races with concurrent readers. This runs at the top of `main`, before
        // any thread, signal handler or logger exists, so there is no reader to
        // race with; every later read sees a fully populated environment.
        unsafe { env::set_var(&name, &value) };
    }
}

/// Parse a NUL-separated `env -0` dump into `(name, value)` pairs.
///
/// Kept separate from the process so it can be tested without touching the
/// real environment.
pub fn parse_env_dump(dump: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in dump.split(|&b| b == 0) {
        if entry.is_empty() {
            continue;
        }
        if let Some(eq) = entry.iter().position(|&b| b == b'=') {
            let name = String::from_utf8_lossy(&entry[..eq]);
            let value = String::from_utf8_lossy(&entry[eq + 1..]);
            if !name.is_empty() {
                out.push((name.into_owned(), value.into_owned()));
            }
        }
    }
    out
}

/// A per-process temp path for the environment dump.
#[cfg(unix)]
fn temp_dump_path() -> std::io::Result<std::path::PathBuf> {
    Ok(env::temp_dir().join(format!("arbor-login-env-{}", std::process::id())))
}

/// Single-quote a string for use inside a `sh -c` script.
#[cfg(unix)]
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nul_separated_dump() {
        let dump = b"PATH=/usr/bin:/bin\0HOME=/home/user\0SHELL=/bin/bash\0";
        assert_eq!(
            parse_env_dump(dump),
            vec![
                ("PATH".to_string(), "/usr/bin:/bin".to_string()),
                ("HOME".to_string(), "/home/user".to_string()),
                ("SHELL".to_string(), "/bin/bash".to_string()),
            ]
        );
    }

    #[test]
    fn keeps_equals_signs_in_values() {
        let dump = b"OPTS=a=b=c\0";
        assert_eq!(
            parse_env_dump(dump),
            vec![("OPTS".to_string(), "a=b=c".to_string())]
        );
    }

    #[test]
    fn keeps_empty_values() {
        let dump = b"EMPTY=\0NEXT=1\0";
        assert_eq!(
            parse_env_dump(dump),
            vec![
                ("EMPTY".to_string(), String::new()),
                ("NEXT".to_string(), "1".to_string()),
            ]
        );
    }

    #[test]
    fn skips_entries_without_an_equals_sign() {
        let dump = b"GOOD=1\0garbage\0";
        assert_eq!(
            parse_env_dump(dump),
            vec![("GOOD".to_string(), "1".to_string())]
        );
    }

    #[test]
    fn skips_empty_names_and_trailing_nul() {
        let dump = b"=nobo\0OK=1\0";
        assert_eq!(
            parse_env_dump(dump),
            vec![("OK".to_string(), "1".to_string())]
        );
    }

    #[test]
    fn empty_dump_yields_nothing() {
        assert!(parse_env_dump(b"").is_empty());
        assert!(parse_env_dump(b"\0\0\0").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn quotes_single_quotes_for_sh() {
        assert_eq!(shell_quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    /// Regression test: `.` is a special builtin, so sourcing a file that does
    /// not exist makes a non-interactive `sh` exit and silently skips every
    /// profile after it. Every source must therefore be guarded.
    #[cfg(unix)]
    #[test]
    fn every_profile_is_guarded_before_it_is_sourced() {
        let script = profile_script("'/tmp/dump'");
        let sources: Vec<&str> = script
            .lines()
            .filter(|line| line.contains(". ") && !line.trim_start().starts_with('#'))
            .collect();
        assert_eq!(sources.len(), PROFILE_FILES.len());
        for line in sources {
            assert!(
                line.trim_start().starts_with("[ -r "),
                "unguarded profile source would abort the profile shell: {line}"
            );
            assert!(
                line.contains("&& . "),
                "source must be conditional on the guard: {line}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn script_dumps_the_environment_after_sourcing() {
        let script = profile_script("'/tmp/dump'");
        let set_a = script.find("set -a").expect("export-all enabled");
        let set_plus_a = script.find("set +a").expect("export-all restored");
        let dump = script.find("env -0 > '/tmp/dump'").expect("dump written");
        assert!(set_a < set_plus_a, "set +a must follow set -a");
        assert!(
            set_plus_a < dump,
            "the dump must be taken after the profiles run"
        );
    }

    #[cfg(unix)]
    #[test]
    fn never_inherit_junk_variables() {
        // PWD/OLDPWD/SHLVL/_ describe the profile shell, not this process.
        assert!(NOT_INHERITED.contains(&"PWD"));
        assert!(NOT_INHERITED.contains(&"SHLVL"));
        assert!(NOT_INHERITED.contains(&"_"));
    }
}
