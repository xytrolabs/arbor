# Arbor — Usage

How to use Arbor as your everyday shell. The Nu language is unchanged from
upstream, so [the Nushell Book](https://www.nushell.sh/book/) is the reference
for commands. This guide focuses on the interactive experience and the
Arbor-specific behaviour.

## Running commands

### Arbor's own commands

Bare identifiers are Arbor commands:

```nushell
ls
open file.toml | get package
str length "hello"
```

### `sysinfo` — a system overview

`sysinfo` prints a compact summary of the machine: host, OS, kernel, uptime,
CPU, memory, the root disk and package temperature. Every value is read
best-effort, so a container or a platform without the data shows `-` instead of
failing.

```nushell
sysinfo
```

```text
Host    raf-cachy
OS      Linux (CachyOS Linux rolling)
Kernel  7.3.3-1-cachyos
Uptime  17hr 41min 47sec
CPU     13th Gen Intel(R) Core(TM) i5-13400F  load 8.99
Memory  13.2 GB / 50.3 GB  26%
Disk /  589.7 GB / 1.0 TB
Temp    37°C
```

### Quality-of-life helpers

Arbor ships these so common navigation needs no extra setup and no plugins.

| Command | Does |
|---|---|
| `mark [name]` | Bookmark the current directory (defaults to its own name) |
| `marks` | List bookmarks as a table |
| `jump [name]` | Change to a bookmark, or list them when called bare |
| `unmark <name>` | Remove a bookmark |
| `up [n]` | Go up `n` parent directories (default 1) |
| `mkcd <dir>` | Create a directory and change into it |
| `timer { … }` | Run a block and print how long it took |
| `docs [topic]` | Show the docs index, or the URL of one guide |

```nushell
cd ~/src/arbor
mark                      # bookmarked "arbor" -> ~/src/arbor
cd /tmp
jump arbor                # straight back
up 2                      # /home/you

mkcd ~/scratch/experiment # create and enter
timer { cargo build }     # took 42s 118ms

docs                      # list the guides
docs 04-configuration     # URL for one guide
```

`timer` passes the block's value through, so it can wrap any expression
without changing the result.

Bookmarks persist in `marks.json` under `$nu.data-dir`, so they survive
sessions. `jump` reports a clear error if a bookmarked directory has since been
deleted rather than silently failing.

### The standard library is loaded by default

Startup imports `std/prelude`, and Arbor's default config also runs
`use std *`, so the entire standard library is available in a fresh install
without touching any dotfile: `sysinfo`, `mark`, `jump`, `up`, `docs`, `assert`,
`ellie`, `iter`, `bench` and the rest. Importing it costs nothing measurable —
startup stays at about 9 ms.

If you would rather keep your namespace minimal, remove the `use std *` line
from `$nu.config-path` and call the commands through their module instead.

### External programs

If a name is not an Arbor command, Arbor looks on `PATH`:

```nushell
git status
cargo build
```

Use `^` to force an external command even if an internal command shares the
name:

```nushell
^echo "this is /usr/bin/echo"
```

Use `which` to see what a name resolves to:

```nushell
which ls        # Arbor's ls
which ^ls       # /usr/bin/ls
```

## Pipelines

Stages are connected with `|` and pass structured values:

```nushell
ls | where type == file | sort-by size --reverse | first 3
ps | where cpu > 5 | select name pid cpu
```

Useful stages:

| Command | Purpose |
|---|---|
| `where <cond>` | Filter rows |
| `sort-by <col>` | Sort |
| `select <cols>` | Pick columns |
| `get <col>` | Extract one value (`get 0`) |
| `each {|x| ...}` | Map over a list |
| `reduce` | Fold to one value |
| `group-by <col>` | Group into a record |

## Variables and strings

```nushell
let name = "raf"
$"hello, ($name)"                  # interpolation
let nums = [1 2 3]
$nums | math sum
```

Variables are immutable by default; use `mut` to reassign:

```nushell
mut count = 0
$count += 1
```

## Custom commands

```nushell
def greet [name: string] {
  $"hello, ($name)"
}

greet "world"
```

Persist them by putting the `def` in `config.nu`.

## Job control

### Start a job in the background

```nushell
sleep 30sec &        # note: Arbor's sleep takes a unit
cargo watch --all &
```

### List and manage jobs

| Command | Purpose |
|---|---|
| `job list` | Show running and frozen jobs |
| `job unfreeze <id>` | Bring a stopped job back to the foreground |
| `job kill <id>` | Terminate a job |

### `Ctrl+B` — move a running command to the background

This is an Arbor feature (inherited from Waash). While a long command is
running in the foreground, press **`Ctrl+B`**:

```nushell
^sleep 300
# press Ctrl+B ...
# "sleep moved to background (pid 12345)"
# prompt returns immediately; the process keeps running
```

How it works:

* Arbor temporarily remaps the terminal's suspend character to your shortcut
  byte, so the terminal delivers `SIGTSTP` to the child.
* The child is immediately continued (`SIGCONT`), so it keeps running rather
  than sitting frozen.
* The terminal is handed back to the shell and the prompt returns.

The remap is **deferred by 300 ms**, so a fast command never touches terminal
settings — this avoids a race that used to eat output.

#### What can be backgrounded

| Command | `Ctrl+B` | Notes |
|---|---|---|
| External commands (`^sleep`, `cargo build`, …) | Yes | The usual case; a child process exists to detach. |
| `sleep` (builtin) | Yes | Interrupts the wait immediately. |
| Other builtins (`ls`, `sort`, …) | Not yet | They run inside the shell, so there is no child to detach. |

Builtins run *inside* the shell process rather than as a child, so there is
nothing to move to the background. `sleep` is special-cased because it is the
one builtin you would realistically want to interrupt. For anything else, run
the external version with `^`:

```nushell
^sleep 300    # external: Ctrl+B works
sleep 300sec  # builtin: also works (special-cased)
```

> `sleep` requires units. `sleep 4000` is a parse error; write `sleep 4000sec`.

Change the key:

```nushell
$env.config.arbor.bg_shortcut = "Ctrl+G"
```

> `Ctrl+B` replaces the older `Ctrl+Z` then `job unfreeze` flow for *running*
> commands. Arbor remaps the terminal's suspend character only while a command
> is actually running, and restores it when the command finishes, which is why
> the remap is deferred by 300 ms. Pressing `Ctrl+Z` at the prompt itself is
> unaffected.

> Note: while a command is running and `Ctrl+B` is available, `Ctrl+Z` is
> remapped, so use `Ctrl+B` if you want the job to keep running.

## Bash control-flow blocks

If you type bash syntax that Arbor's parser does not handle, Arbor collects the
whole block and runs it through `bash`, so muscle memory keeps working.

Single line:

```bash
if [ -f /etc/hostname ]; then echo yes; fi
```

Multi-line — keep typing; Arbor shows a `… ` continuation prompt until the
block is balanced:

```bash
for i in 1 2 3; do
  echo "line $i"
done
```

Supported constructs:

| Construct | Example |
|---|---|
| `if` / `elif` / `else` | `if x; then ...; fi` |
| `for` / `while` / `until` | `while true; do ...; done` |
| `case` | `case $x in a) ... ;; esac` |
| `select` | `select x in a b; do ...; done` |
| Function definitions | `f() { echo hi; }` then `f` |
| Brace groups | `{ echo a; echo b; }` |

Function definitions are remembered for the rest of the session, so `f()` then
`f` works even though each block runs in its own `bash -c`.

Disable the feature:

```nushell
$env.config.arbor.bash_blocks = false
```

## Redirections

```nushell
ls > files.txt
ls o+e> all.txt          # stdout and stderr
open in.txt | save out.txt
```

External commands keep their usual shell redirects:

```bash
^cargo build 2> build.log
```

## Interrupting

* `Ctrl+C` — interrupt the running command (does not exit the shell)
* `Ctrl+D` — exit the shell

## History

Arbor keeps history in the config directory and supports the usual search:

| Key | Action |
|---|---|
| `↑` / `↓` | Previous / next entry |
| `Ctrl+R` | Search history backwards |

History settings live under `$env.config.history`.
