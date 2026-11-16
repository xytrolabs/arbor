#!/usr/bin/env bash
# Arbor — a fork of Nushell
# Cross-platform installer: downloads a prebuilt binary for your OS/arch,
# verifies its SHA-256, and installs it. Falls back to building from source.
#
# Supported platforms (prebuilt):
#   Linux   x86_64, aarch64, armv7   (musl static, plus gnu for x86_64/aarch64)
#   macOS   x86_64, aarch64 (Apple Silicon)
#   Windows x86_64                   (.zip; inside WSL use the Linux build)
#
# Usage:
#   curl -fsSL https://xytro.site/arbor/install.sh | sh
#   curl -fsSL https://xytro.site/arbor/install.sh | sh -s -- --pin v1.0.0
#   ./install.sh                       # auto: binary if available, else source
#   ./install.sh --from-source         # force a source build
#   ./install.sh --no-build            # reuse ./target/release/arbor
#   ./install.sh --prefix DIR          # install elsewhere
#   ./install.sh --dry-run             # show the plan, install nothing
#
# Environment overrides:
#   ARBOR_PREFIX      install prefix      (default: ~/.local)
#   ARBOR_VERSION     version to download (default: latest release)
#   ARBOR_REPO        GitHub repo         (default: xytrolabs/arbor)
#   ARBOR_BASE_URL    asset base URL      (default: GitHub release assets)

set -euo pipefail

# ── Config ──────────────────────────────────────────────────────────────
PREFIX="${ARBOR_PREFIX:-$HOME/.local}"
ARBOR_VERSION="${ARBOR_VERSION:-}"
ARBOR_REPO="${ARBOR_REPO:-xytrolabs/arbor}"
ARBOR_BASE_URL="${ARBOR_BASE_URL:-}"
BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share/arbor"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$PWD}")" 2>/dev/null && pwd || echo "$PWD")"

MODE="auto"        # auto | binary | source | nobuild
DRY_RUN=0
SRC_DIR=""
TMP_DIR=""
ASSET_URL=""
OS_NAME=""
TARGET=""

cleanup() {
  if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then rm -rf "$TMP_DIR"; fi
  return 0
}
trap cleanup EXIT

# ── Pretty printing ─────────────────────────────────────────────────────
if [ -t 1 ] && [ "${NO_COLOR:-}" = "" ]; then
  c_green=$'\033[32m'; c_yellow=$'\033[33m'; c_cyan=$'\033[36m'; c_red=$'\033[31m'; c_reset=$'\033[0m'
else
  c_green=""; c_yellow=""; c_cyan=""; c_red=""; c_reset=""
fi
info() { printf "%s-%s %s\n" "$c_cyan" "$c_reset" "$*"; }
ok()   { printf "%s+%s %s\n" "$c_green" "$c_reset" "$*"; }
warn() { printf "%s!%s %s\n" "$c_yellow" "$c_reset" "$*"; }
die()  { printf "%sx%s %s\n" "$c_red" "$c_reset" "$*" >&2; exit 1; }

have() { command -v "$1" >/dev/null 2>&1; }

# ── Detect OS / arch into a Rust-style target triple ────────────────────
detect_target() {
  local uname_s uname_m
  uname_s="$(uname -s)"
  uname_m="$(uname -m)"

  case "$uname_s" in
    Linux)
      OS_NAME="linux"
      case "$uname_m" in
        x86_64|amd64)  TARGET="x86_64-unknown-linux-musl" ;;
        aarch64|arm64) TARGET="aarch64-unknown-linux-musl" ;;
        armv7l|armv7)  TARGET="armv7-unknown-linux-musleabihf" ;;
        *)             TARGET="" ;;
      esac
      ;;
    Darwin)
      OS_NAME="macos"
      case "$uname_m" in
        arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
        x86_64)        TARGET="x86_64-apple-darwin" ;;
        *)             TARGET="" ;;
      esac
      ;;
    MINGW*|MSYS*|CYGWIN*)
      OS_NAME="windows"; TARGET="x86_64-pc-windows-msvc" ;;
    *)
      OS_NAME="unknown"; TARGET="" ;;
  esac

  if [ -n "$TARGET" ]; then
    ok "Platform: ${OS_NAME} / ${uname_m}  ->  ${TARGET}"
  else
    warn "No prebuilt binary for ${uname_s}/${uname_m}; a source build will be used"
  fi
}

