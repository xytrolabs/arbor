#![doc = include_str!("../README.md")]
#![cfg_attr(
    not(target_arch = "wasm32"),
    allow(
        clippy::disallowed_types,
        reason = "This file may be compiled as host build-script code while building the wasm target"
    )
)]

mod exit_status;
mod foreground;
mod util;

#[cfg(target_family = "unix")]
pub mod background_key;

/// No-op stand-ins for platforms without the Ctrl+B shortcut.
///
/// The shortcut needs a terminal with termios and `poll(2)`. Windows has
/// neither in this form, so every entry point reports "not supported" and the
/// shell behaves as if the feature were switched off. Having the module exist
/// everywhere keeps the protocol crate free of platform gating at every call
/// site.
#[cfg(not(target_family = "unix"))]
pub mod background_key {
    use std::time::Duration;

    #[derive(Default)]
    pub struct ShortcutReader;

    impl ShortcutReader {
        pub fn new() -> Self {
            Self
        }
        pub fn is_active(&self) -> bool {
            false
        }
        pub fn poll(&self) -> bool {
            false
        }
    }

    #[derive(Default)]
    pub struct Watchdog;

    impl Watchdog {
        pub fn new(_remap_delay: Duration) -> Self {
            Self
        }
        pub fn elapsed(&self) -> Duration {
            Duration::ZERO
        }
        pub fn remap_due(&self) -> bool {
            false
        }
        pub fn poll_tick(&mut self) -> bool {
            false
        }
    }

    pub fn set_shortcut(_spec: &str) {}
    pub fn shortcut() -> u8 {
        0
    }
    pub fn poll_interval() -> Duration {
        Duration::from_millis(250)
    }
    pub fn take_shortcut_press() -> bool {
        false
    }
    pub fn poll_shortcut() -> bool {
        false
    }
    pub fn mark_backgrounded() {}
    pub fn was_backgrounded() -> bool {
        false
    }
    pub fn clear_backgrounded() {}
    pub fn remap_active() -> bool {
        false
    }
    pub fn maybe_remap(_elapsed: Duration) -> bool {
        false
    }
    pub fn fd_has_data(_fd: i32) -> bool {
        false
    }
    pub fn restore_susp() {}
    pub fn supported() -> bool {
        false
    }
}

#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(any(target_os = "android", target_os = "linux"))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
mod netbsd;
pub mod os_info;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "windows",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod process;
#[cfg(target_family = "unix")]
mod unix;
#[cfg(target_os = "windows")]
mod windows;

pub use self::exit_status::ExitStatus;
pub use self::foreground::prepare_background_command;
#[cfg(unix)]
pub use self::foreground::stdin_fd;
pub use self::foreground::{
    ForegroundChild, ForegroundGuard, ForegroundWaitStatus, UnfreezeHandle,
};

pub use self::util::*;

#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "windows",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub use self::process::ProcessInfo;

#[cfg(target_os = "freebsd")]
pub use self::freebsd::*;
#[cfg(any(target_os = "android", target_os = "linux"))]
pub use self::linux::*;
#[cfg(target_os = "macos")]
pub use self::macos::*;
#[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
pub use self::netbsd::*;
#[cfg(target_family = "unix")]
pub use self::unix::*;
#[cfg(target_os = "windows")]
pub use self::windows::*;
