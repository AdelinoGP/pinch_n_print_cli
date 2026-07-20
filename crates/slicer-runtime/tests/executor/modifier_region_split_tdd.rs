//! RED tests for packet 132_modifier-region-split.
//!
//! These tests pin the five acceptance criteria (AC-1, AC-2, AC-3, AC-5, AC-N2)
//! as POST-CONDITIONS of the modifier region split. They are written to COMPILE
//! against the current code and FAIL (RED) because the split / sub-region minting
//! does not exist yet.
//!
//! # Split site (implementation target — Step 3 worker)
//!
//! The split is performed at `Layer::Perimeters` commit by
//! `slicer_runtime::region_partition::sync_perimeter_infill_areas_into_slice`
//! (`crates/slicer-runtime/src/region_partition.rs`), which already partitions
//! the four canonical fill polygons (bridge > bottom > top > sparse) and is the
//! place where sub-regions are minted from modifier cross-sections.
//!
//! # Test contract for the implementation worker
//!
//! Each region AC hand-rolls a minimal `SliceIR` staged on a `LayerArena`:
//!   * ONE base `SlicedRegion` (region_id = 0) carrying the object cross-section
//!     in `polygons` + a matching `PerimeterIR` region whose `infill_areas` is the
//!     wall-inset square (so `sync_perimeter_infill_areas_into_slice` can
//!     partition it).
//!   * ONE modifier-footprint `SlicedRegion` carrying the modifier cross-section
//!     in `polygons`/`infill_areas`, flagged with the reserved
//!     `MODIFIER_FOOTPRINT_REGION_ID` (u64::MAX).
//!
//! The implementation of `sync_perimeter_infill_areas_into_slice` MUST: detect
//! the footprint region, intersect its geometry with the base region's four
//! partitioned fill polygons, mint a sub-region whose `region_id` lives in the
//! modifier namespace (`base_region_id * 1_000_003 + modifier_hash`), remove the
//! footprint region, and leave the sub-region WITHOUT its own `PerimeterIR`
//! entry (it borrows the base walls — `wall_source_region_id == Some(base)`).
//!
//! AC-2 exercises `slicer_wasm_host::dispatch::wall_source_region_id` directly:
//! for a modifier sub-region (id in the modifier namespace, empty variant_chain)
//! the predicate must return `Some(base)`; today it returns `None` because the
//! modifier arm is not implemented.

#![allow(missing_docs)]
#![allow(dead_code)]

use slicer_ir::{
    ExPolygon, PerimeterIR, PerimeterRegion, Point2, Polygon, SliceIR, SlicedRegion,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_runtime::blackboard::LayerArena;
use slicer_runtime::region_partition::sync_perimeter_infill_areas_into_slice;
use slicer_wasm_host::dispatch::wall_source_region_id;

/// Reserved `region_id` used to flag a `SlicedRegion` as a modifier footprint to
/// be consumed by the split. The implementation removes this sentinel and mints
/// a proper sub-region in the modifier `region_id` namespace.
const MODIFIER_FOOTPRINT_REGION_ID: u64 = u64::MAX;

/// Modifier `region_id` namespace stride (next prime above paint's 1_000_000),
/// per design.md §FWD-RESOLVED 2. Used by AC-2 to build a representative
/// minted modifier sub-region id for base region 0.
const MODIFIER_VARIANT_REGION_ID_STRIDE: u64 = 1_000_003;

fn square(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: vec![],
    }
}

/// Shoelace area of a set of expolygons, in internal units² (1 unit = 100 nm).
/// Holes are subtracted. Used by AC-1's 1% area-conservation check.
fn poly_area(exps: &[ExPolygon]) -> f64 {
    let mut total = 0.0_f64;
    for ep in exps {
        let pts = &ep.contour.points;
        if pts.len() >= 3 {
            let mut acc = 0i128;
            for i in 0..pts.len() {
                let j = (i + 1) % pts.len();
                acc += (pts[i].x as i128) * (pts[j].y as i128)
                    - (pts[j].x as i128) * (pts[i].y as i128);
            }
            let mut a = (acc as f64).abs() * 0.5;
            for hole in &ep.holes {
                let h = &hole.points;
                if h.len() >= 3 {
                    let mut hacc = 0i128;
                    for i in 0..h.len() {
                        let j = (i + 1) % h.len();
                        hacc += (h[i].x as i128) * (h[j].y as i128)
                            - (h[j].x as i128) * (h[i].y as i128);
                    }
                    a -= (hacc as f64).abs() * 0.5;
                }
            }
            total += a;
        }
    }
    total
}

