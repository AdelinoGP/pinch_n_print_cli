#![allow(missing_docs)]

//! TDD test for packet 35a AC-4: `commit_region_mapping_builtin` stamps each
//! `RegionPlan.config` from the per-object `resolved_configs` map.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_config::{
    assemble_registry, ConfigIngestor, ConfigSchemaRegistry, ConfigScope, ExpansionContext,
    HostChannels, LayerRangeInput, ModuleDeclaration, ScopeDelta, ScopedConfig,
};
use slicer_core::algos::region_mapping::RegionResolutionAuthority;
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::{
    is_modifier_namespace_id, ActiveRegion, BoundingBox3, ConfigDelta, ConfigValue, GlobalLayer,
    IndexedTriangleSet, LayerPlanIR, MeshIR, ModifierKind, ModifierVolume, ObjectMesh, Point3,
    RegionKey, RegionMapIR, ResolvedConfig, SemVer, Transform3d,
};
use slicer_runtime::{
    build_execution_plan, commit_region_mapping_builtin, Blackboard, ExecutionPlanRequest,
    LoadDiagnostic, SortedStageModules,
};

// --- helpers ----------------------------------------------------------------

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn minimal_mesh() -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "obj-A".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    }
}

fn active_region(object_id: &str, region_id: u64) -> ActiveRegion {
    ActiveRegion {
        object_id: object_id.to_string(),
        region_id,
        effective_layer_height: 0.2,
        ..Default::default()
    }
}

fn empty_execution_plan() -> slicer_runtime::ExecutionPlan {
    let request = ExecutionPlanRequest {
        sorted_stages: Vec::<SortedStageModules>::new(),
        module_bindings: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    };
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    build_execution_plan(&request, &mut diagnostics).expect("empty execution plan should build")
}

// --- Packet `config-scope-resolution_09` Step 6c-2 fixtures -----------------
//
// These fixtures make `infill_density` an admitted Float key so a typed
// `ScopedConfig` can carry a layer range for `obj-A`, then drive the real
// `commit_region_mapping_builtin` with a `RegionResolutionAuthority`. The
// expectations are literals: no production helper computes them.

fn float_field() -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: "float".to_owned(),
        ..ConfigFieldEntry::default()
    }
}

fn infill_density_registry() -> ConfigSchemaRegistry {
    assemble_registry(
        &[ModuleDeclaration {
            module_id: "dev.pinch.test.region-mapping-layer-range".to_owned(),
            schema: ConfigSchema {
                entries: BTreeMap::from([("infill_density".to_owned(), float_field())]),
            },
            ..ModuleDeclaration::default()
        }],
        &HostChannels::from_parts(Vec::new(), Vec::new(), Vec::new()),
    )
    .expect("layer-range fixture registry must assemble")
    .registry
}

/// One raw-string transport value from the model-IO parser.
fn range_input(
    object_id: &str,
    source_index: u32,
    min_z: f64,
    max_z: f64,
    values: &[(&str, &str)],
) -> LayerRangeInput {
    // exhaustive: this transport fixture pins every LayerRangeInput field explicitly
    LayerRangeInput {
        object_id: object_id.to_owned(),
        source_index,
        min_z,
        max_z,
        values: values
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    }
}

/// Typed scope world for the range fixture: global 0.20, object `obj-A` 0.25,
/// and one `[0.4, 0.8)` range at 0.35 for `obj-A`.
fn range_scope_world(registry: &ConfigSchemaRegistry) -> ScopedConfig {
    let mut ingestor = ConfigIngestor::new(registry);
    ingestor
        .ingest_layer_ranges(&[range_input(
            "obj-A",
            0,
            0.4,
            0.8,
            &[("infill_density", "0.35")],
        )])
        .expect("the admitted layer range must load");
    let mut scoped = ingestor.finish().scoped;
    scoped.deltas.insert(
        ConfigScope::Global,
        ScopeDelta {
            values: BTreeMap::from([("infill_density".to_owned(), ConfigValue::Float(0.20))]),
        },
    );
    scoped.deltas.insert(
        ConfigScope::Object("obj-A".to_owned()),
        ScopeDelta {
            values: BTreeMap::from([("infill_density".to_owned(), ConfigValue::Float(0.25))]),
        },
    );
    scoped
}

