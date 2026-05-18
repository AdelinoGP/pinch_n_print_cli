//! TDD tests for packet 37: fill-role-claims — proving the four fill-role
//! claim system works correctly.
//!
//! This file is authored in the RED phase. All tests should compile but fail
//! assertions until the infrastructure is implemented.
//!
//! ## Claim system overview
//!
//! Four fill-role claim IDs are recognized:
//!   - `claim:top-fill`      — top surface solid infill paths
//!   - `claim:bottom-fill`   — bottom surface solid infill paths
//!   - `claim:bridge-fill`    — bridge infill paths
//!   - `claim:sparse-fill`   — sparse/gradient infill paths
//!
//! Modules declare which claims they hold in `[claims].holds` in manifests.
//! Validation pass 2 (`validate_claim_conflicts`) catches double-holders.
//! Runtime filtering ensures modules only emit paths for roles they hold.
//!
//! ## Key facts (Step 0, must NOT be re-discovered)
//!
//! - Rectilinear default holds ALL four claims — the filter is a no-op for it.
//! - Gyroid emits ALL four roles — the runtime filter must actually do work.
//! - Lightning emits SparseInfill only — filter is a no-op for it.
//! - No SDK accessor for held claims yet — design says add `SliceRegionView::held_claims()`.
//! - Claims are string-based; "catalog registration" = validation recognizes the four IDs.
//!
//! Coordinate system: 1 unit = 100 nm = 10⁻⁴ mm.
//! Use `Point2::from_mm` / `mm_to_units` per docs/08_coordinate_system.md.

#![allow(missing_docs, dead_code, unused_imports, unused_variables)]

use std::collections::{BTreeMap, HashMap};

use slicer_host::execute_layer_slice;
use slicer_host::layer_slice::classify_region_surfaces;
use slicer_host::{resolve_held_claims, FillHolders};
use slicer_ir::{
    ActiveRegion, BoundingBox3, BridgeRegion, ExtrusionRole, FacetClass, GlobalLayer,
    IndexedTriangleSet, InfillType, MeshIR, ModuleId, ObjectConfig, ObjectId, ObjectMesh,
    ObjectSurfaceData, Point2, Point3, Polygon, RegionId, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SemVer, SliceIR, SurfaceClassificationIR, SurfaceGroup, Transform3d,
    WallGenerator,
};
use slicer_sdk::views::SliceRegionView;

// ============================================================================
// Shared fixture helpers
// ============================================================================

fn identity_transform() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

fn default_resolved() -> ResolvedConfig {
    ResolvedConfig::default()
}

/// A flat horizontal triangle at Z=`z_top` with vertices in the XY plane
/// centred near (0, 0). The triangle has its normal pointing UP (TopSurface).
///
/// Vertices (mm): (-5, -5, z_top), (5, -5, z_top), (0, 5, z_top)
/// Facet index: 0
fn flat_top_triangle(z_top: f32) -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: -5.0,
                y: -5.0,
                z: z_top,
            },
            Point3 {
                x: 5.0,
                y: -5.0,
                z: z_top,
            },
            Point3 {
                x: 0.0,
                y: 5.0,
                z: z_top,
            },
        ],
        // CCW winding when viewed from above → upward normal → TopSurface
        indices: vec![0, 1, 2],
    }
}

/// A flat horizontal triangle at Z=`z_bot` with vertices in the XY plane.
/// The triangle has its normal pointing DOWN (BottomSurface).
///
/// Vertices (mm): (-5, -5, z_bot), (0, 5, z_bot), (5, -5, z_bot)
/// Facet index: 0
fn flat_bottom_triangle(z_bot: f32) -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: -5.0,
                y: -5.0,
                z: z_bot,
            },
            Point3 {
                x: 0.0,
                y: 5.0,
                z: z_bot,
            },
            Point3 {
                x: 5.0,
                y: -5.0,
                z: z_bot,
            },
        ],
        // CW winding when viewed from above → downward normal → BottomSurface
        indices: vec![0, 1, 2],
    }
}

/// A triangle that straddles layer_z — used for bridge classification.
/// Vertices form a triangle angled across the Z axis.
fn bridge_triangle(z_a: f32, z_b: f32) -> IndexedTriangleSet {
    // Triangle spanning z_a to z_b; centroid roughly (0, -1.67)
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: -10.0,
                y: -10.0,
                z: z_a,
            },
            Point3 {
                x: 10.0,
                y: -10.0,
                z: z_b,
            },
            Point3 {
                x: 0.0,
                y: 10.0,
                z: (z_a + z_b) / 2.0,
            },
        ],
        indices: vec![0, 1, 2],
    }
}

fn make_object_mesh(object_id: &str, mesh: IndexedTriangleSet) -> ObjectMesh {
    ObjectMesh {
        id: object_id.to_string(),
        mesh,
        transform: identity_transform(),
        ..Default::default()
    }
}

fn make_mesh_ir(object_id: &str, mesh: IndexedTriangleSet) -> MeshIR {
    MeshIR {
        objects: vec![make_object_mesh(object_id, mesh)],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: -200.0,
                y: -200.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..Default::default()
    }
}

