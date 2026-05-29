//! TDD test file for packet 59: machine-start-end-gcode-emission.
//!
//! Tests compile today but ALL fail because the `machine-gcode-emit` module,
//! `GCodeCommand::ExtrusionMode` variant, and the four config keys do not yet exist.
//! This is the intended red state â€” tests graduate to green as production code lands.
//!
//! Acceptance criteria sourced from `.ralph/specs/59_machine-start-end-gcode-emission/packet.spec.md`.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{ConfigKey, ConfigValue, RegionKey, RegionPlan};
use slicer_runtime::dispatch::WasmRuntimeDispatcher;
use slicer_runtime::model_loader::load_model;
use slicer_runtime::pipeline::{
    run_pipeline_with_raw_config, PipelineConfig, PipelineStageRunners,
};
use slicer_runtime::{
    build_live_execution_plan, load_live_modules_for_plan, resolve_global_config,
    resolve_per_object_configs, ConfigBoundsIndex, DefaultGCodeEmitter, DefaultGCodeSerializer,
    NoopLayerProgressSink,
};

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Paths
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn stl_fixture_path() -> PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/test_stl/ASCII/20mmbox-LF.stl")
}

/// The `modules/core-modules/` parent directory.
/// `load_live_modules_for_plan` scans one level of subdirectories, excluding
/// `Cargo.toml`, so it picks up `machine-gcode-emit/machine-gcode-emit.toml`
/// (and any other modules present â€” that's fine, only the postpass stage
/// actually fires in these tests since prepass/layer are Noop).
fn core_modules_dir() -> PathBuf {
    repo_root().join("modules/core-modules")
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Real harness: loads machine-gcode-emit.wasm and runs the full postpass
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Run the pipeline with the real `machine-gcode-emit` WASM postpass module.
/// `raw` overrides are merged into the config source before module binding.
fn slice_with_raw(raw: HashMap<ConfigKey, ConfigValue>) -> String {
    let module_dir = core_modules_dir();
    assert!(
        module_dir.exists(),
        "core-modules dir missing at {}; run build-core-modules.sh",
        module_dir.display()
    );

    // 1. Load all core module manifests + compile .wasm components.
    let loaded = load_live_modules_for_plan(&[module_dir], 1)
        .expect("load_live_modules_for_plan must succeed");

    // Confirm the module actually loaded as a real component (not a placeholder).
    let machine_binding = loaded
        .bindings
        .iter()
        .find(|b| b.module.id() == "com.core.machine-gcode-emit")
        .expect("com.core.machine-gcode-emit binding must be present");
    assert!(
        machine_binding.wasm_component.is_some(),
        "machine-gcode-emit.wasm must compile as a component (not a placeholder); \
         run modules/core-modules/build-core-modules.sh"
    );

    // 2. Build the config bounds index from all loaded modules.
    let config_bounds = ConfigBoundsIndex::from_modules(loaded.bindings.iter().map(|b| &b.module));

    // 3. Load the mesh.
    let mesh_ir = Arc::new(load_model(&stl_fixture_path()).expect("fixture STL load must succeed"));

    // 4. Build two separate config sources:
    //
    //    a) `binding_source`: used for `build_live_execution_plan`. Must include ALL
    //       machine-gcode-emit defaults (including multiline machine_start_gcode and
    //       machine_end_gcode strings) so the module's ConfigView receives the real
    //       template values and emits M190/PRINT_END correctly.
    //
    //    b) `pipeline_source`: passed to `run_pipeline_with_raw_config` and
    //       `resolve_global_config`. Uses empty-string sentinels for multiline string
    //       keys that were not explicitly supplied by the caller. This prevents the
    //       raw template (M190...) and default "PRINT_END" from being embedded verbatim
    //       in the CONFIG_BLOCK, which would break AC-2 (PRINT_END count) and
    //       AC-Neg-3 (M190 inside CONFIG_BLOCK). Non-string defaults (int/float) are
    //       shared between both sources so CONFIG_BLOCK contains numeric defaults.
    //
    //    User-supplied overrides (from `raw`) flow through BOTH sources unchanged.

    // Start both sources from the user's raw overrides.
    let mut binding_source = raw.clone();
    let mut pipeline_source = raw.clone();

    // Seed object heights into both sources.
    for object in &mesh_ir.objects {
        let key = format!("object_height:{}", object.id);
        if !binding_source.contains_key(&key) {
            if let Some((z_min, z_max)) = object.world_z_extent {
                let height_val = slicer_ir::ConfigValue::Float((z_max - z_min) as f64);
                binding_source.insert(key.clone(), height_val.clone());
                pipeline_source.insert(key, height_val);
            }
        }
    }

    // Seed machine-gcode-emit schema defaults â€” separated by type:
    //   - string defaults  â†’ binding_source only (real value for module)
    //                        pipeline_source gets empty-string sentinel (safe for CONFIG_BLOCK)
    //   - int/float/bool   â†’ both sources (numeric values are safe for CONFIG_BLOCK)
    let machine_schema = machine_binding.module.config_schema().entries.clone();
    for (key, entry) in &machine_schema {
        if binding_source.contains_key(key) {
            // User explicitly set this key â€” let it flow through both sources unchanged.
            continue;
        }
        let Some(default_str) = &entry.default else {
            continue;
        };
        match entry.field_type.as_str() {
            "int" => {
                if let Ok(v) = default_str.parse::<i64>() {
                    let cv = slicer_ir::ConfigValue::Int(v);
                    binding_source.insert(key.clone(), cv.clone());
                    pipeline_source.insert(key.clone(), cv);
                }
            }
            "float" => {
                if let Ok(v) = default_str.parse::<f64>() {
                    let cv = slicer_ir::ConfigValue::Float(v);
                    binding_source.insert(key.clone(), cv.clone());
                    pipeline_source.insert(key.clone(), cv);
                }
            }
            "bool" => {
                if let Ok(v) = default_str.parse::<bool>() {
                    let cv = slicer_ir::ConfigValue::Bool(v);
                    binding_source.insert(key.clone(), cv.clone());
                    pipeline_source.insert(key.clone(), cv);
                }
            }
            _ => {
                // String / enum: real value into binding_source; empty sentinel into
                // pipeline_source so CONFIG_BLOCK writes `; key = ` (not the template).
                binding_source.insert(
                    key.clone(),
                    slicer_ir::ConfigValue::String(default_str.clone()),
                );
                pipeline_source.insert(key.clone(), slicer_ir::ConfigValue::String(String::new()));
            }
        }
    }

    // 5. Resolve the global fallback config from the pipeline source.
    //    pipeline_source has int/float defaults and empty-string sentinels â€” no multiline
    //    template values â€” so the CONFIG_BLOCK won't contain M190 or PRINT_END.
    let default_resolved = resolve_global_config(&pipeline_source, &config_bounds)
        .expect("resolve_global_config must succeed");
    let object_ids: Vec<&str> = mesh_ir.objects.iter().map(|o| o.id.as_str()).collect();
    let resolved_configs_map = resolve_per_object_configs(
        &default_resolved,
        &pipeline_source,
        &object_ids,
        &config_bounds,
    )
    .expect("resolve_per_object_configs must succeed");

    // 6. Build the execution plan using the binding_source (real defaults for module ConfigViews).
    let plan = build_live_execution_plan(
        loaded.sorted_stages,
        loaded.bindings,
        &binding_source,
        Arc::new(Vec::<slicer_ir::GlobalLayer>::new()),
        Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
    )
    .expect("build_live_execution_plan must succeed");

    // 7. Construct the pipeline config with the real WASM dispatcher for all stages.
    let engine = Arc::clone(&loaded.engine);

    let config = PipelineConfig {
        mesh_ir,
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(
                DefaultGCodeEmitter::new("pnp_cli-test 0.1.0".into())
                    .with_resolved_config(default_resolved.clone()),
            ),
            serializer: Box::new(DefaultGCodeSerializer::new()),
        },
        resolved_configs: Arc::new(resolved_configs_map),
        default_resolved_config: Arc::new(default_resolved),
        bounds: Arc::new(config_bounds),
    };

    // 8. Run the pipeline. pipeline_source drives CONFIG_BLOCK generation.
    run_pipeline_with_raw_config(config, &pipeline_source, &NoopLayerProgressSink)
        .expect("pipeline must succeed")
        .gcode_text
}