/// Commit one `obj-A` layer at world `z` through the real builtin with the
/// layer-range authority attached, and return the committed region map.
fn commit_with_authority(
    registry: &ConfigSchemaRegistry,
    scoped: &ScopedConfig,
    mesh: MeshIR,
    z: f32,
) -> Arc<RegionMapIR> {
    let layer_plan = Arc::new(LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![GlobalLayer {
            index: 0,
            z,
            active_regions: vec![active_region("obj-A", 1)],
            ..Default::default()
        }],
        object_participation: HashMap::new(),
    });

    // The object-only precomputed entry, as if the runtime resolved it without
    // layer context: `obj-A` 0.25 from the object scope.
    let mut resolved_configs: BTreeMap<String, ResolvedConfig> = BTreeMap::new();
    resolved_configs.insert(
        "obj-A".to_string(),
        ResolvedConfig {
            infill_density: 0.25,
            ..ResolvedConfig::default()
        },
    );
    let default_resolved_config = ResolvedConfig::default();

    let mesh = Arc::new(mesh);
    let mut blackboard = Blackboard::new(mesh, 0);
    blackboard
        .commit_layer_plan(Arc::clone(&layer_plan))
        .expect("commit layer plan");

    let plan = empty_execution_plan();
    let expansion = ExpansionContext {
        nozzle_diameter_mm: 0.4,
        ..ExpansionContext::default()
    };

    commit_region_mapping_builtin(
        &plan,
        &mut blackboard,
        &resolved_configs,
        &default_resolved_config,
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeMap::new(),
        Some(RegionResolutionAuthority {
            registry,
            scoped,
            expansion: &expansion,
        }),
    )
    .expect("commit_region_mapping_builtin with layer-range authority must succeed");

    Arc::clone(
        blackboard
            .region_map()
            .expect("RegionMapIR must be committed after builtin runs"),
    )
}

/// Modifier-free variant returning the base region's interned config.
fn commit_and_read_region_config(
    registry: &ConfigSchemaRegistry,
    scoped: &ScopedConfig,
    z: f32,
) -> ResolvedConfig {
    let rm = commit_with_authority(registry, scoped, minimal_mesh(), z);
    let key = RegionKey {
        global_layer_index: 0,
        object_id: "obj-A".to_string(),
        region_id: 1,
        variant_chain: Vec::new(),
    };
    rm.config_for(&key).clone()
}

/// A closed 2×2 mm box spanning `z = 0.0..2.0`, so it has a real cross-section
/// at the layer Zs under test.
fn modifier_box_mesh() -> IndexedTriangleSet {
    let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    IndexedTriangleSet {
        vertices: vec![
            v(2.0, 2.0, 0.0),
            v(4.0, 2.0, 0.0),
            v(4.0, 4.0, 0.0),
            v(2.0, 4.0, 0.0),
            v(2.0, 2.0, 2.0),
            v(4.0, 2.0, 2.0),
            v(4.0, 4.0, 2.0),
            v(2.0, 4.0, 2.0),
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ],
    }
}

// --- AC: per-layer range application through the real builtin --------------

/// A layer whose top Z (`0.6`) is covered by `obj-A`'s authored `[0.4, 0.8)`
/// range takes the range value `0.35`, outranking the precomputed object value
/// `0.25`.
#[test]
fn range_covering_layer_top_z_overrides_region_config() {
    let registry = infill_density_registry();
    let scoped = range_scope_world(&registry);

    let resolved = commit_and_read_region_config(&registry, &scoped, 0.6);

    assert_eq!(
        resolved.infill_density, 0.35,
        "the [0.4, 0.8) range covering z=0.6 must outrank the precomputed object value 0.25"
    );
}