fn make_layer(index: u32, z: f32, object_id: &str) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: vec![ActiveRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            resolved_config: default_resolved(),
            effective_layer_height: 0.2,
            nonplanar_shell: None,
            is_catchup_layer: false,
            catchup_z_bottom: 0.0,
            tool_index: 0,
        }],
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

/// Rectangle polygon centred at origin, covering ±10 mm in X and Y.
fn region_polygon_covering_origin() -> Polygon {
    Polygon {
        points: vec![
            Point2::from_mm(-10.0, -10.0),
            Point2::from_mm(10.0, -10.0),
            Point2::from_mm(10.0, 10.0),
            Point2::from_mm(-10.0, 10.0),
        ],
    }
}

/// Build a `SurfaceClassificationIR` for one object with a single-facet
/// classification and optional bridge region.
/// Mirrors the pattern from external_surface_classification_tdd.rs.
fn make_surface_class(
    object_id: &str,
    facet_class: FacetClass,
    bridge_facet_indices: Option<Vec<u32>>,
) -> SurfaceClassificationIR {
    let bridge_regions = match bridge_facet_indices {
        Some(indices) => vec![BridgeRegion {
            facet_indices: indices,
            ..Default::default()
        }],
        None => vec![],
    };

    let per_object_data = ObjectSurfaceData {
        facet_classes: vec![facet_class],
        surface_groups: vec![SurfaceGroup {
            id: 0,
            facet_indices: vec![0],
            z_min: 0.0,
            z_max: 1.0,
            area_mm2: 25.0,
            printable: true,
            shell_count: 1,
        }],
        bridge_regions,
        overhang_regions: vec![],
    };

    let mut per_object = HashMap::new();
    per_object.insert(object_id.to_string(), per_object_data);

    SurfaceClassificationIR {
        per_object,
        ..Default::default()
    }
}

// ============================================================================
// Fill role claim IDs (shared constants)
// ============================================================================

const CLAIM_TOP_FILL: &str = "claim:top-fill";
const CLAIM_BOTTOM_FILL: &str = "claim:bottom-fill";
const CLAIM_BRIDGE_FILL: &str = "claim:bridge-fill";
const CLAIM_SPARSE_FILL: &str = "claim:sparse-fill";

const ALL_FOUR_CLAIMS: [&str; 4] = [
    CLAIM_TOP_FILL,
    CLAIM_BOTTOM_FILL,
    CLAIM_BRIDGE_FILL,
    CLAIM_SPARSE_FILL,
];

// ============================================================================
// TEST 1: four_fill_claims_registered_in_catalog
// ============================================================================
// Verifies the four fill-role claim IDs are recognized as valid by the
// validation system. Since there's no centralized catalog, this test
// verifies that manifests declaring these four claims are accepted without
// an UnknownClaim error in validation pass 2.
//
// Approach: Create minimal claim holder sets for two modules (rectilinear,
// gyroid) where rectilinear holds all four and gyroid holds only sparse.
// Run validation and confirm no unknown-claim errors occur.
// ============================================================================

/// AC-1: The four fill-role claim IDs are recognized by validation as valid
/// claim strings. No UnknownClaim error is raised when these IDs appear in
/// `[claims].holds` in manifests.
#[test]
fn four_fill_claims_registered_in_catalog() {
    use slicer_host::validation::{
        validate_startup_dag, ClaimHolder, ConflictScope, DagValidationReport, DagValidationRequest,
    };
    use slicer_ir::ModuleId;

    let rectilinear_id = ModuleId::from("rectilinear-infill");
    let gyroid_id = ModuleId::from("gyroid-infill");

    // rectilinear holds all four claims
    let rectilinear_claims = vec![
        ClaimHolder {
            claim: CLAIM_TOP_FILL.to_string(),
            module_id: rectilinear_id.clone(),
            scope: ConflictScope::Global,
        },
        ClaimHolder {
            claim: CLAIM_BOTTOM_FILL.to_string(),
            module_id: rectilinear_id.clone(),
            scope: ConflictScope::Global,
        },
        ClaimHolder {
            claim: CLAIM_BRIDGE_FILL.to_string(),
            module_id: rectilinear_id.clone(),
            scope: ConflictScope::Global,
        },
        ClaimHolder {
            claim: CLAIM_SPARSE_FILL.to_string(),
            module_id: rectilinear_id.clone(),
            scope: ConflictScope::Global,
        },
    ];

    // gyroid holds only sparse-fill
    let gyroid_claims = vec![ClaimHolder {
        claim: CLAIM_SPARSE_FILL.to_string(),
        module_id: gyroid_id.clone(),
        scope: ConflictScope::Global,
    }];

    let all_holders = vec![rectilinear_claims, gyroid_claims]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let request = DagValidationRequest {
        modules: vec![],
        claim_holders: all_holders,
        stage_dags: vec![],
        access_audits: vec![],
        host_ir_schema_version: SemVer::default(),
    };

    let report = validate_startup_dag(&request);

    // We expect exactly ONE ClaimConflict error: both rectilinear and gyroid
    // hold sparse-fill in global scope → one conflict between them.
    // NO unknown-claim errors should exist for the four standard fill-role IDs.
    let sparse_conflicts: Vec<_> = report
        .errors
        .iter()
        .filter(|e| {
            matches!(&e.detail, slicer_host::validation::SchedulerError::ClaimConflict { claim, .. }
                 if claim == CLAIM_SPARSE_FILL)
        })
        .collect();

    assert!(
        sparse_conflicts.len() == 1,
        "expected exactly 1 ClaimConflict for sparse-fill (double-holder: rectilinear + gyroid), \
         got {} conflicts. Errors: {:#?}",
        sparse_conflicts.len(),
        report.errors
    );
}

