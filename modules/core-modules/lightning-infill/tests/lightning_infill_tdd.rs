//! TDD tests for the lightning-infill module.
//!
//! Step 3 (packet 140) - classification:
//! KEEP:   from_config_defaults - default config values are stable.
//! KEEP:   from_config_custom - configured module values are read.
//! KEEP:   paths_have_sparse_infill_role - emitted paths retain SparseInfill role tagging.
//! KEEP:   empty_regions_no_output - an empty region emits no paths.
//! KEEP:   paths_at_correct_z - emitted points retain the region's layer height.
//! KEEP:   width_matches_config - emitted points retain the configured line width.
//! ADAPT:  square_region_produces_paths - assert emission from a prepared piece.
//! DELETE: zero_density_no_paths - encoded the deleted stub's density gate.
//! DELETE: branching_pattern_present - encoded deleted branch construction.
//! DELETE: density_affects_coverage - encoded deleted grid sampling.
//! DELETE: interior_first_growth - encoded deleted interior sampling.
//!
//! Step 8a: the ordinary stage reads its own layer's private piece, so every test
//! installs a plan scope through `with_prepared_plan_view`. The encoder below is a
//! **second implementation** of the D3 wire layout, written against the design's
//! table rather than against the module's codec (which is `pub(crate)`): the two
//! implementations agreeing is part of what these tests prove.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{ConfigValue, ConfigView, ExtrusionRole, PaintValue, Point2};
use slicer_schema::preparation::PreparationTransportError;
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::preparation::{with_prepared_plan_view, PreparedPieceRead, PreparedPlanRead};
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

use lightning_infill::LightningInfill;

fn empty_paint_view() -> PaintRegionLayerView {
    PaintRegionLayerView::new(0)
}

/// Exact ordered source chain shared by the square region fixture
/// (`make_square_region`) and its matching plan entry. The chainless sibling in
/// `plan_with_segments` carries the same `(object_id, region_id, layer)` numbers
/// with different segments, so the count and coordinate assertions below also
/// falsify any pair-only lookup in the consumer.
fn fixture_variant_chain() -> Vec<(String, PaintValue)> {
    vec![("fuzzy_skin".to_string(), PaintValue::Flag(true))]
}

// ---------------------------------------------------------------------------
// Independent D3 wire encoder (test-owned)
// ---------------------------------------------------------------------------

