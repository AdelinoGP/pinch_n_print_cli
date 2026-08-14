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
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
        .expect("failed to add WASI preview2 bindings to linker");

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

#[test]
#[ignore]
fn foreign_language_text_postprocess_perf() {
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
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
        .expect("failed to add WASI preview2 bindings to linker");

    let context =
        host::HostExecutionContextBuilder::new("foreign-language-probe".to_string(), 0.0, 0.0)
            .build();
    let mut store = wasmtime::Store::new(engine.wasmtime_engine(), context);

    let load_start = std::time::Instant::now();
    let component = crate::common::wasm_cache::compiled_component_at(&path);
    let bindings = host::TextPostprocessModule::instantiate(
        &mut store,
        component.wasmtime_component(),
        &linker,
    )
    .unwrap_or_else(|e| panic!("foreign component failed to instantiate: {e}"));
    let instantiate_ms = load_start.elapsed().as_secs_f64() * 1000.0;

    let run = bindings.slicer_postpass_text_postprocess_text_postprocess();
    // The WIT `run` signature takes an owned `config-view`, so each call consumes
    // one resource handle; push a fresh view per call (uniform across guests).
    let push_config = |store: &mut wasmtime::Store<host::HostExecutionContext>| {
        store
            .data_mut()
            .push_config_view(host::config_view_to_data(&slicer_ir::ConfigView::default()))
            .expect("failed to push config resource")
    };
    const WARMUP_CALLS: usize = 10;
    const TIMED_CALLS: usize = 1000;
    for _ in 0..WARMUP_CALLS {
        let handle = push_config(&mut store);
        let _ = run
            .call_run(&mut store, "; probe input\n", handle)
            .unwrap_or_else(|e| panic!("foreign component trapped during warmup: {e}"));
    }
    let timed_start = std::time::Instant::now();
    for _ in 0..TIMED_CALLS {
        let handle = push_config(&mut store);
        let _ = run
            .call_run(&mut store, "; probe input\n", handle)
            .unwrap_or_else(|e| panic!("foreign component trapped during timed call: {e}"));
    }
    let total_ms = timed_start.elapsed().as_secs_f64() * 1000.0;
    let mean_us = total_ms * 1000.0 / TIMED_CALLS as f64;
    println!(
        "PERF instantiate_ms={instantiate_ms:.3} calls={TIMED_CALLS} total_ms={total_ms:.3} mean_us={mean_us:.3}"
    );
}
