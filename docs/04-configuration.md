# Arbor — Configuration

Arbor reads its configuration from two files:

| File | Purpose |
|---|---|
| `~/.config/arbor/env.nu` | Environment variables, evaluated first |
| `~/.config/arbor/config.nu` | `$env.config` settings and custom commands |

Ask Arbor where they are:

```nushell
$nu.config-path
$nu.env-path
```

## Where the config directory lives

Resolution order:

1. `--config-home <dir>` on the command line
2. `$XDG_CONFIG_HOME/arbor`
3. `~/.config/arbor`

**Migration:** if `~/.config/arbor` does not exist but `~/.config/nushell` or
`~/.config/waash` does, Arbor uses the legacy directory instead. Existing
setups keep working untouched. Once `~/.config/arbor` exists it always wins, so
migrating is just:

```sh
mv ~/.config/nushell ~/.config/arbor
```

The same fallback applies to the data directory (`~/.local/share/arbor`) and
cache directory (`~/.cache/arbor`).

## A minimal config

`env.nu`:

```nushell
$env.EDITOR = "nvim"
$env.PATH = ($env.PATH | prepend $"($env.HOME)/.local/bin")
```

`config.nu`:

```nushell
$env.config.show_banner = false

$env.config.arbor = {
  bg_shortcut: "Ctrl+B"
  indent_scripts: true
  indent_binary: ""
  bash_blocks: true
}

def ll [] { ls -l }
```

## Arbor settings

All Arbor-specific options live under `$env.config.arbor`.

### `bg_shortcut`

| | |
|---|---|
| Type | string |
| Default | `"Ctrl+B"` |
| Accepts | `Ctrl+<a-z>` |

Key that moves the running foreground command to the background. An
unrecognised value falls back to `Ctrl+B` rather than disabling the feature.

```nushell
$env.config.arbor.bg_shortcut = "Ctrl+G"
```

### `indent_scripts`

| | |
|---|---|
| Type | bool |
| Default | `true` |

Route `.ind`, `.arbor` and `.waash` files to the Indent runtime. Set to `false`
to have Arbor parse them as Nu scripts instead.

```nushell
$env.config.arbor.indent_scripts = false
```

### `indent_binary`

| | |
|---|---|
| Type | string |
| Default | `""` |

Explicit path to the `indent` executable. Empty means "search `PATH`, then
`~/.local/bin`, then the upstream installer location".

```nushell
$env.config.arbor.indent_binary = "/usr/local/bin/indent"
```

You can also use the `ARBOR_INDENT_BINARY` environment variable, which takes
priority over this setting.

### `bash_blocks`

| | |
|---|---|
| Type | bool |
| Default | `true` |

Collect bash control-flow blocks typed at the prompt and run them through
`bash`. Covers `if`, `for`, `while`, `until`, `case`, `select`, function
definitions and `{ ... }` groups.

```nushell
$env.config.arbor.bash_blocks = false
```

### `prompt_git`

| | |
|---|---|
| Type | bool |
| Default | `true` |

Show the git segment (branch, dirty marker, ahead/behind) in the prompt.

The prompt is rebuilt on every render, and reading git state spawns an external
`git` process. That is invisible when you are typing, but it dominates the cost
of running many commands in a row — exactly what scripted and agent-driven work
looks like.

Measured with `scripts/burst_benchmark.py` (200 commands: 100 external, 80
internal, 20 pipelines, in an interactive session):

| `prompt_git` | 200 commands | per command |
|---|---|---|
| `true` (default) | 3.41 s | 17.1 ms |
| `false` | 1.14 s | 5.7 ms |

Every command still ran in both cases (100/100 external commands completed, no
errors), so this setting changes speed, not correctness. Turn it off when you
are driving Arbor from a script or an agent:

```nushell
$env.config.arbor.prompt_git = false
```

The segment is a single `git status --porcelain=v2 --branch` call. Earlier
versions used four separate git invocations per render; if you modify the
prompt, keep it to one git process.

## Common upstream settings

These come from Nushell and are documented in the upstream book. The ones most
worth setting first:

### Prompt

```nushell
$env.config.show_banner = false
```

For a custom prompt string, define `PROMPT_COMMAND`. See **05 — Prompt**.

### History

```nushell
$env.config.history = {
  file_format: "plaintext"    # or "sqlite"
  max_size: 100_000
  sync_on_enter: true
}
```

### Table and colours

```nushell
$env.config.table.mode = "rounded"
$env.config.use_ansi_coloring = true
```

### Display output hook

```nushell
$env.config.hooks.display_output = {||
  if (term size).columns >= 100 { table -e } else { table }
}
```

## Environment variables Arbor reads

| Variable | Effect |
|---|---|
| `ARBOR_INDENT_BINARY` | Path to the Indent runtime (beats the config setting) |
| `ARBOR_INDENT_LIB` | Directory containing the Indent helper library |
| `ARBOR_PREFIX` | Install prefix used by `install.sh` |
| `XDG_CONFIG_HOME` | Config directory base |
| `XDG_DATA_HOME` | Data directory base |
| `XDG_CACHE_HOME` | Cache directory base |

## Reloading configuration

Start a new shell, or re-source the files:

```nushell
source ~/.config/arbor/config.nu
source ~/.config/arbor/env.nu
```

> `arbor -c '...'` does **not** load config files. Use `--config <path>` or the
> interactive shell.

## Checking your config

```nushell
$env.config.arbor | to json
$nu.default-config-dir
```

Invalid options are reported as config errors at startup rather than silently
ignored.

## Startup cost

Whenever a config file is loaded — a login shell, `-l`, or `--config` — Arbor
also loads its own default config, and that default config imports the whole
standard library. Measured on a warm cache on x86_64:

| Phase | Cost |
|---|---|
| Interpreter boot, no config at all | ~10 ms |
| `use std *` (the entire standard library) | ~19 ms |
| Your own `config.nu`, evaluated | < 1 ms |

So interactive startup is almost entirely the standard library, not your
dotfiles. Two consequences worth knowing:

- **Do not add `use std *` to your `config.nu`.** Arbor's built-in default
  config already does it. There is no parsed-module cache, so a second import
  re-parses everything and costs another ~17 ms on every start.
- `help` is the single most expensive std module (~5 ms of the 19).

If a start feels slow, ask Arbor to attribute the time:

```sh
ARBOR_CFG_TIMING=1 arbor -c 'print 1'
ARBOR_LOGIN_TIMING=1 arbor -l -c 'print 1'
```

The first reports default-config and user-config evaluation; the second reports
how long the login profiles blocked and how many variables they imported.