/// A layer outside `[0.4, 0.8)` keeps the precomputed object value `0.25`, and
/// `max_z` itself (`0.8`) is excluded by the half-open interval.
#[test]
fn range_uncovered_layer_keeps_precomputed_config() {
    let registry = infill_density_registry();
    let scoped = range_scope_world(&registry);

    let above = commit_and_read_region_config(&registry, &scoped, 0.9);
    assert_eq!(
        above.infill_density, 0.25,
        "z=0.9 is outside [0.4, 0.8), so the precomputed object value 0.25 survives"
    );

    let at_max_z = commit_and_read_region_config(&registry, &scoped, 0.8);
    assert_eq!(
        at_max_z.infill_density, 0.25,
        "max_z itself is excluded by the half-open interval, so 0.25 survives"
    );
}

/// A modifier-target layer at `z = 0.6`: the modifier scope `0.45` outranks the
/// covering range `0.35`, and the modifier child region carries that value.
#[test]
fn range_covering_layer_top_z_overrides_region_config_at_object_range_modifier_precedence() {
    let registry = infill_density_registry();
    let mut scoped = range_scope_world(&registry);
    scoped.deltas.insert(
        ConfigScope::Modifier {
            object_id: "obj-A".to_owned(),
            modifier_id: "mod-A".to_owned(),
        },
        ScopeDelta {
            values: BTreeMap::from([("infill_density".to_owned(), ConfigValue::Float(0.45))]),
        },
    );

    // The modifier volume must have a non-empty cross-section at z = 0.6 for
    // the kernel to mint its sub-region entry.
    let mut mesh = minimal_mesh();
    mesh.objects[0].modifier_volumes = vec![ModifierVolume::new(
        "mod-A".to_string(),
        modifier_box_mesh(),
        ConfigDelta::default(),
        0,
        ModifierKind::ParameterModifier,
    )];
    let rm = commit_with_authority(&registry, &scoped, mesh, 0.6);

    // The base region keeps the covering-range value, not the modifier value.
    let base_key = RegionKey {
        global_layer_index: 0,
        object_id: "obj-A".to_string(),
        region_id: 1,
        variant_chain: Vec::new(),
    };
    assert_eq!(
        rm.config_for(&base_key).infill_density,
        0.35,
        "the base region must keep the covering range value 0.35, not the modifier value"
    );

    // Exactly one modifier-namespace entry must exist for obj-A at layer 0, and
    // it carries the modifier value 0.45 (modifier outranks the range).
    let modifier_keys: Vec<&RegionKey> = rm
        .entries
        .keys()
        .filter(|key| key.object_id == "obj-A" && is_modifier_namespace_id(key.region_id))
        .collect();
    assert_eq!(
        modifier_keys.len(),
        1,
        "exactly one modifier sub-region entry must be minted for obj-A"
    );
    assert_eq!(
        rm.config_for(modifier_keys[0]).infill_density,
        0.45,
        "the modifier sub-region must carry the modifier scope value 0.45, \
         outranking both the covering range 0.35 and the object value 0.25"
    );
}

// --- AC-4 test --------------------------------------------------------------

