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

# Compute the newest mtime across the workspace's WIT files. Any WIT change
# invalidates every guest's wit-bindgen output, so a guest WASM is stale if
# its mtime is older than any WIT file even when the guest's own source is
# unchanged. Without this, a packet that edits `wit/*.wit` but does not
# touch every guest's `src/lib.rs` leaves stale guest WASMs that import the
# pre-edit WIT shape and fail typed instantiation at runtime.
WIT_DIR="$REPO_ROOT/wit"
wit_mtime=0
if [[ -d "$WIT_DIR" ]]; then
    while IFS= read -r -d '' wit_file; do
        m=$(stat -c %Y "$wit_file" 2>/dev/null || stat -f %m "$wit_file" 2>/dev/null || echo 0)
        (( m > wit_mtime )) && wit_mtime=$m
    done < <(find "$WIT_DIR" -type f -name '*.wit' -print0)
fi

# Compute the newest mtime across workspace crates that get baked into
# every guest .wasm. The proc-macro in slicer-macros emits all wit-bindgen
# glue; slicer-sdk / slicer-ir / slicer-schema are universal guest deps
# with no `cfg(target_arch = "wasm32")` gates, so their source edits
# propagate directly into guest wasm bytes. Without this, editing any of
# these crates leaves stale guest WASMs that fail typed instantiation at
# runtime. slicer-core is intentionally NOT tracked: only a subset of
# guests depend on it, so global tracking would force spurious rebuilds
# for the others. slicer-helpers is host-only and not a guest dep.
SHARED_GUEST_CRATES=(
    "$REPO_ROOT/crates/slicer-macros"
    "$REPO_ROOT/crates/slicer-sdk"
    "$REPO_ROOT/crates/slicer-ir"
    "$REPO_ROOT/crates/slicer-schema"
)
shared_guest_mtime=0
for crate_dir in "${SHARED_GUEST_CRATES[@]}"; do
    if [[ -d "$crate_dir/src" ]]; then
        while IFS= read -r -d '' f; do
            m=$(stat -c %Y "$f" 2>/dev/null || stat -f %m "$f" 2>/dev/null || echo 0)
            (( m > shared_guest_mtime )) && shared_guest_mtime=$m
        done < <(find "$crate_dir/src" -type f -print0)
    fi
    cargo_toml="$crate_dir/Cargo.toml"
    if [[ -f "$cargo_toml" ]]; then
        m=$(stat -c %Y "$cargo_toml" 2>/dev/null || stat -f %m "$cargo_toml" 2>/dev/null || echo 0)
        (( m > shared_guest_mtime )) && shared_guest_mtime=$m
    fi
done

GUESTS=(
    "layer-infill-guest:layer_infill_guest"
    "prepass-guest:prepass_guest"
    "finalization-guest:finalization_guest"
    "postpass-guest:postpass_guest"
    "sdk-postpass-text-guest:sdk_postpass_text_guest"
    "sdk-finalization-guest:sdk_finalization_guest"
    "sdk-prepass-guest:sdk_prepass_guest"
    "sdk-prepass-meshseg-guest:sdk_prepass_meshseg_guest"
    "sdk-layer-infill-guest:sdk_layer_infill_guest"
    "sdk-layer-pathopt-guest:sdk_layer_pathopt_guest"
    "path-optimization-multi-read:path_optimization_multi_read_guest"
    "finalization-mutation-roundtrip-guest:finalization_mutation_roundtrip_guest"
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
        newest_src=$src_mtime
        [[ $cargo_mtime -gt $newest_src ]] && newest_src=$cargo_mtime
        [[ $wit_mtime -gt $newest_src ]] && newest_src=$wit_mtime
        [[ $shared_guest_mtime -gt $newest_src ]] && newest_src=$shared_guest_mtime
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
