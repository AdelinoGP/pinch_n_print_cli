use std::path::PathBuf;

use slicer_wasm_host::host::{HostExecutionContext, HostExecutionContextBuilder, LayerModule};

#[test]
fn production_classic_perimeters_instantiates_with_layer_linker() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .expect("slicer-wasm-host manifest directory should have a workspace root");
    let component_path = workspace_root
        .join("modules")
        .join("core-modules")
        .join("classic-perimeters")
        .join("classic-perimeters.wasm");

    let engine = crate::common::wasm_cache::shared_engine()
        .wasmtime_engine()
        .clone();
    let component = wasmtime::component::Component::from_file(&engine, &component_path)
        .expect("load production classic-perimeters component");
    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx)
        .expect("add_to_linker");
    let ctx = HostExecutionContextBuilder::new("production-smoke", 0.0, 1.0).build();
    let mut store = wasmtime::Store::new(&engine, ctx);

    let result = LayerModule::instantiate(&mut store, &component, &linker);
    assert!(
        result.is_ok(),
        "production classic-perimeters instantiation failed: {:?}",
        result.err()
    );
}