fn base_region(object_id: &str, footprint: ExPolygon) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id: 0,
        polygons: vec![footprint.clone()],
        infill_areas: vec![footprint],
        effective_layer_height: 0.5,
        ..Default::default()
    }
}

fn modifier_footprint_region(object_id: &str, footprint: ExPolygon) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id: MODIFIER_FOOTPRINT_REGION_ID,
        polygons: vec![footprint.clone()],
        infill_areas: vec![footprint],
        effective_layer_height: 0.5,
        ..Default::default()
    }
}

fn base_perimeter(object_id: &str, wall_inset: ExPolygon) -> PerimeterIR {
    PerimeterIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            walls: vec![],
            infill_areas: vec![wall_inset],
            ..Default::default()
        }],
    }
}

/// Stage a base region + a modifier-footprint region on a fresh `LayerArena`
/// and run the partition hook. Returns the post-hook `SliceIR` (taken back out)
/// and the `LayerArena` (so callers can inspect `PerimeterIR` too).
fn run_split(
    object_id: &str,
    base_footprint: ExPolygon,
    modifier_footprint: Option<ExPolygon>,
) -> (SliceIR, LayerArena) {
    let mut arena = LayerArena::new();
    let mut regions = vec![base_region(object_id, base_footprint.clone())];
    if let Some(mf) = modifier_footprint {
        regions.push(modifier_footprint_region(object_id, mf));
    }
    let slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 1.0,
        regions,
    };
    arena.set_slice(slice).expect("stage slice must succeed");
    arena
        .set_perimeter(base_perimeter(object_id, base_footprint))
        .expect("stage perimeter must succeed");

    sync_perimeter_infill_areas_into_slice(&mut arena, 0)
        .expect("sync_perimeter_infill_areas_into_slice must succeed");

    let slice = arena.slice().expect("slice must be restaged").clone();
    (slice, arena)
}

/// Find the minted modifier sub-region (id != base 0 and != the sentinel).
fn find_sub_region(slice: &SliceIR) -> Option<&SlicedRegion> {
    slice
        .regions
        .iter()
        .find(|r| r.region_id != 0 && r.region_id != MODIFIER_FOOTPRINT_REGION_ID)
}

// ---------------------------------------------------------------------------
// AC-1 — partition conservation
// ---------------------------------------------------------------------------

#[test]
fn modifier_split_partition_conservation() {
    // Base 10×10 mm square; modifier is a centered 4×4 mm square.
    let base = square(0.0, 0.0, 10.0, 10.0);
    let modifier = square(3.0, 3.0, 7.0, 7.0);

    let (slice, _arena) = run_split("obj1", base, Some(modifier));

    // A proper sub-region must have been minted (currently absent → RED).
    let sub = find_sub_region(&slice)
        .expect("AC-1: modifier split must mint a sub-region with a modifier-namespace id");

    let base_region = slice
        .regions
        .iter()
        .find(|r| r.region_id == 0)
        .expect("AC-1: base region must remain");

    let original = poly_area(&[square(0.0, 0.0, 10.0, 10.0)]);
    let union = poly_area(&base_region.sparse_infill_area) + poly_area(&sub.sparse_infill_area);

    let rel_err = (original - union).abs() / original;
    assert!(
        rel_err < 0.01,
        "AC-1: base.sparse_infill_area ∪ sub.sparse_infill_area must equal the pre-split \
         area within 1% (rel_err={rel_err:.4})"
    );

    // The sub-region's sparse fill must equal the modifier footprint
    // (∩ wall-inset), and the base's must exclude it.
    let sub_area = poly_area(&sub.sparse_infill_area);
    let base_area = poly_area(&base_region.sparse_infill_area);
    assert!(
        sub_area > 0.0 && base_area > 0.0,
        "AC-1: both base and sub-region must carry non-empty sparse_infill_area"
    );
    assert!(
        (sub_area + base_area - original).abs() / original < 0.01,
        "AC-1: sparse-area conservation (base + sub == original)"
    );
}

