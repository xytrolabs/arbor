# Arbor

**Arbor is a fork of [Nushell](https://github.com/nushell/nushell), "a new type of shell".**

It keeps Nushell's structured-data pipeline, command set and language, under
the `arbor` name, with its own configuration directory.

```nushell
> ls | where size > 10kb | sort-by size --reverse | first 5
```

If you are looking for the documentation of the shell language itself, see the
[Nushell Book](https://www.nushell.sh/book/) — the language is unchanged in this
fork, so the upstream docs apply.

---

## Fork lineage and license

| | |
|---|---|
| Upstream | [nushell/nushell](https://github.com/nushell/nushell) |
| Forked from | `0.117.0` (commit `a481925`) |
| License | MIT — see [LICENSE](LICENSE) |

Arbor is **not** an official Nushell release and is not endorsed by the Nushell
Project. The original Nushell copyright notice is preserved in `LICENSE` and
applies to all inherited code. See [NOTICE](NOTICE) for the full list of
changes and instructions for merging upstream updates.

The original upstream README is kept as
[README.upstream.md](README.upstream.md).

## What changed relative to Nushell

The fork is deliberately small so it stays mergeable with upstream:

1. **Binary name** — the executable is `arbor` instead of `nu`.
2. **Configuration directories** — `arbor` instead of `nushell`:
   - Config: `~/.config/arbor/`
   - Data: `~/.local/share/arbor/`
   - Cache: `~/.cache/arbor/`
3. **Automatic migration** — if `~/.config/arbor` does not exist but a legacy
   `~/.config/nushell` (or `~/.config/waash`) directory does, Arbor uses the
   legacy directory, so an existing setup keeps working untouched. To migrate
   explicitly, just rename the directory:

   ```sh
   mv ~/.config/nushell ~/.config/arbor
   ```

   Once `~/.config/arbor` exists it always takes precedence.
4. **Rebranded help/version output and docs.**

The workspace crates are still named `nu-*` (they are upstream names); only the
produced binary is called `arbor`.

## Building

Rust **1.97.1** is required (pinned by `rust-toolchain.toml`).

```sh
cd arbor
cargo build --release
./target/release/arbor --version
```

The release profile optimises for size (`opt-level = "s"`, thin LTO), so a
release build takes considerably longer than a debug build.

## Installing

`install.sh` builds Arbor and installs it to `~/.local/bin/arbor`:

```sh
./install.sh                 # build + install
./install.sh --no-build      # install an existing target/release/arbor
./install.sh --prefix /usr   # custom prefix (needs write access)
```

## Shell language and commands

Unchanged from Nushell 0.117.0. Useful starting points in the upstream docs:

- [Thinking in Nu](https://www.nushell.sh/book/thinking_in_nu.html)
- [Command reference](https://www.nushell.sh/commands/)
- [Configuration](https://www.nushell.sh/book/configuration.html) — Arbor keeps
  these files in `~/.config/arbor/` (`config.nu`, `env.nu`).

## Keeping up with upstream

```sh
git remote add upstream https://github.com/nushell/nushell
git fetch upstream --tags
git merge upstream/main
```

Conflicts are expected only in `Cargo.toml`, `crates/nu-config/src/paths.rs`,
`crates/nu-config/src/resolve.rs`, `src/command.rs` and this README.

## License

MIT. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
