//! Ctrl+B "move the running foreground command to the background".
//!
//! Ported from Waash's `wait_foreground` / `set_susp_char` machinery, adapted
//! to Arbor's `ForegroundChild` job-control model.
//!
//! # Why this is not just SIGTSTP
//!
//! Arbor (like Waash before it) already supports `Ctrl+Z` suspension followed
//! by `job unfreeze`. That requires two keystrokes and leaves the job frozen.
//! Ctrl+B makes it one keystroke and leaves the job *running* in the
//! background.
//!
//! # Two detection paths
//!
//! 1. **VSUSP remap.** For a foreground child holding the terminal, the
//!    terminal's suspend character is temporarily remapped to the shortcut
//!    byte, so pressing it makes the tty send `SIGTSTP` to the child directly
//!    (the child then reports `WaitStatus::Stopped`).
//! 2. **Direct tty read.** Some children (notably `bash` running a script)
//!    reset the terminal's control characters, defeating path 1. So the wait
//!    loop also polls stdin and reads the byte itself.
//!
//! The remap is **deferred** until the command has been running for
//! [`REMAP_DELAY`]: `tcsetattr` on a fast command raced with its output and
//! ate output in Waash, so short commands must never trigger it.

use std::io::IsTerminal;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

/// Default shortcut: `Ctrl+B` — ETX-adjacent control byte `0x02`.
pub const DEFAULT_BG_KEY: u8 = 0x02;

/// How long a command must run before the VSUSP remap is applied.
///
/// Below this, the command cannot realistically be backgrounded by a human
/// yet, and touching termios could race with its output.
pub const REMAP_DELAY: Duration = Duration::from_millis(300);

/// Poll interval while waiting for a foreground child.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// The terminal's original suspend character, remembered so it can be
/// restored. Process-global because termios state is process-global.
static ORIG_SUSP: Mutex<Option<u8>> = Mutex::new(None);

/// Whether the VSUSP remap is currently installed.
static REMAP_ACTIVE: AtomicBool = AtomicBool::new(false);

/// The configured shortcut byte.
static BG_KEY: AtomicU8 = AtomicU8::new(DEFAULT_BG_KEY);

/// Set the shortcut byte from a config string such as `"Ctrl+B"`.
///
/// Accepts `Ctrl+<a-z>` (case-insensitive). Anything unrecognised falls back
/// to the default so a bad config value can never disable the feature
/// silently.
pub fn set_shortcut(spec: &str) {
    let byte = spec
        .strip_prefix("Ctrl+")
        .or_else(|| spec.strip_prefix("ctrl+"))
        .and_then(|rest| rest.chars().next())
        .map(|c| c.to_ascii_lowercase())
        .filter(|c| c.is_ascii_lowercase())
        .map(|c| c as u8 - b'a' + 1)
        .unwrap_or(DEFAULT_BG_KEY);
    BG_KEY.store(byte, Ordering::Relaxed);
}

/// The currently configured shortcut byte.
pub fn shortcut() -> u8 {
    BG_KEY.load(Ordering::Relaxed)
}

/// The polling interval used while waiting for a foreground child.
pub fn poll_interval() -> std::time::Duration {
    POLL_INTERVAL
}

/// Set once the shortcut byte has been read from the terminal, so the caller
/// can tell "Ctrl+B" apart from a plain `Ctrl+Z` stop.
static SHORTCUT_PRESSED: AtomicBool = AtomicBool::new(false);

/// Consume the "shortcut pressed" flag. Returns `true` if the shortcut was
/// pressed since the last call.
pub fn take_shortcut_press() -> bool {
    SHORTCUT_PRESSED.swap(false, Ordering::SeqCst)
}

/// Poll the terminal for the shortcut byte without blocking.
///
/// Returns `true` (and records the press) when the configured byte is read.
/// Any other pending byte is consumed and discarded: the foreground child owns
/// the terminal at this point, so it could not have used it anyway.
pub fn poll_shortcut() -> bool {
    if !supported() || !tty_readable() {
        return false;
    }
    let Some(byte) = read_tty_byte() else {
        return false;
    };
    if byte == shortcut() {
        SHORTCUT_PRESSED.store(true, Ordering::SeqCst);
        true
    } else {
        false
    }
}

