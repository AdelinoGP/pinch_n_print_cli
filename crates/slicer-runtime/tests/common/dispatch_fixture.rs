//! Fluent test fixture owning a dispatcher, Blackboard, and LayerArena.
//!
//! Provides `DispatchFixture::for_stage("…")` → builder → `.build()` → fixture
//! with four per-runner `run_*` methods. Default = real WAT-compiled test guest
//! + empty ConfigView. Use `.no_wasm()` for the MissingComponent graceful-skip path.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ConfigView, GCodeIR, GlobalLayer, LayerCollectionIR, PerimeterIR, SliceIR};
use slicer_runtime::{
    build_wasm_instance_pool, Blackboard, CompiledModuleBuilder, LayerArena, WasmArtifactMetadata,
    WasmEngine,
};
use slicer_wasm_host::{
    FinalizationStageRunner, PostpassStageRunner, PrepassStageRunner, WasmRuntimeDispatcher,
};

use crate::common::wasm_cache;
use crate::common::{
    finalization_input, postpass_input, prepass_input, run_layer_and_commit_with_bundle,
    TestModuleBundle,
};

/// Fixture owning dispatcher, blackboard, arena, and module bundle.
pub struct DispatchFixture {
    pub dispatcher: WasmRuntimeDispatcher,
    pub blackboard: Blackboard,
    pub arena: LayerArena,
    pub bundle: TestModuleBundle,
    stage_id: String,
}

impl DispatchFixture {
    /// Run a layer-stage dispatch and commit to the arena.
    pub fn run_layer(&mut self, layer: &GlobalLayer) -> Result<(), slicer_ir::LayerStageError> {
        let stage_id = self.stage_id().to_string();
        run_layer_and_commit_with_bundle(
            &self.dispatcher,
            &stage_id,
            layer,
            &self.bundle,
            &self.blackboard,
            &mut self.arena,
        )
    }

    /// Run a prepass-stage dispatch.
    pub fn run_prepass(
        &self,
    ) -> Result<slicer_core::PrepassStageOutput, slicer_ir::PrepassRunnerError> {
        let live = self.bundle.as_live();
        let input = prepass_input(&self.blackboard);
        PrepassStageRunner::run_stage(&self.dispatcher, &self.stage_id().to_string(), &live, input)
    }

    /// Run a finalization-stage dispatch.
    pub fn run_finalization(
        &self,
        layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<slicer_ir::FinalizationOutput, slicer_ir::FinalizationError> {
        let live = self.bundle.as_live();
        let input = finalization_input(&self.blackboard);
        FinalizationStageRunner::run_stage(
            &self.dispatcher,
            &self.stage_id().to_string(),
            &live,
            input,
            layers,
        )
    }

    /// Run a postpass-stage dispatch.
    pub fn run_postpass(
        &self,
        gcode: &mut GCodeIR,
    ) -> Result<slicer_ir::PostpassOutput, slicer_ir::PostpassError> {
        let live = self.bundle.as_live();
        let input = postpass_input(&self.blackboard);
        PostpassStageRunner::run_gcode_postprocess(
            &self.dispatcher,
            &self.stage_id().to_string(),
            &live,
            input,
            &mut gcode.commands,
        )
    }

    fn stage_id(&self) -> &str {
        &self.stage_id
    }
}

/// Builder for `DispatchFixture`.
pub struct DispatchFixtureBuilder {
    stage_id: String,
    slice: Option<SliceIR>,
    perimeter: Option<PerimeterIR>,
    config: ConfigView,
    wat: Option<String>,
    no_wasm: bool,
}

impl DispatchFixtureBuilder {
    /// Set the `SliceIR` to stage into the arena before dispatch.
    pub fn with_slice(mut self, ir: SliceIR) -> Self {
        self.slice = Some(ir);
        self
    }

    /// Set the `PerimeterIR` to stage into the arena before dispatch.
    pub fn with_perimeter(mut self, ir: PerimeterIR) -> Self {
        self.perimeter = Some(ir);
        self
    }

    /// Set the config view (default = empty).
    pub fn with_config(mut self, config: ConfigView) -> Self {
        self.config = config;
        self
    }

    /// Use a custom WAT component instead of the default test guest.
    pub fn with_wat(mut self, wat: &str) -> Self {
        self.wat = Some(wat.to_string());
        self
    }

    /// Opt out of real WASM (no compiled component) — for MissingComponent tests.
    pub fn no_wasm(mut self) -> Self {
        self.no_wasm = true;
        self
    }

    /// Build the `DispatchFixture`.
    pub fn build(self) -> DispatchFixture {
        let engine = wasm_cache::shared_engine();
        let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

        let bundle = if self.no_wasm {
            make_no_wasm_bundle(&self.stage_id, self.config)
        } else if let Some(wat) = self.wat {
            make_wat_bundle(&engine, &self.stage_id, &wat, self.config)
        } else {
            make_default_bundle(&engine, &self.stage_id, self.config)
        };

        let blackboard = Blackboard::new(
            Arc::new(slicer_ir::MeshIR::default()),
            if self.stage_id.starts_with("Layer::") {
                1
            } else {
                0
            },
        );
        let mut arena = LayerArena::new();

        if let Some(slice) = self.slice {
            arena.set_slice(slice).unwrap();
        }
        if let Some(perimeter) = self.perimeter {
            arena.set_perimeter(perimeter).unwrap();
        }

        DispatchFixture {
            dispatcher,
            blackboard,
            arena,
            bundle,
            stage_id: self.stage_id,
        }
    }
}

/// Entry point: begin building a `DispatchFixture` for the given stage.
pub fn for_stage(stage_id: &str) -> DispatchFixtureBuilder {
    DispatchFixtureBuilder {
        stage_id: stage_id.to_string(),
        slice: None,
        perimeter: None,
        config: ConfigView::from_map(HashMap::new()),
        wat: None,
        no_wasm: false,
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn make_default_bundle(
    _engine: &Arc<WasmEngine>,
    stage_id: &str,
    config: ConfigView,
) -> TestModuleBundle {
    use slicer_ir::SemVer;
    use slicer_runtime::manifest::LoadedModuleBuilder;

    let guest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../slicer-wasm-host/test-guests/layer-infill-guest.component.wasm"
    );
    let component = wasm_cache::compiled_component_at(std::path::Path::new(guest_path));

    let loaded = LoadedModuleBuilder::new(
        "com.test.fixture",
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage_id,
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

    let module = CompiledModuleBuilder::new("com.test.fixture")
        .config_view(Arc::new(config))
        .build();

    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn make_wat_bundle(
    _engine: &Arc<WasmEngine>,
    stage_id: &str,
    wat: &str,
    config: ConfigView,
) -> TestModuleBundle {
    use slicer_ir::SemVer;
    use slicer_runtime::manifest::LoadedModuleBuilder;

    let component = wasm_cache::compiled_wat(wat);

    let loaded = LoadedModuleBuilder::new(
        "com.test.fixture",
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage_id,
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

    let module = CompiledModuleBuilder::new("com.test.fixture")
        .config_view(Arc::new(config))
        .build();

    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn make_no_wasm_bundle(stage_id: &str, config: ConfigView) -> TestModuleBundle {
    use slicer_ir::SemVer;
    use slicer_runtime::manifest::LoadedModuleBuilder;

    let loaded = LoadedModuleBuilder::new(
        "com.test.fixture",
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage_id,
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

    let module = CompiledModuleBuilder::new("com.test.fixture")
        .config_view(Arc::new(config))
        .build();

    TestModuleBundle {
        module,
        pool,
        component: None,
    }
}
