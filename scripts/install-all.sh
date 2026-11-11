#!/usr/bin/env bash

set -euo pipefail

DIR=$(readlink -f $(dirname "${BASH_SOURCE[0]}"))
REPO_ROOT=$(dirname $DIR)

echo "-----------------------------------------------------------------"
echo "Installing nushell (nu) and all the plugins"
echo "-----------------------------------------------------------------"
echo ""

echo "Install nushell from local..."
echo "----------------------------------------------"
cargo install --force --path "$REPO_ROOT" --locked

NU_PLUGINS=(
    'arbor_plugin_inc'
    'arbor_plugin_gstat'
    'arbor_plugin_query'
    'arbor_plugin_example'
    'arbor_plugin_custom_values'
    'arbor_plugin_formats'
    'arbor_plugin_polars'
)

for plugin in "${NU_PLUGINS[@]}"
do
    echo ''
    echo "----------------------------------------------"
    echo "Install plugin $plugin from local..."
    echo "----------------------------------------------"
    cargo install --force --path "$REPO_ROOT/crates/$plugin"
done