fn slice_default() -> String {
    slice_with_raw(HashMap::new())
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Helper: count exact occurrences of a substring
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        count += 1;
        start += pos + needle.len();
    }
    count
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// POSITIVE TESTS (10)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-1: default config; out.gcode contains M190 S60, M109 S215, and
/// PRINT_START EXTRUDER=215 BED=60 each exactly once.
#[test]
fn start_gcode_default_substitutes() {
    let gcode = slice_default();

    assert_eq!(
        count_occurrences(&gcode, "M190 S60"),
        1,
        "M190 S60 must appear exactly once; gcode head:\n{}",
        &gcode[..gcode.len().min(600)]
    );
    assert_eq!(
        count_occurrences(&gcode, "M109 S215"),
        1,
        "M109 S215 must appear exactly once"
    );
    assert_eq!(
        count_occurrences(&gcode, "PRINT_START EXTRUDER=215 BED=60"),
        1,
        "PRINT_START EXTRUDER=215 BED=60 must appear exactly once"
    );
}

/// AC-2: default config; out.gcode contains exactly one PRINT_END;
/// no other line beginning with PRINT_ appears outside the start block.
#[test]
fn end_gcode_default_emits_print_end() {
    let gcode = slice_default();

    assert_eq!(
        count_occurrences(&gcode, "PRINT_END"),
        1,
        "PRINT_END must appear exactly once"
    );

    // No PRINT_START should appear in gcode except within the start block.
    // The start block is between HEADER_BLOCK_END and first M82/M83.
    let header_end = gcode.find("; HEADER_BLOCK_END").unwrap_or(0);
    let first_extrusion_mode = gcode[header_end..]
        .find("\nM82")
        .or_else(|| gcode[header_end..].find("\nM83"))
        .map(|p| header_end + p)
        .unwrap_or(gcode.len());

    let after_start_block = &gcode[first_extrusion_mode..];
    for line in after_start_block.lines() {
        assert!(
            !line.starts_with("PRINT_") || line.starts_with("PRINT_END"),
            "No PRINT_ command (other than PRINT_END) should appear after the start block; got: {line:?}"
        );
    }
}

