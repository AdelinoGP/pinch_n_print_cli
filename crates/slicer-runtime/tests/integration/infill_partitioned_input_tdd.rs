//! Regression guard: each fill-claim infill module (rectilinear, gyroid,
//! lightning) must emit each role over its dedicated host-partitioned polygon
//! and ONLY that polygon — never over the wall-inset outline, never over a
//! sibling role's polygon.
//!
//! Contract (per `docs/specs/_OLD/infill-fill-partition-plan.md` Q3 + Q7):
//! - SparseInfill paths confined to `region.sparse_infill_area()`.
//! - TopSolidInfill confined to `region.top_solid_fill()`.
//! - BottomSolidInfill confined to `region.bottom_solid_fill()`.
//! - BridgeInfill confined to `region.bridge_areas()`.
//! - When a claim's source polygon is empty, zero paths of that role emit.
//! - `should_emit()` gates each role by held claim independent of polygon state.
//!
//! History: these started red-first against the per-region role-pick
//! (`top_shell_index.is_some()` ladder) bug. Phase 2.2 landed the per-role,
//! per-polygon emit and the ladder is gone, so this file now serves as a
//! regression guard. Each confinement check is paired with a positive
//! "≥1 path emitted" assertion so a module that silently emits *nothing*
//! cannot pass vacuously.
//!
//! **Which stage the containment checks target.** Under ADR-0025 a
//! `Layer::Infill` module emits RAW, unlinked segments over the wall-inset
//! polygon; applying the infill overlap and re-clipping to the per-role
//! partitioned polygon is the `Layer::InfillPostProcess` linker's job. Per-role
//! containment is therefore a property of the module **+ linker pair**, not of
//! the module alone — `ac7*` accordingly drive `FillModule::run` followed by
//! `infill-linker`'s `run_infill_postprocess` and assert on the linked output.
//! The remaining tests here are about emission *gating* (held claims, empty
//! polygons), which is a module-only property, so they still call the module
//! directly.

use gyroid_infill::GyroidInfill;
use infill_linker::InfillLinker;
use lightning_infill::LightningInfill;
use rectilinear_infill::RectilinearInfill;
use slicer_ir::{
    point_in_polygon_winding, ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole,
    InfillRegion, LightningTreeEntry, LightningTreeIR, Point2, Polygon,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_support::fixtures::{PerimeterRegionViewBuilder, SliceRegionViewBuilder};
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::{PerimeterRegionView, SliceRegionView};

// ── fixture helpers ──────────────────────────────────────────────────────────

fn square(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }
}

/// Concave L-shape: the left column + bottom row of a 10×10 area. The top-right
/// 6×6 quadrant (the "notch") is OUTSIDE the polygon but INSIDE its bounding
/// box — so an AABB check accepts a path leaking into the notch while the
/// winding check rejects it. This is what makes the winding upgrade load-bearing.
fn l_shape() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 4.0),
                Point2::from_mm(4.0, 4.0),
                Point2::from_mm(4.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: Vec::new(),
    }
}

fn min_density_config() -> ConfigView {
    let mut map = std::collections::HashMap::new();
    map.insert("infill_density".into(), ConfigValue::Float(0.5));
    map.insert("line_width".into(), ConfigValue::Float(0.4));
    ConfigView::from_map(map)
}

/// True polygon containment in mm space (winding number + boundary tolerance),
/// not AABB. Returns true when every point of `path` lies inside (or within
/// `CONTAIN_EPS_MM` of) the contour of any polygon in `containers`. Path points
/// are already mm (`Point3WithWidth`); container points are integer units and
/// `point_in_polygon_winding` converts them internally.
fn path_inside_any(path: &ExtrusionPath3D, containers: &[ExPolygon]) -> bool {
    if containers.is_empty() {
        return false;
    }
    path.points.iter().all(|pt| {
        containers
            .iter()
            .any(|c| point_in_polygon_winding(c, pt.x as f64, pt.y as f64, CONTAIN_EPS_MM))
    })
}

