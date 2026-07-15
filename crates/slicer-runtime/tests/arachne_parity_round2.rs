//! # Arachne parity audit round 3 — red TDD tests against `parity/arachne`.
//!
//! Three RED parity-gap tests (G12, G15, G20) for the `parity/arachne` branch.
//! Every test in this file fails on purpose on the current tree, panicking
//! with a message of the form:
//!
//! `PARITY GAP: <feature> | expected: <orcaslicer behavior> | got: <current
//! behavior> | ref: <OrcaSlicer path:line>`
//!
//! The failure message *is* the deliverable. Do not `#[ignore]`, weaken, or
//! delete these tests to get a green build — each one is closed by
//! implementing the named OrcaSlicer behavior.
//!
//! Coordinate convention (`docs/08_coordinate_system.md`): 1 unit = 100 nm =
//! 10⁻⁴ mm. All config keys are snake_case.

#![allow(dead_code)]

#[path = "common/mod.rs"]
mod common;
#[path = "fixtures/arachne_parity/mod.rs"]
mod fixtures;

use slicer_core::arachne::{run_arachne_pipeline, simplify_toolpaths, ArachneParams};
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_ir::{
    ConfigValue, ExPolygon, ExtrusionLine, Point2, Polygon, SemVer, SliceIR, SlicedRegion,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::manifest::LoadedModuleBuilder;
use slicer_runtime::{Blackboard, CompiledModuleBuilder, LayerArena, WasmRuntimeDispatcher};
use slicer_wasm_host::host::HOST_ARACHNE_WALL_SEQUENCE_CAPTURE;
use std::collections::HashMap;
use std::sync::Arc;

// ===========================================================================
// G12 — wall region ordering: inner (odd) region must follow enclosing even
// region (OrcaSlicer `WallToolPaths::getRegionOrder`; PerimeterGenerator's
// finalized-extrusion walk).
// ===========================================================================

/// G12: drive `run_arachne_pipeline` with two concentric square islands and
/// assert the outer-wall (`inset_idx == 0`) `ExtrusionLine`s precede the
/// inner-wall (`inset_idx >= 1`) ones in the returned `Vec`.
///
/// The direct-core fixture establishes only finalized-line ordering. Separate
/// guest-host and end-to-end tests cover three-state wall-sequence propagation.
#[test]
fn arachne_parity_wall_region_order_odd_after_enclosing() {
    let params = ArachneParams {
        wall_sequence: slicer_core::perimeter_utils::WallSequence::OuterInner,
        ..ArachneParams::default()
    };

    let lines = match run_arachne_pipeline(
        &fixtures::ex_polygons_concentric_islands_mm(),
        &params,
        false,
    ) {
        Ok((lines, _inner_contours)) => lines,
        Err(_) => panic!(
            "PARITY GAP: wall region order odd-after-enclosing | expected: \
             emitted wall regions ordered so inner (odd) region follows its \
              enclosing even region (WallToolPaths::getRegionOrder) | got: \
              pipeline did not produce finalized region order | observed \
              first_inner=N/A last_outer=N/A"
        ),
    };

    let unordered_params = ArachneParams {
        wall_sequence: slicer_core::perimeter_utils::WallSequence::InnerOuter,
        ..params.clone()
    };
    let baseline = run_arachne_pipeline(
        &fixtures::ex_polygons_concentric_islands_mm(),
        &unordered_params,
        false,
    )
    .expect("same finalized lines should be available without region ordering")
    .0;

    assert_eq!(
        lines.len(),
        baseline.len(),
        "region ordering must retain every finalized line"
    );
    let mut matched = vec![false; baseline.len()];
    for line in &lines {
        let Some(index) = baseline
            .iter()
            .enumerate()
            .find(|(index, candidate)| !matched[*index] && *candidate == line)
            .map(|(index, _)| index)
        else {
            panic!("region ordering output contains a line absent from the baseline")
        };
        matched[index] = true;
    }

    if lines.is_empty() {
        panic!(
            "PARITY GAP: wall region order odd-after-enclosing | expected: \
             emitted wall regions ordered so inner (odd) region follows its \
              enclosing even region (WallToolPaths::getRegionOrder) | got: \
              pipeline returned no finalized lines | observed \
              first_inner=N/A last_outer=N/A"
        );
    }

    // First index of any inner wall and last index of any outer wall.
    let first_inner = lines.iter().position(|l| l.inset_idx >= 1);
    let last_outer = lines.iter().rposition(|l| l.inset_idx == 0);

    let (first_inner, last_outer) = match (first_inner, last_outer) {
        (Some(o), Some(i)) => (o, i),
        _ => panic!(
            "PARITY GAP: wall region order odd-after-enclosing | expected: \
             emitted wall regions ordered so inner (odd) region follows its \
              enclosing even region (WallToolPaths::getRegionOrder) | got: \
              pipeline omitted an outer or inner line | observed \
              first_inner={first_inner:?} last_outer={last_outer:?}"
        ),
    };

    // The first inner line must follow the last enclosing outer line.
    assert!(
        first_inner > last_outer,
        "PARITY GAP: wall region order odd-after-enclosing | expected: \
         emitted wall regions ordered so inner (odd) region follows its \
          enclosing even region (WallToolPaths::getRegionOrder) | got: \
          finalized pipeline order violates the enclosing-region precedence | observed \
          first_inner={first_inner} last_outer={last_outer}"
    );
}

/// AC-5: invoke the production Arachne guest and observe the resolved wall
/// sequence at the host boundary for every mode on both the first and a later
/// layer.  The config is deliberately changed per invocation; a boolean or a
/// host-side default would make at least one of these exact sequences fail.
#[test]
fn arachne_wall_sequence_survives_wasm_boundary() {
    for &(layer_index, mode) in &[
        (0, "InnerOuter"),
        (3, "InnerOuter"),
        (0, "OuterInner"),
        (3, "OuterInner"),
        (0, "InnerOuterInner"),
        (3, "InnerOuterInner"),
    ] {
        HOST_ARACHNE_WALL_SEQUENCE_CAPTURE
            .lock()
            .expect("Arachne wall-sequence capture mutex poisoned")
            .clear();
        run_real_arachne_guest(layer_index, mode);
        let captured = HOST_ARACHNE_WALL_SEQUENCE_CAPTURE
            .lock()
            .expect("Arachne wall-sequence capture mutex poisoned")
            .clone();
        let expected_wit = match mode {
            "InnerOuter" => slicer_wasm_host::host::layer::slicer::common::host_services::WallSequence::InnerOuter,
            "OuterInner" => slicer_wasm_host::host::layer::slicer::common::host_services::WallSequence::OuterInner,
            "InnerOuterInner" => slicer_wasm_host::host::layer::slicer::common::host_services::WallSequence::InnerOuterInner,
            _ => unreachable!("test case has an unsupported wall sequence"),
        };
        assert_eq!(
            captured,
            vec![expected_wit],
            "AC-5: decoded host input for {mode} at layer {layer_index}"
        );
    }
}

fn run_real_arachne_guest(layer_index: u32, wall_sequence: &str) {
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules/arachne-perimeters/arachne-perimeters.wasm");
    assert!(
        wasm_path.exists(),
        "real arachne guest is missing: {}",
        wasm_path.display()
    );

    let engine = crate::common::wasm_cache::shared_engine();
    let component = crate::common::wasm_cache::compiled_component_at(&wasm_path);
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let loaded = LoadedModuleBuilder::new(
        "com.core.arachne-perimeters",
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::Perimeters",
        "slicer:world-layer@1.0.0",
        wasm_path,
    )
    .ir_reads(vec!["SliceIR".to_string(), "PaintRegionIR".to_string()])
    .ir_writes(vec!["PerimeterIR".to_string()])
    .claims(vec!["perimeter-generator".to_string()])
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 5,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .unwrap(),
    );
    let mut config = HashMap::new();
    config.insert(
        "wall_sequence".to_string(),
        ConfigValue::String(wall_sequence.to_string()),
    );
    let bundle = crate::common::TestModuleBundle {
        module: CompiledModuleBuilder::new(loaded.id().to_string())
            .config_view(Arc::new(slicer_ir::ConfigView::from_map(config)))
            .build(),
        pool,
        component: Some(component),
    };
    let side = slicer_ir::mm_to_units(10.0);
    let region = SlicedRegion {
        object_id: "obj-a".to_string(),
        region_id: 0,
        polygons: vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 { x: side, y: 0 },
                    Point2 { x: side, y: side },
                    Point2 { x: 0, y: side },
                ],
            },
            holes: Vec::new(),
        }],
        infill_areas: Vec::new(),
        nonplanar_surface: None,
        effective_layer_height: 0.2,
        segment_annotations: HashMap::new(),
        variant_chain: Vec::new(),
        top_shell_index: None,
        bottom_shell_index: None,
        top_solid_fill: Vec::new(),
        bottom_solid_fill: Vec::new(),
        is_bridge: false,
        bridge_areas: Vec::new(),
        bridge_orientation_deg: 0.0,
        sparse_infill_area: Vec::new(),
    };
    let mut arena = LayerArena::new();
    arena
        .set_slice(SliceIR {
            global_layer_index: layer_index,
            z: 0.2,
            regions: vec![region],
            ..Default::default()
        })
        .unwrap();
    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let layer = slicer_ir::GlobalLayer {
        index: layer_index,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    crate::common::run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::Perimeters",
        &layer,
        &bundle,
        &blackboard,
        &mut arena,
    )
    .unwrap();
}