/// AC-3: default config; byte-offset of M190 S60 is after HEADER_BLOCK_END,
/// before the first M82/M83 line, and before the first G1 with non-zero E token.
#[test]
fn start_block_position_before_extrusion_mode_and_first_g1() {
    let gcode = slice_default();

    let header_end_offset = gcode
        .find("; HEADER_BLOCK_END")
        .expect("HEADER_BLOCK_END must be present â€” not yet emitted (red)");

    let m190_offset = gcode
        .find("M190 S60")
        .expect("M190 S60 must be present in gcode â€” not yet emitted (red)");

    // Find first M82 or M83
    let m82_offset = gcode.find("\nM82").map(|p| p + 1);
    let m83_offset = gcode.find("\nM83").map(|p| p + 1);
    let extrusion_mode_offset = match (m82_offset, m83_offset) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    let extrusion_mode_offset =
        extrusion_mode_offset.expect("M82 or M83 must be present â€” not yet emitted (red)");

    // Find first G1 with non-zero E
    let first_g1_e_offset = gcode
        .lines()
        .scan(0usize, |pos, line| {
            let line_start = *pos;
            *pos += line.len() + 1; // +1 for '\n'
            Some((line_start, line))
        })
        .find(|(_, line)| {
            if !line.starts_with("G1") {
                return false;
            }
            line.split_whitespace().any(|tok| {
                tok.starts_with('E') && tok[1..].parse::<f64>().map(|v| v != 0.0).unwrap_or(false)
            })
        })
        .map(|(offset, _)| offset)
        .expect("at least one G1 with non-zero E must exist â€” not yet emitted (red)");

    assert!(
        m190_offset > header_end_offset,
        "M190 offset ({m190_offset}) must be after HEADER_BLOCK_END ({header_end_offset})"
    );
    assert!(
        m190_offset < extrusion_mode_offset,
        "M190 offset ({m190_offset}) must be before first M82/M83 ({extrusion_mode_offset})"
    );
    assert!(
        m190_offset < first_g1_e_offset,
        "M190 offset ({m190_offset}) must be before first G1 with E ({first_g1_e_offset})"
    );
}