/// 10 µm boundary tolerance: fill strokes whose terminators land exactly on the
/// source-polygon contour count as inside, while a leak into the (≥6 mm wide)
/// notch is far outside tolerance and still fails.
const CONTAIN_EPS_MM: f64 = 0.01;

/// Mirror of `SliceRegionView::should_emit`'s role→claim mapping
/// (`crates/slicer-sdk/src/views.rs`). Kept in lockstep so `held_claims`
/// fixtures gate exactly the way production dispatch does. The SDK exposes no
/// public constant for these strings, so this is the single source in the test.
fn claim_for_role(role: &ExtrusionRole) -> &'static str {
    match role {
        ExtrusionRole::SparseInfill => "claim:sparse-fill",
        ExtrusionRole::TopSolidInfill => "claim:top-fill",
        ExtrusionRole::BottomSolidInfill => "claim:bottom-fill",
        ExtrusionRole::BridgeInfill => "claim:bridge-fill",
        other => panic!("claim_for_role: non-fill role {other:?}"),
    }
}

fn role_in(roles: &[ExtrusionRole], role: &ExtrusionRole) -> bool {
    roles.iter().any(|r| r == role)
}

/// Build a region with the four canonical polygons placed in disjoint quadrants
/// of a 10×10 wall_inset. Each quadrant is 5×5 = 25 mm².
fn region_with_disjoint_quadrants() -> SliceRegionView {
    let bottom_left = square(0.0, 0.0, 5.0, 5.0);
    let bottom_right = square(5.0, 0.0, 10.0, 5.0);
    let top_left = square(0.0, 5.0, 5.0, 10.0);
    let top_right = square(5.0, 5.0, 10.0, 10.0);
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        // sparse → bottom_left quadrant
        .sparse_infill_area(vec![bottom_left])
        // top → top_left quadrant; top_shell_index participation
        .top_shell_index(Some(0))
        .top_solid_fill(vec![top_left])
        // bottom → bottom_right quadrant; bottom_shell_index participation
        .bottom_shell_index(Some(0))
        .bottom_solid_fill(vec![bottom_right])
        // bridge → top_right quadrant
        .is_bridge(true)
        .bridge_areas(vec![top_right])
        .bridge_orientation_deg(0.0)
        .build()
}

/// `(role, source-polygon)` pairs matching `region_with_disjoint_quadrants`.
fn role_quadrants() -> Vec<(ExtrusionRole, Vec<ExPolygon>)> {
    vec![
        (
            ExtrusionRole::SparseInfill,
            vec![square(0.0, 0.0, 5.0, 5.0)],
        ),
        (
            ExtrusionRole::TopSolidInfill,
            vec![square(0.0, 5.0, 5.0, 10.0)],
        ),
        (
            ExtrusionRole::BottomSolidInfill,
            vec![square(5.0, 0.0, 10.0, 5.0)],
        ),
        (
            ExtrusionRole::BridgeInfill,
            vec![square(5.0, 5.0, 10.0, 10.0)],
        ),
    ]
}

/// All collected paths (sparse + solid + ironing) for assertion.
fn collect_all_paths(output: &InfillOutputBuilder) -> Vec<ExtrusionPath3D> {
    output
        .sparse_paths()
        .iter()
        .chain(output.solid_paths().iter())
        .cloned()
        .collect()
}

fn paths_with_role(paths: &[ExtrusionPath3D], role: ExtrusionRole) -> Vec<ExtrusionPath3D> {
    paths
        .iter()
        .filter(|p| p.role == role && p.points.len() > 1)
        .cloned()
        .collect()
}

// ── module dispatch table ────────────────────────────────────────────────────

const ALL_FILL_ROLES: [ExtrusionRole; 4] = [
    ExtrusionRole::SparseInfill,
    ExtrusionRole::TopSolidInfill,
    ExtrusionRole::BottomSolidInfill,
    ExtrusionRole::BridgeInfill,
];
const SPARSE_ONLY: [ExtrusionRole; 1] = [ExtrusionRole::SparseInfill];

enum FillModule {
    Rectilinear(RectilinearInfill),
    Gyroid(GyroidInfill),
    Lightning(LightningInfill),
}