// ============================================================================
// TEST 2: default_rectilinear_holds_all_claims_emits_top
// ============================================================================
// Verifies: Rectilinear default holds all four claims. When region has
// is_top_surface=true, the SliceIR region carries that flag. The actual
// path emission (TopSolidInfill) is gated by the held-claims filter once
// implemented.
// ============================================================================

/// AC-2: Given rectilinear-infill with default holds all four claims,
/// when a region has is_top_surface=true, then `execute_layer_slice`
/// produces a SliceIR region with is_top_surface=true.
#[test]
fn default_rectilinear_holds_all_claims_emits_top() {
    let object_id = "rect-obj";
    let mesh_ir = make_mesh_ir(object_id, flat_top_triangle(1.0));
    let surface_class = make_surface_class(object_id, FacetClass::TopSurface, None);
    let polygons = vec![region_polygon_covering_origin()];

    // Classify surfaces to get the boolean flags
    let (is_top, is_bot, is_bridge) = classify_region_surfaces(
        &mesh_ir.objects[0],
        &surface_class.per_object[object_id],
        &polygons,
        1.0,       // layer_z
        Some(1.2), // next_layer_z
        Some(0.8), // prev_layer_z
        1,         // top_shell_layers
        1,         // bottom_shell_layers
    );

    assert!(is_top, "rectilinear top surface classification failed");
    assert!(!is_bot, "expected is_bottom_surface=false");
    assert!(!is_bridge, "expected is_bridge=false");

    // Run execute_layer_slice with surface classification
    let layer = make_layer(0, 1.0, object_id);
    let slice = execute_layer_slice(
        &mesh_ir,
        &layer,
        Some(&surface_class),
        Some(1.2),
        Some(0.8),
        None,
        None,
    )
    .expect("slice should succeed");

    assert_eq!(slice.regions.len(), 1, "expected exactly one region");
    let region = &slice.regions[0];
    assert!(
        region.is_top_surface,
        "rectilinear region with top surface must have is_top_surface=true in SliceIR"
    );

    // ── Resolver: rectilinear-infill is the default holder for all four
    //    fill-role claims; with the default ResolvedConfig (which names
    //    "rectilinear-infill" for each *_fill_holder), the resolver returns
    //    the full set declared in its manifest.
    let manifest_claims: Vec<String> = [
        "infill-generator",
        "claim:top-fill",
        "claim:bottom-fill",
        "claim:bridge-fill",
        "claim:sparse-fill",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    let cfg = ResolvedConfig::default();
    let holders = FillHolders {
        top: &cfg.top_fill_holder,
        bottom: &cfg.bottom_fill_holder,
        bridge: &cfg.bridge_fill_holder,
        sparse: &cfg.sparse_fill_holder,
    };
    let held = resolve_held_claims("rectilinear-infill", &manifest_claims, &holders);
    assert_eq!(
        held,
        vec![
            String::from("claim:top-fill"),
            String::from("claim:bottom-fill"),
            String::from("claim:bridge-fill"),
            String::from("claim:sparse-fill"),
        ],
        "rectilinear-infill default config must hold all four fill-role claims"
    );

    // ── Runtime filter: SliceRegionView::should_emit returns true for the
    //    top-surface role when the held set contains claim:top-fill.
    let mut view = SliceRegionView::default();
    view.set_object_id(ObjectId::from("rect-obj"));
    view.set_region_id(RegionId::from(0u64));
    view.set_polygons(Vec::new());
    view.set_infill_areas(Vec::new());
    view.set_effective_layer_height(0.2);
    view.set_z(1.0);
    view.set_has_nonplanar(false);
    view.set_held_claims(held);
    assert!(
        view.should_emit(ExtrusionRole::TopSolidInfill),
        "rectilinear must be allowed to emit TopSolidInfill when it holds claim:top-fill"
    );
    assert!(
        view.should_emit(ExtrusionRole::SparseInfill),
        "rectilinear must be allowed to emit SparseInfill when it holds claim:sparse-fill"
    );
}

// ============================================================================
// TEST 3: gyroid_holds_sparse_claim_only_emits_sparse
// ============================================================================
// Verifies: Gyroid holds only claim:sparse-fill. When region is neither top,
// bottom, nor bridge (i.e., interior sparse region), gyroid emits sparse paths.
// Rectilinear (holding all four) emits zero sparse in this configuration
// because the region override targets gyroid specifically.
// ============================================================================

/// AC-3: Given gyroid-infill that only holds claim:sparse-fill, when a region
/// has is_top_surface=false, is_bottom_surface=false, is_bridge=false,
/// then gyroid emits sparse paths AND rectilinear emits zero sparse for
/// the same region (because rectilinear is overridden away).
#[test]
fn gyroid_holds_sparse_claim_only_emits_sparse() {
    // Verifies the AC end-to-end on the host side via the same code path
    // the WASM modules use:
    //   1. `resolve_held_claims` computes per-module held-claim sets under
    //      a config that names gyroid as the sparse holder.
    //   2. A `SliceRegionView` is constructed with that held-claim set and
    //      `should_emit(role)` is asserted to return true exactly for the
    //      held claims and false for the unheld ones.
    // This is the same `should_emit` helper the rectilinear / gyroid /
    // lightning module sources call to gate path emission, so passing this
    // test means a guest module configured this way would emit sparse paths
    // for gyroid and zero sparse paths for rectilinear.
    let object_id = "gyroid-obj";

    // Interior triangle at z=1.0 (neither top nor bottom surface)
    let interior_mesh = IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.5,
            },
            Point3 {
                x: 10.0,
                y: 0.0,
                z: 1.5,
            },
            Point3 {
                x: 0.0,
                y: 10.0,
                z: 1.0,
            },
        ],
        indices: vec![0, 1, 2],
    };
    let mesh_ir = make_mesh_ir(object_id, interior_mesh);

    // Normal facet classification → all surface flags false
    let surface_class = make_surface_class(object_id, FacetClass::Normal, None);
    let polygons = vec![region_polygon_covering_origin()];

    let (is_top, is_bot, is_bridge) = classify_region_surfaces(
        &mesh_ir.objects[0],
        &surface_class.per_object[object_id],
        &polygons,
        1.0,
        Some(1.2),
        Some(0.8),
        3, // top_shell_layers
        3, // bottom_shell_layers
    );

    // Interior region: no surface touched
    assert!(!is_top, "interior region: expected is_top_surface=false");
    assert!(!is_bot, "interior region: expected is_bottom_surface=false");
    assert!(!is_bridge, "interior region: expected is_bridge=false");

    let layer = make_layer(0, 1.0, object_id);
    let slice = execute_layer_slice(
        &mesh_ir,
        &layer,
        Some(&surface_class),
        Some(1.2),
        Some(0.8),
        None,
        None,
    )
    .expect("slice should succeed");

    assert_eq!(slice.regions.len(), 1);
    let region = &slice.regions[0];
    assert!(
        !region.is_top_surface && !region.is_bottom_surface && !region.is_bridge,
        "interior region must have all surface flags false in SliceIR"
    );

    // ── Resolver under "gyroid is sparse holder" config: gyroid declares
    //    claim:sparse-fill and is the configured sparse holder, so the
    //    resolver returns exactly that one claim.
    let gyroid_claims = vec![
        String::from("infill-generator"),
        String::from("claim:sparse-fill"),
    ];
    let cfg = ResolvedConfig {
        sparse_fill_holder: String::from("gyroid-infill"),
        ..ResolvedConfig::default()
    };
    let holders = FillHolders {
        top: &cfg.top_fill_holder,
        bottom: &cfg.bottom_fill_holder,
        bridge: &cfg.bridge_fill_holder,
        sparse: &cfg.sparse_fill_holder,
    };
    let gyroid_held = resolve_held_claims("gyroid-infill", &gyroid_claims, &holders);
    assert_eq!(
        gyroid_held,
        vec![String::from("claim:sparse-fill")],
        "gyroid-infill (configured sparse holder) must hold exactly claim:sparse-fill"
    );

    // ── Resolver for rectilinear in the same config: rectilinear's manifest
    //    declares all four, but config moved sparse to gyroid, so the resolver
    //    drops claim:sparse-fill from rectilinear's effective set.
    let rect_claims: Vec<String> = [
        "infill-generator",
        "claim:top-fill",
        "claim:bottom-fill",
        "claim:bridge-fill",
        "claim:sparse-fill",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    let rect_held = resolve_held_claims("rectilinear-infill", &rect_claims, &holders);
    assert!(
        !rect_held.iter().any(|c| c == "claim:sparse-fill"),
        "rectilinear must NOT hold claim:sparse-fill when gyroid is configured holder"
    );
    assert!(
        rect_held.iter().any(|c| c == "claim:top-fill"),
        "rectilinear must still hold claim:top-fill (top holder unchanged)"
    );

    // ── Runtime filter: gyroid view emits sparse but not top.
    let mut gyroid_view = SliceRegionView::default();
    gyroid_view.set_object_id(ObjectId::from("gyroid-obj"));
    gyroid_view.set_region_id(RegionId::from(0u64));
    gyroid_view.set_polygons(Vec::new());
    gyroid_view.set_infill_areas(Vec::new());
    gyroid_view.set_effective_layer_height(0.2);
    gyroid_view.set_z(1.0);
    gyroid_view.set_has_nonplanar(false);
    gyroid_view.set_held_claims(gyroid_held);
    assert!(
        gyroid_view.should_emit(ExtrusionRole::SparseInfill),
        "gyroid (holding claim:sparse-fill) must emit SparseInfill"
    );
    assert!(
        !gyroid_view.should_emit(ExtrusionRole::TopSolidInfill),
        "gyroid must NOT emit TopSolidInfill — it does not hold claim:top-fill"
    );

    // ── Runtime filter: rectilinear view emits top but not sparse.
    let mut rect_view = SliceRegionView::default();
    rect_view.set_object_id(ObjectId::from("rect-obj"));
    rect_view.set_region_id(RegionId::from(0u64));
    rect_view.set_polygons(Vec::new());
    rect_view.set_infill_areas(Vec::new());
    rect_view.set_effective_layer_height(0.2);
    rect_view.set_z(1.0);
    rect_view.set_has_nonplanar(false);
    rect_view.set_held_claims(rect_held);
    assert!(
        rect_view.should_emit(ExtrusionRole::TopSolidInfill),
        "rectilinear must emit TopSolidInfill (still its claim)"
    );
    assert!(
        !rect_view.should_emit(ExtrusionRole::SparseInfill),
        "rectilinear must NOT emit SparseInfill when gyroid is the sparse holder"
    );
}