/// One plan entry the test encodes itself.
struct TestEntry {
    object_id: &'static str,
    region_id: u64,
    layer: i32,
    chain: Vec<(&'static str, PaintValue)>,
    segments: Vec<[Point2; 2]>,
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_string(out: &mut Vec<u8>, text: &str) {
    push_u32(out, text.len() as u32);
    out.extend_from_slice(text.as_bytes());
}

fn push_paint(out: &mut Vec<u8>, value: &PaintValue) {
    match value {
        PaintValue::Flag(flag) => {
            out.push(0);
            out.push(u8::from(*flag));
        }
        PaintValue::Scalar(scalar) => {
            out.push(1);
            push_u32(out, scalar.to_bits());
        }
        PaintValue::ToolIndex(index) => {
            out.push(2);
            push_u32(out, *index);
        }
        PaintValue::Custom(text) => {
            out.push(3);
            push_string(out, text);
        }
    }
}

/// Encode one layer's piece exactly as D3 specifies, from the design's table.
fn encode_piece(entries: &[TestEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"LTPL");
    out.extend_from_slice(&1_u16.to_le_bytes());
    push_u32(&mut out, entries.len() as u32);
    for entry in entries {
        push_string(&mut out, entry.object_id);
        out.extend_from_slice(&entry.region_id.to_le_bytes());
        push_u32(&mut out, entry.layer as u32);
        push_u32(&mut out, entry.chain.len() as u32);
        for (key, value) in &entry.chain {
            push_string(&mut out, key);
            push_paint(&mut out, value);
        }
        push_u32(&mut out, entry.segments.len() as u32);
        for [start, end] in &entry.segments {
            for point in [start, end] {
                out.extend_from_slice(&point.x.to_le_bytes());
                out.extend_from_slice(&point.y.to_le_bytes());
            }
        }
    }
    out
}

/// In-memory prepared plan: piece name -> bytes.
#[derive(Clone, Default)]
struct MemoryPlan {
    pieces: BTreeMap<String, Vec<u8>>,
}

impl MemoryPlan {
    /// One layer's piece, named with the module's own `lightning-layer-{index}`
    /// spelling written as a literal here (the module's name helper is private).
    fn with_layer(layer: i32, entries: &[TestEntry]) -> Self {
        let mut pieces = BTreeMap::new();
        pieces.insert(format!("lightning-layer-{layer}"), encode_piece(entries));
        Self { pieces }
    }
}

struct MemoryPiece {
    bytes: Vec<u8>,
}

impl PreparedPieceRead for MemoryPiece {
    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }

    fn read(&self, offset: u64, max_bytes: u64) -> Result<Vec<u8>, PreparationTransportError> {
        let start = usize::try_from(offset).map_err(|_| PreparationTransportError::RangeError)?;
        if start > self.bytes.len() {
            return Err(PreparationTransportError::RangeError);
        }
        let take = usize::try_from(max_bytes.min((self.bytes.len() - start) as u64))
            .map_err(|_| PreparationTransportError::RangeError)?;
        Ok(self.bytes[start..start + take].to_vec())
    }
}

impl PreparedPlanRead for MemoryPlan {
    fn open(
        &self,
        name: &str,
    ) -> Result<Option<Arc<dyn PreparedPieceRead + Send + Sync>>, PreparationTransportError> {
        Ok(self.pieces.get(name).map(|bytes| {
            Arc::new(MemoryPiece {
                bytes: bytes.clone(),
            }) as Arc<dyn PreparedPieceRead + Send + Sync>
        }))
    }
}

/// Run one ordinary-stage call under a call-local plan scope.
fn run_with_plan<T>(plan: &MemoryPlan, body: impl FnOnce() -> T) -> T {
    with_prepared_plan_view(Arc::new(plan.clone()), body)
}

/// The fixture's plan: one chain-bearing entry with `segments`, plus a chainless
/// sibling with the same source numbers and different segments.
fn plan_with_segments(segments: Vec<[Point2; 2]>) -> MemoryPlan {
    MemoryPlan::with_layer(
        0,
        &[
            TestEntry {
                object_id: "obj1",
                region_id: 1,
                layer: 0,
                chain: vec![("fuzzy_skin", PaintValue::Flag(true))],
                segments,
            },
            TestEntry {
                object_id: "obj1",
                region_id: 1,
                layer: 0,
                chain: Vec::new(),
                segments: vec![[
                    Point2::from_mm(90.0, 90.0),
                    Point2::from_mm(91.0, 91.0),
                ]],
            },
        ],
    )
}

fn make_config(density: f64, speed: f64, line_width: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .float("infill_density", density)
        .float("infill_speed", speed)
        // Packet 06 (AC-3): `bridge_line_width` / `initial_layer_line_width`
        // are contract-required reads (`require_abs_value` over the fixed
        // 0.4 mm nozzle base); a bound view always holds them at their
        // resolved default (0.4 at the module's nozzle base).
        .float("bridge_line_width", 0.4)
        .float("initial_layer_line_width", 0.4)
        .float("line_width", line_width)
        .build()
}

fn make_square_region(size_mm: f32, z: f32) -> SliceRegionView {
    // Post-host-partition fixture: populate `sparse_infill_area` so lightning's
    // sparse-fill emission has its canonical polygon (see
    // `crates/slicer-runtime/src/region_partition.rs`).
    let sq = square_polygon(0.0, 0.0, size_mm);
    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .z(z)
        .add_polygon(sq.clone())
        .sparse_infill_area(vec![sq])
        .build();
    // Lightning manifest declares only `claim:sparse-fill`; set held_claims
    // so should_emit gates correctly (empty held_claims = emit nothing).
    region.set_held_claims(vec!["claim:sparse-fill".into()]);
    // Arm the region with the same exact ordered chain as the fixture's
    // chain-bearing plan entry, so the exact source lookup selects that entry
    // and not the same-number chainless sibling.
    region.set_variant_chain(fixture_variant_chain());
    region
}

fn sample_segments() -> Vec<[Point2; 2]> {
    vec![[Point2::from_mm(1.0, 2.0), Point2::from_mm(3.0, 4.0)]]
}

/// Test 1: Default config values when no fields provided.
#[test]
fn from_config_defaults() {
    // Bound-view fixture (mirrors the `#[cfg(test)] seeded_config` in
    // `modules/core-modules/lightning-infill/src/lib.rs`):
    // Packet 06 (AC-3): `bridge_line_width` / `initial_layer_line_width`
    // are contract-required reads (`require_abs_value` over the fixed
    // 0.4 mm nozzle base); a bound view always holds them at their
    // resolved default (0.4 at the module's nozzle base).
    let mut values = HashMap::new();
    values.insert("bridge_line_width".into(), ConfigValue::Float(0.4));
    values.insert("initial_layer_line_width".into(), ConfigValue::Float(0.4));
    // Packet 04 (TASK-565): the host expands the line_width auto-0
    // sentinel (1.125 × nozzle_diameter) before guests see the view, so
    // a bound view already holds the expanded width (0.45 at the
    // module's fixed 0.4 mm nozzle). Seed the bound value; the
    // production auto-expansion lives host-side now.
    values.insert("line_width".into(), ConfigValue::Float(0.45));
    // Declared keys (lightning-infill.toml) seeded at their manifest
    // defaults, mirroring the production `seed_registry_defaults` step.
    values.insert("infill_density".into(), ConfigValue::Float(0.2));
    values.insert("infill_angle".into(), ConfigValue::Float(45.0));
    values.insert("infill_speed".into(), ConfigValue::Float(60.0));
    let config = ConfigView::from_map(values);
    let module = LightningInfill::from_config(&config).unwrap();
    assert!((module.density() - 0.2).abs() < 0.001);
    // Packet 185 (AC-5): absent line_width resolves to the canonical auto
    // width 1.125 × nozzle_diameter (0.45 at the fixed 0.4 mm nozzle).
    assert!((module.line_width() - 0.45).abs() < 0.001);
}

/// Test 2: Custom config values are read correctly.
#[test]
fn from_config_custom() {
    let config = make_config(0.3, 80.0, 0.5);
    let module = LightningInfill::from_config(&config).unwrap();
    assert!((module.density() - 0.3).abs() < 0.001);
    assert!((module.line_width() - 0.5).abs() < 0.001);
}

/// Test 3: prepared piece segments become sparse paths.
#[test]
fn square_region_produces_paths() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::from_config(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();
    let plan = plan_with_segments(sample_segments());

    let result = run_with_plan(&plan, || {
        module.run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &config,
        )
    });
    result.unwrap();