#[test]
fn commit_stamps_per_object_resolved_config() {
    // Build a LayerPlanIR with one layer containing two active regions:
    // one on "obj-A" and one on "obj-B".
    let layer_plan = Arc::new(LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![active_region("obj-A", 1), active_region("obj-B", 1)],
            ..Default::default()
        }],
        object_participation: HashMap::new(),
    });

    // Build per-object resolved configs:
    // obj-A.top_shell_layers = 5, obj-B.top_shell_layers = 3.
    let config_a = ResolvedConfig {
        top_shell_layers: 5,
        ..ResolvedConfig::default()
    };

    let config_b = ResolvedConfig {
        top_shell_layers: 3,
        ..ResolvedConfig::default()
    };

    let mut resolved_configs: BTreeMap<String, ResolvedConfig> = BTreeMap::new();
    resolved_configs.insert("obj-A".to_string(), config_a);
    resolved_configs.insert("obj-B".to_string(), config_b);

    let default_resolved_config = ResolvedConfig::default();

    // Build a blackboard and commit the layer plan.
    let mesh = Arc::new(minimal_mesh());
    let mut blackboard = Blackboard::new(mesh, 0);
    blackboard
        .commit_layer_plan(Arc::clone(&layer_plan))
        .expect("commit layer plan");

    let plan = empty_execution_plan();

    // Invoke commit_region_mapping_builtin directly (not via execute_prepass_with_builtins).
    commit_region_mapping_builtin(
        &plan,
        &mut blackboard,
        &resolved_configs,
        &default_resolved_config,
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeMap::new(),
        None,
    )
    .expect("commit_region_mapping_builtin must succeed");

    let rm = blackboard
        .region_map()
        .expect("RegionMapIR must be committed after builtin runs");

    // Exactly two entries.
    assert_eq!(rm.entries.len(), 2, "expected exactly 2 region entries");

    // obj-A entry has top_shell_layers == 5.
    let key_a = RegionKey {
        global_layer_index: 0,
        object_id: "obj-A".to_string(),
        region_id: 1,
        variant_chain: Vec::new(),
    };
    assert!(
        rm.entries.contains_key(&key_a),
        "entry for obj-A must be present"
    );
    let resolved_a = rm.config_for(&key_a);
    assert_eq!(
        resolved_a.top_shell_layers, 5,
        "obj-A region plan must have top_shell_layers=5 from per-object config"
    );

    // obj-B entry has top_shell_layers == 3.
    let key_b = RegionKey {
        global_layer_index: 0,
        object_id: "obj-B".to_string(),
        region_id: 1,
        variant_chain: Vec::new(),
    };
    assert!(
        rm.entries.contains_key(&key_b),
        "entry for obj-B must be present"
    );
    let resolved_b = rm.config_for(&key_b);
    assert_eq!(
        resolved_b.top_shell_layers, 3,
        "obj-B region plan must have top_shell_layers=3 from per-object config"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// P95 W-P1: loader → ResolvedConfig.extensions["extruder"] chain
//
// The 3MF loader (slicer-model-io::loader) rebases OrcaSlicer's 1-indexed
// `extruder` metadata to the runtime's 0-indexed convention before stamping
// `ObjectMesh.config.data["extruder"] = ConfigValue::Int(0)` (for raw
// `extruder=1` in the 3MF). `run.rs` then lifts each `ObjectMesh.config.data`
// entry into a `config_source` key of the form `object_config:<obj>:<key>`,
// which typed scope resolution overlays onto each per-object `ResolvedConfig`.
// Because `extruder` is not a declared
// `ResolvedConfig` field, it must fall through to the `extensions` overflow
// bucket as `ConfigValue::Int(0)`.
//
// This test pins that handoff so a regression in any of the three hops
// (loader rebase / `run.rs` lift / scheduler overlay) is caught here rather
// than only at the gcode-output gate.
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn loader_extruder_int_zero_lands_in_extensions() {
    use slicer_config::resolution::resolve_scope_stack;
    use slicer_config::{ExpansionContext, ResolutionTarget};
    use slicer_scheduler::config_resolution::ingest_resolution_config;
    use slicer_scheduler::ConfigBoundsIndex;

    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    // Simulate the `run.rs` lift of `ObjectMesh.config.data["extruder"] = Int(0)`
    // (which is what the loader stamps for OrcaSlicer-1-indexed `extruder=1`).
    source.insert(
        "object_config:obj-A:extruder".to_string(),
        ConfigValue::Int(0),
    );
    source.insert("nozzle_diameter".to_string(), ConfigValue::Float(0.4));

    let bounds = ConfigBoundsIndex::default();
    let scoped = ingest_resolution_config(&source, &bounds).expect("typed ingestion must succeed");
    let cfg = resolve_scope_stack(
        bounds.registry(),
        &scoped,
        &ResolutionTarget {
            object_id: "obj-A".to_string(),
            ..ResolutionTarget::default()
        },
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
    )
    .expect("per-object resolution must succeed");

    assert_eq!(
        cfg.extensions.get("extruder"),
        Some(&ConfigValue::Int(0)),
        "loader-stamped `Int(0)` for `extruder` must land in `ResolvedConfig.extensions` \
         (Int(0) is meaningful — tool 0 — and must not be dropped or coerced)"
    );
}
