//! Packet 234a AC-N1: dense-region internal-bridge rejection.
//!
//! Fixture-repair rewrite (round 9-B): the original model-driven capture
//! (`cube_cilindrical_modifier.3mf`) collapsed into a single object-wide region
//! (region_id=0, empty variant_chain, density=0.2 everywhere), so per-region
//! gating had nothing distinct to gate on. Instead of relying on that fixture,
//! this test constructs the AC-N1 scenario in-process — a shared-ceiling object
//! whose two side-by-side halves carry DISTINCT resolved `infill_density`
//! (dense >= 0.999 beside sparse) — then drives the host prepass
//! (`commit_shell_classification_builtin`, the same stage `pnp_cli visual-debug`
//! exercises) and asserts the frozen bar:
//!
//! * ZERO internal-bridge qualification above the dense half.
//! * Qualification preserved above the sparse half.
//!
//! The mirror recipe is `modifier_split_subregion_density_tdd` (packet 132): the
//! dense/base region and the sparse modifier sub-region get distinct resolved
//! configs via `stamp_modifier_sub_region_configs`, and the region map is
//! committed before the prepass reads per-region density.

use slicer_core::algos::region_mapping::stamp_modifier_sub_region_configs;
use slicer_ir::{
    ConfigValue, ExPolygon, ModifierScope, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig,
    SliceIR, SlicedRegion, CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_runtime::{commit_shell_classification_builtin, Blackboard};
use std::collections::HashMap;
use std::sync::Arc;

const DENSE_DENSITY: f32 = 1.0; // resolved >= 0.999 → fully dense supporter
const SPARSE_DENSITY: f32 = 0.25; // sparse half

fn square(x0: f32, y0: f32, x1: f32, y1: f32) -> slicer_ir::ExPolygon {
    use slicer_ir::{Point2, Polygon};
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

fn region(object_id: &str, region_id: u64, footprint: slicer_ir::ExPolygon) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id,
        polygons: vec![footprint.clone()],
        infill_areas: vec![footprint],
        effective_layer_height: 0.2,
        ..Default::default()
    }
}

fn region_config(density: f32) -> ResolvedConfig {
    ResolvedConfig {
        infill_density: density,
        top_shell_layers: 2,
        bottom_shell_layers: 0,
        ..Default::default()
    }
}

fn build_region_map(
    object_id: &str,
    base_cfg: &ResolvedConfig,
    sub_cfg: &ResolvedConfig,
    sub_id: u64,
    layer_count: u32,
) -> RegionMapIR {
    let mut map = RegionMapIR::default();
    for (region_id, cfg) in [(0u64, base_cfg), (sub_id, sub_cfg)] {
        let mut resolved = cfg.clone();
        resolved.extensions.insert(
            "bridge_line_width".into(),
            slicer_ir::ConfigValue::Float(0.4),
        );
        let config = map.intern_config(resolved);
        for layer_index in 0..layer_count {
            map.entries.insert(
                RegionKey {
                    global_layer_index: layer_index,
                    object_id: object_id.into(),
                    region_id,
                    variant_chain: Vec::new(),
                },
                RegionPlan {
                    config,
                    ..Default::default()
                },
            );
        }
    }
    map
}

#[test]
fn mixed_density_internal_bridge_rejection_e2e_tdd() {
    let object_id = "obj-0";
    // Mint the modifier sub-region id exactly as the packet-132 split does.
    let sub_id = 7u64;

    // Phase 1 — per-region configs with DISTINCT resolved densities: the base
    // region is fully dense, the modifier sub-region is sparse.
    let mut base_config = region_config(DENSE_DENSITY);
    base_config
        .extensions
        .insert("bridge_line_width".into(), ConfigValue::Float(0.4));
    // exhaustive: fixture explicitly pins every ModifierVolume field
    let modifier_volume = slicer_ir::ModifierVolume {
        id: "mod-dense".into(),
        mesh: slicer_ir::IndexedTriangleSet::default(),
        config_delta: slicer_ir::ConfigDelta {
            fields: HashMap::from([(
                "infill_density".into(),
                ConfigValue::Float(f64::from(SPARSE_DENSITY)),
            )]),
        },
        priority: 0,
        applies_to: ModifierScope::AllFeatures,
    };
    let per_region =
        stamp_modifier_sub_region_configs(base_config.clone(), 0, sub_id, &[modifier_volume]);
    let mut base_cfg = per_region
        .get(&0)
        .expect("base region config must be present")
        .clone();
    let mut sub_cfg = per_region
        .get(&sub_id)
        .expect("sub-region config must be present")
        .clone();
    // The stamp helper routes density through `extensions`; the prepass
    // (`gate_internal_bridge_sites::density_for`) reads the struct field, so
    // pin the resolved struct field explicitly for the fixture.
    base_cfg.infill_density = DENSE_DENSITY;
    sub_cfg.infill_density = SPARSE_DENSITY;
    base_cfg.top_shell_layers = 2;
    base_cfg.bottom_shell_layers = 0;
    sub_cfg.top_shell_layers = 2;
    sub_cfg.bottom_shell_layers = 0;

    let region_map = build_region_map(object_id, &base_cfg, &sub_cfg, sub_id, 4);

    // Phase 2 — shared-ceiling timeline: dense left half beside sparse right
    // half, present on four layers. Layer 3 is the object's true top; layer 2
    // is the shared CEILING whose top surface is propagated down from layer 3
    // (the internal-bridge candidate). Layers 0/1 sit below it (dense left,
    // sparse right), so the ceiling overhangs unsupported sparse fill on the
    // right and supported dense solid on the left.
    let dense_half = square(0.0, 0.0, 10.0, 10.0);
    let sparse_half = square(10.0, 0.0, 20.0, 10.0);
    let slices: Vec<SliceIR> = (0..4)
        .map(|index| SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: index,
            z: 0.2 * (index + 1) as f32,
            regions: vec![
                region(object_id, 0, dense_half.clone()),
                region(object_id, sub_id, sparse_half.clone()),
            ],
        })
        .collect();

    // Phase 3 — commit the timeline + region map and run the prepass.
    let mut blackboard = Blackboard::new(Arc::new(Default::default()), 4);
    blackboard
        .commit_region_map(Arc::new(region_map))
        .expect("commit region map");
    blackboard
        .commit_slice_ir(Arc::new(slices))
        .expect("commit slice IR");
    commit_shell_classification_builtin(&mut blackboard).expect("shell classification");

    let classified = blackboard.slice_ir().expect("classified slices");
    let ceiling = &classified[2];
    let dense_region = ceiling
        .regions
        .iter()
        .find(|r| r.region_id == 0)
        .expect("dense region on ceiling");
    let sparse_region = ceiling
        .regions
        .iter()
        .find(|r| r.region_id == sub_id)
        .expect("sparse region on ceiling");
    let dense_bridges = dense_region.internal_bridge_areas.len();
    let sparse_bridges = sparse_region.internal_bridge_areas.len();
    println!(
        "AC-N1 fixture: regions=[dense(0) density={} bridges={} | sparse({sub_id}) density={} bridges={sparse_bridges}]",
        base_cfg.infill_density, dense_bridges, sub_cfg.infill_density,
    );

    // Frozen bar: ZERO qualification above the dense half; preserved above the
    // sparse half.
    assert_eq!(
        dense_bridges, 0,
        "dense half must reject internal bridges; region 0 qualified {}",
        dense_bridges
    );
    assert!(
        sparse_bridges > 0,
        "sparse half must retain qualified internal bridges; region {sub_id} has {sparse_bridges}"
    );
}