# ── Network helpers ─────────────────────────────────────────────────────
fetch_to() {  # fetch_to URL DEST
  local url="$1" dest="$2"
  if have curl; then
    curl -fsSL --retry 3 --connect-timeout 15 "$url" -o "$dest"
  elif have wget; then
    wget -qO "$dest" "$url"
  else
    return 1
  fi
}

fetch_stdout() {  # fetch_stdout URL
  local url="$1"
  if have curl; then
    curl -fsSL --retry 2 --connect-timeout 15 "$url" 2>/dev/null
  elif have wget; then
    wget -qO- "$url" 2>/dev/null
  else
    return 1
  fi
}

resolve_version() {
  [ -n "$ARBOR_VERSION" ] && { info "Version: $ARBOR_VERSION (pinned)"; return 0; }
  local tag
  tag="$(fetch_stdout "https://api.github.com/repos/${ARBOR_REPO}/releases/latest" \
        | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 \
        | sed 's/.*"\([^"]*\)"[[:space:]]*$/\1/')" || true
  if [ -n "$tag" ]; then
    ARBOR_VERSION="$tag"
    info "Version: $ARBOR_VERSION (latest)"
  fi
  return 0
}

archive_ext() {
  case "$TARGET" in *windows*) echo zip ;; *) echo tar.gz ;; esac
}

asset_stem() {
  # arbor-<version>-<target>, or empty when the version is unknown.
  [ -n "$ARBOR_VERSION" ] || return 0
  printf 'arbor-%s-%s' "${ARBOR_VERSION#v}" "$TARGET"
}

# Sets ASSET_URL when a downloadable asset exists.
try_binary() {
  [ -n "$TARGET" ] || return 1
  local ext stem
  ext="$(archive_ext)"
  stem="$(asset_stem)"

  local -a candidates=()
  if [ -n "$ARBOR_BASE_URL" ]; then
    [ -n "$stem" ] && candidates+=("${ARBOR_BASE_URL%/}/${stem}.${ext}")
    candidates+=("${ARBOR_BASE_URL%/}/${TARGET}.${ext}")
  else
    if [ -n "$stem" ]; then
      candidates+=("https://github.com/${ARBOR_REPO}/releases/download/${ARBOR_VERSION}/${stem}.${ext}")
    fi
    # `latest/download` needs a stable, version-less name; try the tag-less form.
    candidates+=("https://github.com/${ARBOR_REPO}/releases/latest/download/arbor-${TARGET}.${ext}")
  fi

  local url
  for url in "${candidates[@]}"; do
    [ -n "$url" ] || continue
    if [ "$DRY_RUN" -eq 1 ]; then
      info "Would try: $url"
      continue
    fi
    if have curl && curl -fsIL --connect-timeout 15 "$url" >/dev/null 2>&1; then
      ASSET_URL="$url"
      return 0
    fi
  done
  return 1
}