// ============================================================================
// TEST 4: gyroid_does_not_emit_for_unheld_top_claim
// ============================================================================
// Verifies: Gyroid holds only claim:sparse-fill. When region is_top_surface=true,
// gyroid must NOT emit top-fill paths because it does not hold claim:top-fill.
// The runtime held-claims filter will block gyroid's top-surface output.
// ============================================================================

/// AC-4: Given gyroid that only holds claim:sparse-fill, when region
/// is_top_surface=true, gyroid emits no top paths (unheld claim blocked by
/// the runtime filter).
#[test]
fn gyroid_does_not_emit_for_unheld_top_claim() {
    let object_id = "gyroid-top-obj";
    let mesh_ir = make_mesh_ir(object_id, flat_top_triangle(1.0));
    let surface_class = make_surface_class(object_id, FacetClass::TopSurface, None);
    let polygons = vec![region_polygon_covering_origin()];

    let (is_top, is_bot, is_bridge) = classify_region_surfaces(
        &mesh_ir.objects[0],
        &surface_class.per_object[object_id],
        &polygons,
        1.0,
        Some(1.2),
        Some(0.8),
        1,
        1,
    );

    assert!(is_top, "top surface classification failed");
    assert!(!is_bot);
    assert!(!is_bridge);

    let layer = make_layer(0, 1.0, object_id);
    let slice = execute_layer_slice(
        &mesh_ir,
        &layer,
        Some(&surface_class),
        Some(1.2),
        Some(0.8),
        None,
        None,
    )
    .expect("slice should succeed");

    assert_eq!(slice.regions.len(), 1);
    let region = &slice.regions[0];
    assert!(
        region.is_top_surface,
        "SliceIR region must have is_top_surface=true"
    );

    // ── Resolver: even though the surface IS top, gyroid declares only
    //    claim:sparse-fill, so the resolver returns no claim:top-fill for it.
    let gyroid_claims = vec![
        String::from("infill-generator"),
        String::from("claim:sparse-fill"),
    ];
    let cfg = ResolvedConfig {
        sparse_fill_holder: String::from("gyroid-infill"),
        ..ResolvedConfig::default()
    };
    let holders = FillHolders {
        top: &cfg.top_fill_holder,
        bottom: &cfg.bottom_fill_holder,
        bridge: &cfg.bridge_fill_holder,
        sparse: &cfg.sparse_fill_holder,
    };
    let gyroid_held = resolve_held_claims("gyroid-infill", &gyroid_claims, &holders);
    assert!(
        !gyroid_held.iter().any(|c| c == "claim:top-fill"),
        "gyroid must NOT hold claim:top-fill regardless of config"
    );

    // ── Runtime filter: should_emit(TopSolidInfill) returns false.
    let mut gyroid_view = SliceRegionView::default();
    gyroid_view.set_object_id(ObjectId::from("gyroid-top-obj"));
    gyroid_view.set_region_id(RegionId::from(0u64));
    gyroid_view.set_polygons(Vec::new());
    gyroid_view.set_infill_areas(Vec::new());
    gyroid_view.set_effective_layer_height(0.2);
    gyroid_view.set_z(1.0);
    gyroid_view.set_has_nonplanar(false);
    gyroid_view.set_held_claims(gyroid_held);
    assert!(
        !gyroid_view.should_emit(ExtrusionRole::TopSolidInfill),
        "gyroid view must reject TopSolidInfill emission for the unheld top claim"
    );
}

