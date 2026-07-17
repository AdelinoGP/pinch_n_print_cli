//! Packet 130 — `Layer::InfillPostProcess` boundary contract (ADR-0028
//! §Amendment).
//!
//! Drives the `infill-postprocess-echo-guest` test guest, which:
//! - echoes its `prior-infill` input back through the output builder with
//!   per-region `begin_region` tagging (AC-1), and
//! - when config `emit_view_witness == 1`, emits solid-bucket witness paths
//!   encoding the six enrichment fields of each incoming
//!   `perimeter-region-view` (AC-2/3/4). See the guest's crate docs for the
//!   exact encoding (header marker 777.0, field marker 888.0).

use crate::common::*;
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, GlobalLayer, InfillIR,
    InfillRegion, PaintValue, Point2, Point3WithWidth, Polygon, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SemVer,
};
use slicer_runtime::manifest::LoadedModuleBuilder;
use slicer_runtime::{build_wasm_instance_pool, CompiledModuleBuilder, WasmArtifactMetadata};
use std::collections::HashMap;
use std::sync::Arc;

const HEADER_MARKER: f32 = 777.0;
const FIELD_MARKER: f32 = 888.0;

// ── Fixture helpers ───────────────────────────────────────────────────────

fn make_echo_bundle(config: ConfigView) -> TestModuleBundle {
    let component = wasm_cache::compiled_component_at(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../slicer-wasm-host/test-guests/infill-postprocess-echo-guest.component.wasm"
    )));
    let loaded = LoadedModuleBuilder::new(
        "com.test.infill-echo",
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Layer::InfillPostProcess",
        slicer_schema::WORLD_LAYER,
        std::path::PathBuf::from("/dev/null"),
    )
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
        major: 2,
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
    let module = CompiledModuleBuilder::new("com.test.infill-echo")
        .config_view(Arc::new(config))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn witness_config() -> ConfigView {
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert("emit_view_witness".into(), ConfigValue::Int(1));
    ConfigView::from_map(fields)
}

fn layer_at(index: u32, z: f32) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

fn expoly(contour: &[(i64, i64)], holes: &[&[(i64, i64)]]) -> ExPolygon {
    let mk = |pts: &[(i64, i64)]| Polygon {
        points: pts.iter().map(|&(x, y)| Point2 { x, y }).collect(),
    };
    ExPolygon {
        contour: mk(contour),
        holes: holes.iter().map(|h| mk(h)).collect(),
    }
}

fn path(role: ExtrusionRole, pts: &[(f32, f32, f32)]) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: pts
            .iter()
            .map(|&(x, y, z)| Point3WithWidth {
                x,
                y,
                z,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            })
            .collect(),
        role,
        speed_factor: 1.0,
    }
}

/// Per-region `(sparse, solid, ironing)` path counts keyed by
/// `(object_id, region_id)`.
fn bucket_counts(ir: &InfillIR) -> HashMap<(String, u64), (usize, usize, usize)> {
    ir.regions
        .iter()
        .map(|r| {
            (
                (r.object_id.clone(), r.region_id),
                (r.sparse_infill.len(), r.solid_infill.len(), r.ironing.len()),
            )
        })
        .collect()
}

/// Decoded per-view witness content emitted by the echo guest.
struct ViewWitness {
    tool_index: u32,
    /// `None` encoded as -1.0 by the guest.
    wall_source_region_id: Option<u64>,
    /// `fields[field_id]` = ordered polygons; each polygon =
    /// `(hole_count, flat vertex stream contour-then-holes as (x, y))`.
    fields: [Vec<(usize, Vec<(f32, f32)>)>; 4],
}

fn decode_witness(region: &InfillRegion) -> ViewWitness {
    let mut header: Option<(u32, Option<u64>)> = None;
    let mut fields: [Vec<(usize, Vec<(f32, f32)>)>; 4] = Default::default();
    for p in &region.solid_infill {
        let p0 = p.points.first().expect("witness path has a header point");
        if p0.width == HEADER_MARKER {
            assert!(header.is_none(), "exactly one header per witness region");
            let ws = if p0.y < 0.0 { None } else { Some(p0.y as u64) };
            header = Some((p0.x as u32, ws));
        } else if p0.width == FIELD_MARKER {
            let field_id = p0.x as usize;
            let poly_idx = p0.y as usize;
            let hole_count = p0.flow_factor as usize;
            assert_eq!(
                fields[field_id].len(),
                poly_idx,
                "field {field_id} polygons must arrive in order"
            );
            let verts = p.points[1..].iter().map(|v| (v.x, v.y)).collect();
            fields[field_id].push((hole_count, verts));
        } else {
            panic!(
                "unexpected solid path in witness region (width {})",
                p0.width
            );
        }
    }
    let (tool_index, wall_source_region_id) = header.expect("witness header path present");
    ViewWitness {
        tool_index,
        wall_source_region_id,
        fields,
    }
}