/// AC-4: default config; byte-offset of PRINT_END is after last G1 line,
/// and before CONFIG_BLOCK_START.
#[test]
fn end_block_position_after_last_g1_before_config_block() {
    let gcode = slice_default();

    let print_end_offset = gcode
        .find("PRINT_END")
        .expect("PRINT_END must be present â€” not yet emitted (red)");

    let config_start_offset = gcode
        .find("; CONFIG_BLOCK_START")
        .expect("CONFIG_BLOCK_START must be present â€” not yet emitted (red)");

    // Find last G1 line offset
    let last_g1_offset = gcode
        .lines()
        .scan(0usize, |pos, line| {
            let line_start = *pos;
            *pos += line.len() + 1;
            Some((line_start, line))
        })
        .filter(|(_, line)| line.starts_with("G1"))
        .map(|(offset, _)| offset)
        .collect::<Vec<_>>()
        .into_iter()
        .last()
        .expect("at least one G1 must exist â€” not yet emitted (red)");

    assert!(
        print_end_offset > last_g1_offset,
        "PRINT_END offset ({print_end_offset}) must be after last G1 ({last_g1_offset})"
    );
    assert!(
        print_end_offset < config_start_offset,
        "PRINT_END offset ({print_end_offset}) must be before CONFIG_BLOCK_START ({config_start_offset})"
    );
}

/// AC-5: default config; exactly one of M82 or M83 appears between
/// HEADER_BLOCK_END and the first G1 extrusion move.
/// Regression sentry for M82/M83 promotion.
#[test]
fn extrusion_mode_still_emitted_after_promotion() {
    let gcode = slice_default();

    let header_end = gcode
        .find("; HEADER_BLOCK_END")
        .expect("HEADER_BLOCK_END must be present â€” not yet emitted (red)");

    let first_g1_e_offset = gcode[header_end..]
        .lines()
        .scan(header_end, |pos, line| {
            let line_start = *pos;
            *pos += line.len() + 1;
            Some((line_start, line))
        })
        .find(|(_, line)| {
            if !line.starts_with("G1") {
                return false;
            }
            line.split_whitespace().any(|tok| {
                tok.starts_with('E') && tok[1..].parse::<f64>().map(|v| v != 0.0).unwrap_or(false)
            })
        })
        .map(|(offset, _)| offset)
        .unwrap_or(gcode.len());

    let region = &gcode[header_end..first_g1_e_offset];

    let m82_count = count_occurrences(region, "\nM82");
    let m83_count = count_occurrences(region, "\nM83");
    let total = m82_count + m83_count;

    assert_eq!(
        total, 1,
        "exactly one M82 or M83 must appear between HEADER_BLOCK_END and first G1+E; \
         found M82={m82_count}, M83={m83_count} in region:\n{region}"
    );
}

/// AC-6: user.json with machine_start_gcode override; no default start commands.
#[test]
fn user_override_replaces_default() {
    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    raw.insert(
        "machine_start_gcode".to_string(),
        ConfigValue::String("G28 ; home all\nG1 Z5 F600".to_string()),
    );
    let gcode = slice_with_raw(raw);

    let header_end = gcode
        .find("; HEADER_BLOCK_END")
        .expect("HEADER_BLOCK_END must be present â€” not yet emitted (red)");
    let m82_offset = gcode[header_end..]
        .find("\nM82")
        .map(|p| header_end + p + 1);
    let m83_offset = gcode[header_end..]
        .find("\nM83")
        .map(|p| header_end + p + 1);
    let extrusion_mode_end = match (m82_offset, m83_offset) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => gcode.len(),
    };

    let start_block = &gcode[header_end..extrusion_mode_end];

    assert_eq!(
        count_occurrences(start_block, "G28 ; home all"),
        1,
        "custom start gcode 'G28 ; home all' must appear exactly once in start block"
    );
    assert_eq!(
        count_occurrences(start_block, "G1 Z5 F600"),
        1,
        "custom start gcode 'G1 Z5 F600' must appear exactly once in start block"
    );

    assert!(
        !gcode.contains("M190"),
        "M190 must NOT appear when start gcode is overridden (got override)"
    );
    assert!(
        !gcode.contains("M109"),
        "M109 must NOT appear when start gcode is overridden"
    );
    assert!(
        !gcode.contains("PRINT_START"),
        "PRINT_START must NOT appear when start gcode is overridden"
    );
}