impl FillModule {
    fn run(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        paint: &PaintRegionLayerView,
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) {
        let result = match self {
            Self::Rectilinear(m) => m.run_infill(layer_index, regions, paint, output, config),
            Self::Gyroid(m) => m.run_infill(layer_index, regions, paint, output, config),
            Self::Lightning(m) => m.run_infill(layer_index, regions, paint, output, config),
        };
        result.unwrap_or_else(|e| panic!("[{}] run_infill failed: {e:?}", self.name()));
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Rectilinear(_) => "rectilinear-infill",
            Self::Gyroid(_) => "gyroid-infill",
            Self::Lightning(_) => "lightning-infill",
        }
    }

    /// Roles this module is expected to populate given a non-empty source
    /// polygon. Rectilinear holds all four fill claims (top, bottom, bridge,
    /// sparse). Gyroid and lightning declare only `claim:sparse-fill`
    /// (packet 37); solid/bridge are delegated to sibling modules.
    fn expected_roles(&self) -> &'static [ExtrusionRole] {
        match self {
            Self::Rectilinear(_) => &ALL_FILL_ROLES,
            Self::Gyroid(_) | Self::Lightning(_) => &SPARSE_ONLY,
        }
    }

    /// Return the held-claim strings matching this module's manifest claims.
    /// Mirrors what `resolve_held_claims` in the production dispatch would
    /// produce when this module is the configured holder for those roles.
    fn held_claims(&self) -> Vec<String> {
        self.expected_roles()
            .iter()
            .map(|r| claim_for_role(r).to_string())
            .collect()
    }
}

/// A `PaintRegionLayerView` carrying lightning tree-edge segments for
/// `(obj-1, region 0, layer 0)`.
///
/// `lightning-infill::run_infill` renders exclusively from
/// `PaintRegionLayerView::lightning_tree_segments_for` — its geometry comes
/// from the `PrePass::LightningTreeGen` product (ADR-0029), not from the region
/// polygons. A tree-less paint view therefore makes lightning emit nothing, and
/// a containment assertion over zero paths is vacuous. Supplying a tree is what
/// makes the lightning arm of `ac7*` a real check.
///
/// The segments are given in mm and deliberately **overshoot** the role
/// polygon: under ADR-0025 a `Layer::Infill` module emits raw geometry and
/// confinement is the linker's job, so a fixture whose raw segments already fit
/// would prove nothing about the re-clip.
fn lightning_paint(segments: &[((f32, f32), (f32, f32))]) -> PaintRegionLayerView {
    let ir = LightningTreeIR {
        entries: vec![LightningTreeEntry {
            object_id: "obj-1".to_string(),
            global_layer_index: 0,
            region_id: 0,
            tree_edge_segments: segments
                .iter()
                .map(|(start, end)| {
                    [
                        Point2::from_mm(start.0, start.1),
                        Point2::from_mm(end.0, end.1),
                    ]
                })
                .collect(),
        }],
        ..Default::default()
    };
    PaintRegionLayerView::new(0).with_lightning_tree_ir(std::sync::Arc::new(ir))
}

/// The `Layer::InfillPostProcess` view of the same region: the host's four
/// partitioned fill polygons plus the wall-inset union, mirrored off the
/// `SliceRegionView` the fill module saw so the two cannot drift.
fn perimeter_view_of(region: &SliceRegionView) -> PerimeterRegionView {
    let mut view = PerimeterRegionViewBuilder::new()
        .object_id(region.object_id().clone())
        .region_id(*region.region_id())
        .sparse_infill_area(region.sparse_infill_area().to_vec())
        .top_solid_fill(region.top_solid_fill().to_vec())
        .bottom_solid_fill(region.bottom_solid_fill().to_vec())
        .bridge_areas(region.bridge_areas().to_vec())
        .build();
    // `infill_areas` is the wall-inset union the perimeters stage publishes.
    // The linker must NOT confine a role to it — that is the ADR-0025 §2 hole
    // this fixture guards.
    view.set_infill_areas(region.polygons().to_vec());
    view.set_config(min_density_config());
    view
}

