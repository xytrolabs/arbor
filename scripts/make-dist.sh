#!/usr/bin/env bash
# Build distributable archives for the website and for GitHub releases.
#
#   scripts/make-dist.sh                       # host target + static musl
#   TARGETS="x86_64-unknown-linux-musl" scripts/make-dist.sh
#   FEATURES=minimal scripts/make-dist.sh      # smaller, fewer features
#
# Produces, in dist/:
#
#   arbor-<version>-<target>.tar.gz           versioned asset (CI naming)
#   arbor-<version>-<target>.tar.gz.sha256
#   arbor-<target>.tar.gz                     version-less "latest" alias
#   arbor-<target>.tar.gz.sha256
#
# The version-less alias is what the installer falls back to when it cannot ask
# GitHub for the latest tag, which is the normal case for site-hosted downloads.
#
# Each archive contains:
#   arbor-<version>-<target>/arbor
#   arbor-<version>-<target>/share/arbor/arbor.ind
#   arbor-<version>-<target>/docs/*.md
#   arbor-<version>-<target>/{LICENSE,NOTICE,README.md}
#
# The installer looks next to the extracted binary for `share/` and `docs/`, so
# a prebuilt install gets the helper library and the documentation too.
set -uo pipefail
cd "$(dirname "$0")/.."

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
[ -n "$VERSION" ] || { echo "error: could not read version from Cargo.toml" >&2; exit 1; }

DIST="${DIST:-dist}"
mkdir -p "$DIST"

host_target() { rustc -vV | sed -n 's/^host: //p'; }

default_targets() {
    host_target
    # A static musl binary runs on any glibc or musl distribution, so it is the
    # one Linux download that covers Breeze, Core and Evergreen alike.
    case "$(host_target)" in
        x86_64-unknown-linux-gnu) echo x86_64-unknown-linux-musl ;;
        aarch64-unknown-linux-gnu) echo aarch64-unknown-linux-musl ;;
    esac
}

features_args() {
    if [ -n "${FEATURES:-}" ]; then
        printf -- '--no-default-features\n--features\n%s\n' "$FEATURES"
    fi
}

# Map a rust target to the musl.cc cross-toolchain prefix, e.g.
# x86_64-unknown-linux-musl -> x86_64-linux-musl.
cross_prefix() {
    case "$1" in
        x86_64-unknown-linux-musl)  echo x86_64-linux-musl ;;
        aarch64-unknown-linux-musl) echo aarch64-linux-musl ;;
        armv7-unknown-linux-musleabihf) echo armv7l-linux-musleabihf ;;
        *) echo "" ;;
    esac
}

# Point cargo at a cross gcc/ar so crates with C sources (ring, for one) can
# build for musl. Installing musl-tools needs root; a musl.cc toolchain unpacked
# into ~/.local/opt does not, and it is what CI uses for the ARM targets.
use_cross_toolchain() {
    local target="$1" prefix bin gcc
    prefix="$(cross_prefix "$target")"
    [ -n "$prefix" ] || return 0

    for bin in "$HOME/.local/opt/$prefix-cross/bin" "$HOME/$prefix-cross/bin"; do
        gcc="$bin/$prefix-gcc"
        if [ -x "$gcc" ]; then
            local upper="${target^^}"
            export "CC_${target//-/_}=$gcc"
            export "AR_${target//-/_}=$bin/$prefix-ar"
            export "CARGO_TARGET_${upper//-/_}_LINKER=$gcc"
            echo "    using cross toolchain $gcc" >&2
            return 0
        fi
    done
    return 0
}

built=()
failed=()

build_target() {
    local target="$1" args
    mapfile -t args < <(features_args)

    # Progress goes to stderr: this function's stdout is the binary path.
    if [ "$target" = "$(host_target)" ]; then
        echo "==> building $target (host)" >&2
        cargo build --release --bin arbor "${args[@]}" 2>&1 | tail -3 >&2
        echo "target/release/arbor"
    else
        echo "==> building $target" >&2
        use_cross_toolchain "$target"
        if ! rustup target list --installed | grep -qx "$target"; then
            echo "    adding rustup target $target" >&2
            rustup target add "$target" >/dev/null 2>&1 || {
                echo "    error: could not add target $target" >&2
                return 1
            }
        fi
        cargo build --release --bin arbor --target "$target" "${args[@]}" 2>&1 | tail -3 >&2
        echo "target/$target/release/arbor"
    fi
}

package_target() {
    local target="$1" bin="$2"
    local stem="arbor-${VERSION}-${target}"
    local stage="$DIST/$stem"

    if [ ! -f "$bin" ]; then
        echo "    error: $bin not found" >&2
        return 1
    fi

    rm -rf "$stage"
    mkdir -p "$stage/share/arbor" "$stage/docs"
    cp "$bin" "$stage/arbor"
    chmod +x "$stage/arbor"
    cp share/arbor/arbor.ind "$stage/share/arbor/" 2>/dev/null || true
    cp docs/*.md "$stage/docs/" 2>/dev/null || true
    for f in LICENSE NOTICE README.md; do
        [ -f "$f" ] && cp "$f" "$stage/"
    done

    tar -czf "$DIST/$stem.tar.gz" -C "$DIST" "$stem"
    ( cd "$DIST" && sha256sum "$stem.tar.gz" > "$stem.tar.gz.sha256" )

    # "latest" alias: same bytes, stable name, so a site-hosted install works
    # without knowing the version.
    cp "$DIST/$stem.tar.gz" "$DIST/arbor-$target.tar.gz"
    ( cd "$DIST" && sha256sum "arbor-$target.tar.gz" > "arbor-$target.tar.gz.sha256" )

    rm -rf "$stage"

    local size
    size="$(du -h "$DIST/$stem.tar.gz" | cut -f1)"
    echo "    packaged $stem.tar.gz ($size)"
}

TARGETS="${TARGETS:-$(default_targets)}"

for target in $TARGETS; do
    if bin="$(build_target "$target")"; then
        if package_target "$target" "$bin"; then
            built+=("$target")
        else
            failed+=("$target (packaging)")
        fi
    else
        failed+=("$target (build)")
    fi
done

echo
echo "version: $VERSION"
echo "built:   ${built[*]:-none}"
[ ${#failed[@]} -gt 0 ] && echo "failed:  ${failed[*]}"
echo
ls -la "$DIST"/*.tar.gz 2>/dev/null | awk '{print "  " $5 "  " $9}'
echo
echo "To publish: copy dist/* to the site's dl/ directory, e.g."
echo "  cp dist/*.tar.gz dist/*.sha256 ../XytroLabs/arbor-site/dl/"

[ ${#failed[@]} -eq 0 ]