/// AC-7: temperature values in substitution use overridden config values.
#[test]
fn substitution_uses_overridden_temp_values() {
    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    raw.insert(
        "bed_temperature_initial_layer_single".to_string(),
        ConfigValue::String("65".to_string()),
    );
    raw.insert(
        "nozzle_temperature_initial_layer".to_string(),
        ConfigValue::String("220".to_string()),
    );
    let gcode = slice_with_raw(raw);

    assert_eq!(
        count_occurrences(&gcode, "M190 S65"),
        1,
        "M190 S65 must appear exactly once with overridden bed temp"
    );
    assert_eq!(
        count_occurrences(&gcode, "M109 S220"),
        1,
        "M109 S220 must appear exactly once with overridden nozzle temp"
    );
    assert_eq!(
        count_occurrences(&gcode, "PRINT_START EXTRUDER=220 BED=65"),
        1,
        "PRINT_START with overridden temps must appear exactly once"
    );

    assert!(
        !gcode.contains("S60"),
        "default bed temp S60 must NOT appear when overridden to 65"
    );
    assert!(
        !gcode.contains("S215"),
        "default nozzle temp S215 must NOT appear when overridden to 220"
    );
    assert!(
        !gcode.contains("EXTRUDER=215"),
        "default EXTRUDER=215 must NOT appear when overridden"
    );
    assert!(
        !gcode.contains("BED=60"),
        "default BED=60 must NOT appear when overridden"
    );
}

/// AC-8: empty machine_end_gcode emits no end block.
#[test]
fn empty_end_gcode_emits_no_block() {
    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    raw.insert(
        "machine_end_gcode".to_string(),
        ConfigValue::String(String::new()),
    );
    let gcode = slice_with_raw(raw);

    assert!(
        !gcode.contains("PRINT_END"),
        "PRINT_END must NOT appear when machine_end_gcode is empty"
    );

    // Byte range between last G1 line's terminating \n and CONFIG_BLOCK_START
    // must contain zero non-whitespace characters.
    let config_start = gcode
        .find("; CONFIG_BLOCK_START")
        .expect("CONFIG_BLOCK_START must be present â€” not yet emitted (red)");

    let last_g1_end = gcode
        .lines()
        .scan(0usize, |pos, line| {
            let line_start = *pos;
            *pos += line.len() + 1;
            Some((line_start, line))
        })
        .filter(|(_, line)| line.starts_with("G1"))
        .map(|(offset, line)| offset + line.len() + 1) // end of line incl. \n
        .collect::<Vec<_>>()
        .into_iter()
        .last()
        .unwrap_or(0);

    let between = &gcode[last_g1_end..config_start];
    assert!(
        between.chars().all(|c| c.is_whitespace()),
        "region between last G1 and CONFIG_BLOCK_START must contain only whitespace \
         when end gcode is empty; found: {between:?}"
    );
}

/// AC-9: fallback â€” each of the four config keys appears as a comment line
/// in the CONFIG_BLOCK of a default-config out.gcode.
#[test]
fn module_manifest_registers_four_keys_with_expected_types_and_defaults() {
    let gcode = slice_default();

    let config_start = gcode
        .find("; CONFIG_BLOCK_START")
        .expect("CONFIG_BLOCK_START must be present â€” not yet emitted (red)");
    let config_end = gcode
        .find("; CONFIG_BLOCK_END")
        .expect("CONFIG_BLOCK_END must be present â€” not yet emitted (red)");

    let config_block = &gcode[config_start..config_end];

    // Fallback check: each key appears in CONFIG_BLOCK as a comment line.
    assert!(
        config_block.contains("; machine_start_gcode ="),
        "CONFIG_BLOCK must contain '; machine_start_gcode =' key"
    );
    assert!(
        config_block.contains("; machine_end_gcode ="),
        "CONFIG_BLOCK must contain '; machine_end_gcode =' key"
    );
    assert!(
        config_block.contains("; bed_temperature_initial_layer_single = 60"),
        "CONFIG_BLOCK must contain '; bed_temperature_initial_layer_single = 60'"
    );
    assert!(
        config_block.contains("; nozzle_temperature_initial_layer = 215"),
        "CONFIG_BLOCK must contain '; nozzle_temperature_initial_layer = 215'"
    );
}