// ---------------------------------------------------------------------------
// AC-2 — wall-source predicate for the sub-region
// ---------------------------------------------------------------------------

#[test]
fn modifier_split_wall_source() {
    // Representative minted modifier sub-region id for base region 0, index 0.
    // Derivation: base_region_id * MODIFIER_VARIANT_REGION_ID_STRIDE + modifier_hash.
    // With base=0 this is just modifier_hash; we pick 7 (any value < stride works,
    // since the predicate inverts it back to base=0 via integer division).
    let sub_id: u64 = 7;

    let sub = SlicedRegion {
        object_id: "obj1".to_string(),
        region_id: sub_id,
        // Modifier sub-regions reuse the base variant_chain (empty here).
        variant_chain: vec![],
        ..Default::default()
    };

    // The sub-region shares the base walls → wall_source_region_id == Some(base).
    let ws = wall_source_region_id(false, &sub);
    assert_eq!(
        ws,
        Some(0),
        "AC-2: modifier sub-region (id in modifier namespace) must report \
         wall_source_region_id == Some(base); got {ws:?}"
    );

    // The base region itself must report None.
    let base = SlicedRegion {
        object_id: "obj1".to_string(),
        region_id: 0,
        variant_chain: vec![],
        ..Default::default()
    };
    assert_eq!(
        wall_source_region_id(false, &base),
        None,
        "AC-2: base region must report wall_source_region_id == None"
    );
}

// ---------------------------------------------------------------------------
// AC-3 — sub-region carries no own wall loops
// ---------------------------------------------------------------------------

#[test]
fn modifier_split_no_subregion_walls() {
    let base = square(0.0, 0.0, 10.0, 10.0);
    let modifier = square(3.0, 3.0, 7.0, 7.0);

    let (slice, arena) = run_split("obj1", base, Some(modifier));

    // A sub-region must exist (currently absent → RED).
    let sub = find_sub_region(&slice).expect("AC-3: modifier split must mint a sub-region");

    // The sub-region must NOT have its own PerimeterIR entry — it borrows the
    // base walls. Only the base (region_id 0) may appear in PerimeterIR.
    let perimeter = arena.perimeter().expect("perimeter must be staged");
    assert!(
        perimeter.regions.iter().all(|p| p.region_id == 0),
        "AC-3: PerimeterIR must contain wall loops ONLY for the base region; \
         found a non-base (sub-region) PerimeterIR entry"
    );
    assert_eq!(
        perimeter.regions.len(),
        1,
        "AC-3: exactly one PerimeterIR region (the base) must be present"
    );

    // The sub-region must be keyed distinctly from the base.
    assert_ne!(sub.region_id, 0, "AC-3: sub-region must have its own id");
}

// ---------------------------------------------------------------------------
// AC-5 — z-scoping: no sub-region above the modifier's top
// ---------------------------------------------------------------------------

#[test]
fn modifier_split_z_scoping() {
    let base = square(0.0, 0.0, 10.0, 10.0);
    // Lower layer (z=1) overlaps the modifier; upper layer (z=9) is above the
    // modifier's top, so its footprint polygon is empty.
    let modifier_lower = square(3.0, 3.0, 7.0, 7.0);
    let modifier_upper: ExPolygon = square(100.0, 100.0, 100.0, 100.0); // degenerate (empty area)

    let (lower, _a1) = run_split("obj1", base.clone(), Some(modifier_lower));
    let (upper, _a2) = run_split("obj1", base, Some(modifier_upper));

    // Lower layer (within modifier Z) MUST contain a sub-region (absent now → RED).
    let _lower_sub =
        find_sub_region(&lower).expect("AC-5: layer within modifier Z must mint a sub-region");

    // Upper layer (above modifier top) MUST contain ONLY the base region.
    let has_sub_up = find_sub_region(&upper).is_some();
    assert!(
        !has_sub_up,
        "AC-5: layer above the modifier's top must contain no sub-region"
    );
    assert_eq!(
        upper.regions.len(),
        1,
        "AC-5: layer above modifier top must contain only the base region"
    );
}