/// Expected flat vertex stream for an `ExPolygon` under the guest encoding.
fn expected_verts(poly: &ExPolygon) -> Vec<(f32, f32)> {
    poly.contour
        .points
        .iter()
        .chain(poly.holes.iter().flat_map(|h| h.points.iter()))
        .map(|p| (p.x as f32, p.y as f32))
        .collect()
}

fn run_echo_postprocess(fx: &mut DispatchFixtureHarness, layer: &GlobalLayer, config: ConfigView) {
    let bundle = make_echo_bundle(config);
    run_layer_and_commit_with_bundle(
        &fx.dispatcher,
        "Layer::InfillPostProcess",
        layer,
        &bundle,
        &fx.blackboard,
        &mut fx.arena,
    )
    .expect("Layer::InfillPostProcess echo dispatch should succeed");
}

type DispatchFixtureHarness = dispatch_fixture::DispatchFixture;

// ── AC-1: prior-infill round-trip ─────────────────────────────────────────

/// A layer where `Layer::Infill` actually emitted paths into `InfillIR`:
/// the echo guest re-emits its `prior-infill` input, and the committed
/// replacement `InfillIR` must carry the SAME per-region bucket counts,
/// keyed by `(object_id, region_id)` — a flat total is not asserted.
#[test]
fn infill_postprocess_prior_ir() {
    let mut fx = dispatch_fixture::for_stage("Layer::Infill")
        .with_slice(
            ir_builders::slice_ir::with_count(1)
                .at_layer(3)
                .at_z(1.0)
                .build(),
        )
        .build();
    let layer = layer_at(3, 1.0);
    fx.run_layer(&layer)
        .expect("Layer::Infill dispatch should succeed");
    let prior = fx
        .arena
        .infill()
        .expect("Layer::Infill must have committed an InfillIR")
        .clone();
    assert!(
        !prior.regions.is_empty(),
        "precondition: Layer::Infill emitted at least one region"
    );
    let prior_counts = bucket_counts(&prior);

    run_echo_postprocess(&mut fx, &layer, ConfigView::from_map(HashMap::new()));

    let committed = fx
        .arena
        .infill()
        .expect("postprocess must commit a replacement InfillIR");
    assert_eq!(
        bucket_counts(committed),
        prior_counts,
        "echoed per-region (sparse, solid, ironing) counts must match the \
         committed InfillIR per (object_id, region_id) key"
    );
}

/// Multi-region, multi-bucket variant: a committed `InfillIR` with two
/// regions carrying distinct sparse/solid/ironing cardinalities must
/// round-trip per-region through the prior-infill parameter and the echo.
#[test]
fn infill_postprocess_prior_ir_multi_region_buckets() {
    let mk = |n: usize, role: ExtrusionRole| -> Vec<ExtrusionPath3D> {
        (0..n)
            .map(|i| {
                path(
                    role.clone(),
                    &[(i as f32, 0.0, 0.0), (i as f32 + 1.0, 2.0, 0.0)],
                )
            })
            .collect()
    };
    let prior = InfillIR {
        global_layer_index: 0,
        regions: vec![
            InfillRegion {
                object_id: "obj-a".into(),
                region_id: 7,
                sparse_infill: mk(2, ExtrusionRole::SparseInfill),
                solid_infill: mk(1, ExtrusionRole::TopSolidInfill),
                ironing: mk(3, ExtrusionRole::Ironing),
            },
            InfillRegion {
                object_id: "obj-b".into(),
                region_id: 9,
                sparse_infill: mk(1, ExtrusionRole::SparseInfill),
                solid_infill: mk(2, ExtrusionRole::TopSolidInfill),
                ironing: Vec::new(),
            },
        ],
        ..Default::default()
    };

    let mut fx = dispatch_fixture::for_stage("Layer::InfillPostProcess").build();
    fx.arena
        .set_infill(prior.clone())
        .expect("stage prior InfillIR");
    let layer = layer_at(0, 0.0);

    run_echo_postprocess(&mut fx, &layer, ConfigView::from_map(HashMap::new()));

    let committed = fx
        .arena
        .infill()
        .expect("postprocess must commit a replacement InfillIR");
    assert_eq!(
        bucket_counts(committed),
        bucket_counts(&prior),
        "per-region keyed bucket counts must survive the echo round-trip"
    );
    // Stronger: the echoed path geometry itself is preserved region-by-region.
    assert_eq!(
        committed.regions, prior.regions,
        "echoed regions must be content-identical to the prior InfillIR regions"
    );
}

