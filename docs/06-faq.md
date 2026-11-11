# Arbor — FAQ

## What is Arbor?

Arbor is a fork of [Nushell](https://github.com/nushell/nushell), a shell where
pipelines carry structured data instead of text. It keeps Nushell's language and
commands, and adds features inherited from Waash (its predecessor): Indent
scripting, `Ctrl+B` backgrounding and bash control-flow blocks.

## How do I install it without Rust?

Use the website installer — it downloads a prebuilt binary and needs no
toolchain:

```sh
curl -fsSL https://xytro.site/arbor/install.sh | bash
```

Prebuilt binaries exist for Linux (x86_64, aarch64, armv7), macOS (Intel and
Apple Silicon) and Windows (x86_64). The Linux builds are static musl binaries,
so one download works across distributions. If no binary matches your platform,
the installer builds from source instead. See
[Getting Started](01-getting-started.md#installing).

## How do I change the startup banner or prompt?

The banner is a normal Nushell value — hide it in your config:

```nu
$env.config.show_banner = false
```

The tree glyph in the default prompt comes from `default_env.nu`. Set your own
prompt in `config.nu`:

```nu
$env.PROMPT_COMMAND = {|| $"($env.PWD)> " }
```

The glyph is omitted automatically when `use_ansi_coloring` is off, so piped
and plain-terminal output stay clean. See [The Prompt](05-prompt.md).

## Is Arbor the same as Nushell?

The language is identical — every Nushell command and idiom works. Arbor differs
in packaging, naming and presentation:

| | Nushell | Arbor |
|---|---|---|
| Binary | `nu` | `arbor` |
| Config | `~/.config/nushell` | `~/.config/arbor` |
| Data | `~/.local/share/nushell` | `~/.local/share/arbor` |
| Cache | `~/.cache/nushell` | `~/.cache/arbor` |
| Prompt | path only | tree glyph + path |

See the `NOTICE` file in the repository for the full change list, the exact
upstream version and commit, and instructions for merging upstream.

## Does my existing config still work?

Yes. If `~/.config/arbor` does not exist, Arbor automatically uses
`~/.config/nushell` or `~/.config/waash` instead. Nothing breaks.

To take ownership explicitly:

```sh
mv ~/.config/waash ~/.config/arbor
# or
mv ~/.config/nushell ~/.config/arbor
```

Ask which one is in use:

```nushell
$nu.default-config-dir
```

## Where does Arbor store things?

| Item | Location |
|---|---|
| Config | `~/.config/arbor/config.nu`, `env.nu` |
| History | `~/.config/arbor/` (see `$env.config.history`) |
| Data | `~/.local/share/arbor/` |
| Cache | `~/.cache/arbor/` |
| Indent helpers | `~/.local/share/arbor/lib/arbor.ind` |
| Binary | `~/.local/bin/arbor` |

## `arbor: command not found`

The install directory is not on your `PATH`:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Add that line to your shell's startup file to make it permanent.

## My config changes have no effect

`arbor -c '...'` does **not** load config files — that matches upstream. Config
loads in the interactive shell, when running a script, and with an explicit
`--config <path>`.

Check for startup config errors: invalid options are reported rather than
silently ignored.

## `Ctrl+B` does nothing

Requirements:

1. You must be in the **interactive shell** on a real terminal (not `-c`, not
   piped input).
2. The command must have been running for **more than ~300 ms** — the shortcut
   is deliberately deferred so fast commands never touch terminal settings.
3. Your terminal must deliver control characters normally. Some terminal
   multiplexers intercept them; try a plain terminal to confirm.

Check the configured key:

```nushell
$env.config.arbor.bg_shortcut
```

If you set an unrecognised value, Arbor falls back to `Ctrl+B`.

Note that `Ctrl+Z` no longer suspends a foreground command while `Ctrl+B` is
active, because the terminal's suspend character is remapped.

## Which output ends up where after `Ctrl+B`?

The process keeps running and keeps its own output destination. If it was
writing to the terminal, it continues to do so. Arbor returns the prompt
immediately without waiting for it to finish.

## Indent script says "could not find the `indent` runtime"

Arbor needs the `indent` binary. Install Indent, or point Arbor at it:

```nushell
$env.config.arbor.indent_binary = "/path/to/indent"
```

or set `ARBOR_INDENT_BINARY` in the environment.

Check what Arbor will use:

```sh
arbor --no-config-file -c 'print $env.config.arbor.indent_binary'
```

## Why does my Indent script fail on `#` comments?

Indent comments must start with `#!`. A bare `#` is parsed as the start of a hex
colour and fails:

```
runtime_error
  Line 1: Invalid hex color ''. Hex colors need 3, 4, 6, or 8 hex digits.
```

Fix:

```
#! this is a comment
#! and so is this one, even inside a block
```

## Why does `say string (len s)` fail?

Indent cannot parse a bare function call inside another call's parentheses. Store
the result first:

```
var n = len s
say "len: " + (string n)
```

The same applies to `+` concatenation used directly as an argument — build the
string in a variable first.

## Why does `[1 2 3]` fail?

Indent list literals require commas:

```
var nums = [1, 2, 3]
```

## Can I use `else` in Indent?

No. Indent uses `or` for "else if" and `otherwise` for "else":

```
if x == 1
  say "one"
or x == 2
  say "two"
otherwise
  say "other"
```

A bare `else` is a syntax error.

## Bash blocks only work in the interactive shell

Bash block delegation is a REPL feature. For scripts use a `.sh` file:

```sh
arbor script.sh
```

`.sh` and `.bash` files are run by `bash`.

## Can I disable a feature I don't want?

```nushell
$env.config.arbor.bash_blocks = false
$env.config.arbor.indent_scripts = false
```

## How do I update Arbor?

Pull and rebuild:

```sh
cd arbor
git pull
./install.sh
```

If you forked from upstream, merge Nushell changes with:

```sh
git remote add upstream https://github.com/nushell/nushell
git fetch upstream --tags
git merge upstream/main
```

Conflicts are normally limited to `Cargo.toml`,
`crates/nu-config/src/paths.rs`, `crates/nu-config/src/resolve.rs`,
`src/command.rs`, the docs and the README.

## Where are the docs for the language itself?

Indent scripting and Arbor-specific behaviour are documented here. For the
shell language, use the upstream Nushell documentation — the language is
unchanged:

* [The Nushell Book](https://www.nushell.sh/book/)
* [Command reference](https://www.nushell.sh/commands/)

## Is Arbor an official Nushell project?

No. Arbor is an independent fork and is not endorsed by the Nushell Project.
Arbor is MIT licensed and preserves the original Nushell copyright notice in
full — see `LICENSE` and `NOTICE`.
