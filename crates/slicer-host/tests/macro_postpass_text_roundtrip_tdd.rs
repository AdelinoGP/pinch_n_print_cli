//! TASK-109 round-trip witness: a guest authored purely via
//! `#[slicer_module]` (no hand-rolled `wit_bindgen::generate!` +
//! `export!(Component)`) must round-trip through
//! `WasmRuntimeDispatcher::run_text_postprocess` for the documented
//! `PostPass::TextPostProcess` world (docs/03 wit/world-postpass.wit;
//! docs/05 §Module Entry Point).
//!
//! Proves the macro-generated typed export path is real: input text is
//! seen by the user's trait method, the `ConfigView` is pre-filtered
//! and typed (key/value accessors return real values), and the typed
//! `Result<String, ModuleError>` surface round-trips back through the
//! component-model boundary into the host.
//!
//! Test guest source: `test-guests/sdk-postpass-text-guest/`. Its
//! `lib.rs` contains only `#[slicer_module] impl PostpassModule for
//! SdkPostpassTextModule { ... }` — no `wit_bindgen::generate!` /
//! `export!` is written by hand, so any failure here means the macro's
//! emitted glue is not actually wiring the typed export.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_host::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_host::{
    Blackboard, CompiledModule, IrAccessMask, LoadedModule, PostpassOutput, PostpassStageRunner,
    WasmEngine, WasmRuntimeDispatcher,
};
use slicer_ir::{ConfigValue, ConfigView, StageId};

fn semver(major: u32, minor: u32, patch: u32) -> slicer_ir::SemVer {
    slicer_ir::SemVer {
        major,
        minor,
        patch,
    }
}

fn make_loaded_module(id: &str, stage: &str) -> LoadedModule {
    LoadedModule {
        id: id.to_string(),
        version: semver(1, 0, 0),
        stage: stage.to_string(),
        wit_world: "slicer:world-postpass@1.0.0".to_string(),
        ir_reads: Vec::new(),
        ir_writes: Vec::new(),
        claims: Vec::new(),
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: Default::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: false,
        wasm_path: std::path::PathBuf::from("/dev/null"),
        placeholder_wasm: false,
    }
}

const GUEST_COMPONENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/sdk-postpass-text-guest.component.wasm"
);

fn empty_mesh_ir() -> Arc<slicer_ir::MeshIR> {
    Arc::new(slicer_ir::MeshIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: Vec::new(),
        build_volume: slicer_ir::BoundingBox3 {
            min: slicer_ir::Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: slicer_ir::Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
    })
}

fn load_guest(engine: &WasmEngine) -> Arc<slicer_host::WasmComponent> {
    let path = PathBuf::from(GUEST_COMPONENT);
    assert!(
        path.exists(),
        "guest component missing at {}: rebuild test-guests/sdk-postpass-text-guest \
         via `cargo build --target wasm32-unknown-unknown --release` + `wasm-tools component new`",
        path.display()
    );
    let bytes = std::fs::read(&path).expect("read guest .component.wasm");
    Arc::new(
        engine
            .compile_component(&bytes)
            .expect("compile guest component"),
    )
}

fn make_module_with_config(
    module_id: &str,
    component: Arc<slicer_host::WasmComponent>,
    config: ConfigView,
) -> CompiledModule {
    let loaded = make_loaded_module(module_id, "PostPass::TextPostProcess");
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build instance pool"),
    );
    CompiledModule {
        module_id: module_id.to_string(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: Vec::new() },
        ir_write_mask: IrAccessMask { paths: Vec::new() },
        config_view: Arc::new(config),
        wasm_component: Some(component),
    }
}

fn text_of(out: PostpassOutput) -> String {
    match out {
        PostpassOutput::TextSuccess { text } => text,
        other => panic!("expected TextSuccess, got {other:?}"),
    }
}

#[test]
fn macro_authored_guest_round_trips_text_with_default_prefix() {
    // No `postpass_text_prefix` in config — the trait body falls back
    // to its default prefix, proving the macro-emitted export actually
    // reached the trait method (placeholder shims would return nothing
    // useful here).
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine);

    let module = make_module_with_config(
        "com.test.sdk-postpass-text-default",
        component,
        ConfigView::new(),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);

    let stage: StageId = "PostPass::TextPostProcess".to_string();
    let out = text_of(
        dispatcher
            .run_text_postprocess(&stage, &module, &bb, "G1 X0 Y0\n".to_string())
            .expect("macro-emitted glue must round-trip text successfully"),
    );

    assert_eq!(out, ";; task-109 guest: G1 X0 Y0\n");
}

#[test]
fn macro_authored_guest_round_trips_typed_config_string_value() {
    // Supplying a declared-key config entry must reach the trait
    // method through the wit-bindgen ConfigView resource, be adapted
    // back into `slicer_ir::ConfigView`, and be readable via the
    // typed `get_string` accessor in the guest's trait body.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine);

    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "postpass_text_prefix".to_string(),
        ConfigValue::String("[stamped] ".to_string()),
    );
    let module = make_module_with_config(
        "com.test.sdk-postpass-text-stamped",
        component,
        ConfigView::from_map(fields),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);

    let stage: StageId = "PostPass::TextPostProcess".to_string();
    let out = text_of(
        dispatcher
            .run_text_postprocess(&stage, &module, &bb, "M104 S200\n".to_string())
            .expect("macro-emitted glue + typed string config must round-trip"),
    );

    assert_eq!(out, "[stamped] M104 S200\n");
}

#[test]
fn macro_authored_guest_is_deterministic_across_repeated_dispatch_calls() {
    // Same module / same input text / same config must produce
    // byte-identical output on every dispatch invocation.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine);

    let module = make_module_with_config(
        "com.test.sdk-postpass-text-det",
        component,
        ConfigView::new(),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);
    let stage: StageId = "PostPass::TextPostProcess".to_string();

    let a = text_of(
        dispatcher
            .run_text_postprocess(&stage, &module, &bb, "; A\n".to_string())
            .unwrap(),
    );
    let b = text_of(
        dispatcher
            .run_text_postprocess(&stage, &module, &bb, "; A\n".to_string())
            .unwrap(),
    );
    let c = text_of(
        dispatcher
            .run_text_postprocess(&stage, &module, &bb, "; A\n".to_string())
            .unwrap(),
    );
    assert_eq!(a, b);
    assert_eq!(b, c);
    assert!(
        a.ends_with("; A\n"),
        "macro-glue must preserve input text suffix: {a:?}"
    );
}
