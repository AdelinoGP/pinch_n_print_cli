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
    ConfigDelta, ExPolygon, IndexedTriangleSet, MeshIR, ModifierScope, ModifierVolume, ObjectMesh,
    PaintValue, PerimeterIR, PerimeterRegion, Point2, Point3, Polygon, SliceIR, SlicedRegion,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_runtime::blackboard::LayerArena;
use slicer_runtime::region_partition::sync_perimeter_infill_areas_into_slice;
use slicer_wasm_host::dispatch::wall_source_region_id;

/// Reserved `region_id` used to flag a `SlicedRegion` as a modifier footprint to
/// be consumed by the split. The implementation removes this sentinel and mints
/// a proper sub-region in the modifier `region_id` namespace.
const MODIFIER_FOOTPRINT_REGION_ID: u64 = u64::MAX;

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

fn canonicalize_ring(ring: &mut Polygon) {
    let Some(start) = ring
        .points
        .iter()
        .enumerate()
        .min_by_key(|(_, point)| **point)
        .map(|(index, _)| index)
    else {
        return;
    };
    let points = ring.points.clone();
    ring.points = points[start..]
        .iter()
        .chain(points[..start].iter())
        .copied()
        .collect();
}

fn canonicalize_region_rings(regions: &mut [SlicedRegion]) {
    for region in regions {
        for polygons in [
            &mut region.polygons,
            &mut region.infill_areas,
            &mut region.bridge_areas,
            &mut region.bottom_solid_fill,
            &mut region.top_solid_fill,
            &mut region.sparse_infill_area,
            &mut region.internal_solid_fill,
            &mut region.internal_bridge_areas,
        ] {
            for expolygon in polygons {
                canonicalize_ring(&mut expolygon.contour);
                for hole in &mut expolygon.holes {
                    canonicalize_ring(hole);
                }
            }
        }
    }
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

fn modifier_box_mesh(x0: f32, y0: f32, x1: f32, y1: f32) -> IndexedTriangleSet {
    let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    IndexedTriangleSet {
        vertices: vec![
            v(x0, y0, 0.0),
            v(x1, y0, 0.0),
            v(x1, y1, 0.0),
            v(x0, y1, 0.0),
            v(x0, y0, 1.0),
            v(x1, y0, 1.0),
            v(x1, y1, 1.0),
            v(x0, y1, 1.0),
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ],
    }
}

fn parameter_modifier(id: &str, priority: u32, mesh: IndexedTriangleSet) -> ModifierVolume {
    // exhaustive: ModifierVolume has no Default impl; this fixture pins every field.
    ModifierVolume {
        id: id.to_string(),
        mesh,
        config_delta: ConfigDelta::default(),
        priority,
        applies_to: ModifierScope::AllFeatures,
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
        .find(|r| slicer_ir::is_modifier_namespace_id(r.region_id))
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
    let sub_id = slicer_ir::modifier_sub_region_id(0, "obj1", &[square(3.0, 3.0, 7.0, 7.0)]);

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
// Ticket 19 follow-up — prepass materialization must preserve classified roles
// ---------------------------------------------------------------------------

#[test]
fn prepass_materialized_subregion_roles_survive_perimeter_sync() {
    let base = square(0.0, 0.0, 10.0, 10.0);
    let modifier = square(3.0, 3.0, 7.0, 7.0);
    let sub_id = slicer_ir::modifier_sub_region_id(0, "obj1", &[modifier.clone()]);

    // The overlapping base top role models a stale/recomputed source. The
    // already-materialized sub-region's bridge role must never be inferred from
    // the base fields at the Tier-2 seam.
    let mut base_region = base_region("obj1", base.clone());
    base_region.top_solid_fill = vec![modifier.clone()];
    let sub_region = SlicedRegion {
        object_id: "obj1".to_string(),
        region_id: sub_id,
        polygons: vec![modifier.clone()],
        infill_areas: vec![modifier.clone()],
        bridge_areas: vec![modifier.clone()],
        effective_layer_height: 0.5,
        ..Default::default()
    };
    let expected_bridge = sub_region.bridge_areas.clone();

    let slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 1.0,
        regions: vec![base_region, sub_region],
    };
    let mut arena = LayerArena::new();
    arena.set_slice(slice).expect("stage slice must succeed");
    arena
        .set_perimeter(base_perimeter("obj1", base.clone()))
        .expect("stage perimeter must succeed");

    sync_perimeter_infill_areas_into_slice(&mut arena, 0)
        .expect("sync_perimeter_infill_areas_into_slice must succeed");

    let sub = find_sub_region(arena.slice().expect("slice must be restaged"))
        .expect("prepass-materialized sub-region must remain present");
    assert_eq!(
        sub.bridge_areas, expected_bridge,
        "Tier-2 must preserve prepass-classified bridge geometry"
    );
}

// ---------------------------------------------------------------------------
// Ticket 19 follow-up — priority-first geometry ownership
// ---------------------------------------------------------------------------

#[test]
fn prepass_modifier_overlap_assigns_geometry_to_highest_priority_first() {
    let object_id = "obj1";
    let base = square(0.0, 0.0, 10.0, 10.0);
    let low_mesh = modifier_box_mesh(1.0, 1.0, 8.0, 8.0);
    let high_mesh = modifier_box_mesh(5.0, 1.0, 9.0, 8.0);
    let low_polygons = slicer_core::slice_mesh_ex(&low_mesh, &[0.5])
        .into_iter()
        .next()
        .expect("low modifier must slice");
    let high_polygons = slicer_core::slice_mesh_ex(&high_mesh, &[0.5])
        .into_iter()
        .next()
        .expect("high modifier must slice");
    let low_id = slicer_ir::modifier_sub_region_id(0, object_id, &low_polygons);
    let high_id = slicer_ir::modifier_sub_region_id(0, object_id, &high_polygons);

    let mut slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.5,
        regions: vec![base_region(object_id, base)],
    };
    let mesh = MeshIR {
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            modifier_volumes: vec![
                parameter_modifier("low", 1, low_mesh),
                parameter_modifier("high", 9, high_mesh),
            ],
            ..Default::default()
        }],
        ..Default::default()
    };

    slicer_runtime::region_partition::split_modifier_sub_regions_for_prepass(&mut slice, &mesh)
        .expect("prepass modifier split must succeed");

    let low = slice
        .regions
        .iter()
        .find(|region| region.region_id == low_id)
        .expect("low-priority sub-region must remain");
    let high = slice
        .regions
        .iter()
        .find(|region| region.region_id == high_id)
        .expect("high-priority sub-region must remain");
    let low_area = poly_area(&low.polygons);
    let high_area = poly_area(&high.polygons);
    assert!(
        (low_area - 28.0e8).abs() < 0.01e8,
        "low-priority modifier must receive only the non-overlapping remainder; area={low_area}"
    );
    assert!(
        (high_area - 28.0e8).abs() < 0.01e8,
        "high-priority modifier must own the full overlap and its footprint; area={high_area}"
    );
}

