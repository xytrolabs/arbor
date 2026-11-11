#!/usr/bin/env bash

set -euo pipefail

DIR=$(readlink -f $(dirname "${BASH_SOURCE[0]}"))
REPO_ROOT=$(dirname $DIR)

echo "---------------------------------------------------------------"
echo "Building nushell (nu) and all the plugins"
echo "---------------------------------------------------------------"
echo ""

NU_PLUGINS=(
    'arbor_plugin_example'
    'arbor_plugin_gstat'
    'arbor_plugin_inc'
    'arbor_plugin_query'
    'arbor_plugin_custom_values'
    'arbor_plugin_polars'
)

echo "Building nushell"
(
    cd $REPO_ROOT
    cargo build --locked
)

for plugin in "${NU_PLUGINS[@]}"
do
    echo "Building $plugin..."
    echo "-----------------------------"
    (
        cd "$REPO_ROOT/crates/$plugin"
        cargo build
    )
done