// ── AC-2: partitioned fill polygons on the view ───────────────────────────

#[test]
fn infill_postprocess_partitioned_polygons() {
    let sparse = vec![expoly(&[(0, 0), (5_000, 0), (5_000, 5_000)], &[])];
    let top = vec![expoly(
        &[(1_000, 1_000), (2_000, 1_000), (2_000, 2_000)],
        &[],
    )];
    let bottom = vec![
        expoly(&[(0, 0), (100, 0), (100, 100)], &[]),
        expoly(&[(300, 300), (400, 300), (400, 400)], &[]),
    ];
    let bridge = vec![expoly(
        &[(0, 0), (9_000, 0), (9_000, 9_000), (0, 9_000)],
        &[&[(3_000, 3_000), (3_000, 6_000), (6_000, 6_000)]],
    )];

    let mut slice = ir_builders::slice_ir::with_ids(&[("obj-0", 0)]).build();
    {
        let region = &mut slice.regions[0];
        region.sparse_infill_area = sparse.clone();
        region.top_solid_fill = top.clone();
        region.bottom_solid_fill = bottom.clone();
        region.bridge_areas = bridge.clone();
    }

    let mut fx = dispatch_fixture::for_stage("Layer::InfillPostProcess")
        .with_slice(slice)
        .with_perimeter(ir_builders::perimeter_ir::with_ids(&[("obj-0", 0)]).build())
        .build();
    let layer = layer_at(0, 0.0);

    run_echo_postprocess(&mut fx, &layer, witness_config());

    let committed = fx.arena.infill().expect("witness output committed");
    assert_eq!(committed.regions.len(), 1, "one view → one witness region");
    let region = &committed.regions[0];
    assert_eq!(region.object_id, "obj-0");
    assert_eq!(region.region_id, 0);
    let w = decode_witness(region);

    let expected: [&[ExPolygon]; 4] = [&sparse, &top, &bottom, &bridge];
    let names = [
        "sparse_infill_area",
        "top_solid_fill",
        "bottom_solid_fill",
        "bridge_areas",
    ];
    for (field_id, (exp, name)) in expected.iter().zip(names).enumerate() {
        assert_eq!(
            w.fields[field_id].len(),
            exp.len(),
            "{name}: polygon count must match the SliceIR region"
        );
        for (poly_idx, poly) in exp.iter().enumerate() {
            let (hole_count, verts) = &w.fields[field_id][poly_idx];
            assert_eq!(
                *hole_count,
                poly.holes.len(),
                "{name}[{poly_idx}]: hole count must match"
            );
            assert_eq!(
                verts,
                &expected_verts(poly),
                "{name}[{poly_idx}]: vertex data must match the SliceIR region"
            );
        }
    }
}

// ── AC-3: tool-index precedence ───────────────────────────────────────────

#[test]
fn infill_postprocess_tool_index_precedence() {
    // (a) painted material variant → ToolIndex(2) wins.
    // (b) no material variant, RegionMapIR extensions["extruder"] = 1 → 1.
    // (c) neither → 0.
    let mut slice = ir_builders::slice_ir::with_ids(&[
        ("obj-0", 1_000_002), // virtual paint variant of base region 1
        ("obj-0", 0),
        ("obj-1", 5),
    ])
    .build();
    slice.regions[0].variant_chain = vec![("material".to_string(), PaintValue::ToolIndex(2))];

    let mut rm = RegionMapIR::default();
    let mut rc = ResolvedConfig::default();
    rc.extensions
        .insert("extruder".to_string(), ConfigValue::Int(1));
    // When a region-map entry exists for the layer, the dispatcher rebuilds
    // the module's effective config from the region-map's resolved config
    // (filtered by the module's declared keys) — so the witness flag must be
    // present in the interned config too, or the guest never sees it.
    rc.extensions
        .insert("emit_view_witness".to_string(), ConfigValue::Int(1));
    let cfg_id = rm.intern_config(rc);
    rm.entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: "obj-0".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config: cfg_id,
            ..Default::default()
        },
    );

    let mut fx = dispatch_fixture::for_stage("Layer::InfillPostProcess")
        .with_slice(slice)
        .with_perimeter(ir_builders::perimeter_ir::with_ids(&[("obj-0", 0), ("obj-1", 5)]).build())
        .build();
    fx.blackboard
        .commit_region_map(Arc::new(rm))
        .expect("commit region map");
    let layer = layer_at(0, 0.0);

    run_echo_postprocess(&mut fx, &layer, witness_config());

    let committed = fx.arena.infill().expect("witness output committed");
    let by_key: HashMap<(String, u64), &InfillRegion> = committed
        .regions
        .iter()
        .map(|r| ((r.object_id.clone(), r.region_id), r))
        .collect();

    let variant = decode_witness(by_key[&("obj-0".to_string(), 1_000_002)]);
    assert_eq!(
        variant.tool_index, 2,
        "(a) variant_chain material ToolIndex(2) must win"
    );

    let mapped = decode_witness(by_key[&("obj-0".to_string(), 0)]);
    assert_eq!(
        mapped.tool_index, 1,
        "(b) RegionMapIR extensions[\"extruder\"]=1 must apply when no material variant"
    );

    let default = decode_witness(by_key[&("obj-1".to_string(), 5)]);
    assert_eq!(default.tool_index, 0, "(c) neither source → default tool 0");
}

