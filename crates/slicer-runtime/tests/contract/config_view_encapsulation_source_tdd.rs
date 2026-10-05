//! Source-level regression guard for the ConfigView encapsulation
//! contract. Complements
//! `crates/slicer-ir/tests/config_view_encapsulation_tdd.rs`: that test
//! proves the contract is enforced at compile time for external crates;
//! this one guards against the `ConfigView.fields` field being
//! re-exposed as `pub` in the future, which would silently re-open the
//! deviation (docs/03 §host-boundary access enforcement;
//! `wit/deps/config.wit` `resource config-view`).

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_config::{ExpansionContext, ResolutionTarget};
use slicer_ir::{ConfigValue, SemVer};
use slicer_runtime::{
    build_live_execution_plan, build_wasm_instance_pool, ConfigFieldEntry, ConfigSchema,
    LiveModuleBinding, LoadedModuleBuilder, SortedStageModules, WasmArtifactMetadata,
};
use slicer_scheduler::config_resolution::{resolve_config, ConfigBoundsIndex};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

#[test]
fn config_view_backing_map_stays_private() {
    let ir_src = repo_root().join("crates/slicer-ir/src/slice_ir.rs");
    let text = fs::read_to_string(&ir_src).expect("read slice_ir.rs");
    // Find the `pub struct ConfigView { ... }` block and assert no `pub`
    // field inside it. We look for the literal declaration since the
    // grep needs to ignore unrelated structs in the same file.
    let start = text
        .find("pub struct ConfigView")
        .expect("ConfigView struct present");
    let tail = &text[start..];
    let brace_open = tail.find('{').expect("ConfigView struct open brace");
    // Walk to the matching close brace.
    let mut depth = 0i32;
    let mut end_idx = None;
    for (i, ch) in tail[brace_open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end_idx = Some(brace_open + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let body_end = end_idx.expect("ConfigView struct close brace");
    let body = &tail[brace_open..body_end];
    assert!(
        !body.contains("pub fields"),
        "regression: `ConfigView.fields` is `pub` again — the \
         read-only/declared-reads-only contract in `wit/deps/config.wit` \
         requires the backing map to stay private (docs/03 \
         §host-boundary enforcement). ConfigView body was:\n{body}"
    );
}

#[test]
fn main_production_entry_path_uses_bind_module_config_view() {
    // Exercise the exact production plan builder called by `run_slice`
    // (`crates/slicer-runtime/src/run.rs`). Inspect the compiled module view
    // dispatched to the guest, rather than matching text in the caller.
    let sem = SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let module = LoadedModuleBuilder::new(
        "com.example.binding-probe",
        sem,
        "PrePass::MeshAnalysis",
        slicer_schema::TIER_PREPASS,
        PathBuf::from("fixtures/binding-probe.wasm"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(sem)
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .config_schema(ConfigSchema {
        entries: BTreeMap::from([
            (
                "layer_height".to_string(),
                ConfigFieldEntry {
                    field_type: "float".to_string(),
                    ..Default::default()
                },
            ),
            (
                "binding_probe".to_string(),
                ConfigFieldEntry {
                    field_type: "float".to_string(),
                    default: Some("7.0".to_string()),
                    ..Default::default()
                },
            ),
        ]),
    })
    .build();
    let source = HashMap::from([("layer_height".to_string(), ConfigValue::Float(0.28))]);
    let bounds = ConfigBoundsIndex::from_modules([&module]);
    let default_resolved_config = resolve_config(
        &source,
        &bounds,
        &ResolutionTarget::default(),
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..Default::default()
        },
    )
    .expect("resolve production default config");
    let pool = Arc::new(
        build_wasm_instance_pool(
            module.id(),
            module.stage(),
            module.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build module pool"),
    );
    let plan = build_live_execution_plan(
        vec![SortedStageModules {
            stage_id: module.stage().to_string(),
            module_ids: vec![module.id().to_string()],
        }],
        vec![LiveModuleBinding {
            module,
            instance_pool: pool,
            wasm_component: None,
            native_entry: None,
        }],
        &default_resolved_config,
        Arc::new(Vec::new()),
        Arc::new(HashMap::new()),
        &mut Vec::new(),
    )
    .expect("build production live plan");
    assert_eq!(plan.prepass_stages.len(), 1);
    assert_eq!(plan.prepass_stages[0].modules.len(), 1);
    let view = plan.prepass_stages[0].modules[0].config_view();
    // A blank/default replacement loses the authored value. A raw-map
    // replacement loses the seeded extension. An unfiltered map leaks host keys.
    assert_eq!(view.require_float("layer_height"), Ok(0.28));
    assert_eq!(view.require_float("binding_probe"), Ok(7.0));
    assert_eq!(
        view.keys(),
        vec!["binding_probe".to_string(), "layer_height".to_string()]
    );
    assert_eq!(
        view.require_float("nozzle_diameter").unwrap_err().key,
        "nozzle_diameter"
    );
}