# ── Checksum verification ───────────────────────────────────────────────
verify_checksum() {  # verify_checksum FILE URL
  local file="$1" url="$2" expected actual
  TMP_DIR="${TMP_DIR:-$(mktemp -d)}"
  if ! fetch_to "${url}.sha256" "$TMP_DIR/sum" 2>/dev/null || [ ! -s "$TMP_DIR/sum" ]; then
    warn "No checksum published for $(basename "$file") - skipping verification"
    return 0
  fi
  expected="$(awk '{print $1}' "$TMP_DIR/sum" | head -1 | tr 'A-Z' 'a-z')"
  if have sha256sum; then
    actual="$(sha256sum "$file" | awk '{print $1}')"
  elif have shasum; then
    actual="$(shasum -a 256 "$file" | awk '{print $1}')"
  elif have openssl; then
    actual="$(openssl dgst -sha256 "$file" | awk '{print $NF}')"
  else
    warn "No sha256sum/shasum/openssl - skipping verification"
    return 0
  fi
  if [ "$expected" != "$actual" ]; then
    die "Checksum mismatch for $(basename "$file")
  expected: $expected
  actual:   $actual"
  fi
  ok "Checksum verified"
}

download_binary() {
  TMP_DIR="${TMP_DIR:-$(mktemp -d)}"
  local asset
  asset="$TMP_DIR/$(basename "$ASSET_URL")"
  info "Downloading $(basename "$ASSET_URL") ..."
  fetch_to "$ASSET_URL" "$asset" || die "Download failed: $ASSET_URL"
  verify_checksum "$asset" "$ASSET_URL"

  mkdir -p "$TMP_DIR/x"
  case "$asset" in
    *.zip)
      if have unzip; then
        unzip -q "$asset" -d "$TMP_DIR/x"
      elif have python3; then
        python3 -c 'import sys,zipfile;zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' \
          "$asset" "$TMP_DIR/x"
      else
        die "Need unzip (or python3) to extract the .zip"
      fi
      ;;
    *)
      tar -xzf "$asset" -C "$TMP_DIR/x" || die "Failed to extract archive"
      ;;
  esac

  local found
  found="$(find "$TMP_DIR/x" -type f \( -name arbor -o -name arbor.exe \) -print -quit)"
  [ -n "$found" ] || die "Archive did not contain an arbor binary"
  BIN_PATH="$found"
  chmod +x "$BIN_PATH" 2>/dev/null || true
  ok "Extracted $(basename "$BIN_PATH")"
}

# ── Source build (fallback) ─────────────────────────────────────────────
ensure_cargo() {
  if have cargo; then
    ok "cargo found: $(cargo --version 2>/dev/null | head -n1)"
    return 0
  fi
  warn "cargo not found - installing Rust via rustup..."
  if have curl; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
      || die "Failed to install Rust via rustup"
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env" || true
  elif have wget; then
    wget -qO- https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
      || die "Failed to install Rust via rustup"
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env" || true
  else
    die "Need a Rust toolchain. Install from https://rustup.rs first."
  fi
  have cargo || die "cargo still not on PATH. Restart your shell and re-run install.sh"
  ok "Rust installed: $(cargo --version 2>/dev/null | head -n1)"
}

resolve_source() {
  if [ -f "$REPO_DIR/Cargo.toml" ] && grep -q 'name *= *"arbor"' "$REPO_DIR/Cargo.toml" 2>/dev/null; then
    SRC_DIR="$REPO_DIR"
    info "Using source at $SRC_DIR"
    return 0
  fi
  TMP_DIR="${TMP_DIR:-$(mktemp -d)}"
  SRC_DIR="$TMP_DIR/src"
  local repo_url="https://github.com/${ARBOR_REPO}"
  info "Fetching source from $repo_url ..."
  if have git; then
    git clone --depth 1 ${ARBOR_VERSION:+--branch "$ARBOR_VERSION"} "$repo_url" "$SRC_DIR" \
      || die "git clone failed"
  elif have curl; then
    curl -fsSL "$repo_url/archive/refs/heads/main.tar.gz" \
      | tar -xz -C "$TMP_DIR" --strip-components=1 -C "$TMP_DIR" || die "source download failed"
    [ -d "$SRC_DIR" ] || SRC_DIR="$TMP_DIR"
  else
    die "Need git or curl to fetch the Arbor source"
  fi
  [ -f "$SRC_DIR/Cargo.toml" ] || die "Fetched source is missing Cargo.toml"
  ok "Source ready"
}

