# Arbor — Getting Started

Arbor is a **fork of [Nushell](https://github.com/nushell/nushell)** — "a new
type of shell" where everything is structured data rather than text.

The language is unchanged from Nushell 0.117.0. This guide covers what is
Arbor-specific: installing it, where its files live, and the features inherited
from Waash (Arbor's predecessor) — Indent scripting, `Ctrl+B` backgrounding and
bash control-flow blocks.

## Why "a fork"?

Arbor is not a rewrite. It tracks upstream Nushell and adds a small set of
changes, deliberately kept mergeable so upstream updates stay cheap:

| Change | Detail |
|---|---|
| Binary name | `arbor` instead of `nu` |
| Config directory | `~/.config/arbor` instead of `~/.config/nushell` |
| Data directory | `~/.local/share/arbor` |
| Cache directory | `~/.cache/arbor` |
| Waash features | Indent scripts, `Ctrl+B`, bash blocks |

See the `NOTICE` file in the repository for the full change list and the exact
upstream commit.

## Installing

### From the website

```sh
curl -fsSL https://xytro.site/arbor/install.sh | bash
```

### From a source checkout

Rust **1.97.1** is required (pinned by `rust-toolchain.toml`).

```sh
git clone <your-arbor-repo-url>
cd arbor
./install.sh
```

`install.sh` options:

| Option | Effect |
|---|---|
| *(none)* | Build release and install to `~/.local/bin/arbor` |
| `--no-build` | Install an existing `target/release/arbor` |
| `--prefix DIR` | Install to `DIR/bin` instead of `~/.local` |
| `--version` | Print the Arbor version and exit |

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