#[test]
fn prepass_and_tier2_modifier_splits_produce_identical_regions() {
    let object_id = "obj1";
    let base = square(0.0, 0.0, 10.0, 10.0);
    let low_mesh = modifier_box_mesh(1.0, 1.0, 8.0, 8.0);
    let high_mesh = modifier_box_mesh(5.0, 1.0, 9.0, 8.0);
    let mesh = MeshIR {
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            modifier_volumes: vec![
                parameter_modifier("low", 1, low_mesh.clone()),
                parameter_modifier("high", 9, high_mesh.clone()),
            ],
            ..Default::default()
        }],
        ..Default::default()
    };

    let mut prepass_slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.5,
        regions: vec![base_region(object_id, base.clone())],
    };
    slicer_runtime::region_partition::split_modifier_sub_regions_for_prepass(
        &mut prepass_slice,
        &mesh,
    )
    .expect("prepass modifier split must succeed");
    let mut prepass_arena = LayerArena::new();
    prepass_arena
        .set_slice(prepass_slice)
        .expect("stage prepass slice must succeed");
    prepass_arena
        .set_perimeter(base_perimeter(object_id, base.clone()))
        .expect("stage prepass perimeter must succeed");
    sync_perimeter_infill_areas_into_slice(&mut prepass_arena, 0)
        .expect("prepass partition must succeed");

    let high_footprint = slicer_core::slice_mesh_ex(&high_mesh, &[0.5])
        .into_iter()
        .next()
        .expect("high modifier must slice");
    let low_footprint = slicer_core::slice_mesh_ex(&low_mesh, &[0.5])
        .into_iter()
        .next()
        .expect("low modifier must slice");
    let mut tier2_arena = LayerArena::new();
    tier2_arena
        .set_slice(SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.5,
            regions: vec![
                base_region(object_id, base.clone()),
                modifier_footprint_region(
                    object_id,
                    high_footprint
                        .into_iter()
                        .next()
                        .expect("high footprint polygon"),
                ),
                modifier_footprint_region(
                    object_id,
                    low_footprint
                        .into_iter()
                        .next()
                        .expect("low footprint polygon"),
                ),
            ],
        })
        .expect("stage Tier-2 slice must succeed");
    tier2_arena
        .set_perimeter(base_perimeter(object_id, base))
        .expect("stage Tier-2 perimeter must succeed");
    sync_perimeter_infill_areas_into_slice(&mut tier2_arena, 0)
        .expect("Tier-2 partition must succeed");

    let mut prepass_regions = prepass_arena
        .slice()
        .expect("prepass slice must remain")
        .regions
        .clone();
    let mut tier2_regions = tier2_arena
        .slice()
        .expect("Tier-2 slice must remain")
        .regions
        .clone();
    // Polygon booleans preserve geometry but may choose a different cyclic
    // start point for an otherwise identical ring depending on call order.
    canonicalize_region_rings(&mut prepass_regions);
    canonicalize_region_rings(&mut tier2_regions);
    assert_eq!(
        prepass_regions,
        tier2_regions,
        "prepass and Tier-2 must agree on modifier ids, geometry, priority ownership, and fill roles"
    );
}

