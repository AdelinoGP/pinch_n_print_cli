//! Contract: a guest-authored `ExtrusionPath3D.tool_index` is honored on
//! infill paths when — and only when — it names a configured tool.
//!
//! Rule under test (packet `226-authored-coloring-carrier`, revised design):
//!
//! > `Some(t)` with `t < SupportToolSelection::tool_count` wins outright over
//! > every host-side resolver. `None`, or `Some(t)` with `t >= tool_count`,
//! > leaves host resolution exactly as it was. Silent, deterministic, never an
//! > error — the same trust model `speed_factor` already has.
//!
//! These tests drive the real `assemble_ordered_entities` through the public
//! `execute_per_layer_with_events_and_support_tools` entry point rather than
//! reimplementing the precedence chain, so a regression in the resolver is
//! caught here.
//!
//! Each case is made falsifying by staging a *painted* `SlicedRegion` whose
//! `variant_chain` names tool 3. That is the value the host resolves on its
//! own, so case 1 (authored `Some(1)` → 1) can only pass if the authored value
//! actually beat the region default, and cases 2 and 3 pin the exact fallback.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use slicer_ir::LayerStageCommit;
use slicer_ir::{
    ActiveRegion, ConfigValue, ConfigView, ExPolygon, GlobalLayer, PaintValue, Point2,
    Point3WithWidth, Polygon, ResolvedConfig, SemVer, SliceIR, SlicedRegion, StageId,
};
use slicer_runtime::layer_executor::{
    execute_per_layer_with_events_and_support_tools, SupportToolSelection,
};
use slicer_runtime::{
    Blackboard, CompiledModule, CompiledModuleBuilder, CompiledModuleLive, CompiledStage,
    ExecutionPlan, IrAccessMask, LayerStageError, LayerStageInput, LayerStageRunner,
    LoadedModuleBuilder, NoopLayerProgressSink,
};

/// Object the fixture mesh and the staged IR agree on.
const OBJECT_ID: &str = "test-object";
/// Region id shared by the staged `SlicedRegion` and the staged `InfillRegion`.
const REGION_ID: u64 = 7;
/// Tool the painted `SlicedRegion.variant_chain` carries. This is what the
/// host resolves on its own, i.e. the value an authored index must beat.
const REGION_DEFAULT_TOOL: u32 = 3;
/// Number of configured tools for these runs. Indices 0..=3 are valid.
const TOOL_COUNT: u32 = 4;

// ── Scaffolding ────────────────────────────────────────────────────────────

fn semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

/// Runner that hands a pre-made `InfillIR` to `Layer::Infill` so the executor
/// assembles entities from it. Deliberately dumb: it stages bytes, it does not
/// resolve tools.
struct InfillStagingRunner {
    infill: Mutex<Option<slicer_ir::InfillIR>>,
}

impl LayerStageRunner for InfillStagingRunner {
    fn run_stage(
        &self,
        stage_id: &StageId,
        _layer: &GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        match stage_id.as_str() {
            "Layer::Infill" => Ok(self
                .infill
                .lock()
                .unwrap()
                .take()
                .map(LayerStageCommit::Infill)),
            _ => Ok(None),
        }
    }
}

fn compiled_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModuleBuilder::new(
        module_id,
        semver(),
        stage_id,
        slicer_schema::TIER_LAYER,
        PathBuf::from(format!("fixtures/{module_id}.wasm")),
    )
    .ir_reads(vec![String::from("SliceIR.regions")])
    .ir_writes(vec![String::from("InfillIR.paths")])
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(semver())
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();

    CompiledModuleBuilder::new(loaded.id().to_string())
        .ir_read_mask(IrAccessMask {
            paths: loaded.ir_reads().to_vec(),
        })
        .ir_write_mask(IrAccessMask {
            paths: loaded.ir_writes().to_vec(),
        })
        .config_view(Arc::new(ConfigView::from_map(HashMap::from([(
            String::from("fixture.enabled"),
            ConfigValue::Bool(true),
        )]))))
        .build()
}

fn square(size: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            // Point2 is in 100 nm units; build from mm.
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(size, 0.0),
                Point2::from_mm(size, size),
                Point2::from_mm(0.0, size),
            ],
        },
        holes: Vec::new(),
    }
}