// ── AC-4: wall-source region id ───────────────────────────────────────────

#[test]
fn infill_postprocess_wall_source() {
    // Virtual paint-variant region WITHOUT its own PerimeterIR entry: shares
    // the base region's walls → wall_source_region_id == Some(base).
    // Region WITH its own PerimeterIR entry: owns its walls → None.
    let mut slice = ir_builders::slice_ir::with_ids(&[
        ("obj-0", 2_000_001), // virtual variant of base region 2, no own entry
        ("obj-0", 3),         // has its own PerimeterIR entry
    ])
    .build();
    slice.regions[0].variant_chain = vec![("material".to_string(), PaintValue::ToolIndex(0))];

    let mut fx = dispatch_fixture::for_stage("Layer::InfillPostProcess")
        .with_slice(slice)
        .with_perimeter(ir_builders::perimeter_ir::with_ids(&[("obj-0", 3)]).build())
        .build();
    let layer = layer_at(0, 0.0);

    run_echo_postprocess(&mut fx, &layer, witness_config());

    let committed = fx.arena.infill().expect("witness output committed");
    let by_key: HashMap<(String, u64), &InfillRegion> = committed
        .regions
        .iter()
        .map(|r| ((r.object_id.clone(), r.region_id), r))
        .collect();

    let virtual_variant = decode_witness(by_key[&("obj-0".to_string(), 2_000_001)]);
    assert_eq!(
        virtual_variant.wall_source_region_id,
        Some(2),
        "virtual variant without its own PerimeterIR entry borrows base walls"
    );

    let owner = decode_witness(by_key[&("obj-0".to_string(), 3)]);
    assert_eq!(
        owner.wall_source_region_id, None,
        "region with its own PerimeterIR entry owns its walls"
    );
}

// ── AC-N1: absent module preserves the committed InfillIR ────────────────

#[test]
fn infill_postprocess_absent_module_preserves_infill() {
    let prior = InfillIR {
        global_layer_index: 0,
        regions: vec![InfillRegion {
            object_id: "obj-0".into(),
            region_id: 0,
            sparse_infill: vec![path(
                ExtrusionRole::SparseInfill,
                &[(0.0, 0.0, 0.0), (4.0, 4.0, 0.0)],
            )],
            solid_infill: Vec::new(),
            ironing: vec![path(
                ExtrusionRole::Ironing,
                &[(1.0, 1.0, 0.0), (2.0, 2.0, 0.0)],
            )],
        }],
        ..Default::default()
    };

    // No module component registered at Layer::InfillPostProcess — the
    // MissingComponent graceful-skip path must leave the committed InfillIR
    // byte-identical to the post-Layer::Infill IR.
    let mut fx = dispatch_fixture::for_stage("Layer::InfillPostProcess")
        .no_wasm()
        .build();
    fx.arena
        .set_infill(prior.clone())
        .expect("stage prior InfillIR");
    let before_bytes =
        serde_json::to_vec(fx.arena.infill().unwrap()).expect("serialize prior InfillIR");

    let layer = layer_at(0, 0.0);
    fx.run_layer(&layer)
        .expect("absent module must be a graceful skip, not an error");

    let after = fx
        .arena
        .infill()
        .expect("infill slot must still be populated");
    let after_bytes = serde_json::to_vec(after).expect("serialize committed InfillIR");
    assert_eq!(
        after_bytes, before_bytes,
        "committed InfillIR must be byte-identical when no InfillPostProcess module runs"
    );
    assert_eq!(after, &prior, "structural equality must also hold");
}
