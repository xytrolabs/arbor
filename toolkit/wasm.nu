const nushell_dir = path self ..

# these crates should compile for wasm
const wasm_compatible_crates = [
    "arbor-cmd-base",
    "arbor-cmd-extra",
    "arbor-cmd-lang",
    "arbor-color-config",
    "arbor-command",
    "arbor-derive-value",
    "arbor-engine",
    "arbor-glob",
    "arbor-json",
    "arbor-parser",
    "arbor-path",
    "arbor-pretty-hex",
    "arbor-protocol",
    "arbor-std",
    "arbor-system",
    "arbor-table",
    "arbor-term-grid",
    "arbor-utils",
    "nuon"
]

def "prep wasm" [] {
    ^rustup target add wasm32-unknown-unknown
}

# Build crates for wasm.
@category "toolkit"
@search-terms wasm webassembly build wasm32 wasm32-unknown-unknown
@example "Build all WASM-compatible crates" { toolkit build wasm }
export def "build wasm" [] {
    prep wasm

    for crate in $wasm_compatible_crates {
        print $'(char nl)Building ($crate) for wasm'
        print '----------------------------'
        (
            ^cargo build
                -p $crate
                --target wasm32-unknown-unknown
                --no-default-features
        )
    }
}

# Make sure no api is used that doesn't work with wasm.
@category "toolkit"
@search-terms wasm clippy lint wasm32 compatibility
@example "Lint crates for WASM compatibility" { toolkit clippy wasm }
export def "clippy wasm" [] {
    prep wasm

    for crate in $wasm_compatible_crates {
        print $'(char nl)Checking ($crate) for wasm'
        print '----------------------------'
        (
            ^cargo clippy
                -p $crate
                --target wasm32-unknown-unknown
                --no-default-features
                --
                -D warnings
                -D clippy::unwrap_used
                -D clippy::unchecked_time_subtraction
        )
    }
}