/// Set once a foreground child was moved to the background with the shortcut.
///
/// The pipeline that spawned the child reads this so it does not block
/// draining output from a process that is deliberately still running.
static BACKGROUNDED: AtomicBool = AtomicBool::new(false);

/// Record that the current foreground child moved to the background.
pub fn mark_backgrounded() {
    BACKGROUNDED.store(true, Ordering::SeqCst);
}

/// Whether a child was backgrounded (and the flag has not been cleared).
pub fn was_backgrounded() -> bool {
    BACKGROUNDED.load(Ordering::SeqCst)
}

/// Clear the backgrounded flag before starting a new foreground command.
pub fn clear_backgrounded() {
    BACKGROUNDED.store(false, Ordering::SeqCst);
}

/// Whether the VSUSP remap is currently installed.
///
/// When it is, a `WaitStatus::Stopped` on a foreground child means the user
/// pressed the shortcut (the tty turned that key into SIGTSTP), not Ctrl+Z.
pub fn remap_active() -> bool {
    REMAP_ACTIVE.load(Ordering::SeqCst)
}

/// Whether the watchdog should install the VSUSP remap now.
///
/// Installs it as a side effect the first time it returns `true`, so callers
/// do not have to track that themselves. Deferring is important: a fast
/// command must never trigger `tcsetattr`, which raced with child output and
/// ate it in Waash.
pub fn maybe_remap(elapsed: std::time::Duration) -> bool {
    if REMAP_ACTIVE.load(Ordering::SeqCst) || elapsed < REMAP_DELAY {
        return false;
    }
    install_susp_remap(shortcut());
    REMAP_ACTIVE.load(Ordering::SeqCst)
}

/// Whether the terminal has input pending, WITHOUT consuming it.
///
/// Used so a quick `Ctrl+C`/`Ctrl+B` burst is noticed even between polls.
fn tty_readable() -> bool {
    use std::os::unix::io::AsRawFd;

    if !std::io::stdin().is_terminal() {
        return false;
    }
    let fd = std::io::stdin().as_raw_fd();
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: `pfd` is a valid, fully-initialised single-element buffer and the
    // timeout is 0, so this cannot block.
    let ready = unsafe { libc::poll(&mut pfd, 1, 0) };
    ready > 0 && (pfd.revents & libc::POLLIN) != 0
}

/// Read one byte from the terminal without blocking, if one is available.
fn read_tty_byte() -> Option<u8> {
    use std::os::fd::BorrowedFd;
    use std::os::unix::io::AsRawFd;

    if !std::io::stdin().is_terminal() {
        return None;
    }
    let raw_fd = std::io::stdin().as_raw_fd();
    // SAFETY: stdin is open for the lifetime of the process, so the borrowed
    // descriptor is valid for this call.
    let fd = unsafe { BorrowedFd::borrow_raw(raw_fd) };

    let flags = nix::fcntl::fcntl(fd, nix::fcntl::FcntlArg::F_GETFL).ok()?;
    let original = nix::fcntl::OFlag::from_bits_truncate(flags);
    nix::fcntl::fcntl(
        fd,
        nix::fcntl::FcntlArg::F_SETFL(original | nix::fcntl::OFlag::O_NONBLOCK),
    )
    .ok()?;

    let mut buf = [0u8; 1];
    // SAFETY: `buf` is a valid 1-byte destination for a 1-byte read.
    let n = unsafe { libc::read(raw_fd, buf.as_mut_ptr().cast(), 1) };

    // Always restore the original flags, even on failure.
    let _ = nix::fcntl::fcntl(fd, nix::fcntl::FcntlArg::F_SETFL(original));

    if n == 1 { Some(buf[0]) } else { None }
}