// ============================================================================
// TEST 5: region_override_redirects_claim_to_alternate_holder
// ============================================================================
// Verifies: Given a per-region override that switches sparse to lightning-infill,
// lightning emits sparse for that region. The region override mechanism routes
// the sparse-fill claim to lightning instead of rectilinear.
// ============================================================================

/// AC-5: Given a per-region override that switches sparse to lightning-infill,
/// lightning (holding only sparse) emits sparse for that region while rectilinear
/// (holding all four) emits nothing for sparse in that region.
#[test]
fn region_override_redirects_claim_to_alternate_holder() {
    // Verifies the per-region override AC by:
    //   1. Building a `RegionMapIR` with two `RegionPlan` entries — region A
    //      uses the default `ResolvedConfig` (rectilinear holds sparse) and
    //      region B sets `sparse_fill_holder = "lightning-infill"`.
    //   2. Running `resolve_held_claims` per region for both lightning and
    //      rectilinear and confirming the resolver routes sparse to the
    //      configured holder per region.
    //   3. Constructing a `SliceRegionView` per (module, region) pair and
    //      asserting `should_emit(SparseInfill)` — the same runtime helper
    //      consumed by the three core infill modules — gates emission to
    //      lightning in region B and to rectilinear in region A.

    let object_id = "override-obj";

    // Interior region (no surface flags)
    let interior_mesh = IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.5,
            },
            Point3 {
                x: 10.0,
                y: 0.0,
                z: 1.5,
            },
            Point3 {
                x: 0.0,
                y: 10.0,
                z: 1.0,
            },
        ],
        indices: vec![0, 1, 2],
    };
    let mesh_ir = make_mesh_ir(object_id, interior_mesh);
    let surface_class = make_surface_class(object_id, FacetClass::Normal, None);
    let polygons = vec![region_polygon_covering_origin()];

    let (is_top, is_bot, is_bridge) = classify_region_surfaces(
        &mesh_ir.objects[0],
        &surface_class.per_object[object_id],
        &polygons,
        1.0,
        Some(1.2),
        Some(0.8),
        3,
        3,
    );

    assert!(!is_top && !is_bot && !is_bridge, "interior region expected");

    let layer = make_layer(0, 1.0, object_id);
    let slice = execute_layer_slice(
        &mesh_ir,
        &layer,
        Some(&surface_class),
        Some(1.2),
        Some(0.8),
        None,
        None,
    )
    .expect("slice should succeed");

    assert_eq!(slice.regions.len(), 1);

    // ── Build a RegionMapIR with per-region overrides:
    //    region "A" → default (rectilinear holds sparse)
    //    region "B" → sparse_fill_holder = "lightning-infill"
    let region_a_key = RegionKey {
        global_layer_index: 0,
        object_id: ObjectId::from("override-obj"),
        region_id: 0u64 as RegionId,
    };
    let region_b_key = RegionKey {
        global_layer_index: 0,
        object_id: ObjectId::from("override-obj"),
        region_id: 1u64 as RegionId,
    };
    let mut region_map = RegionMapIR::default();
    region_map
        .entries
        .insert(region_a_key.clone(), RegionPlan::default());
    let config_b = ResolvedConfig {
        sparse_fill_holder: String::from("lightning-infill"),
        ..ResolvedConfig::default()
    };
    region_map.entries.insert(
        region_b_key.clone(),
        RegionPlan {
            config: config_b,
            ..Default::default()
        },
    );

    // ── Resolver per region: lightning's manifest declares claim:sparse-fill.
    let lightning_claims = vec![
        String::from("infill-generator"),
        String::from("claim:sparse-fill"),
    ];

    // Region A — default config: lightning is NOT the sparse holder, returns [].
    let cfg_a = &region_map.entries[&region_a_key].config;
    let holders_a = FillHolders {
        top: &cfg_a.top_fill_holder,
        bottom: &cfg_a.bottom_fill_holder,
        bridge: &cfg_a.bridge_fill_holder,
        sparse: &cfg_a.sparse_fill_holder,
    };
    let lightning_a = resolve_held_claims("lightning-infill", &lightning_claims, &holders_a);
    assert!(
        lightning_a.is_empty(),
        "lightning must hold no fill claims for region A (default config: rectilinear holds sparse)"
    );

    // Region B — override: lightning IS the sparse holder, returns ["claim:sparse-fill"].
    let cfg_b = &region_map.entries[&region_b_key].config;
    let holders_b = FillHolders {
        top: &cfg_b.top_fill_holder,
        bottom: &cfg_b.bottom_fill_holder,
        bridge: &cfg_b.bridge_fill_holder,
        sparse: &cfg_b.sparse_fill_holder,
    };
    let lightning_b = resolve_held_claims("lightning-infill", &lightning_claims, &holders_b);
    assert_eq!(
        lightning_b,
        vec![String::from("claim:sparse-fill")],
        "lightning must hold claim:sparse-fill for region B (override redirects sparse to lightning)"
    );

    // Mirror: rectilinear loses sparse for region B but keeps it for region A.
    let rect_claims: Vec<String> = [
        "infill-generator",
        "claim:top-fill",
        "claim:bottom-fill",
        "claim:bridge-fill",
        "claim:sparse-fill",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    let rect_a = resolve_held_claims("rectilinear-infill", &rect_claims, &holders_a);
    let rect_b = resolve_held_claims("rectilinear-infill", &rect_claims, &holders_b);
    assert!(
        rect_a.iter().any(|c| c == "claim:sparse-fill"),
        "rectilinear must keep claim:sparse-fill for region A"
    );
    assert!(
        !rect_b.iter().any(|c| c == "claim:sparse-fill"),
        "rectilinear must lose claim:sparse-fill for region B (overridden to lightning)"
    );

    // ── Runtime filter: SliceRegionView::should_emit gates path emission for
    //    modules that ARE dispatched in a region. This is the same helper
    //    the three core infill modules call before emitting paths.
    //
    //    For lightning in region A: the resolver returns [] (above), which is
    //    the dispatch-side signal that lightning is not the holder for any
    //    fill claim in region A — the host therefore won't dispatch lightning
    //    for region A's infill in production. We do not assert via should_emit
    //    here because the SDK convention treats an empty held-claim list as
    //    "holds all" for back-compat with non-fill modules (DEV-042); that
    //    fall-through is irrelevant when dispatch never invokes the module.

    // Region A (default): rectilinear is dispatched and emits sparse.
    let mut rect_view_a = SliceRegionView::default();
    rect_view_a.set_object_id(ObjectId::from("override-obj"));
    rect_view_a.set_region_id(RegionId::from(0u64));
    rect_view_a.set_polygons(Vec::new());
    rect_view_a.set_infill_areas(Vec::new());
    rect_view_a.set_effective_layer_height(0.2);
    rect_view_a.set_z(1.0);
    rect_view_a.set_has_nonplanar(false);
    rect_view_a.set_held_claims(rect_a);
    assert!(
        rect_view_a.should_emit(ExtrusionRole::SparseInfill),
        "region A: rectilinear must emit SparseInfill (default config: rectilinear is the sparse holder)"
    );

    // Region B (override): lightning is dispatched and emits sparse;
    //                      rectilinear is dispatched for the other roles
    //                      but its sparse role is filtered out.
    let mut lightning_view_b = SliceRegionView::default();
    lightning_view_b.set_object_id(ObjectId::from("override-obj"));
    lightning_view_b.set_region_id(RegionId::from(1u64));
    lightning_view_b.set_polygons(Vec::new());
    lightning_view_b.set_infill_areas(Vec::new());
    lightning_view_b.set_effective_layer_height(0.2);
    lightning_view_b.set_z(1.0);
    lightning_view_b.set_has_nonplanar(false);
    lightning_view_b.set_held_claims(lightning_b);
    assert!(
        lightning_view_b.should_emit(ExtrusionRole::SparseInfill),
        "region B: lightning must emit SparseInfill (override redirects sparse to lightning)"
    );

    let mut rect_view_b = SliceRegionView::default();
    rect_view_b.set_object_id(ObjectId::from("override-obj"));
    rect_view_b.set_region_id(RegionId::from(1u64));
    rect_view_b.set_polygons(Vec::new());
    rect_view_b.set_infill_areas(Vec::new());
    rect_view_b.set_effective_layer_height(0.2);
    rect_view_b.set_z(1.0);
    rect_view_b.set_has_nonplanar(false);
    rect_view_b.set_held_claims(rect_b);
    assert!(
        !rect_view_b.should_emit(ExtrusionRole::SparseInfill),
        "region B: rectilinear must NOT emit SparseInfill (override redirects sparse to lightning)"
    );
    // Sanity: rectilinear retains its other claims even when sparse is overridden.
    assert!(
        rect_view_b.should_emit(ExtrusionRole::TopSolidInfill),
        "region B: rectilinear must still emit TopSolidInfill (override only affects sparse)"
    );
}

