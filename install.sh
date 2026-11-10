#!/usr/bin/env bash
# Arbor — installer
#
# Arbor is a fork of Nushell. This script builds it and installs the
# `arbor` binary to $PREFIX/bin.
#
# Usage:
#   bash install.sh                 # build from source and install
#   bash install.sh --no-build      # install an existing target/release/arbor
#   bash install.sh --prefix DIR    # install prefix (default: ~/.local)
#   bash install.sh --version       # print the Arbor version and exit
#   bash install.sh --help

set -euo pipefail

PREFIX="${ARBOR_PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
ARBOR_REPO_URL="${ARBOR_REPO_URL:-}"

DO_BUILD=1
PRINT_VERSION=0

print_help() {
    cat <<'EOF'
Arbor installer

Usage: bash install.sh [options]

Options:
  --no-build        Install an existing target/release/arbor without building
  --prefix DIR      Install to DIR/bin (default: ~/.local)
  --version         Print the Arbor version and exit
  -h, --help        Show this help

Environment:
  ARBOR_PREFIX      Same as --prefix
  ARBOR_REPO_URL    Git URL used when not running inside a source checkout
EOF
}

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

info() {
    printf '  %s\n' "$*"
}

# ── argument parsing ──────────────────────────────────────────────────────
# Uses an index loop so `--prefix VALUE` (space form) consumes its value.
args=("$@")
i=0
while [ $i -lt ${#args[@]} ]; do
    arg="${args[$i]}"
    case "$arg" in
        --no-build) DO_BUILD=0 ;;
        --version)  PRINT_VERSION=1 ;;
        --prefix)
            if [ $((i + 1)) -ge ${#args[@]} ]; then
                die "--prefix requires a value"
            fi
            i=$((i + 1))
            PREFIX="${args[$i]}"
            ;;
        --prefix=*) PREFIX="${arg#--prefix=}" ;;
        -h|--help)  print_help; exit 0 ;;
        *) die "unknown option: $arg (try --help)" ;;
    esac
    i=$((i + 1))
done

# Recompute derived directories AFTER prefix parsing so a custom --prefix is
# actually honoured.
BIN_DIR="$PREFIX/bin"

# ── locate source ─────────────────────────────────────────────────────────
here="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
if [ -f "$here/Cargo.toml" ] && grep -q 'name = "nu"' "$here/Cargo.toml"; then
    SRC_DIR="$here"
else
    [ -n "$ARBOR_REPO_URL" ] || die "not inside an Arbor source tree; set ARBOR_REPO_URL"
    TMP_SRC="$(mktemp -d -t arbor-src-XXXXXX)"
    # shellcheck disable=SC2064
    trap "rm -rf '$TMP_SRC'" EXIT
    info "fetching source from $ARBOR_REPO_URL ..."
    git clone --depth 1 "$ARBOR_REPO_URL" "$TMP_SRC" >/dev/null 2>&1 \
        || die "git clone failed"
    SRC_DIR="$TMP_SRC"
fi

version_of() {
    sed -n 's/^version = "\(.*\)"$/\1/p' "$1/Cargo.toml" | head -1
}

if [ "$PRINT_VERSION" -eq 1 ]; then
    v="$(version_of "$SRC_DIR")"
    printf 'arbor %s\n' "${v:-unknown}"
    exit 0
fi

# ── build ─────────────────────────────────────────────────────────────────
BIN="$SRC_DIR/target/release/arbor"
if [ "$DO_BUILD" -eq 1 ]; then
    command -v cargo >/dev/null 2>&1 || die "cargo not found; install a Rust toolchain"
    info "building Arbor (release) ..."
    ( cd "$SRC_DIR" && cargo build --release )
else
    info "skipping build (--no-build)"
fi

[ -f "$BIN" ] || die "build produced no binary at $BIN"

# ── install ───────────────────────────────────────────────────────────────
mkdir -p "$BIN_DIR"

# Copy to a temp name then rename: replacing the binary atomically avoids
# "Text file busy" when an arbor process is currently running.
tmp_bin="$BIN_DIR/.arbor.new.$$"
cp "$BIN" "$tmp_bin"
chmod 755 "$tmp_bin"
mv -f "$tmp_bin" "$BIN_DIR/arbor"

info "installed $("$BIN_DIR/arbor" --version) to $BIN_DIR/arbor"

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) printf '\nnote: %s is not on your PATH\n' "$BIN_DIR" ;;
esac

printf '\nDone. Configuration lives in ~/.config/arbor (existing ~/.config/nushell is reused automatically).\n'