/// Runs the fill module, then the `Layer::InfillPostProcess` linker over its
/// raw output, and returns the linked result.
fn run_and_link(
    module: &FillModule,
    region: &SliceRegionView,
    paint: &PaintRegionLayerView,
) -> InfillOutputBuilder {
    let mut raw = InfillOutputBuilder::new();
    module.run(
        0,
        std::slice::from_ref(region),
        paint,
        &mut raw,
        &min_density_config(),
    );

    let prior = vec![InfillRegion {
        object_id: region.object_id().clone(),
        region_id: *region.region_id(),
        sparse_infill: raw.sparse_paths().to_vec(),
        solid_infill: raw.solid_paths().to_vec(),
        ironing: raw.ironing_paths().to_vec(),
    }];
    let views = vec![perimeter_view_of(region)];

    let config = min_density_config();
    let linker = InfillLinker::on_print_start(&config).expect("linker init");
    let mut linked = InfillOutputBuilder::new();
    linker
        .run_infill_postprocess(0, &views, &prior, &mut linked, &config)
        .unwrap_or_else(|e| panic!("[{}] infill-linker failed: {e:?}", module.name()));
    linked
}

fn all_three_modules() -> Vec<FillModule> {
    let cfg = min_density_config();
    vec![
        FillModule::Rectilinear(RectilinearInfill::on_print_start(&cfg).unwrap()),
        FillModule::Gyroid(GyroidInfill::on_print_start(&cfg).unwrap()),
        FillModule::Lightning(LightningInfill::on_print_start(&cfg).unwrap()),
    ]
}

// ── AC-7: per-role paths confined to per-role source polygon ─────────────────

#[test]
fn ac7_each_role_confined_to_its_own_canonical_polygon_for_all_three_modules() {
    let region = region_with_disjoint_quadrants();
    // Sparse is the bottom-left 5×5 quadrant; two of these three branches run
    // straight out of it into the bottom-right and top-left quadrants.
    let paint = lightning_paint(&[
        ((1.0, 1.0), (9.0, 1.0)),
        ((1.0, 1.0), (1.0, 9.0)),
        ((1.0, 4.0), (4.0, 1.0)),
    ]);

    for module in all_three_modules() {
        let mut region_clone = region.clone();
        region_clone.set_held_claims(module.held_claims());
        let output = run_and_link(&module, &region_clone, &paint);
        let all = collect_all_paths(&output);
        let expected = module.expected_roles();

        for (role, source) in role_quadrants() {
            let paths = paths_with_role(&all, role.clone());

            if role_in(expected, &role) {
                // Positive: a module that holds this claim MUST emit at least
                // one path. Without this, the confinement loop below is vacuous
                // — a module emitting nothing would pass silently.
                assert!(
                    !paths.is_empty(),
                    "[{}] expected ≥1 {:?} path over its source polygon; got 0 \
                     (vacuous-confinement regression)",
                    module.name(),
                    role
                );
                for p in &paths {
                    assert!(
                        path_inside_any(p, &source),
                        "[{}] {:?} path escaped its source polygon",
                        module.name(),
                        role
                    );
                }
            } else {
                // Negative: lightning delegates solid/bridge to sibling modules,
                // so it must emit nothing for those roles.
                assert!(
                    paths.is_empty(),
                    "[{}] {:?} is delegated to sibling modules; expected 0 paths, got {}",
                    module.name(),
                    role,
                    paths.len()
                );
            }
        }
    }
}

// ── REGRESSION: empty held_claims suppresses all fill emission ──────────────
//
// Pre-fix: `should_emit` had a fail-open — empty held_claims returned true
// for every role. When dispatch correctly resolved that gyroid/lightning hold
// nothing (all four holders default to rectilinear-infill), they still emitted
// duplicate sparse infill paths overlapping rectilinear's output.
// Post-fix: empty held_claims = emit nothing.

