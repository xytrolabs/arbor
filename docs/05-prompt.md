# Arbor — Prompt

Arbor's prompt is configurable through `PROMPT_COMMAND`, the same mechanism as
upstream Nushell. Because the language is unchanged, any Nushell prompt works
in Arbor — you only need to point it at `~/.config/arbor` when loading files.

## The default

Left side shows the current directory:

```
~/Desktop/Arbor>
```

Right side (when the terminal is wide enough) shows the time:

```
                                                       Tue 20:24:54
```

## Setting a prompt

Define `PROMPT_COMMAND` in `~/.config/arbor/config.nu`. It must be a closure
taking no arguments and returning a string:

```nushell
$env.PROMPT_COMMAND = {|| $"($env.PWD)> " }
```

Multi-line prompts are allowed — return a string containing newlines:

```nushell
$env.PROMPT_COMMAND = {||
  $"($env.USER)@(sys host | get hostname) ($env.PWD)
$ "
}
```

## The right prompt

```nushell
$env.PROMPT_COMMAND_RIGHT = {|| date now | format date "%H:%M:%S" }
```

Hide it entirely:

```nushell
$env.PROMPT_COMMAND_RIGHT = ""
```

## A useful prompt

A two-line prompt showing user, host, path and git branch:

```nushell
$env.PROMPT_COMMAND = {||
  let branch = (do -i { git rev-parse --abbrev-ref HEAD } | str trim)
  let b = if ($branch | is-empty) { "" } else { $" ($branch)" }
  $"($env.USER)@(sys host | get hostname) ($env.PWD)($b)
$ "
}
```

## Showing exit status

```nushell
$env.PROMPT_COMMAND = {||
  let code = ($env.LAST_EXIT_CODE? | default 0)
  let mark = if $code == 0 { "" } else { $" [($code)]" }
  $"($env.PWD)($mark)> "
}
```

## Transient prompt

`PROMPT_COMMAND_TRANSIENT` replaces the prompt after you press Enter, so old
prompts collapse to something compact:

```nushell
$env.PROMPT_COMMAND_TRANSIENT = {|| "$ " }
```

## Multi-line indicators

When using a multi-line prompt, Nushell-style indicators keep the right prompt
aligned:

```nushell
$env.PROMPT_MULTILINE_INDICATOR = {|| "::: " }
```

## Prompt tips

* Keep the prompt function **fast**. It runs before every keystroke refresh, so
  avoid `git` calls on very large repositories without a cache.
* Use `do -i { ... }` around commands that may fail, so a missing repository
  does not print an error on every prompt.
* `$env.PWD` is the current directory; `$env.USER` the user name.

## Colour

Styling uses `ansi`:

```nushell
$env.PROMPT_COMMAND = {||
  let path = (ansi green) + $env.PWD + (ansi reset)
  $"($path)> "
}
```

## Verifying

Start a new shell, or reload:

```nushell
source ~/.config/arbor/config.nu
```

If the prompt does not change, check for a config error printed at startup —
Arbor reports invalid config options rather than ignoring them.