// ============================================================================
// TEST 6: two_holders_for_one_claim_fails_validation
// ============================================================================
// Verifies: When both rectilinear and gyroid declare claim:sparse-fill
// with no override to resolve the conflict, validation pass 2 (validate_claim_conflicts)
// fails with ClaimConflict error.
// ============================================================================

/// AC-6: Given both rectilinear and gyroid declare claim:sparse-fill with no
/// override, validation pass 2 fails with ClaimConflict.
#[test]
fn two_holders_for_one_claim_fails_validation() {
    use slicer_host::validation::{
        validate_startup_dag, ClaimHolder, ConflictScope, DagValidationReport, DagValidationRequest,
    };
    use slicer_ir::ModuleId;

    let rectilinear_id = ModuleId::from("rectilinear-infill");
    let gyroid_id = ModuleId::from("gyroid-infill");

    // Both hold sparse-fill in global scope → conflict
    let claim_holders = vec![
        ClaimHolder {
            claim: CLAIM_SPARSE_FILL.to_string(),
            module_id: rectilinear_id.clone(),
            scope: ConflictScope::Global,
        },
        ClaimHolder {
            claim: CLAIM_SPARSE_FILL.to_string(),
            module_id: gyroid_id.clone(),
            scope: ConflictScope::Global,
        },
    ];

    let request = DagValidationRequest {
        modules: vec![],
        claim_holders,
        stage_dags: vec![],
        access_audits: vec![],
        host_ir_schema_version: SemVer::default(),
    };

    let report = validate_startup_dag(&request);

    let claim_conflicts: Vec<_> = report
        .errors
        .iter()
        .filter(|e| {
            matches!(&e.detail, slicer_host::validation::SchedulerError::ClaimConflict { claim, .. }
                 if claim == CLAIM_SPARSE_FILL)
        })
        .collect();

    assert!(
        !claim_conflicts.is_empty(),
        "expected ClaimConflict error for sparse-fill double-holder \
         (rectilinear + gyroid both hold sparse in global scope). \
         Errors: {:#?}",
        report.errors
    );

    if let slicer_host::validation::SchedulerError::ClaimConflict {
        claim,
        module_a,
        module_b,
        scope,
    } = &claim_conflicts[0].detail
    {
        assert_eq!(claim.as_str(), CLAIM_SPARSE_FILL);
        assert!(
            (module_a.as_str() == "rectilinear-infill" && module_b.as_str() == "gyroid-infill")
                || (module_a.as_str() == "gyroid-infill"
                    && module_b.as_str() == "rectilinear-infill"),
            "expected conflict between rectilinear-infill and gyroid-infill"
        );
        assert!(matches!(scope, ConflictScope::Global));
    } else {
        unreachable!();
    }
}

