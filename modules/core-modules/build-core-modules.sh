#!/usr/bin/env bash
# Builds component-model .wasm artifacts for every core module.
#
# Each core-module crate has a companion `wit-guest/` subcrate (standalone
# workspace, `crate-type = ["cdylib"]`) that re-exports the core-module's
# `#[slicer_module]`-decorated type. Compiling the wit-guest for the
# `wasm32-unknown-unknown` target pulls the core-module crate into the
# wasm build so the macro-emitted component-export module (guarded by
# `#[cfg(target_arch = "wasm32")]`) is included in the final `.wasm`.
# The resulting core module is then converted to a component-model
# binary via `wasm-tools component new` and copied to the canonical
# `<module-dir>/<module-dir>.wasm` path the manifest loader resolves
# from `<module-dir>.toml` via `with_extension("wasm")`.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-tools
#
# Usage:
#   modules/core-modules/build-core-modules.sh          # build all
#   modules/core-modules/build-core-modules.sh --check  # verify freshness

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Compute the newest mtime across the workspace's WIT files. Any WIT change
# invalidates every guest's wit-bindgen output, so a guest WASM is stale if
# its mtime is older than any WIT file even when the guest's own source is
# unchanged. Without this, a packet that edits `wit/*.wit` but does not
# touch every guest's `src/lib.rs` leaves stale guest WASMs that import the
# pre-edit WIT shape and fail typed instantiation at runtime.
WIT_DIR="$SCRIPT_DIR/../../wit"
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
# runtime. slicer-core is intentionally NOT tracked: only 6 of 20 guests
# depend on it, so global tracking would force spurious rebuilds for the
# other 14. slicer-helpers is host-only and not a guest dep.
SHARED_GUEST_CRATES=(
    "$SCRIPT_DIR/../../crates/slicer-macros"
    "$SCRIPT_DIR/../../crates/slicer-sdk"
    "$SCRIPT_DIR/../../crates/slicer-ir"
    "$SCRIPT_DIR/../../crates/slicer-schema"
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

# Entries: "<module-dir>:<guest-crate-lib-name>"
# The produced component is copied to "<module-dir>/<module-dir>.wasm".
MODULES=(
    "layer-planner-default:layer_planner_default_guest"
    "mesh-segmentation:mesh_segmentation_guest"
    "paint-segmentation:paint_segmentation_guest"
    "arachne-perimeters:arachne_perimeters_guest"
    "classic-perimeters:classic_perimeters_guest"
    "fuzzy-skin:fuzzy_skin_guest"
    "gyroid-infill:gyroid_infill_guest"
    "lightning-infill:lightning_infill_guest"
    "paint-region-annotator:paint_region_annotator_guest"
    "rectilinear-infill:rectilinear_infill_guest"
    "seam-planner-default:seam_planner_default_guest"
    "seam-placer:seam_placer_guest"
    "part-cooling:part_cooling_guest"
    "skirt-brim:skirt_brim_guest"
    "support-planner:support_planner_guest"
    "support-surface-ironing:support_surface_ironing_guest"
    "top-surface-ironing:top_surface_ironing_guest"
    "traditional-support:traditional_support_guest"
    "tree-support:tree_support_guest"
    "wipe-tower:wipe_tower_guest"
    "path-optimization-default:path_optimization_default_guest"
)

# Modules whose stages the host dispatcher does not yet route
# (`PrePass::MeshSegmentation`, `PrePass::PaintSegmentation`; see the
# `resolve_world_glue` comment in `crates/slicer-macros/src/lib.rs`).
# The `#[slicer_module]` macro intentionally leaves these stages on the
# placeholder-export path, which would produce a small but incomplete
# `.wasm` that is detected as a real component and then fails typed
# instantiation. Until the host is extended, keep these entries at the
# documented 8-byte WASM-magic placeholder so `manifest::is_placeholder_wasm`
# flags them and `WasmRuntimeDispatcher` skips them gracefully (docs/07
# Known Deviations §TASK-109).
PLACEHOLDER_MODULES=(
)

check_only=false
if [[ "${1:-}" == "--check" ]]; then
    check_only=true
fi

stale=0

for entry in "${MODULES[@]}"; do
    IFS=: read -r dir_name lib_name <<< "$entry"
    module_dir="$SCRIPT_DIR/$dir_name"
    guest_dir="$module_dir/wit-guest"
    component_path="$module_dir/$dir_name.wasm"
    source_file="$guest_dir/src/lib.rs"
    cargo_file="$guest_dir/Cargo.toml"

    if [[ ! -d "$guest_dir" ]]; then
        echo "SKIP: $dir_name has no wit-guest/ subdirectory"
        continue
    fi

    if [[ -f "$component_path" ]]; then
        comp_size=$(stat -c %s "$component_path" 2>/dev/null || stat -f %z "$component_path")
        if (( comp_size > 8 )); then
            src_mtime=$(stat -c %Y "$source_file" 2>/dev/null || stat -f %m "$source_file")
            cargo_mtime=$(stat -c %Y "$cargo_file" 2>/dev/null || stat -f %m "$cargo_file")
            # Also consider the module's own source (not just the wit-guest
            # shim): edits to the `#[slicer_module]`-decorated impl block
            # need to rebuild the wasm even when the wit-guest shim is stable.
            module_src="$module_dir/src/lib.rs"
            module_cargo="$module_dir/Cargo.toml"
            mod_src_mtime=$(stat -c %Y "$module_src" 2>/dev/null || stat -f %m "$module_src" 2>/dev/null || echo 0)
            mod_cargo_mtime=$(stat -c %Y "$module_cargo" 2>/dev/null || stat -f %m "$module_cargo" 2>/dev/null || echo 0)
            wasm_mtime=$(stat -c %Y "$component_path" 2>/dev/null || stat -f %m "$component_path")
            newest_src=$src_mtime
            [[ $cargo_mtime -gt $newest_src ]] && newest_src=$cargo_mtime
            [[ $mod_src_mtime -gt $newest_src ]] && newest_src=$mod_src_mtime
            [[ $mod_cargo_mtime -gt $newest_src ]] && newest_src=$mod_cargo_mtime
            [[ $wit_mtime -gt $newest_src ]] && newest_src=$wit_mtime
            [[ $shared_guest_mtime -gt $newest_src ]] && newest_src=$shared_guest_mtime
            if (( newest_src <= wasm_mtime )); then
                if $check_only; then
                    echo "  ok: $dir_name.wasm is up to date"
                fi
                continue
            fi
        fi
    fi

    if $check_only; then
        echo "STALE: $dir_name.wasm needs rebuild"
        stale=1
        continue
    fi

    echo "Building $dir_name..."
    (
        cd "$guest_dir"
        cargo build --target wasm32-unknown-unknown --release --quiet
    )

    core_wasm="$guest_dir/target/wasm32-unknown-unknown/release/$lib_name.wasm"
    if [[ ! -f "$core_wasm" ]]; then
        echo "ERROR: expected wasm at $core_wasm" >&2
        exit 1
    fi

    wasm-tools component new "$core_wasm" -o "$component_path"
    echo "  -> $dir_name.wasm"
done

# Ensure the documented placeholder modules (stages not yet routed by
# the host dispatcher) carry the canonical 8-byte WASM-magic header so
# `manifest::is_placeholder_wasm` continues to flag them and the
# runtime skips them gracefully. 8 bytes = `\0asm\x01\x00\x00\x00`.
placeholder_bytes_needed() {
    local p=$1
    if [[ ! -f "$p" ]]; then
        return 0
    fi
    local size
    size=$(stat -c %s "$p" 2>/dev/null || stat -f %z "$p")
    (( size != 8 ))
}

for dir_name in "${PLACEHOLDER_MODULES[@]}"; do
    component_path="$SCRIPT_DIR/$dir_name/$dir_name.wasm"
    if placeholder_bytes_needed "$component_path"; then
        if $check_only; then
            echo "STALE (placeholder): $dir_name.wasm must be 8-byte stub"
            stale=1
            continue
        fi
        printf '\x00asm\x01\x00\x00\x00' > "$component_path"
        echo "  -> $dir_name.wasm (8-byte placeholder; stage not yet routed by host)"
    else
        if $check_only; then
            echo "  ok: $dir_name.wasm is the documented 8-byte placeholder"
        fi
    fi
done

if $check_only && (( stale > 0 )); then
    echo ""
    echo "Run: modules/core-modules/build-core-modules.sh"
    exit 1
fi

if ! $check_only; then
    echo "All core module components built."
fi