#[test]
fn modifier_split_rejects_overflow_without_mutating_slice() {
    let stride = slicer_ir::MODIFIER_VARIANT_REGION_ID_STRIDE;
    let max_parent = ((1_u64 << 63) - 2 - (stride - 1)) / stride;
    let invalid_parent = max_parent + 1;
    assert!(!slicer_ir::modifier_sub_region_id_fits(invalid_parent));

    let modifier_mesh = modifier_box_mesh(2.0, 2.0, 8.0, 8.0);
    let mesh = MeshIR {
        objects: vec![ObjectMesh {
            id: "obj1".to_string(),
            modifier_volumes: vec![parameter_modifier("modifier", 0, modifier_mesh)],
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.5,
        regions: vec![base_region("obj1", square(0.0, 0.0, 10.0, 10.0))],
    };
    slice.regions[0].region_id = invalid_parent;
    let before = slice.clone();

    let error =
        slicer_runtime::region_partition::split_modifier_sub_regions_for_prepass(&mut slice, &mesh)
            .expect_err("an intersecting modifier must reject an unencodable parent id");
    assert!(error.contains("parent_region_id"));
    assert_eq!(slice, before, "failed splitting must be atomic");
}

#[test]
fn modifier_split_rejects_existing_child_identity_collision() {
    let base = square(0.0, 0.0, 10.0, 10.0);
    let modifier_mesh = modifier_box_mesh(2.0, 2.0, 8.0, 8.0);
    let footprint = slicer_core::slice_mesh_ex(&modifier_mesh, &[0.5])
        .into_iter()
        .next()
        .expect("modifier must slice");
    let child_id = slicer_ir::modifier_sub_region_id(0, "obj1", &footprint);
    let child = SlicedRegion {
        object_id: "obj1".to_string(),
        region_id: child_id,
        polygons: footprint.clone(),
        infill_areas: footprint,
        effective_layer_height: 0.5,
        ..Default::default()
    };
    let mut slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.5,
        regions: vec![base_region("obj1", base), child],
    };
    let before = slice.clone();
    let mesh = MeshIR {
        objects: vec![ObjectMesh {
            id: "obj1".to_string(),
            modifier_volumes: vec![parameter_modifier("modifier", 0, modifier_mesh)],
            ..Default::default()
        }],
        ..Default::default()
    };

    let error =
        slicer_runtime::region_partition::split_modifier_sub_regions_for_prepass(&mut slice, &mesh)
            .expect_err("re-materializing an existing child must reject duplicate identity");
    assert!(error.contains("identity collision"));
    assert_eq!(slice, before, "failed splitting must be atomic");
}

// ---------------------------------------------------------------------------
// DEV-130 — the footprint binds to BASE, never to a painted variant that
// happens to be emitted first
// ---------------------------------------------------------------------------

/// `split_modifier_footprints` locates the parent region with a `position(...)`
/// scan over the already-emitted prefix. Before DEV-130 that scan matched on
/// `object_id` and "not a footprint" only, so on an object carrying BOTH paint
/// variants and a modifier volume it bound to whichever region was emitted
/// first — which can be a painted variant. The minted sub-region id encodes its
/// parent (recoverable through `modifier_base_region_id`), so the parent choice
/// is directly observable.
#[test]
fn modifier_split_binds_to_base_not_painted_variant() {
    let footprint = square(0.0, 0.0, 10.0, 10.0);
    let modifier = square(3.0, 3.0, 7.0, 7.0);

    // A painted variant region, emitted BEFORE the base region. Non-empty
    // `variant_chain` is what distinguishes it from BASE.
    let mut variant = base_region("obj1", footprint.clone());
    variant.region_id = 7;
    variant.variant_chain = vec![("material".to_string(), PaintValue::ToolIndex(1))];
    let variant_area_before = poly_area(&variant.polygons);

    let regions = vec![
        variant,
        base_region("obj1", footprint.clone()),
        modifier_footprint_region("obj1", modifier),
    ];

    let mut arena = LayerArena::new();
    arena
        .set_slice(SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 1.0,
            regions,
        })
        .expect("stage slice must succeed");
    arena
        .set_perimeter(base_perimeter("obj1", footprint))
        .expect("stage perimeter must succeed");

    sync_perimeter_infill_areas_into_slice(&mut arena, 0)
        .expect("sync_perimeter_infill_areas_into_slice must succeed");
    let slice = arena.slice().expect("slice must be restaged").clone();

    let sub = slice
        .regions
        .iter()
        .find(|r| slicer_ir::modifier_base_region_id(r.region_id) == Some(0))
        .expect("DEV-130: a BASE modifier sub-region must still be minted");

    assert_eq!(
        slicer_ir::modifier_base_region_id(sub.region_id),
        Some(0),
        "DEV-130: minted sub-region must encode BASE (region_id 0) as its parent, \
         not the painted variant (region_id 7) that precedes it in emission order"
    );

    // The base child must be rooted in BASE, not in the painted variant.
    let variant_after = slice
        .regions
        .iter()
        .find(|r| r.region_id == 7)
        .expect("the painted variant region must survive the split");
    let variant_area_after = poly_area(&variant_after.polygons);
    assert!(
        variant_area_after < variant_area_before,
        "painted modifier coverage must be removed from the painted parent; before={variant_area_before}, after={variant_area_after}"
    );

    let painted_sub = slice
        .regions
        .iter()
        .find(|r| {
            r.variant_chain == vec![("material".to_string(), PaintValue::ToolIndex(1))]
                && slicer_ir::is_modifier_namespace_id(r.region_id)
        })
        .expect("a modifier child must be materialized for the painted parent");
    assert_eq!(
        slicer_ir::modifier_base_region_id(painted_sub.region_id),
        Some(7),
        "painted modifier child must identify its painted parent as wall source"
    );
    assert_eq!(painted_sub.variant_chain, variant_after.variant_chain);
    assert!(
        poly_area(&painted_sub.polygons) > 0.0,
        "painted modifier child must carry the overlapping geometry"
    );
}