// ===========================================================================
// G15 — BeadingStrategy::get_split_middle_threshold exposed and consumed by
// RedistributeBeadingStrategy.
// ===========================================================================

/// G15 (TDD-red → closed): `BeadingStrategy::get_split_middle_threshold` and
/// `get_add_middle_threshold` must exist on the `BeadingStrategy` trait and be
/// observable on the `Limited` top of a fully-decorated stack built by
/// `BeadingStrategyFactory::create_stack`.
///
/// OrcaSlicer ref: `BeadingStrategy.hpp:97`
/// (`getSplitMiddleThreshold(lower_bead_count)`); `BeadingStrategy.cpp:54-57`
/// (consumed by `RedistributeBeadingStrategy` to pick the optimal bead count).
#[test]
fn arachne_parity_beading_split_middle_threshold_exposed() {
    // AC-2: G15. The factory-computed thresholds must be observable on the
    // `Limited` top of a fully-decorated stack. The previous `assert!(false)`
    // body is replaced per the test's own doc note at lines 120-132 of this
    // file.
    let params = BeadingFactoryParams {
        print_thin_walls: true,
        outer_wall_offset: 1.0,
        ..BeadingFactoryParams::default()
    };
    let stack = BeadingStrategyFactory::create_stack(&params);

    let split = stack.get_split_middle_threshold();
    let add = stack.get_add_middle_threshold();
    assert_eq!(
        split, 0.99,
        "AC-2 G15: get_split_middle_threshold on Limited top must equal factory-computed 0.99"
    );
    assert_eq!(
        add, 0.99,
        "AC-2 G15: get_add_middle_threshold on Limited top must equal factory-computed 0.99"
    );
}

