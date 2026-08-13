use std::path::PathBuf;

use slicer_wasm_host::host;

#[test]
#[ignore]
fn foreign_language_text_postprocess_component() {
    let path = std::env::var_os("PNP_FOREIGN_COMPONENT")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("PNP_FOREIGN_COMPONENT must point to a component"));
    let path = if path.is_absolute() || path.exists() {
        path
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    };
    let component = crate::common::wasm_cache::compiled_component_at(&path);
    let engine = crate::common::wasm_cache::shared_engine();

    let mut linker =
        wasmtime::component::Linker::<host::HostExecutionContext>::new(engine.wasmtime_engine());
    host::TextPostprocessModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
        &mut linker,
        |ctx| ctx,
    )
    .expect("failed to add text postprocess bindings to linker");
    host::postpass_gcode::slicer::postpass_gcode_postprocess::gcode_postprocess_types::add_to_linker::<
        _,
        wasmtime::component::HasSelf<_>,
    >(&mut linker, |ctx| ctx)
    .expect("failed to add gcode postprocess bindings to linker");

    let context =
        host::HostExecutionContextBuilder::new("foreign-language-probe".to_string(), 0.0, 0.0)
            .build();
    let mut store = wasmtime::Store::new(engine.wasmtime_engine(), context);
    let config_handle = store
        .data_mut()
        .push_config_view(host::config_view_to_data(&slicer_ir::ConfigView::default()))
        .expect("failed to push config resource");
    let bindings = host::TextPostprocessModule::instantiate(
        &mut store,
        component.wasmtime_component(),
        &linker,
    )
    .unwrap_or_else(|e| panic!("foreign component failed to instantiate: {e}"));
    let output = bindings
        .slicer_postpass_text_postprocess_text_postprocess()
        .call_run(&mut store, "; probe input\n", config_handle)
        .unwrap_or_else(|e| panic!("foreign component failed to run: {e}"))
        .unwrap_or_else(|e| panic!("foreign component returned module error: {e}"));

    assert_eq!(
        output, ";; foreign-language-probe\n; probe input\n",
        "foreign component returned wrong output: {output:?}"
    );
}