// ---------------------------------------------------------------------------
// AC-N2 — degenerate (out-of-bounds) modifier ⇒ no split, no panic
// ---------------------------------------------------------------------------

#[test]
fn modifier_split_degenerate_no_split() {
    let base = square(0.0, 0.0, 10.0, 10.0);

    // Non-degenerate control: modifier overlaps the base → must split (absent
    // now → drives this test RED until the impl ships).
    let modifier_control = square(3.0, 3.0, 7.0, 7.0);
    let (control, _c) = run_split("obj1", base.clone(), Some(modifier_control));
    let _control_sub = find_sub_region(&control)
        .expect("AC-N2: non-degenerate modifier must mint a sub-region (control)");

    // Degenerate: modifier entirely outside the base XY box → empty intersection
    // → NO sub-region, base region set unchanged, no panic.
    let modifier_outside = square(100.0, 100.0, 110.0, 110.0);
    let (degenerate, _d) = run_split("obj1", base, Some(modifier_outside));

    let has_sub = find_sub_region(&degenerate).is_some();
    assert!(
        !has_sub,
        "AC-N2: degenerate (out-of-bounds) modifier must NOT create a sub-region"
    );
    assert_eq!(
        degenerate.regions.len(),
        1,
        "AC-N2: degenerate modifier must leave the region set identical to the \
         no-modifier case (single base region)"
    );
}

// ---------------------------------------------------------------------------
// Follow-up #3 — sub-region inherits the base's shell-classification fields
// ---------------------------------------------------------------------------

#[test]
fn modifier_split_inherits_shell_classification() {
    let base_footprint = square(0.0, 0.0, 10.0, 10.0);
    let modifier = square(3.0, 3.0, 7.0, 7.0);

    // Hand-roll the base region with explicit shell-classification fields so we
    // can assert they propagate onto the minted sub-region.
    let base = SlicedRegion {
        object_id: "obj1".to_string(),
        region_id: 0,
        polygons: vec![base_footprint.clone()],
        infill_areas: vec![base_footprint.clone()],
        effective_layer_height: 0.5,
        top_shell_index: Some(0),
        bottom_shell_index: Some(0),
        is_bridge: false,
        bridge_orientation_deg: 37.0,
        ..Default::default()
    };
    let modifier_region = SlicedRegion {
        object_id: "obj1".to_string(),
        region_id: MODIFIER_FOOTPRINT_REGION_ID,
        polygons: vec![modifier.clone()],
        infill_areas: vec![modifier],
        effective_layer_height: 0.5,
        ..Default::default()
    };

    let mut arena = LayerArena::new();
    let slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 1.0,
        regions: vec![base, modifier_region],
    };
    arena.set_slice(slice).expect("stage slice must succeed");
    arena
        .set_perimeter(base_perimeter("obj1", base_footprint))
        .expect("stage perimeter must succeed");

    sync_perimeter_infill_areas_into_slice(&mut arena, 0)
        .expect("sync_perimeter_infill_areas_into_slice must succeed");

    let slice = arena.slice().expect("slice must be restaged").clone();

    let sub = find_sub_region(&slice).expect("Follow-up #3: modifier split must mint a sub-region");

    assert_eq!(
        sub.top_shell_index,
        Some(0),
        "Follow-up #3: sub-region must inherit top_shell_index from base"
    );
    assert_eq!(
        sub.bottom_shell_index,
        Some(0),
        "Follow-up #3: sub-region must inherit bottom_shell_index from base"
    );
    assert_eq!(
        sub.is_bridge, false,
        "Follow-up #3: sub-region must inherit is_bridge from base"
    );
    assert_eq!(
        sub.bridge_orientation_deg, 37.0,
        "Follow-up #3: sub-region must inherit bridge_orientation_deg from base"
    );
}