#[test]
fn modifier_split_composes_base_and_painted_geometry() {
    let base = square(0.0, 0.0, 10.0, 10.0);
    let modifier = modifier_box_mesh(3.0, 3.0, 7.0, 7.0);
    let mut variant = base_region("obj1", base.clone());
    variant.region_id = 7;
    variant.variant_chain = vec![("material".to_string(), PaintValue::ToolIndex(1))];
    let variant_before = poly_area(&variant.polygons);
    let mut slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.5,
        regions: vec![base_region("obj1", base), variant],
    };
    let mesh = MeshIR {
        objects: vec![ObjectMesh {
            id: "obj1".to_string(),
            modifier_volumes: vec![parameter_modifier("modifier", 0, modifier)],
            ..Default::default()
        }],
        ..Default::default()
    };

    slicer_runtime::region_partition::split_modifier_sub_regions_for_prepass(&mut slice, &mesh)
        .expect("prepass modifier split must succeed");

    let painted_sub = slice
        .regions
        .iter()
        .find(|r| {
            r.variant_chain == vec![("material".to_string(), PaintValue::ToolIndex(1))]
                && slicer_ir::is_modifier_namespace_id(r.region_id)
        })
        .expect("painted modifier child must be materialized");
    let painted_parent = slice
        .regions
        .iter()
        .find(|r| r.region_id == 7)
        .expect("painted parent must remain");
    assert!(poly_area(&painted_parent.polygons) < variant_before);
    assert!(poly_area(&painted_sub.polygons) > 0.0);
    assert_eq!(wall_source_region_id(false, painted_sub), Some(7));
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