/// Temporarily set the terminal's suspend character (VSUSP) to `byte`.
fn install_susp_remap(byte: u8) {
    use nix::sys::termios::{SetArg, SpecialCharacterIndices, tcgetattr, tcsetattr};

    let Ok(mut termios) = tcgetattr(std::io::stdin()) else {
        return;
    };

    {
        // Remember the original exactly once.
        let mut guard = match ORIG_SUSP.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if guard.is_none() {
            *guard = Some(termios.control_chars[SpecialCharacterIndices::VSUSP as usize]);
        }
    }

    termios.control_chars[SpecialCharacterIndices::VSUSP as usize] = byte;
    if tcsetattr(std::io::stdin(), SetArg::TCSANOW, &termios).is_ok() {
        REMAP_ACTIVE.store(true, Ordering::SeqCst);
    }
}

/// Restore the terminal's original suspend character, if it was changed.
pub fn restore_susp() {
    use nix::sys::termios::{SetArg, SpecialCharacterIndices, tcgetattr, tcsetattr};

    // Only act if a remap really is installed.
    if !REMAP_ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }

    let original = {
        let guard = match ORIG_SUSP.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard
    };

    if let Some(original) = original
        && let Ok(mut termios) = tcgetattr(std::io::stdin())
    {
        termios.control_chars[SpecialCharacterIndices::VSUSP as usize] = original;
        let _ = tcsetattr(std::io::stdin(), SetArg::TCSANOW, &termios);
    }
}

/// Should the foreground wait poll for the shortcut at all?
///
/// Only meaningful for an interactive shell on a real terminal.
pub fn supported() -> bool {
    std::io::stdin().is_terminal()
}

/// Drives the Ctrl+B check while a foreground child runs.
///
/// Owns the two timing rules so they are unit-testable without a terminal:
/// * the VSUSP remap is deferred by [`REMAP_DELAY`];
/// * the shortcut byte is checked on every [`poll_tick`](Self::poll_tick).
#[derive(Debug)]
pub struct Watchdog {
    start: Instant,
    remap_delay: Duration,
}

impl Watchdog {
    /// Create a watchdog that installs the VSUSP remap after `remap_delay`.
    pub fn new(remap_delay: Duration) -> Self {
        Self {
            start: Instant::now(),
            remap_delay,
        }
    }

    /// Time elapsed since the child started.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Whether the VSUSP remap is due (and not yet installed).
    pub fn remap_due(&self) -> bool {
        self.start.elapsed() >= self.remap_delay && !REMAP_ACTIVE.load(Ordering::SeqCst)
    }

    /// Perform one poll cycle: install the remap when due, then check for the
    /// shortcut. Returns `true` when the shortcut was pressed.
    pub fn poll_tick(&mut self) -> bool {
        if self.remap_due() {
            install_susp_remap(shortcut());
        }
        poll_shortcut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_parsing() {        set_shortcut("Ctrl+B");
        assert_eq!(shortcut(), 0x02);

        set_shortcut("Ctrl+z");
        assert_eq!(shortcut(), 0x1a);

        set_shortcut("Ctrl+A");
        assert_eq!(shortcut(), 0x01);

        // Invalid values fall back to the default.
        set_shortcut("B");
        assert_eq!(shortcut(), DEFAULT_BG_KEY);

        set_shortcut("Ctrl+1");
        assert_eq!(shortcut(), DEFAULT_BG_KEY);

        set_shortcut("");
        assert_eq!(shortcut(), DEFAULT_BG_KEY);

        // Restore for other tests.
        set_shortcut("Ctrl+B");
    }

    #[test]
    fn watchdog_defers_the_remap() {
        let w = Watchdog::new(Duration::from_millis(50));
        assert!(!w.remap_due(), "must not remap immediately");
        std::thread::sleep(Duration::from_millis(70));
        assert!(w.remap_due(), "remap should be due after the delay");
        assert!(w.elapsed() >= Duration::from_millis(70));
    }

    #[test]
    fn watchdog_zero_delay_is_immediate() {
        let w = Watchdog::new(Duration::ZERO);
        assert!(w.remap_due());
    }

    #[test]
    fn restore_susp_is_safe_when_no_remap() {
        // Must be a no-op, not a panic, when nothing was installed.
        restore_susp();
        restore_susp();
    }

    #[test]
    fn shortcut_press_flag_is_consumed_once() {
        SHORTCUT_PRESSED.store(true, Ordering::SeqCst);
        assert!(take_shortcut_press());
        assert!(!take_shortcut_press());
    }
}