// ===========================================================================
// G20 — simplify: intersection-distance gate preserves near-colinear middle
// junctions whose chord-intersection lies too far from neighbors.
// ===========================================================================

/// G20: build an `ExtrusionLine` from
/// `fixtures::simplify_input_intersection_distance_gate()` (a thin "Z" polyline
/// of four junctions) and run `simplify_toolpaths` with parameters that place
/// the middle junctions *inside* the intersection-distance gate. OrcaSlicer's
/// `ExtrusionLine::simplify` rejects removal when the intersection of the
/// extended `(prev, curr)` lines lies more than
/// `smallest_line_segment_squared` from either neighbor, so the middle
/// junctions are PRESERVED (4 junctions remain).
///
/// OrcaSlicer ref: `Arachne/utils/ExtrusionLine.cpp:163-175`.
#[test]
fn arachne_parity_simplify_intersection_distance_gate_present() {
    let line = ExtrusionLine {
        junctions: fixtures::simplify_input_intersection_distance_gate(),
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };
    let expected: Vec<(f64, f64)> = line
        .junctions
        .iter()
        .map(|j| (j.p.x as f64, j.p.y as f64))
        .collect();

    // AC-6: G20. The previous `smallest_line_segment_squared = 0.0` made the
    // tier-3 gate (`ExtrusionLine.cpp:162-164`) reduce to `length2 < 0` —
    // unsatisfiable for every input — so the intersection/`dist_greater` path
    // (`:166-220`) was dead and the old test could not have exercised the gate
    // it names. The new parameters place junction 2 inside the gate. **The
    // assertion is strengthened, never weakened.**
    let result = simplify_toolpaths(vec![line], 0.01, 1e-3, 1.0, f64::INFINITY);

    assert!(
        !result.is_empty(),
        "AC-6 G20: simplify must return at least one ExtrusionLine"
    );

    let kept = result[0].junctions.len();
    assert_eq!(
        kept, 4,
        "AC-6 G20: intersection-distance gate must preserve all 4 junctions; observed {kept}"
    );

    // Exact junction-sequence check: the four original junctions must be
    // preserved unchanged (the middle two survive the dist_greater gate).
    let got: Vec<(f64, f64)> = result[0]
        .junctions
        .iter()
        .map(|j| (j.p.x as f64, j.p.y as f64))
        .collect();
    assert_eq!(
        got, expected,
        "AC-6 G20: preserved junction sequence must exactly match the fixture input"
    );

    // Touch Point2 so the import is meaningful for coordinate hygiene.
    let _ = Point2::from_mm(0.0, 0.0);
}
