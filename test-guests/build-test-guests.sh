#!/usr/bin/env bash
# Builds all test guest components from source.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-tools
#
# Usage:
#   ./test-guests/build-test-guests.sh          # build all
#   ./test-guests/build-test-guests.sh --check  # verify freshness only (exit 1 if stale)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

GUESTS=(
    "layer-infill-guest:layer_infill_guest"
    "prepass-guest:prepass_guest"
    "finalization-guest:finalization_guest"
    "postpass-guest:postpass_guest"
    "sdk-postpass-text-guest:sdk_postpass_text_guest"
    "sdk-finalization-guest:sdk_finalization_guest"
    "sdk-prepass-guest:sdk_prepass_guest"
    "sdk-layer-infill-guest:sdk_layer_infill_guest"
)

check_only=false
if [[ "${1:-}" == "--check" ]]; then
    check_only=true
fi

stale=0

for entry in "${GUESTS[@]}"; do
    IFS=: read -r dir_name lib_name <<< "$entry"
    guest_dir="$SCRIPT_DIR/$dir_name"
    component_path="$SCRIPT_DIR/$dir_name.component.wasm"
    source_file="$guest_dir/src/lib.rs"
    cargo_file="$guest_dir/Cargo.toml"

    # Check if source is newer than the component
    if [[ -f "$component_path" ]]; then
        src_mtime=$(stat -c %Y "$source_file" 2>/dev/null || stat -f %m "$source_file")
        cargo_mtime=$(stat -c %Y "$cargo_file" 2>/dev/null || stat -f %m "$cargo_file")
        wasm_mtime=$(stat -c %Y "$component_path" 2>/dev/null || stat -f %m "$component_path")
        newest_src=$((src_mtime > cargo_mtime ? src_mtime : cargo_mtime))
        if (( newest_src <= wasm_mtime )); then
            if $check_only; then
                echo "  ok: $dir_name.component.wasm is up to date"
            fi
            continue
        fi
    fi

    if $check_only; then
        echo "STALE: $dir_name.component.wasm needs rebuild (source is newer)"
        stale=1
        continue
    fi

    echo "Building $dir_name..."
    (
        cd "$guest_dir"
        cargo build --target wasm32-unknown-unknown --release --quiet
    )

    wasm_path="$guest_dir/target/wasm32-unknown-unknown/release/$lib_name.wasm"
    if [[ ! -f "$wasm_path" ]]; then
        echo "ERROR: expected wasm at $wasm_path" >&2
        exit 1
    fi

    wasm-tools component new "$wasm_path" -o "$component_path"
    echo "  -> $dir_name.component.wasm"
done

if $check_only && (( stale > 0 )); then
    echo ""
    echo "Run: ./test-guests/build-test-guests.sh"
    exit 1
fi

if ! $check_only; then
    echo "All test guest components built."
fi
