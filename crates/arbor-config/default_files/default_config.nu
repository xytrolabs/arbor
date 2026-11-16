# Built-in default config file.
#
# All `$env.config` defaults (including color_config, menus, and Nushell menu
# keybindings) now live in Rust `Config::default()` so they are visible under
# `nu -n` / `nu -n --no-std-lib` via `$env.config`.
#
# This file is still evaluated before the user's `config.nu` during interactive
# startup so users can rely on the load order. Leave it empty (or assign only
# intentional overrides) unless you need Nu-side default setup.
#
# version = "1.0.0"
$env.config = {}

# Arbor: bring the whole standard library into scope by default.
#
# Startup only imports `std/prelude`, so without this a fresh install answers
# "command not found" for `sysinfo`, `mark`, `jump`, `up`, `docs`, `ellie`,
# `assert` and the rest. Importing it here means Arbor works out of the box —
# the same promise Fish makes — instead of requiring a dotfile edit first.
use std *