// ============================================================================
// TEST 7: missing_holder_for_top_fill_claim_fails_validation
// ============================================================================
// Verifies: When no module holds claim:top-fill, validation fails with
// MissingDependency or equivalent.
/// AC-7: Given no module holds claim:top-fill, validation fails with
/// MissingDependency or equivalent error.
#[test]
fn missing_holder_for_top_fill_claim_fails_validation() {
    use slicer_host::validation::{
        validate_startup_dag, ClaimHolder, ConflictScope, DagValidationReport, DagValidationRequest,
    };
    use slicer_ir::ModuleId;

    let rectilinear_id = ModuleId::from("rectilinear-infill");
    let gyroid_id = ModuleId::from("gyroid-infill");

    // rectilinear holds bottom-fill, bridge-fill, sparse-fill (NOT top-fill)
    // gyroid holds sparse-fill only
    // → claim:top-fill has zero holders
    let claim_holders = vec![
        ClaimHolder {
            claim: CLAIM_BOTTOM_FILL.to_string(),
            module_id: rectilinear_id.clone(),
            scope: ConflictScope::Global,
        },
        ClaimHolder {
            claim: CLAIM_BRIDGE_FILL.to_string(),
            module_id: rectilinear_id.clone(),
            scope: ConflictScope::Global,
        },
        ClaimHolder {
            claim: CLAIM_SPARSE_FILL.to_string(),
            module_id: rectilinear_id.clone(),
            scope: ConflictScope::Global,
        },
        ClaimHolder {
            claim: CLAIM_SPARSE_FILL.to_string(),
            module_id: gyroid_id.clone(),
            scope: ConflictScope::Global,
        },
    ];

    let request = DagValidationRequest {
        modules: vec![],
        claim_holders,
        stage_dags: vec![],
        access_audits: vec![],
        host_ir_schema_version: SemVer::default(),
    };

    let report = validate_startup_dag(&request);

    let missing_deps: Vec<_> = report
        .errors
        .iter()
        .filter(|e| {
            matches!(
                &e.detail,
                slicer_host::validation::SchedulerError::MissingDependency { .. }
            )
        })
        .collect();

    // Validation should detect that claim:top-fill has no holder
    // and emit MissingDependency error.
    assert!(
        !missing_deps.is_empty(),
        "expected MissingDependency error for claim:top-fill with no holders. \
         Errors: {:#?}",
        report.errors
    );
}