#[test]
fn prepass_materialized_subregion_receives_parent_wall_inset_fill() {
    let base = square(0.0, 0.0, 10.0, 10.0);
    let modifier = square(3.0, 3.0, 7.0, 7.0);
    let sub_id = slicer_ir::modifier_sub_region_id(0, "obj1", &[modifier.clone()]);
    let base_remaining = slicer_core::polygon_ops::difference(
        std::slice::from_ref(&base),
        std::slice::from_ref(&modifier),
    );
    let mut base_region = base_region("obj1", base.clone());
    base_region.polygons = base_remaining.clone();
    base_region.infill_areas = base_remaining;
    let slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 1.0,
        regions: vec![
            base_region,
            SlicedRegion {
                object_id: "obj1".to_string(),
                region_id: sub_id,
                polygons: vec![modifier],
                // Paint segmentation can leave this pre-perimeter field empty;
                // partitioning must still use the donor's wall inset.
                infill_areas: Vec::new(),
                ..Default::default()
            },
        ],
    };
    let mut arena = LayerArena::new();
    arena.set_slice(slice).expect("stage slice must succeed");
    arena
        .set_perimeter(base_perimeter("obj1", base.clone()))
        .expect("stage perimeter must succeed");

    sync_perimeter_infill_areas_into_slice(&mut arena, 0)
        .expect("sync_perimeter_infill_areas_into_slice must succeed");

    let slice = arena.slice().expect("slice must be restaged");
    let base_area = slice
        .regions
        .iter()
        .find(|region| region.region_id == 0)
        .map(|region| poly_area(&region.sparse_infill_area))
        .expect("base region must remain");
    let sub_area = slice
        .regions
        .iter()
        .find(|region| region.region_id == sub_id)
        .map(|region| poly_area(&region.sparse_infill_area))
        .expect("modifier child must remain");
    let expected_area = poly_area(std::slice::from_ref(&base));

    assert!(base_area > 0.0 && sub_area > 0.0);
    assert!(
        (base_area + sub_area - expected_area).abs() / expected_area < 0.01,
        "parent and child sparse fill must conserve the donor wall inset: base={base_area}, sub={sub_area}"
    );
}