#[test]
fn empty_held_claims_suppresses_all_fill_emission() {
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        .sparse_infill_area(vec![square(0.0, 0.0, 10.0, 10.0)])
        .top_shell_index(Some(0))
        .top_solid_fill(vec![square(0.0, 0.0, 10.0, 10.0)])
        .build();

    for module in all_three_modules() {
        // held_claims left empty — dispatch resolved this module holds nothing.
        let mut output = InfillOutputBuilder::new();
        module.run(
            0,
            std::slice::from_ref(&region),
            &PaintRegionLayerView::new(0),
            &mut output,
            &min_density_config(),
        );
        let all = collect_all_paths(&output);
        assert!(
            all.is_empty(),
            "[{}] empty held_claims → expected 0 paths (all roles suppressed); got {}",
            module.name(),
            all.len()
        );
    }
}

#[test]
fn empty_held_claims_suppresses_sparse_even_when_polygon_populated() {
    // Sparse polygon is populated but held_claims is empty — must emit nothing.
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        .sparse_infill_area(vec![square(0.0, 0.0, 10.0, 10.0)])
        .build();

    for module in all_three_modules() {
        let mut output = InfillOutputBuilder::new();
        module.run(
            0,
            std::slice::from_ref(&region),
            &PaintRegionLayerView::new(0),
            &mut output,
            &min_density_config(),
        );
        let sparse = paths_with_role(&collect_all_paths(&output), ExtrusionRole::SparseInfill);
        assert!(
            sparse.is_empty(),
            "[{}] empty held_claims → expected 0 SparseInfill paths; got {}",
            module.name(),
            sparse.len()
        );
    }
}

#[test]
fn ac7b_concave_sparse_area_confined_via_winding_not_just_aabb() {
    let area = l_shape();
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        .sparse_infill_area(vec![area.clone()])
        .build();

    let containers = [area];
    // Each branch crosses the notch: the diagonal cuts the reflex corner at
    // (4,4), the other two run along y=8 / x=8 out of the L's arms.
    let paint = lightning_paint(&[
        ((1.0, 1.0), (9.0, 9.0)),
        ((1.0, 8.0), (8.0, 8.0)),
        ((8.0, 1.0), (8.0, 8.0)),
    ]);
    for module in all_three_modules() {
        let mut region_clone = region.clone();
        region_clone.set_held_claims(module.held_claims());
        let output = run_and_link(&module, &region_clone, &paint);
        let sparse = paths_with_role(&collect_all_paths(&output), ExtrusionRole::SparseInfill);
        assert!(
            !sparse.is_empty(),
            "[{}] expected ≥1 SparseInfill path over the L-shaped area; got 0",
            module.name()
        );
        for p in &sparse {
            assert!(
                path_inside_any(p, &containers),
                "[{}] SparseInfill path leaked outside the concave L-shape \
                 (into the notch an AABB check would miss)",
                module.name()
            );
        }
    }
}

// ── AC-8: empty sparse_infill_area → zero SparseInfill paths regardless of flag ─

#[test]
fn ac8_empty_sparse_infill_area_yields_zero_sparse_paths_even_with_top_flag_set() {
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        // sparse_infill_area is empty (pure top layer)
        .sparse_infill_area(Vec::new())
        // top_shell_index flag set + top_solid_fill populated
        .top_shell_index(Some(0))
        .top_solid_fill(vec![square(0.0, 0.0, 10.0, 10.0)])
        .build();

    for module in all_three_modules() {
        let mut output = InfillOutputBuilder::new();
        let mut region_clone = region.clone();
        region_clone.set_held_claims(module.held_claims());
        module.run(
            0,
            std::slice::from_ref(&region_clone),
            &PaintRegionLayerView::new(0),
            &mut output,
            &min_density_config(),
        );
        let all = collect_all_paths(&output);

        let sparse = paths_with_role(&all, ExtrusionRole::SparseInfill);
        assert!(
            sparse.is_empty(),
            "[{}] expected 0 SparseInfill paths when sparse_infill_area is empty; got {}",
            module.name(),
            sparse.len()
        );

        // Positive guard: for modules that produce solid fill, top MUST emit —
        // proving the zero-sparse result is the partition working, not the
        // module no-op'ing on this fixture.
        if role_in(module.expected_roles(), &ExtrusionRole::TopSolidInfill) {
            let top = paths_with_role(&all, ExtrusionRole::TopSolidInfill);
            assert!(
                !top.is_empty(),
                "[{}] top_solid_fill is populated; expected ≥1 TopSolidInfill path \
                 (zero would mean the module did nothing, not that sparse was empty)",
                module.name()
            );
        }
    }
}