    assert_eq!(output.sparse_paths().len(), 1);
}

/// Test 4: All paths have SparseInfill extrusion role.
#[test]
fn paths_have_sparse_infill_role() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::from_config(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();
    let plan = plan_with_segments(sample_segments());

    let result = run_with_plan(&plan, || {
        module.run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &config,
        )
    });
    result.unwrap();

    assert_eq!(output.sparse_paths().len(), 1);
    assert_eq!(output.sparse_paths()[0].role, ExtrusionRole::SparseInfill);
}

/// Test 5: Empty regions produce no output (but the layer's piece is still required).
#[test]
fn empty_regions_no_output() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::from_config(&config).unwrap();

    let mut region = SliceRegionView::default();
    region.set_object_id("obj1".to_string());
    region.set_region_id(1);
    region.set_polygons(vec![]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.3);
    region.set_has_nonplanar(false);

    let mut output = InfillOutputBuilder::new();
    // A layer with no trees still publishes its (header-only) piece.
    let plan = MemoryPlan::with_layer(0, &[]);
    let result = run_with_plan(&plan, || {
        module.run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &config,
        )
    });
    result.unwrap();

    assert_eq!(output.sparse_paths().len(), 0);
}

/// Test 6: All output points have the correct z value.
#[test]
fn paths_at_correct_z() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::from_config(&config).unwrap();
    let z = 1.5;
    let region = make_square_region(10.0, z);
    let mut output = InfillOutputBuilder::new();
    let plan = plan_with_segments(sample_segments());

    let result = run_with_plan(&plan, || {
        module.run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &config,
        )
    });
    result.unwrap();

    assert_eq!(output.sparse_paths().len(), 1);
    for point in &output.sparse_paths()[0].points {
        assert!((point.z - z).abs() < 0.001);
    }
}