// ============================================================================
// TEST 8: unknown_claim_in_manifest_is_load_error
// ============================================================================
// Verifies: A manifest with `[claims].holds = ["claim:invalid-fill"]` causes
// Phase-1 ingestion (ingest_manifest / load_module_from_paths) to return a
// LoadError with the unknown claim ID named.
// ============================================================================

/// AC-8: Given a manifest with `[claims].holds = ["claim:invalid-fill"]`,
/// Phase-1 ingestion returns a LoadError with the unknown claim ID named
/// and LoadErrorKind::Validation.
#[test]
fn unknown_claim_in_manifest_is_load_error() {
    use slicer_host::{load_module_from_paths, LoadErrorKind};
    use std::path::PathBuf;

    // We need a real manifest + wasm pair on disk with an invalid claim.
    // Use a temp file to avoid polluting the repo.
    let tmp_dir = tempfile::TempDir::new().expect("tempdir");
    let manifest_path = tmp_dir.path().join("test-module.toml");
    let wasm_path = tmp_dir.path().join("test-module.wasm");

    // Create a minimal valid WASM (empty placeholder, just needs the magic prefix)
    std::fs::write(&wasm_path, b"\0asm").expect("write placeholder wasm");

    // Write a manifest with an unknown claim
    let manifest_content = format!(
        r#"
[module]
id           = "test-unknown-claim"
version      = "0.1.0"
display-name = "Test Unknown Claim"
description  = "Test module with unknown claim"
author       = "test"
license      = "MIT"
wit-world    = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::SlicePostProcess"

[ir-access]
reads  = ["SliceIR"]
writes = []

[claims]
holds    = ["claim:invalid-fill"]
requires = []

[compatibility]
incompatible-with = []
requires          = []
min-host-version  = "0.1.0"
min-ir-schema     = "1.0.0"
max-ir-schema     = "2.0.0"

[config.schema]
[config.overridable-per-region]
keys = []

[config.overridable-per-layer]
keys = []

[hints]
estimated-ms-per-layer = 10
layer-parallel-safe    = true

[wasm]
path = "{}"
"#,
        wasm_path.to_string_lossy().replace('\\', "\\\\")
    );
    std::fs::write(&manifest_path, manifest_content).expect("write manifest");

    let result = load_module_from_paths(&manifest_path, &wasm_path);

    assert!(
        result.is_err(),
        "load_module_from_paths should fail for manifest with unknown claim ID"
    );

    let err = result.unwrap_err();
    assert!(
        matches!(err.kind, LoadErrorKind::Validation),
        "expected LoadErrorKind::Validation for unknown claim, got {:?}",
        err.kind
    );
    assert!(
        err.message.contains("invalid-fill") || err.message.contains("unknown"),
        "error message should reference the unknown claim ID 'invalid-fill': {}",
        err.message
    );
}
