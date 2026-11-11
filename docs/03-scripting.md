# Arbor — Scripting with Indent

Arbor inherits **Indent scripting** from Waash. Indent is a small, readable
programming language; Arbor runs `.ind`, `.arbor` and `.waash` files through the
`indent` runtime.

Inside an Indent script you can also write **plain shell commands**, so a script
can mix readable logic with the shell commands you already know.

## Requirements

The `indent` binary must be installed. Arbor looks for it in this order:

1. `$ARBOR_INDENT_BINARY`
2. `$env.config.arbor.indent_binary`
3. `indent` on `PATH`
4. `~/.local/bin/indent`
5. `~/.local/share/indent/bin/indent`

Check with:

```sh
indent --version
```

If Arbor cannot find it you'll see:

```
arbor: could not find the `indent` runtime (needed for build.ind).
Install Indent, or set $env.config.arbor.indent_binary.
```

## Your first script

Create `hello.ind`:

```
#! first Arbor script
say "hello from Indent"
```

Run it:

```sh
arbor hello.ind
```

```
hello from Indent
```

> **Indent comments must start with `#!`.** A bare `#` is a syntax error, not a
> comment. This applies to indented comments inside blocks too.

## Mixing shell commands

Any line that is not an Indent construct is treated as a shell command and
wrapped in `sh("...")` automatically:

```
fun main
  header "building"
  cargo build --release
  var size = sh_capture "ls -lh target/release/arbor"
  say "binary: " + size
  success "done"

main
```

```sh
arbor build.ind
```

```
=== building ===

    Finished `release` profile [optimized] target(s)
binary: -rwxr-xr-x 1 raf raf 90M ...
[ok] done
```

Arbor auto-imports these helpers, so you never need to import them yourself.

## Helper functions

| Helper | Purpose |
|---|---|
| `sh command` | Run a command, return its exit code |
| `sh_capture command` | Run a command, return combined stdout/stderr as a string |
| `has_command name` | `TRUE` when the command exists on `PATH` |
| `env name fallback` | Environment variable, or `fallback` when empty |
| `set_env name value` | Set an environment variable |
| `exists path` | `TRUE` when the path exists |
| `read_file path` | Read a whole file |
| `write_file path content` | Write a file |
| `append_file path content` | Append to a file |
| `header text` | Print a `=== text ===` heading |
| `success text` / `error text` / `info text` | Print a tagged line |
| `require_ok code message` | Exit with an error unless `code` is 0 |

Example:

```
fun main
  if has_command "git"
    var branch = sh_capture "git rev-parse --abbrev-ref HEAD"
    say "on branch: " + branch
  otherwise
    error "git is not installed"

main
```
## Language basics

### Variables

```
var count = 0
var name = "arbor"
var nums = [1, 2, 3]
```

Reassign with `=`:

```
count = count + 1
```

List literals **require commas**: `[1, 2, 3]`. `[1 2 3]` is a syntax error.

### Strings

```
say "val: " + name
say "len: " + (string (len name))
```

> **Gotcha:** Indent cannot parse `+` concatenation directly inside a
> function-call argument. Build the value in a variable first:
>
> ```
> var msg = "len: " + (string (len name))
> say msg
> ```

### If / else

Indent uses `or` for "else if" and `otherwise` for "else". A bare `else` is a
syntax error.

```
var v = 2
if v == 1
  say "one"
or v == 2
  say "two"
otherwise
  say "other"
```

```
two
```

### Loops

Iterate a list:

```
var total = 0
for n in [1, 2, 3, 4]
  total = total + n
say "total: " + (string total)
```

```
total: 10
```

Counted repeat:

```
repeat 3
  say "tick"
```

Loop until a condition holds:

```
var x = 0
repeat until x == 3
  x = x + 1
say "x: " + (string x)
```

```
x: 3
```

### Functions

```
fun add a b
  give a + b
```

`give` is how a function returns a value. Call it and also store the result in
a variable before using it:

```
var r = add 2 3
var msg = "sum: " + (string r)
say msg
```

```
sum: 5
```

> Function definitions only parse on a **one-line** signature:
> `fun add a b` ✓.

## Dictionaries and lists

```
var d = {"a": 1, "b": 2}
say "a is " + (string (d["a"]))

var xs = [10, 20, 30]
say "first: " + (string (xs[0]))
say "count: " + (string (len xs))
```

Key facts:

* Quote dictionary keys in literals: `{"a": 1}`, not `{a: 1}`.
* `has_key d "a"` tests for a key. Store its result before printing it.
* Missing keys raise an error rather than returning `nil` — test with
  `has_key` first.

## Writing files

```
fun main
  var report = "build ok"
  write_file "/tmp/report.txt" report
  var back = read_file "/tmp/report.txt"
  say "wrote: " + back

main
```

## Exit codes

```
fun main
  if not exists "/etc/hostname"
    error "expected /etc/hostname"
    process_exit 1
  success "all checks passed"

main
```

`arbor script.ind` exits with the script's exit code, so it composes with
`&&`, `||` and CI scripts.

## Turning Indent off

To make Arbor treat `.ind` files as ordinary Nu scripts instead:

```nushell
$env.config.arbor.indent_scripts = false
```

## Next steps

* **04 — Configuration** — every Arbor setting
* **06 — FAQ** — common Indent questions and fixes