/// A `SliceIR` for layer 0 holding one PAINTED region on `REGION_DEFAULT_TOOL`.
/// `assemble_ordered_entities` builds `variant_tool_by_region` from exactly
/// this `variant_chain`, so it is the host's own answer for the staged infill.
fn painted_slice_ir() -> SliceIR {
    SliceIR {
        global_layer_index: 0,
        regions: vec![SlicedRegion {
            object_id: OBJECT_ID.into(),
            region_id: REGION_ID,
            polygons: vec![square(10.0)],
            infill_areas: vec![square(10.0)],
            effective_layer_height: 0.2,
            variant_chain: vec![(
                String::from("material"),
                PaintValue::ToolIndex(REGION_DEFAULT_TOOL),
            )],
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn infill_ir(tool_index: Option<u32>) -> slicer_ir::InfillIR {
    slicer_ir::InfillIR {
        schema_version: semver(),
        global_layer_index: 0,
        regions: vec![slicer_ir::InfillRegion {
            object_id: OBJECT_ID.into(),
            region_id: REGION_ID,
            // exhaustive: tool-index fixture pins the complete IR path
            sparse_infill: vec![slicer_ir::ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: 1.0,
                        y: 1.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                        ..Default::default()
                    },
                    Point3WithWidth {
                        x: 5.0,
                        y: 1.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                        ..Default::default()
                    },
                ],
                role: slicer_ir::ExtrusionRole::SparseInfill,
                speed_factor: 1.0,
                tool_index,
                order_lock: None,
            }],
            ..Default::default()
        }],
    }
}

/// Run one layer with a single staged infill path carrying `tool_index`, and
/// return the `PrintEntity.tool_index` the executor committed for it.
fn committed_tool_index(tool_index: Option<u32>, tool_count: u32) -> u32 {
    let mesh = crate::common::mesh_fixture(vec![crate::common::flat_plate_object(
        OBJECT_ID,
        0.0,
        crate::common::identity_transform(),
    )]);

    let plan = ExecutionPlan {
        per_layer_stages: vec![CompiledStage {
            stage_id: String::from("Layer::Infill"),
            modules: vec![compiled_module("Layer::Infill", "com.example.infill")],
        }],
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![ActiveRegion {
                object_id: OBJECT_ID.into(),
                region_id: REGION_ID,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: 0.2,
                ..Default::default()
            }],
            ..Default::default()
        }]),
        ..Default::default()
    };

    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);
    // Commit the painted slice directly rather than going through
    // `seed_slice_ir`: the real prepass slicer would emit an UNpainted region,
    // and `variant_tool_by_region` must be non-empty for these cases to say
    // anything.
    blackboard
        .commit_slice_ir(Arc::new(vec![painted_slice_ir()]))
        .expect("commit painted slice_ir");

    let runner = InfillStagingRunner {
        infill: Mutex::new(Some(infill_ir(tool_index))),
    };

    let (layers, _audits) = execute_per_layer_with_events_and_support_tools(
        &plan,
        &blackboard,
        &runner,
        &NoopLayerProgressSink,
        &HashMap::new(),
        SupportToolSelection {
            tool_count,
            ..Default::default()
        },
    )
    .expect("per-layer execution");

    assert_eq!(layers.len(), 1, "exactly one layer");
    let entities = &layers[0].ordered_entities;
    assert_eq!(
        entities.len(),
        1,
        "exactly one infill entity should be assembled; got {entities:?}"
    );
    entities[0].tool_index
}

// ── Cases ──────────────────────────────────────────────────────────────────

/// Case 1: an in-range authored tool is honored and OVERRIDES the host's own
/// per-region answer.
#[test]
fn authored_in_range_tool_index_overrides_region_default() {
    assert_ne!(
        REGION_DEFAULT_TOOL, 1,
        "fixture precondition: the region default must differ from the authored \
         value, otherwise case 1 could pass by coincidence"
    );

    let got = committed_tool_index(Some(1), TOOL_COUNT);

    assert_eq!(
        got, 1,
        "an authored tool_index of Some(1) with tool_count={TOOL_COUNT} must be \
         stamped onto the PrintEntity, overriding the painted region's tool \
         {REGION_DEFAULT_TOOL}; got {got}"
    );
}

/// Case 2: an out-of-range authored tool is ignored, and the host resolves the
/// tool exactly as it would have. Asserted against the exact expected value,
/// not merely `!= 99`.
#[test]
fn authored_out_of_range_tool_index_falls_back_to_host_resolution() {
    let got = committed_tool_index(Some(99), 2);

    assert_eq!(
        got, REGION_DEFAULT_TOOL,
        "an authored tool_index of Some(99) with tool_count=2 is out of range \
         and must be ignored silently, leaving the painted region's tool \
         {REGION_DEFAULT_TOOL}; got {got}"
    );
    assert_ne!(
        got, 99,
        "the out-of-range authored value must never be stamped"
    );
}

/// Case 3: `None` leaves host resolution untouched — the pre-change behaviour.
#[test]
fn absent_authored_tool_index_leaves_host_resolution_unchanged() {
    let got = committed_tool_index(None, TOOL_COUNT);

    assert_eq!(
        got, REGION_DEFAULT_TOOL,
        "tool_index: None must resolve exactly as before the authored-coloring \
         carrier landed — the painted region's tool {REGION_DEFAULT_TOOL}; got {got}"
    );
}

/// Guard on the resolver's range check itself: `tool_count == 1` means only
/// tool 0 exists, so `Some(1)` must fall back even though 1 is a plausible
/// index on a multi-tool machine.
#[test]
fn authored_tool_index_is_range_checked_against_tool_count() {
    let got = committed_tool_index(Some(1), 1);

    assert_eq!(
        got, REGION_DEFAULT_TOOL,
        "with tool_count=1 only tool 0 exists, so Some(1) is out of range and \
         must fall back to {REGION_DEFAULT_TOOL}; got {got}"
    );
}
