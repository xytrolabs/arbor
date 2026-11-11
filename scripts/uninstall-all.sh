#!/usr/bin/env bash

set -euo pipefail

echo ''
echo "----------------------------------------------"
echo "Uninstall nu and all plugins from cargo/bin..."
echo "----------------------------------------------"

NU_PLUGINS=(
    'arbor_plugin_inc'
    'arbor_plugin_gstat'
    'arbor_plugin_query'
    'arbor_plugin_example'
    'arbor_plugin_formats'
    'arbor_plugin_custom_values'
    'arbor_plugin_polars'
)

cargo uninstall nu
for plugin in "${NU_PLUGINS[@]}"
do
    cargo uninstall "$plugin"
done
