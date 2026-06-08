#![allow(missing_docs)]

//! AC-9 integration test — per-layer host dispatch filter (packet 92).
//!
//! Verifies `module_invocation_allowed_on_layer`:
//! - Module with `[[region_split]] semantic = "material"` is filtered out on
//!   layers where no region's `variant_chain` mentions "material".
//! - Paint-transparent module (no region_split declarations) is always invoked.
//!
//! Test name: `region_split_dispatch_filter`
//! Verification command:
//!   cargo test -p slicer-runtime --test integration region_split_dispatch_filter

use std::collections::HashSet;

use slicer_ir::{PaintValue, SliceIR, SlicedRegion, CURRENT_SLICE_IR_SCHEMA_VERSION};
use slicer_runtime::layer_executor::module_invocation_allowed_on_layer;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal `SliceIR` with the given regions.
fn slice_ir(global_layer_index: u32, regions: Vec<SlicedRegion>) -> SliceIR {
    SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index,
        z: global_layer_index as f32 * 0.2,
        regions,
    }
}

/// A region whose `variant_chain` carries exactly one `("material", ToolIndex(n))` entry.
fn region_with_material_chain(tool_index: u32) -> SlicedRegion {
    SlicedRegion {
        variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(tool_index))],
        ..Default::default()
    }
}

/// A region with an empty `variant_chain` (legacy / no paint semantics).
fn region_no_chain() -> SlicedRegion {
    SlicedRegion {
        variant_chain: vec![],
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Module semantic sets
// ---------------------------------------------------------------------------

/// M_A: declares `[[region_split]] semantic = "material"`.
fn declared_material() -> HashSet<String> {
    let mut s = HashSet::new();
    s.insert("material".to_string());
    s
}

/// M_B: paint-transparent — no region_split declarations.
fn declared_empty() -> HashSet<String> {
    HashSet::new()
}

// ---------------------------------------------------------------------------
// AC-9 scenarios
// ---------------------------------------------------------------------------

/// Layer_1 contains region_X (variant_chain = [("material", ToolIndex(2))])
/// AND region_Y (empty variant_chain).
///
/// M_A (declares "material") → ALLOWED: region_X matches.
/// M_B (declares nothing)    → ALLOWED: paint-transparent.
#[test]
fn region_split_dispatch_filter_layer1_both_modules_allowed() {
    let layer1 = slice_ir(
        0,
        vec![
            region_with_material_chain(2), // region_X
            region_no_chain(),             // region_Y
        ],
    );

    // M_A: "material" declared, layer has a matching region → allowed.
    assert!(
        module_invocation_allowed_on_layer(&declared_material(), Some(&layer1)),
        "M_A must be allowed on Layer_1 (region_X has variant_chain = material)"
    );

    // M_B: empty declared → always allowed.
    assert!(
        module_invocation_allowed_on_layer(&declared_empty(), Some(&layer1)),
        "M_B must be allowed on Layer_1 (paint-transparent)"
    );
}

/// Layer_2 contains only region_Y (empty variant_chain).
///
/// M_A (declares "material") → FILTERED: no region matches "material".
/// M_B (declares nothing)    → ALLOWED: paint-transparent.
#[test]
fn region_split_dispatch_filter_layer2_ma_filtered_mb_allowed() {
    let layer2 = slice_ir(
        1,
        vec![
            region_no_chain(), // region_Y only
        ],
    );

    // M_A: "material" declared but no region has variant_chain containing "material" → filtered.
    assert!(
        !module_invocation_allowed_on_layer(&declared_material(), Some(&layer2)),
        "M_A must be FILTERED on Layer_2 (no region has variant_chain = material)"
    );

    // M_B: empty declared → always allowed.
    assert!(
        module_invocation_allowed_on_layer(&declared_empty(), Some(&layer2)),
        "M_B must be allowed on Layer_2 (paint-transparent)"
    );
}

/// Composite assertion: the exact invocation set across two layers and two
/// modules is `{(M_A, Layer_1), (M_B, Layer_1), (M_B, Layer_2)}`.
///
/// M_A is NOT invoked on Layer_2 (AC-9 key assertion).
#[test]
fn region_split_dispatch_filter() {
    let layer1 = slice_ir(0, vec![region_with_material_chain(2), region_no_chain()]);
    let layer2 = slice_ir(1, vec![region_no_chain()]);

    // Simulate the dispatch decision for each (module × layer) combination.
    let ma_layer1 = module_invocation_allowed_on_layer(&declared_material(), Some(&layer1));
    let ma_layer2 = module_invocation_allowed_on_layer(&declared_material(), Some(&layer2));
    let mb_layer1 = module_invocation_allowed_on_layer(&declared_empty(), Some(&layer1));
    let mb_layer2 = module_invocation_allowed_on_layer(&declared_empty(), Some(&layer2));

    // Build the invocation set.
    let mut invoked = Vec::new();
    if ma_layer1 {
        invoked.push(("M_A", "Layer_1"));
    }
    if ma_layer2 {
        invoked.push(("M_A", "Layer_2"));
    }
    if mb_layer1 {
        invoked.push(("M_B", "Layer_1"));
    }
    if mb_layer2 {
        invoked.push(("M_B", "Layer_2"));
    }

    // Exact membership assertions.
    assert!(
        invoked.contains(&("M_A", "Layer_1")),
        "M_A must be invoked on Layer_1"
    );
    assert!(
        !invoked.contains(&("M_A", "Layer_2")),
        "M_A must NOT be invoked on Layer_2"
    );
    assert!(
        invoked.contains(&("M_B", "Layer_1")),
        "M_B must be invoked on Layer_1"
    );
    assert!(
        invoked.contains(&("M_B", "Layer_2")),
        "M_B must be invoked on Layer_2"
    );

    // Exactly 3 invocations total.
    assert_eq!(
        invoked.len(),
        3,
        "expected exactly 3 invocations {{(M_A, Layer_1), (M_B, Layer_1), (M_B, Layer_2)}}, got {invoked:?}"
    );
}

/// Edge case: `slice = None` → always allow (conservative).
#[test]
fn region_split_dispatch_filter_no_slice_conservatively_allows() {
    assert!(
        module_invocation_allowed_on_layer(&declared_material(), None),
        "No SliceIR available: must conservatively allow invocation"
    );
}