build_source() {
  info "Building Arbor (release) - this takes a while..."
  have cargo || export PATH="$HOME/.cargo/bin:$PATH"
  ( cd "$SRC_DIR" && cargo build --release ) || die "Build failed. See errors above."
  [ -f "$SRC_DIR/target/release/arbor" ] || die "Build produced no binary"
  BIN_PATH="$SRC_DIR/target/release/arbor"
  ok "Build complete"
}

# ── Install ─────────────────────────────────────────────────────────────
install_binary() {
  local src="$1"
  mkdir -p "$BIN_DIR" "$SHARE_DIR"

  # Atomic replace: a RUNNING arbor keeps its inode, and we never hit
  # "Text file busy".
  local tmp_bin="$BIN_DIR/arbor.new.$$"
  cp "$src" "$tmp_bin"
  chmod +x "$tmp_bin"
  mv -f "$tmp_bin" "$BIN_DIR/arbor"
  ok "Installed arbor -> $BIN_DIR/arbor"

  local base="$REPO_DIR"
  [ -n "$SRC_DIR" ] && base="$SRC_DIR"
  if [ -f "$base/share/arbor/arbor.ind" ]; then
    cp "$base/share/arbor/arbor.ind" "$SHARE_DIR/arbor.ind"
    ok "Installed helper library -> $SHARE_DIR/arbor.ind"
  fi
  if [ -d "$base/docs" ] && [ -f "$base/docs/01-getting-started.md" ]; then
    mkdir -p "$SHARE_DIR/docs"
    cp "$base"/docs/*.md "$SHARE_DIR/docs/" 2>/dev/null || true
    ok "Installed docs -> $SHARE_DIR/docs"
  fi
  add_to_path
}

add_to_path() {
  case ":$PATH:" in
    *":$BIN_DIR:"*) : ;;
    *)
      warn "Adding $BIN_DIR to your PATH..."
      local shell_name
      shell_name="${SHELL:-$(basename "$(command -v sh)")}"
      case "$shell_name" in
        *fish)
          fish -c "fish_add_path $BIN_DIR" 2>/dev/null \
            || echo "fish_add_path $BIN_DIR" >> "$HOME/.config/fish/config.fish" ;;
        *zsh)
          grep -q "$BIN_DIR" "$HOME/.zshrc" 2>/dev/null \
            || echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$HOME/.zshrc" ;;
        *)
          grep -q "$BIN_DIR" "$HOME/.bashrc" 2>/dev/null \
            || echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$HOME/.bashrc" ;;
      esac
      ;;
  esac
}

check_indent() {
  if have indent || [ -x "$HOME/.local/bin/indent" ]; then
    ok "Indent runtime found (Indent scripts will work)"
  else
    warn "Indent runtime not found. Nushell scripts still work, but .ind scripts won't."
    warn "Install from https://indent.xytro.site or set ARBOR_INDENT_BINARY."
  fi
}

# ── Version / help ──────────────────────────────────────────────────────
print_version() {
  local ver=""
  if [ -f "$REPO_DIR/Cargo.toml" ]; then
    ver=$(grep -m1 '^version' "$REPO_DIR/Cargo.toml" || true)
  fi
  if [ -n "$ver" ]; then
    ver="${ver#* =}"; ver="${ver//\"/}"; ver="${ver// /}"
    printf 'arbor %s\n' "$ver"
  elif have arbor; then
    arbor --version 2>/dev/null | head -1
  else
    echo "arbor (installer)"
  fi
}

print_help() {
  cat <<'EOF'
Arbor installer - a fork of Nushell

Downloads a prebuilt binary for your OS/arch, verifies its SHA-256, and
installs it. Falls back to building from source when no binary fits.

Usage:
  curl -fsSL https://xytro.site/arbor/install.sh | sh
  curl -fsSL https://xytro.site/arbor/install.sh | sh -s -- --pin v1.0.0
  ./install.sh [options]

Options:
  --prefix DIR     Install under DIR (default ~/.local)
  --pin VERSION    Download this release (e.g. v1.0.0) instead of latest
  --from-source    Skip the prebuilt binary; build with cargo
  --binary         Require a prebuilt binary (never build from source)
  --no-build       Reuse ./target/release/arbor (no download, no build)
  --dry-run        Show what would happen without changing anything
  -h, --help       Show this help
  --version        Print version and exit

Platforms with prebuilt binaries:
  Linux    x86_64, aarch64, armv7  (musl static)
  macOS    x86_64, aarch64 (Apple Silicon)
  Windows  x86_64 (.zip; inside WSL use the Linux build)

Environment:
  ARBOR_PREFIX, ARBOR_VERSION, ARBOR_REPO, ARBOR_BASE_URL
EOF
}

# ── Main ────────────────────────────────────────────────────────────────
main() {
  local i=0
  while [ "$i" -lt "$#" ]; do
    local arg="${*:i+1:1}"
    case "$arg" in
      --from-source)  MODE="source" ;;
      --binary)       MODE="binary" ;;
      --no-build)     MODE="nobuild" ;;
      --dry-run)      DRY_RUN=1 ;;
      --version)      print_version; exit 0 ;;
      --pin)
        i=$((i+1)); [ "$i" -lt "$#" ] || die "--pin requires a version"
        ARBOR_VERSION="${*:i+1:1}" ;;
      --pin=*)        ARBOR_VERSION="${arg#--pin=}" ;;
      --prefix)
        i=$((i+1)); [ "$i" -lt "$#" ] || die "--prefix requires a value"
        PREFIX="${*:i+1:1}" ;;
      --prefix=*)     PREFIX="${arg#--prefix=}" ;;
      -h|--help)      print_help; exit 0 ;;
      *) die "Unknown option: $arg (see --help)" ;;
    esac
    i=$((i+1))
  done

  # Recompute paths after --prefix parsing.
  BIN_DIR="$PREFIX/bin"
  SHARE_DIR="$PREFIX/share/arbor"

  detect_target

  echo
  echo "  ${c_green}Arbor - a fork of Nushell${c_reset}"
  echo "  $(print_version)${TARGET:+ | $TARGET}"
  echo

  BIN_PATH=""

  if [ "$MODE" = "nobuild" ]; then
    [ -f "$REPO_DIR/target/release/arbor" ] || die "--no-build needs target/release/arbor (build first)"
    SRC_DIR="$REPO_DIR"
    BIN_PATH="$REPO_DIR/target/release/arbor"
    ok "Using existing build"
  elif [ "$MODE" = "source" ]; then
    ensure_cargo; resolve_source; build_source
  else
    resolve_version
    if [ "$DRY_RUN" -eq 1 ]; then
      if try_binary; then info "Would download: $ASSET_URL"; else info "No prebuilt binary for ${TARGET:-this platform} - would build from source"; fi
      info "Would install arbor -> $BIN_DIR/arbor"
      exit 0
    fi
    if try_binary; then
      download_binary
    elif [ "$MODE" = "binary" ]; then
      die "No prebuilt binary available for ${TARGET:-this platform} (--binary)"
    else
      warn "No prebuilt binary for ${TARGET:-this platform} - building from source"
      ensure_cargo; resolve_source; build_source
    fi
  fi

  install_binary "$BIN_PATH"
  check_indent

  echo
  ok "Arbor installed! Start it with:  arbor"
  info "Docs: $SHARE_DIR/docs"
  info "If 'arbor' isn't found, restart your shell or run: export PATH=\"$BIN_DIR:\$PATH\""
  echo
}

main "$@"