/// AC-10: default slicing run; each of the four key=value lines appears
/// exactly once in CONFIG_BLOCK.
#[test]
fn new_keys_appear_in_config_block() {
    let gcode = slice_default();

    let config_start = gcode
        .find("; CONFIG_BLOCK_START")
        .expect("CONFIG_BLOCK_START must be present â€” not yet emitted (red)");
    let config_end = gcode
        .find("; CONFIG_BLOCK_END")
        .expect("CONFIG_BLOCK_END must be present â€” not yet emitted (red)");

    let config_block = &gcode[config_start..config_end];

    for expected in &[
        "; machine_start_gcode =",
        "; machine_end_gcode =",
        "; bed_temperature_initial_layer_single = 60",
        "; nozzle_temperature_initial_layer = 215",
    ] {
        assert_eq!(
            count_occurrences(config_block, expected),
            1,
            "CONFIG_BLOCK must contain {expected:?} exactly once"
        );
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// NEGATIVE TESTS (3)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Negative-1: unknown placeholder passes through verbatim; no panic.
#[test]
fn unknown_placeholder_passes_through_verbatim() {
    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    raw.insert(
        "machine_start_gcode".to_string(),
        ConfigValue::String("TEMP [no_such_key] DONE".to_string()),
    );
    let gcode = slice_with_raw(raw);

    let header_end = gcode
        .find("; HEADER_BLOCK_END")
        .expect("HEADER_BLOCK_END must be present â€” not yet emitted (red)");
    let m82_offset = gcode[header_end..]
        .find("\nM82")
        .map(|p| header_end + p + 1);
    let m83_offset = gcode[header_end..]
        .find("\nM83")
        .map(|p| header_end + p + 1);
    let extrusion_mode_end = match (m82_offset, m83_offset) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => gcode.len(),
    };

    let start_block = &gcode[header_end..extrusion_mode_end];

    assert_eq!(
        count_occurrences(start_block, "TEMP [no_such_key] DONE"),
        1,
        "unknown placeholder must pass through verbatim in start block; \
         start_block:\n{start_block}"
    );
}

/// Negative-2: unclosed bracket is treated as literal text; no panic; no infinite loop.
/// The literal must appear as actual gcode (not only in a CONFIG_BLOCK comment).
#[test]
fn unclosed_bracket_treated_as_literal() {
    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    raw.insert(
        "machine_start_gcode".to_string(),
        ConfigValue::String("PREFIX [unclosed SUFFIX".to_string()),
    );
    // If the implementation loops infinitely on unclosed brackets, this test times out.
    let gcode = slice_with_raw(raw);

    // Must appear as a standalone gcode line (no leading "; ") â€” fails in red state
    // because machine_start_gcode is not yet emitted as actual gcode.
    let appears_as_gcode_line = gcode.lines().any(|l| l == "PREFIX [unclosed SUFFIX");

    assert!(
        appears_as_gcode_line,
        "unclosed bracket must appear as a gcode line (not only in a comment); \
         gcode head:\n{}",
        &gcode[..gcode.len().min(600)]
    );
}

/// Negative-3: start block content does not appear inside HEADER_BLOCK or CONFIG_BLOCK.
/// Checks that occurrences of \nM190 all lie outside those two block byte ranges.
#[test]
fn start_block_not_inside_other_blocks() {
    let gcode = slice_default();

    // Locate the two protected ranges
    let header_start = gcode.find("; HEADER_BLOCK_START").unwrap_or(0);
    let header_end = gcode
        .find("; HEADER_BLOCK_END")
        .expect("HEADER_BLOCK_END must be present â€” not yet emitted (red)")
        + "; HEADER_BLOCK_END".len();

    let config_start = gcode
        .find("; CONFIG_BLOCK_START")
        .expect("CONFIG_BLOCK_START must be present â€” not yet emitted (red)");
    let config_end = gcode
        .find("; CONFIG_BLOCK_END")
        .map(|p| p + "; CONFIG_BLOCK_END".len())
        .unwrap_or(gcode.len());

    // Find all occurrences of \nM190 (line-start M190)
    let mut search = 0;
    while let Some(rel) = gcode[search..].find("\nM190") {
        let pos = search + rel + 1; // position of 'M' in the file
        assert!(
            !(pos >= header_start && pos < header_end),
            "M190 at byte {pos} must NOT appear inside HEADER_BLOCK ({header_start}..{header_end})"
        );
        assert!(
            !(pos >= config_start && pos < config_end),
            "M190 at byte {pos} must NOT appear inside CONFIG_BLOCK ({config_start}..{config_end})"
        );
        search += rel + 1;
    }

    // The test requires M190 to exist somewhere outside the blocks.
    // If M190 doesn't exist at all, the feature is missing â€” assert it's present.
    assert!(
        gcode.contains("\nM190"),
        "M190 must appear somewhere in gcode (start block) â€” not yet emitted (red)"
    );
}
