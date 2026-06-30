//! Regression test for the "missing infill across internal painted regions" bug.
//!
//! On `resources/cube_4color.3mf`, the classic-perimeters WASM (and any
//! perimeters guest that calls `set_infill_areas` more than once per
//! layer dispatch) drives the SDK builder's `set_infill_areas` once per
//! region. Prior to the multi-origin bucket drain, every call's origin
//! was `replace`d in a single `PerimeterIR` bucket — only the LAST
//! region's infill survived downstream, dropping the painted-cube's
//! interior infill into other tools.
//!
//! This test pins the marshal contract: `convert_perimeter_output` with
//! `N` distinct (object_id, region_id) tagged `infill_areas` entries
//! MUST produce `N` distinct `PerimeterRegion` entries in the output
//! `PerimeterIR`, each with its own non-empty `infill_areas` list.
//!
//! It does NOT validate the SDK→WIT origin propagation (a separate
//! architectural fix required to wire the SDK call sites through the
//! WIT builder with the right `effective_perimeter_origin`); it only
//! locks down the marshal's "multiple entries → multiple buckets"
//! behaviour, which is the necessary precondition for the upstream
//! fix to take effect.
//!
//! The bug's symptom in the gcode: TMP/PnP shared tmp/pnp_cube_4color.gcode
//! has T1 sparse_infill = 30 (just unretract priming lines) and T3 = 2425
//! on a 4-color cube where OrcaSlicer's golden tmp/orca_cube_4color.gcode
//! has T1 = 1243, T3 = 992 — internal painted regions lose their infill
//! entirely. See .ralph/specs/126_mmu-painted-cube-parity/ for context
//! (the bug is still latent: SDK→WIT origin propagation needs an
//! immediate-forward SDK builder to be fixed at root).
//!
//! See docs/02_ir_schemas.md §IR 2 (SlicedRegion.variant_chain) and
//! docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md for the
//! paint-segmentation rationale that makes this test necessary.

#![allow(missing_docs)]

use slicer_wasm_host::host::layer::slicer::ir_handles::ir_handles::HostPerimeterOutputBuilder;
use slicer_wasm_host::host::{
    convert_perimeter_output, ExPolygon, HostExecutionContextBuilder, Point2, Polygon,
};
use slicer_wasm_host::marshal::OriginId;

fn square(min_x: i64, min_y: i64, max_x: i64, max_y: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: min_x, y: min_y },
                Point2 { x: max_x, y: min_y },
                Point2 { x: max_x, y: max_y },
                Point2 { x: min_x, y: max_y },
            ],
        },
        holes: Vec::new(),
    }
}

const TEST_UUID: &str = "uuid-painted-cube-regression";

/// AC: marshal distributes MULTIPLE `set_infill_areas` calls (one per
/// painted region) into MULTIPLE `PerimeterRegion` buckets, one per
/// (object_id, region_id) origin.
#[test]
fn infill_areas_routes_per_call_to_distinct_origins() {
    let mut ctx = HostExecutionContextBuilder::new("com.test.painted-infill", 0.0, 0.2).build();

    // Five painted variant regions (BASE unpainted + four paint colors).
    let regions: Vec<(u64, ExPolygon)> = vec![
        (0, square(100, 100, 200, 200)),         // BASE
        (1_000_001, square(110, 110, 190, 190)), // orange
        (1_000_002, square(120, 120, 180, 180)), // green
        (1_000_003, square(130, 130, 170, 170)), // blue
        (1_000_004, square(140, 140, 160, 160)), // red
    ];

    for (region_id, polys) in &regions {
        // Touch the slice region first so `effective_perimeter_origin()`
        // returns the right tagged origin for the WIT push (the set
        // itself only records it; the actual WIT-level bucket is
        // populated in `convert_perimeter_output`).
        ctx.set_current_slice_region(Some(OriginId {
            object_id: TEST_UUID.to_string(),
            region_id: *region_id,
        }));

        let handle = ctx
            .push_perimeter_output_builder()
            .expect("push_perimeter_output_builder");
        let result = <slicer_wasm_host::host::HostExecutionContext as HostPerimeterOutputBuilder>::set_infill_areas(
            &mut ctx,
            handle,
            vec![polys.clone()],
        )
        .expect("set_infill_areas host call must succeed");
        assert!(
            result.is_ok(),
            "set_infill_areas region={region_id}: {result:?}"
        );
    }

    let perimeter_ir = convert_perimeter_output(ctx.perimeter_output(), 0)
        .expect("convert_perimeter_output must succeed");

    assert_eq!(
        perimeter_ir.regions.len(),
        regions.len(),
        "every set_infill_areas origin must produce its own PerimeterRegion; \
         got {} regions, expected {} — pre-fix every entry collapsed to one \
         bucket (the LIFO-touch SDK bug). With this marshal fix the entries \
         survive the round trip and the per-region infill downstream can see \
         them.",
        perimeter_ir.regions.len(),
        regions.len()
    );

    for (idx, (expected_rid, _)) in regions.iter().enumerate() {
        let region = &perimeter_ir.regions[idx];
        assert_eq!(
            region.object_id, TEST_UUID,
            "region[{idx}] object_id must match the painted origin"
        );
        assert_eq!(
            region.region_id, *expected_rid,
            "region[{idx}] region_id must match the painted origin"
        );
        assert_eq!(
            region.infill_areas.len(),
            1,
            "region[{idx}] (region_id={expected_rid}) must have non-empty \
             infill_areas; got {} (regression: pre-fix only the LAST origin's \
             infill_areas survived, so the painted-cube's interior infill \
             collapsed into the last-processed tool)",
            region.infill_areas.len()
        );
    }
}