/// Test 7: All point widths match configured line_width.
#[test]
fn width_matches_config() {
    let line_width = 0.6;
    let config = make_config(0.2, 50.0, line_width);
    let module = LightningInfill::from_config(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();
    let plan = plan_with_segments(sample_segments());

    let result = run_with_plan(&plan, || {
        module.run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &config,
        )
    });
    result.unwrap();

    assert_eq!(output.sparse_paths().len(), 1);
    for point in &output.sparse_paths()[0].points {
        assert!((point.width - line_width as f32).abs() < 0.001);
    }
}

/// AC-1: emit each prepared piece segment as one raw SparseInfill path.
#[test]
fn samples_prepared_piece_raw_emit() {
    let config = make_config(0.2, 80.0, 0.4);
    let module = LightningInfill::from_config(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let expected = vec![
        [Point2::from_mm(1.0, 2.0), Point2::from_mm(3.0, 4.0)],
        [Point2::from_mm(-2.0, 1.5), Point2::from_mm(0.0, 5.0)],
    ];
    let plan = plan_with_segments(expected.clone());
    let mut output = InfillOutputBuilder::new();

    let result = run_with_plan(&plan, || {
        module.run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &config,
        )
    });
    result.unwrap();

    let paths = output.sparse_paths();
    assert_eq!(paths.len(), expected.len());
    for (path, segment) in paths.iter().zip(expected.iter()) {
        assert_eq!(path.points.len(), 2);
        let start = &path.points[0];
        let end = &path.points[1];
        assert!((start.x - slicer_ir::units_to_mm(segment[0].x)).abs() < 0.001);
        assert!((start.y - slicer_ir::units_to_mm(segment[0].y)).abs() < 0.001);
        assert!((end.x - slicer_ir::units_to_mm(segment[1].x)).abs() < 0.001);
        assert!((end.y - slicer_ir::units_to_mm(segment[1].y)).abs() < 0.001);
        assert_eq!(path.role, ExtrusionRole::SparseInfill);
        assert!((path.speed_factor - 1.6).abs() < 0.001);
    }
}

/// AC-N2: a piece with no entries is a successful no-op.
#[test]
fn empty_piece_emits_nothing() {
    let config = make_config(0.2, 80.0, 0.4);
    let module = LightningInfill::from_config(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let plan = plan_with_segments(Vec::new());
    let mut output = InfillOutputBuilder::new();

    let result = run_with_plan(&plan, || {
        module.run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &config,
        )
    });

    assert!(result.is_ok());
    assert_eq!(output.sparse_paths().len(), 0);
}

/// A layer with no published piece is the inherited `57401`, not a silent empty
/// layer: the consumer must not degrade when its plan is absent.
#[test]
fn missing_piece_is_fatal_57401() {
    let config = make_config(0.2, 80.0, 0.4);
    let module = LightningInfill::from_config(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let plan = MemoryPlan::default();
    let mut output = InfillOutputBuilder::new();

    let error = run_with_plan(&plan, || {
        module.run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &config,
        )
    })
    .expect_err("a missing piece must fail the layer");

    assert_eq!(error.code, 57401);
    assert!(error.fatal);
    assert_eq!(
        error.message,
        "required preparation piece missing: lightning-layer-0"
    );
    assert_eq!(output.sparse_paths().len(), 0);
}
