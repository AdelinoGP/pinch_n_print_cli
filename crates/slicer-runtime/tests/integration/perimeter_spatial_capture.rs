#[cfg(not(feature = "perimeter-spatial-test-support"))]
#[test]
fn perimeter_spatial_capture_requires_test_support() {
    panic!("perimeter-spatial-test-support feature required");
}

#[cfg(feature = "perimeter-spatial-test-support")]
mod perimeter_spatial_tests {
    use std::collections::{BTreeSet, HashMap};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use arachne_perimeters::ArachnePerimeters;
    use classic_perimeters::ClassicPerimeters;
    use slicer_core::perimeter_spatial::diagnostics::{
        with_capture, with_context_accounting, ContextAccountingRow, QueryCounters,
        RegionCaptureRecord,
    };
    use slicer_gcode::{DefaultGCodeEmitter, DefaultGCodeSerializer};
    use slicer_ir::{
        ConfigValue, ConfigView, GlobalLayer, LayerStageCommit, ModuleId, PerimeterIR, SemVer,
        SliceIR, StageId,
    };
    use slicer_runtime::pipeline::{run_pipeline_with_raw_config, PipelineStageRunners};
    use slicer_runtime::{
        assemble_search_roots, build_live_execution_plan, load_live_modules_for_plan_with_config,
        resolve_global_config, resolve_per_object_configs, resolve_per_tool_configs,
        validate_support_layer_heights, CompiledModuleLive, ConfigBoundsIndex, LayerStageError,
        LayerStageInput, NoopLayerProgressSink, WasmComponent, WasmInstancePool,
        WasmRuntimeDispatcher,
    };
    use slicer_sdk::native::NativeStageEntry;