// ── AC-9: all four polygons empty → zero paths, no panic ─────────────────────

#[test]
fn ac9_all_four_polygons_empty_yields_zero_paths_no_panic() {
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        .sparse_infill_area(Vec::new())
        .top_solid_fill(Vec::new())
        .bottom_solid_fill(Vec::new())
        .bridge_areas(Vec::new())
        .build();

    for module in all_three_modules() {
        let mut output = InfillOutputBuilder::new();
        let mut region_clone = region.clone();
        region_clone.set_held_claims(module.held_claims());
        module.run(
            0,
            std::slice::from_ref(&region_clone),
            &PaintRegionLayerView::new(0),
            &mut output,
            &min_density_config(),
        );
        let all = collect_all_paths(&output);
        assert!(
            all.is_empty(),
            "[{}] expected 0 paths when all four polygons empty; got {}",
            module.name(),
            all.len()
        );
    }
}

// ── NEG-1: should_emit gating filters roles by held_claims ───────────────────

#[test]
fn neg1_should_emit_gating_filters_top_role_by_held_claims() {
    // Region with a populated top_solid_fill (the whole 10×10 square) and the
    // top-shell flag set. Whether TopSolidInfill paths appear must depend ONLY
    // on whether `claim:top-fill` is held — not on the flag, not on the polygon
    // being non-empty.
    let base = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        .top_shell_index(Some(0))
        .top_solid_fill(vec![square(0.0, 0.0, 10.0, 10.0)])
        .build();

    // Case A — claim:top-fill NOT held (only sparse) → zero TopSolidInfill paths.
    let mut gated_out = base.clone();
    gated_out.set_held_claims(vec![claim_for_role(&ExtrusionRole::SparseInfill).into()]);

    // Case B — claim:top-fill held → TopSolidInfill paths DO emit. This positive
    // counterpart is what distinguishes "gating works" from "the module never
    // emits top at all" — Case A alone cannot tell those apart.
    let mut gated_in = base.clone();
    gated_in.set_held_claims(vec![claim_for_role(&ExtrusionRole::TopSolidInfill).into()]);

    for module in all_three_modules() {
        // A module's code may be capable of emitting top fill even if its
        // manifest only declares sparse-fill (gyroid). `code_can_emit_top`
        // captures that: rectilinear and gyroid both have top-fill code paths;
        // lightning does not.
        let code_can_emit_top = !matches!(module.name(), "lightning-infill");

        let mut out_a = InfillOutputBuilder::new();
        module.run(
            0,
            std::slice::from_ref(&gated_out),
            &PaintRegionLayerView::new(0),
            &mut out_a,
            &min_density_config(),
        );
        let top_a = paths_with_role(&collect_all_paths(&out_a), ExtrusionRole::TopSolidInfill);
        assert!(
            top_a.is_empty(),
            "[{}] claim:top-fill not held → expected 0 TopSolidInfill paths; got {}",
            module.name(),
            top_a.len()
        );

        let mut out_b = InfillOutputBuilder::new();
        module.run(
            0,
            std::slice::from_ref(&gated_in),
            &PaintRegionLayerView::new(0),
            &mut out_b,
            &min_density_config(),
        );
        let top_b = paths_with_role(&collect_all_paths(&out_b), ExtrusionRole::TopSolidInfill);
        if code_can_emit_top {
            assert!(
                !top_b.is_empty(),
                "[{}] claim:top-fill held + top_solid_fill populated → \
                 expected ≥1 TopSolidInfill path; got 0",
                module.name()
            );
        } else {
            // Lightning produces no solid fill regardless of the held claim.
            assert!(
                top_b.is_empty(),
                "[{}] produces no solid fill; expected 0 TopSolidInfill paths even when \
                 claim:top-fill held; got {}",
                module.name(),
                top_b.len()
            );
        }
    }
}
