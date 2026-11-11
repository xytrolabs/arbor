# Arbor — Getting Started

Arbor is a **shell for structured data**: commands exchange typed values —
records, tables, lists — rather than lines of text, so `where`, `select`,
`sort-by` and `group-by` work on real fields instead of parsed strings.

Arbor is an independent project. It began as a rewrite of
[Nushell](https://github.com/nushell/nushell) 0.117.0 and keeps that project's
language and command set, because they are good and well understood. On top of
that it adds Indent scripting, `Ctrl+B` backgrounding and bash control-flow
blocks, and it owns its own versioning, crate names and error namespace.

If you already know Nushell, every command and idiom you use will work here.

## What Arbor owns

Arbor is not a repackaged Nushell. It has its own identity end to end:

| | Nushell | Arbor |
|---|---|---|
| Version | `0.117.0` | `1.0.0` |
| Binary | `nu` | `arbor` |
| Crates | `nu-protocol`, `nu-engine`, … | `arbor-protocol`, `arbor-engine`, … |
| Error prefix | `Error: nu::parser::…` | `Error: arbor::parser::…` |
| Config directory | `~/.config/nushell` | `~/.config/arbor` |
| Data directory | `~/.local/share/nushell` | `~/.local/share/arbor` |
| Cache directory | `~/.cache/nushell` | `~/.cache/arbor` |
| Prompt | path only | glyph, user@host, git, load, clock |
| Extras | — | Indent scripts, `Ctrl+B`, bash blocks |

The `NOTICE` file records the exact upstream commit this work began from, and
the `LICENSE` file (unchanged, MIT) preserves upstream's copyright.

### What deliberately did *not* change

Some names are part of the language rather than the build, so they were left
alone — changing them would silently break scripts and plugins:

| Kept as-is | Why |
|---|---|
| `to nuon` / `from nuon` | `nuon` is a data format name, not a crate name |
| `nu-highlight`, `nu-check` | command names used in scripts |
| `@"nu-complete foo"` | argument-completer convention |
| Plugin protocol `NuPlugin` | wire protocol identifier shared with plugins |

## Installing

### Prebuilt binaries

Arbor ships ready-made binaries, so you do not need a Rust toolchain:

| Platform | Architectures | Format |
|---|---|---|
| Linux | x86_64, aarch64, armv7 | `tar.gz`, statically linked against musl |
| macOS | x86_64, Apple Silicon | `tar.gz` |
| Windows | x86_64 | `.zip` (inside WSL, use the Linux build) |

The Linux builds are **static musl** binaries — the same download runs on
Debian, Fedora, Arch, openSUSE and Alpine. No glibc version matching needed.

### From the website

```sh
curl -fsSL https://xytro.site/arbor/install.sh | bash
```

The installer detects your OS and architecture, downloads the matching archive,
**verifies its SHA-256**, and installs the binary. If no prebuilt binary matches
your platform, it falls back to building from source automatically.

### From a source checkout

Rust **1.97.1** is required (pinned by `rust-toolchain.toml`).

```sh
git clone <your-arbor-repo-url>
cd arbor
./install.sh
```

### Installer options

| Option | Effect |
|---|---|
| *(none)* | Download a prebuilt binary; build from source if none matches |
| `--pin VERSION` | Install a specific release, e.g. `--pin v1.0.0` |
| `--from-source` | Skip the download and build with cargo |
| `--binary` | Require a prebuilt binary; never build from source |
| `--no-build` | Install an existing `./target/release/arbor` |
| `--prefix DIR` | Install to `DIR/bin` instead of `~/.local` |
| `--dry-run` | Print the plan without changing anything |
| `--version` | Print the Arbor version and exit |

Environment overrides: `ARBOR_PREFIX`, `ARBOR_VERSION`, `ARBOR_BASE_URL`
(point this at a mirror to install from somewhere other than GitHub).

The installer replaces the binary atomically (copy to a temporary name, then
`mv`), so it is safe to run while Arbor is open.

### Verifying

```sh
arbor --version
# arbor 0.117.0
```

## Your first commands

Everything is data. Pipelines pass structured values, not text:

```nushell
1..5 | each {|n| $n * $n } | math sum        # 55
ls | where size > 10kb | sort-by size --reverse | first 5
open package.json | get dependencies | columns
```

Bare words are commands. To run something from `PATH`, prefix it with `^` or
let Arbor find it when no internal command matches:

```nushell
^cargo build --release
git status
```

## Where Arbor keeps things

Arbor resolves its directories in this order:

1. CLI flags (`--config-home`, `--config`, `--env-config`)
2. `XDG_CONFIG_HOME` / `XDG_DATA_HOME` / `XDG_CACHE_HOME`
3. Platform defaults (`~/.config`, `~/.local/share`, `~/.cache`)

You can always ask:

```nushell
$nu.default-config-dir
$nu.config-path
$nu.env-path
$nu.data-dir
$nu.cache-dir
```

### Migrating from Nushell or Waash automatically

If `~/.config/arbor` does **not** exist, but `~/.config/nushell` or
`~/.config/waash` does, Arbor uses the legacy directory as-is. Your existing
configuration keeps working with no action required.

Once `~/.config/arbor` exists it always takes precedence. So migrating is just:

```sh
mv ~/.config/nushell ~/.config/arbor
```

The same fallback applies to the data and cache directories and to the Indent
helper library.

## Config files

Arbor reads two files from the config directory:

| File | Purpose |
|---|---|
| `env.nu` | Environment variables, evaluated first |
| `config.nu` | `$env.config` settings and custom commands |

Create them with `arbor --config` editing helpers, or by hand.

> **Note:** `arbor -c '...'` does **not** load your config files (this matches
> upstream Nushell). Only the interactive shell, a script run, and an explicit
> `--config <path>` load them.

### The `arbor` config table

Arbor-specific settings live under `$env.config.arbor`:

```nushell
$env.config.arbor = {
  bg_shortcut: "Ctrl+B"   # key that backgrounds a running command
  indent_scripts: true    # run .ind / .arbor / .waash with the Indent runtime
  indent_binary: ""       # explicit path to `indent` ("" = search PATH)
  bash_blocks: true       # delegate bash control-flow blocks to bash
}
```

| Key | Type | Default | Purpose |
|---|---|---|---|
| `bg_shortcut` | string | `"Ctrl+B"` | Move the running foreground command to the background |
| `indent_scripts` | bool | `true` | Route `.ind`/`.arbor`/`.waash` to the Indent runtime |
| `indent_binary` | string | `""` | Path to `indent`; empty means search `PATH` |
| `bash_blocks` | bool | `true` | Collect bash `if`/`for`/`while`/`case` blocks |

An unrecognised `bg_shortcut` falls back to `Ctrl+B` rather than disabling the
feature.

## Running scripts

```sh
arbor script.nu          # Nu script
arbor build.ind          # Indent script (via the Indent runtime)
arbor setup.sh           # bash script (via bash)
```

`.ind`, `.arbor` and `.waash` files go to the Indent runtime; `.sh`/`.bash` go to
bash; everything else is parsed by Arbor itself.

## Next steps

* **02 — Usage** — interactive shell basics, jobs, `Ctrl+B`, bash blocks
* **03 — Scripting** — writing Indent scripts with the Arbor helpers
* **04 — Configuration** — the full config reference
* **05 — Prompt** — prompt customisation
* **06 — FAQ** — troubleshooting and common questions