    use crate::common::integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec};
    use crate::common::perimeter_harness::{
        PerimeterCapturingLayerStageRunner, PerimeterHarnessError, WallGenerator,
    };

    const FIXTURE_FORMAT_VERSION: &str = "perimeter-spatial-self-baseline-v1";
    const INPUT_FIXTURE_FORMAT_VERSION: &str = "perimeter-spatial-input-v1";
    const INPUT_CONFIG: &str = "{\n  \"first_layer_height\": 0.2,\n  \"layer_height\": 0.2,\n  \"top_shell_layers\": 3,\n  \"bottom_shell_layers\": 3,\n  \"only_one_wall_top\": true\n}\n";

    struct DiagnosticLayerStageRunner {
        inner: Box<dyn slicer_runtime::LayerStageRunner + Sync>,
        native_entries: HashMap<String, NativeStageEntry>,
        captured: Arc<Mutex<HashMap<u32, PerimeterIR>>>,
        source_slices: Arc<Mutex<HashMap<u32, SliceIR>>>,
        counters: Arc<Mutex<QueryCounters>>,
        records: Arc<Mutex<Vec<RegionCaptureRecord>>>,
        context_rows: Arc<Mutex<HashMap<u32, Vec<ContextAccountingRow>>>>,
        accelerated: bool,
    }

    impl slicer_runtime::LayerStageRunner for DiagnosticLayerStageRunner {
        fn run_stage(
            &self,
            stage_id: &StageId,
            layer: &GlobalLayer,
            module: &CompiledModuleLive<'_>,
            input: LayerStageInput<'_>,
        ) -> Result<Option<LayerStageCommit>, LayerStageError> {
            if let Some(perimeter) = input.perimeter {
                self.captured
                    .lock()
                    .expect("capture mutex poisoned")
                    .insert(perimeter.global_layer_index, perimeter.clone());
                if let Some(slice) = input.slice {
                    self.source_slices
                        .lock()
                        .expect("source-slice mutex poisoned")
                        .insert(slice.global_layer_index, slice.clone());
                }
            }
            // The production pipeline schedules layer stages on worker threads;
            // keep the diagnostic scope around each native stage call so its
            // thread-local counters still cover every generator pass.
            let source_layer_index = input
                .slice
                .map(|slice| slice.global_layer_index)
                .unwrap_or(layer.index);
            let ((result, counters, records), context_rows) =
                with_context_accounting(self.accelerated, false, || {
                    with_capture(self.accelerated, false, || {
                        if let Some(native_entry) =
                            self.native_entries.get(&module.module_id.to_string())
                        {
                            let native_live = CompiledModuleLive::new(
                                &module.module_id,
                                WasmInstancePool::placeholder(),
                                None,
                                module.claims,
                                Arc::clone(&module.config_view),
                            )
                            .with_native_entry(*native_entry);
                            self.inner.run_stage(stage_id, layer, &native_live, input)
                        } else {
                            self.inner.run_stage(stage_id, layer, module, input)
                        }
                    })
                });
            self.context_rows
                .lock()
                .expect("context-row mutex poisoned")
                .entry(source_layer_index)
                .or_default()
                .extend(context_rows);
            let mut total = self.counters.lock().expect("counter mutex poisoned");
            total.indexed_queries += counters.indexed_queries;
            total.legacy_queries += counters.legacy_queries;
            total.fallback_queries += counters.fallback_queries;
            total.exact_evaluations += counters.exact_evaluations;
            total.contexts_constructed += counters.contexts_constructed;
            self.records
                .lock()
                .expect("record mutex poisoned")
                .extend(records);
            result
        }

        fn last_wasm_mem_sample(&self) -> (u64, u64) {
            self.inner.last_wasm_mem_sample()
        }

        fn last_runtime_reads(&self) -> Vec<String> {
            self.inner.last_runtime_reads()
        }

        fn last_log_messages(&self) -> Vec<(String, String)> {
            self.inner.last_log_messages()
        }
    }

    fn parity_spec(
        module_root: &Path,
        wall_generator: WallGenerator,
    ) -> (&'static str, IntegratedParitySpec) {
        let (module_id, module_dir, native_entry) = match wall_generator {
            WallGenerator::Classic => (
                "com.core.classic-perimeters",
                "classic-perimeters",
                ClassicPerimeters::__slicer_native_entry(),
            ),
            WallGenerator::Arachne => (
                "com.core.arachne-perimeters",
                "arachne-perimeters",
                ArachnePerimeters::__slicer_native_entry(),
            ),
        };
        (
            module_id,
            IntegratedParitySpec {
                module_id: module_id.to_string(),
                wasm_path: module_root.join(format!("{module_dir}/{module_dir}.wasm")),
                stage: "layer".to_string(),
                version: SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                min_ir_schema: SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                max_ir_schema: SemVer {
                    major: 99,
                    minor: 0,
                    patch: 0,
                },
                tier: String::new(),
                claims: Vec::new(),
                config: Arc::new(ConfigView::new()),
                native_entry,
            },
        )
    }

    fn run_native_pipeline(
        mesh_path: &Path,
        config_path: &Path,
        module_dirs: &[PathBuf],
        wall_generator: WallGenerator,
        accelerated: bool,
    ) -> Result<
        (
            Vec<PerimeterIR>,
            Vec<SliceIR>,
            QueryCounters,
            Vec<RegionCaptureRecord>,
            Vec<ContextAccountingRow>,
        ),
        PerimeterHarnessError,
    > {
        let module_root = module_dirs
            .first()
            .ok_or_else(|| PerimeterHarnessError("missing core module root".to_string()))?;
        let (module_id, parity) = parity_spec(module_root, wall_generator);
        let native_entry = parity.native_entry;
        run_integrated_parity(parity, |_dispatcher, _native_live, _wasm_live| {
            run_native_pipeline_bound(
                mesh_path,
                config_path,
                module_dirs,
                wall_generator,
                accelerated,
                HashMap::from([(module_id.to_string(), native_entry)]),
            )
        })
    }

    fn run_native_pipeline_bound(
        mesh_path: &Path,
        config_path: &Path,
        module_dirs: &[PathBuf],
        wall_generator: WallGenerator,
        accelerated: bool,
        native_entries: HashMap<String, NativeStageEntry>,
    ) -> Result<
        (
            Vec<PerimeterIR>,
            Vec<SliceIR>,
            QueryCounters,
            Vec<RegionCaptureRecord>,
            Vec<ContextAccountingRow>,
        ),
        PerimeterHarnessError,
    > {
        let mesh = Arc::new(
            slicer_model_io::load_model(mesh_path)
                .map_err(|e| PerimeterHarnessError(format!("model load failed: {e:?}")))?,
        );
        let config_text = std::fs::read_to_string(config_path)
            .map_err(|e| PerimeterHarnessError(format!("failed to read config file: {e}")))?;
        let mut config_source = slicer_runtime::parse_cli_config_source(&config_text)
            .map_err(|e| PerimeterHarnessError(format!("failed to parse config: {e:?}")))?;
        let wall_generator_name = match wall_generator {
            WallGenerator::Classic => "classic",
            WallGenerator::Arachne => "arachne",
        };
        config_source.insert(
            "wall_generator".to_string(),
            ConfigValue::String(wall_generator_name.to_string()),
        );

        for object in &mesh.objects {
            let key = format!("object_height:{}", object.id);
            if let std::collections::hash_map::Entry::Vacant(entry) = config_source.entry(key) {
                if let Some((z_min, z_max)) = object.world_z_extent {
                    entry.insert(ConfigValue::Float((z_max - z_min) as f64));
                }
            }
        }
        for object in &mesh.objects {
            for (subkey, value) in &object.config.data {
                let key = format!("object_config:{}:{}", object.id, subkey);
                config_source.entry(key).or_insert_with(|| value.clone());
            }
        }
        if mesh
            .objects
            .iter()
            .any(|object| object.paint_data.is_some())
            && !config_source.contains_key("slice_has_paint")
        {
            config_source.insert("slice_has_paint".to_string(), ConfigValue::Bool(true));
        }
        if !config_source.contains_key("wipe_tower_enabled") {
            let mut tools = BTreeSet::new();
            for object in &mesh.objects {
                if let Some(paint_data) = &object.paint_data {
                    for layer in &paint_data.layers {
                        for value in layer.facet_values.iter().flatten() {
                            if let slicer_ir::PaintValue::ToolIndex(tool) = value {
                                tools.insert(*tool);
                            }
                        }
                    }
                }
            }
            if tools.len() >= 2 {
                config_source.insert("wipe_tower_enabled".to_string(), ConfigValue::Bool(true));
            }
        }

        let search_roots = assemble_search_roots(module_dirs, true);
        let mut loaded = load_live_modules_for_plan_with_config(
            &search_roots,
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            &config_source,
        )
        .map_err(|e| PerimeterHarnessError(format!("failed to load modules: {e}")))?;
        let config_bounds =
            ConfigBoundsIndex::from_modules(loaded.bindings.iter().map(|b| &b.module));
        let default_resolved_config = resolve_global_config(&config_source, &config_bounds)
            .map_err(|e| PerimeterHarnessError(format!("config resolution failed: {e:?}")))?;
        let object_ids: Vec<&str> = mesh
            .objects
            .iter()
            .map(|object| object.id.as_str())
            .collect();
        let resolved_configs_map = resolve_per_object_configs(
            &default_resolved_config,
            &config_source,
            &object_ids,
            &config_bounds,
        )
        .map_err(|e| {
            PerimeterHarnessError(format!("per-object config resolution failed: {e:?}"))
        })?;
        validate_support_layer_heights(&resolved_configs_map).map_err(|e| {
            PerimeterHarnessError(format!("support layer height validation failed: {e:?}"))
        })?;
        let per_tool_configs_map =
            resolve_per_tool_configs(&default_resolved_config, &config_source, &config_bounds)
                .map_err(|e| {
                    PerimeterHarnessError(format!("per-tool config resolution failed: {e:?}"))
                })?;
        let wasm_handles: HashMap<
            ModuleId,
            (
                Arc<WasmInstancePool>,
                Option<Arc<WasmComponent>>,
                Option<slicer_sdk::native::NativeStageEntry>,
            ),
        > = loaded
            .bindings
            .iter()
            .map(|binding| {
                (
                    binding.module.id().to_string(),
                    (
                        Arc::clone(&binding.instance_pool),
                        binding.wasm_component.clone(),
                        binding.native_entry,
                    ),
                )
            })
            .collect();
        let plan = build_live_execution_plan(
            loaded.sorted_stages,
            loaded.bindings,
            &config_source,
            Arc::new(Vec::new()),
            Arc::new(HashMap::new()),
            &mut loaded.diagnostics,
        )
        .map_err(|e| PerimeterHarnessError(format!("failed to build execution plan: {e:?}")))?;
        let engine = Arc::clone(&loaded.engine);
        let relative = match config_source.get("use_relative_e_distances") {
            Some(ConfigValue::Bool(value)) => *value,
            _ => slicer_runtime::run::DEFAULT_USE_RELATIVE_E_DISTANCES,
        };
        let captured = Arc::new(Mutex::new(HashMap::new()));
        let source_slices = Arc::new(Mutex::new(HashMap::new()));
        let counters = Arc::new(Mutex::new(QueryCounters::default()));
        let records = Arc::new(Mutex::new(Vec::new()));
        let context_rows = Arc::new(Mutex::new(HashMap::new()));
        let layer_runner = DiagnosticLayerStageRunner {
            inner: Box::new(PerimeterCapturingLayerStageRunner::new(Box::new(
                WasmRuntimeDispatcher::new(Arc::clone(&engine)),
            ))),
            native_entries,
            captured: Arc::clone(&captured),
            source_slices: Arc::clone(&source_slices),
            counters: Arc::clone(&counters),
            records: Arc::clone(&records),
            context_rows: Arc::clone(&context_rows),
            accelerated,
        };
        let mut pipeline_config = crate::common::pipeline_config_base(
            mesh,
            plan,
            // exhaustive: PipelineStageRunners owns the runtime trait-object boundary for this harness.
            PipelineStageRunners {
                prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                layer: Box::new(layer_runner),
                finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                emitter: Box::new(
                    DefaultGCodeEmitter::new("perimeter_spatial_capture".to_string())
                        .with_resolved_config(default_resolved_config.clone())
                        .with_tool_configs(per_tool_configs_map.clone()),
                ),
                serializer: Box::new(DefaultGCodeSerializer::with_extrusion_mode(relative)),
            },
        );
        pipeline_config.resolved_configs = Arc::new(resolved_configs_map);
        pipeline_config.default_resolved_config = Arc::new(default_resolved_config);
        pipeline_config.bounds = Arc::new(config_bounds);
        pipeline_config.wasm_handles = wasm_handles;
        run_pipeline_with_raw_config(pipeline_config, &config_source, &NoopLayerProgressSink)
            .map_err(|e| PerimeterHarnessError(format!("pipeline run failed: {e}")))?;

        let mut captured_perimeters: Vec<_> = captured
            .lock()
            .map_err(|_| PerimeterHarnessError("capture mutex poisoned".to_string()))?
            .values()
            .cloned()
            .collect();
        captured_perimeters.sort_by_key(|perimeter| perimeter.global_layer_index);
        let mut source_slices: Vec<_> = source_slices
            .lock()
            .map_err(|_| PerimeterHarnessError("source-slice mutex poisoned".to_string()))?
            .values()
            .cloned()
            .collect();
        source_slices.sort_by_key(|slice| slice.global_layer_index);
        let counters = *counters
            .lock()
            .map_err(|_| PerimeterHarnessError("counter mutex poisoned".to_string()))?;
        let records = records
            .lock()
            .map_err(|_| PerimeterHarnessError("record mutex poisoned".to_string()))?
            .clone();
        let mut context_rows_by_layer = context_rows
            .lock()
            .map_err(|_| PerimeterHarnessError("context-row mutex poisoned".to_string()))?
            .clone();
        // Layers execute in parallel, so restore the pipeline's source order
        // before pairing rows with source regions. Within each layer, stages
        // and context construction remain sequential.
        let context_rows = source_slices
            .iter()
            .flat_map(|slice| {
                context_rows_by_layer
                    .remove(&slice.global_layer_index)
                    .unwrap_or_default()
            })
            .collect();
        Ok((
            captured_perimeters,
            source_slices,
            counters,
            records,
            context_rows,
        ))
    }

    fn run_wasm_pipeline(
        mesh_path: &Path,
        config_path: &Path,
        module_dirs: &[PathBuf],
    ) -> Result<
        (
            Vec<PerimeterIR>,
            Vec<SliceIR>,
            QueryCounters,
            Vec<RegionCaptureRecord>,
            Vec<ContextAccountingRow>,
        ),
        PerimeterHarnessError,
    > {
        let (run, _outer_counters, _outer_records) = with_capture(true, false, || {
            run_native_pipeline_bound(
                mesh_path,
                config_path,
                module_dirs,
                WallGenerator::Classic,
                true,
                HashMap::new(),
            )
        });
        run
    }

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/perimeter_spatial")
    }

    fn module_dirs() -> [PathBuf; 1] {
        [PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules")]
    }

    fn record_input_fixture_files() -> Result<(PathBuf, PathBuf), String> {
        let root = fixture_root();
        std::fs::create_dir_all(&root)
            .map_err(|e| format!("create perimeter-spatial fixture directory: {e}"))?;
        let mesh_path = root.join("bridge.stl");
        let arachne_second_pass_path = root.join("arachne_second_pass.3mf");
        let config_path = root.join("config.json");
        let provenance_path = root.join("INPUT.provenance.txt");
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/support-family/SupportAdversarial.stl");
        let arachne_second_pass_source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/perimeter_parity/cube_4color_arachne/cube_4color.3mf");
        std::fs::copy(source, &mesh_path)
            .map_err(|e| format!("copy perimeter-spatial STL fixture: {e}"))?;
        std::fs::copy(arachne_second_pass_source, &arachne_second_pass_path)
            .map_err(|e| format!("copy Arachne second-pass 3MF fixture: {e}"))?;
        std::fs::write(&config_path, INPUT_CONFIG)
            .map_err(|e| format!("write perimeter-spatial config fixture: {e}"))?;
        std::fs::write(
            &provenance_path,
            format!(
                "fixture_format_version={INPUT_FIXTURE_FORMAT_VERSION}\n\
                 recorded_by=record_perimeter_spatial_fixtures\n\
                 mesh=committed support fixture copied verbatim\n\
                 mesh_source=tests/fixtures/support-family/SupportAdversarial.stl\n\
                 arachne_second_pass_mesh=committed painted perimeter parity fixture copied verbatim\n\
                 arachne_second_pass_mesh_source=tests/fixtures/perimeter_parity/cube_4color_arachne/cube_4color.3mf\n\
                 config=top_shell_layers=3,bottom_shell_layers=3,only_one_wall_top=true\n\
                 coordinates_unit=millimetres\n\
                 coordinate_integer_scale=1\n\
                 coordinate_provenance=source fixture vertices remain integer-scaled input\n\
                 layer_height_f32_bits=0x3f800000\n\
                 first_layer_height_f32_bits=0x3f800000\n"
            ),
        )
        .map_err(|e| format!("write perimeter-spatial provenance: {e}"))?;
        Ok((mesh_path, config_path))
    }

    fn required_file(path: &Path, description: &str) -> Result<(), String> {
        if !path.is_file() {
            return Err(format!(
                "required {description} is missing: {}; run record_perimeter_spatial_fixtures explicitly",
                path.display()
            ));
        }
        let metadata = std::fs::metadata(path)
            .map_err(|e| format!("read metadata for required {description}: {e}"))?;
        if metadata.len() == 0 {
            return Err(format!(
                "required {description} is empty: {}; run record_perimeter_spatial_fixtures explicitly",
                path.display()
            ));
        }
        Ok(())
    }

    fn required_input_fixture() -> Result<(PathBuf, PathBuf), String> {
        let root = fixture_root();
        let mesh_path = root.join("bridge.stl");
        let config_path = root.join("config.json");
        required_file(&mesh_path, "perimeter-spatial mesh fixture")?;
        required_file(&config_path, "perimeter-spatial config fixture")?;
        required_file(
            &root.join("INPUT.provenance.txt"),
            "perimeter-spatial input provenance",
        )?;
        Ok((mesh_path, config_path))
    }

    fn baseline_path(mode: &str) -> PathBuf {
        fixture_root().join(format!("{mode}.bin"))
    }

    fn record_baseline(mode: &str, observed: &[u8]) -> Result<(), String> {
        if observed.is_empty() {
            return Err(format!("{mode} perimeter-spatial baseline is empty"));
        }
        let path = baseline_path(mode);
        std::fs::write(&path, observed)
            .map_err(|e| format!("write {mode} perimeter-spatial baseline: {e}"))?;
        let provenance = format!(
            "fixture_format_version={FIXTURE_FORMAT_VERSION}\n\
             recorded_by=record_perimeter_spatial_fixtures\n\
             mode={mode}\n\
             serialization=postcard\n\
             serialization_schema=PerimeterIR\n\
             input=bridge.stl\n\
             config=top_shell_layers=3,bottom_shell_layers=3,only_one_wall_top=true\n\
             coordinates_unit=millimetres\n\
             coordinate_integer_scale=1\n\
             layer_height_f32_bits=0x3f800000\n\
             first_layer_height_f32_bits=0x3f800000\n"
        );
        std::fs::write(
            fixture_root().join(format!("{mode}.provenance.txt")),
            provenance,
        )
        .map_err(|e| format!("write {mode} baseline provenance: {e}"))?;
        Ok(())
    }

    fn required_baseline(mode: &str) -> Result<Vec<u8>, String> {
        let path = baseline_path(mode);
        required_file(&path, &format!("{mode} perimeter-spatial baseline"))?;
        required_file(
            &fixture_root().join(format!("{mode}.provenance.txt")),
            &format!("{mode} perimeter-spatial baseline provenance"),
        )?;
        std::fs::read(&path).map_err(|e| format!("read {mode} perimeter-spatial baseline: {e}"))
    }

    fn native_region_identities(perimeters: &[PerimeterIR]) -> BTreeSet<(usize, usize)> {
        perimeters
            .iter()
            .flat_map(|perimeter| {
                perimeter
                    .regions
                    .iter()
                    .enumerate()
                    .map(move |(region_ordinal, _)| {
                        (perimeter.global_layer_index as usize, region_ordinal)
                    })
            })
            .collect()
    }

    fn source_region_identities(slices: &[SliceIR]) -> BTreeSet<(usize, usize)> {
        slices
            .iter()
            .flat_map(|slice| {
                slice
                    .regions
                    .iter()
                    .enumerate()
                    .filter(|(_, region)| !region.polygons.is_empty())
                    .map(move |(region_ordinal, _)| {
                        (slice.global_layer_index as usize, region_ordinal)
                    })
            })
            .collect()
    }

    fn fixture_inputs() -> (PathBuf, PathBuf, [PathBuf; 1]) {
        let (mesh_path, config_path) = required_input_fixture()
            .expect("required perimeter-spatial input fixture is unavailable");
        (mesh_path, config_path, module_dirs())
    }

    fn arachne_second_pass_input() -> PathBuf {
        let path = fixture_root().join("arachne_second_pass.3mf");
        required_file(&path, "Arachne second-pass mesh fixture")
            .expect("required Arachne second-pass fixture is unavailable");
        path
    }

    fn postcard_bytes(perimeters: &[PerimeterIR]) -> Vec<u8> {
        postcard::to_stdvec(perimeters).expect("serialize complete PerimeterIR postcard")
    }

    fn source_distance_record_counts(slices: &[SliceIR]) -> Vec<usize> {
        // `PerimeterSpatialContext` receives `prev_layer_boundary`, which the
        // overhang annotation producer takes from the previous layer's
        // complete object footprint. Mirror that source construction here,
        // rather than counting the current perimeter region's rings: the
        // distance index contains one record for every contour or hole edge
        // in that previous-layer footprint, including records from sibling
        // regions of the same object.
        //
        // This traversal remains in the source-region construction order used
        // by the native perimeter generators: layers first, then regions.
        slices
            .iter()
            .enumerate()
            .flat_map(|(slice_index, slice)| {
                let previous_slice = slice_index
                    .checked_sub(1)
                    .and_then(|index| slices.get(index));
                slice
                    .regions
                    .iter()
                    .filter(|region| !region.polygons.is_empty())
                    .map(move |region| {
                        previous_slice.map_or(0, |previous_slice| {
                            previous_slice
                                .regions
                                .iter()
                                .filter(|previous_region| {
                                    previous_region.object_id == region.object_id
                                })
                                .flat_map(|previous_region| &previous_region.polygons)
                                .map(|expolygon| {
                                    expolygon.contour.points.len()
                                        + expolygon
                                            .holes
                                            .iter()
                                            .map(|hole| hole.points.len())
                                            .sum::<usize>()
                                })
                                .sum()
                        })
                    })
            })
            .collect()
    }

    fn arachne_second_pass_region_count(slices: &[SliceIR]) -> usize {
        slices
            .iter()
            .flat_map(|slice| &slice.regions)
            .filter(|region| {
                region.top_shell_index != Some(0)
                    && !slicer_core::polygon_ops::difference_ex(
                        &region.top_solid_fill,
                        &region.internal_solid_fill,
                    )
                    .is_empty()
            })
            .count()
    }

    #[test]
    fn record_perimeter_spatial_fixtures() {
        let (mesh_path, config_path) =
            record_input_fixture_files().expect("record deterministic perimeter-spatial input");
        let module_dirs = module_dirs();

        let (run, _outer_counters, _outer_records) = with_capture(true, false, || {
            run_native_pipeline(
                &mesh_path,
                &config_path,
                &module_dirs,
                WallGenerator::Classic,
                true,
            )
        });
        let (perimeters, _source_slices, _counters, _records, _context_rows) =
            run.expect("record indexed native perimeter pipeline");
        assert!(
            !perimeters.is_empty(),
            "recorded indexed output must be nonempty"
        );
        record_baseline("native_indexed", &postcard_bytes(&perimeters))
            .expect("record indexed native perimeter baseline");

        let (run, _outer_counters, _outer_records) = with_capture(false, false, || {
            run_native_pipeline(
                &mesh_path,
                &config_path,
                &module_dirs,
                WallGenerator::Classic,
                false,
            )
        });
        let (perimeters, _source_slices, _counters, _records, _context_rows) =
            run.expect("record legacy native perimeter pipeline");
        assert!(
            !perimeters.is_empty(),
            "recorded legacy output must be nonempty"
        );
        record_baseline("native_legacy", &postcard_bytes(&perimeters))
            .expect("record legacy native perimeter baseline");

        let (perimeters, _source_slices, _counters, _records, _context_rows) =
            run_wasm_pipeline(&mesh_path, &config_path, &module_dirs)
                .expect("record WASM perimeter pipeline");
        assert!(
            !perimeters.is_empty(),
            "recorded WASM output must be nonempty"
        );
        record_baseline("wasm", &postcard_bytes(&perimeters))
            .expect("record WASM perimeter baseline");
    }

    #[test]
    fn perimeter_spatial_capture_and_nonvacuity() {
        let (mesh_path, config_path, module_dirs) = fixture_inputs();
        // Production prepass_slice constructs every SlicedRegion with
        // nonplanar_surface=None; no production path currently assigns
        // Some(...). Classic nonplanar annotation is therefore not
        // fixture-reachable. Its run below still requires one context per
        // region, indexed queries, and nonempty output.
        for wall_generator in [WallGenerator::Classic, WallGenerator::Arachne] {
            let (perimeters, source_slices, counters, records, context_rows) =
                run_native_pipeline(&mesh_path, &config_path, &module_dirs, wall_generator, true)
                    .expect("real native perimeter pipeline");
            assert!(
                !perimeters.is_empty(),
                "native perimeter pipeline must produce at least one layer"
            );
            assert!(
                perimeters.iter().any(|perimeter| {
                    !perimeter
                        .regions
                        .iter()
                        .all(|region| region.walls.is_empty())
                }),
                "native perimeter pipeline must produce non-empty perimeter output"
            );
            let source_region_identities = source_region_identities(&source_slices);
            let region_count = source_region_identities.len();
            assert_eq!(
                counters.contexts_constructed, region_count,
                "one immutable spatial context must be constructed per perimeter region"
            );
            assert!(
                counters.indexed_queries > 0,
                "accelerated native perimeter queries must exercise an index"
            );

            let region_identities = records
                .iter()
                .map(|record| (record.layer_index, record.region_ordinal))
                .collect::<BTreeSet<_>>();
            assert_eq!(
                region_identities, source_region_identities,
                "prepared-region capture must identify every processed region"
            );
            assert_eq!(
                context_rows.len(),
                region_count,
                "one accounting row must be opened for every constructed perimeter context"
            );
            let distance_record_counts = source_distance_record_counts(&source_slices);
            assert_eq!(
                distance_record_counts.len(),
                region_count,
                "distance-record populations must cover every perimeter region"
            );
            assert!(
                counters.exact_evaluations > 0,
                "indexed native perimeter queries must perform exact candidate checks"
            );
            assert!(
                context_rows.iter().any(|row| row.queries > 0),
                "at least one perimeter context must exercise indexed queries"
            );
            // Context rows and distance-record populations are paired in
            // construction order: layers are restored to sorted SliceIR order
            // above, and regions are constructed sequentially within each
            // layer.
            let distance_exact_evaluations: usize =
                context_rows.iter().map(|row| row.exact_evaluations).sum();
            let unpruned_distance_full_scan: usize = distance_record_counts
                .iter()
                .zip(&context_rows)
                .map(|(record_count, row)| {
                    record_count
                        .checked_mul(row.queries)
                        .expect("per-context distance full-scan candidate count overflowed")
                })
                .sum();
            assert!(
                distance_exact_evaluations > 0,
                "indexed native contexts must perform distance exact candidate checks"
            );
            // DISTANCE-FAMILY-SPECIFIC pruning oracle:
            // Σ eᵢ < Σ (cᵢ × qᵢ), where eᵢ/qᵢ are this row's distance-edge
            // exact-evaluation/query counters and cᵢ is the corresponding
            // distance-record population. With zero pruning, every distance
            // query scans all cᵢ records, so eᵢ == cᵢ × qᵢ per row and the
            // strict inequality fails exactly, for any number of contexts.
            assert!(
                distance_exact_evaluations < unpruned_distance_full_scan,
                "per-context distance exact evaluations ({distance_exact_evaluations}) must be below the unpruned distance full-scan total ({unpruned_distance_full_scan})"
            );
        }

        let (perimeters, source_slices, counters, _records, _context_rows) = run_native_pipeline(
            &arachne_second_pass_input(),
            &config_path,
            &module_dirs,
            WallGenerator::Arachne,
            true,
        )
        .expect("Arachne only_one_wall_top pipeline");
        let region_count = source_region_identities(&source_slices).len();
        assert!(
            !perimeters.is_empty(),
            "Arachne multi-pass output must be nonempty"
        );
        assert!(
            arachne_second_pass_region_count(&source_slices) > 0,
            "only_one_wall_top fixture must contain a non-topmost region with a nonempty exposed top sub-area"
        );
        // Arachne emits no distinct second-pass marker. The qualifying source
        // geometry above makes the pass run; context equality proves both
        // passes reused one region-owned spatial context.
        assert_eq!(counters.contexts_constructed, region_count);
    }

    #[test]
    fn perimeter_spatial_self_baseline_native_indexed() {
        let (mesh_path, config_path, module_dirs) = fixture_inputs();
        let (run, _outer_counters, _outer_records) = with_capture(true, false, || {
            run_native_pipeline(
                &mesh_path,
                &config_path,
                &module_dirs,
                WallGenerator::Classic,
                true,
            )
        });
        let (perimeters, _source_slices, _counters, _records, _context_rows) =
            run.expect("indexed native perimeter pipeline");
        let observed = postcard_bytes(&perimeters);
        let baseline =
            required_baseline("native_indexed").expect("read native indexed perimeter baseline");
        assert_eq!(
            observed, baseline,
            "indexed native PerimeterIR must match its own postcard baseline"
        );
    }

    #[test]
    fn perimeter_spatial_self_baseline_native_legacy() {
        let (mesh_path, config_path, module_dirs) = fixture_inputs();
        let (run, _outer_counters, _outer_records) = with_capture(false, false, || {
            run_native_pipeline(
                &mesh_path,
                &config_path,
                &module_dirs,
                WallGenerator::Classic,
                false,
            )
        });
        let (perimeters, _source_slices, _counters, _records, _context_rows) =
            run.expect("legacy native perimeter pipeline");
        let observed = postcard_bytes(&perimeters);
        let baseline =
            required_baseline("native_legacy").expect("read native legacy perimeter baseline");
        assert_eq!(
            observed, baseline,
            "legacy native PerimeterIR must match its own postcard baseline"
        );
    }

    #[test]
    fn perimeter_spatial_self_baseline_wasm() {
        let (mesh_path, config_path, module_dirs) = fixture_inputs();
        let (
            native_perimeters,
            _source_slices,
            _native_counters,
            _native_records,
            _native_context_rows,
        ) = run_native_pipeline(
            &mesh_path,
            &config_path,
            &module_dirs,
            WallGenerator::Classic,
            true,
        )
        .expect("native identity source perimeter pipeline");
        let native_identities = native_region_identities(&native_perimeters);
        assert!(
            native_identities.len() >= 2,
            "deterministic bridge input must produce at least two native regions; got {} identities across {} layers",
            native_identities.len(),
            native_perimeters.len()
        );

        let (
            wasm_perimeters,
            _wasm_source_slices,
            _wasm_counters,
            wasm_records,
            _wasm_context_rows,
        ) = run_wasm_pipeline(&mesh_path, &config_path, &module_dirs)
            .expect("WASM perimeter pipeline");
        let wasm_identities = wasm_records
            .iter()
            .map(|record| (record.layer_index, record.region_ordinal))
            .collect::<BTreeSet<_>>();
        assert!(
            wasm_identities.len() >= 2,
            "WASM dispatch must capture at least two filtered prepared regions; got {} from {} records",
            wasm_identities.len(),
            wasm_records.len()
        );
        assert!(
            wasm_identities.is_subset(&native_identities),
            "WASM prepared-region identities must come from the native output region set"
        );

        let observed = postcard_bytes(&wasm_perimeters);
        let baseline = required_baseline("wasm").expect("read WASM perimeter baseline");
        assert_eq!(
            observed, baseline,
            "WASM PerimeterIR must match its own postcard baseline"
        );
    }
}
