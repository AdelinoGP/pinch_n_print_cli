//! Real frozen native/component preparation, with committed input and no ready-plan injection.

use std::{path::PathBuf, sync::Arc};

use preparation_visual_guest::PreparationVisualGuest;
use slicer_ir::{
    ConfigValue, ConfigView, GlobalLayer, LayerPlanIR, MeshIR, RegionMapIR, RegionPlan,
    ResolvedConfig, SliceIR, SlicedRegion,
};
use slicer_schema::preparation::PreparationDeclaration;
use slicer_wasm_host::{
    host::HostExecutionContextBuilder,
    preparation::{
        freeze_native, freeze_wasm, CommittedPreparationInputs, FrozenPreparationBinding,
        NativePreparationArtifact, PreparationFailure, PreparationInputData, PreparationOwner,
        PreparationProjectionRequest, PreparationSelectionContext, PreparedOwnerStore,
        StagedPreparation,
    },
};
use wasmtime::component::Component;

pub const MODULE: &str = "test.preparation-visual";
pub const VIEW: &str = "probe_xy";
pub const STAGE: &str = "Layer::PathOptimization";

#[derive(Clone, Copy, Debug)]
pub enum Leg {
    Native,
    Component,
}

pub fn artifact_path() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-guests/preparation-visual-guest.component.wasm");
    assert!(
        path.is_file(),
        "required preparation visual artifact missing: {} (no skip or replacement permitted)",
        path.display()
    );
    path
}

pub fn declaration(views: Vec<String>) -> PreparationDeclaration {
    PreparationDeclaration::new(
        "slicer:layer-preparation/prepare@1.0.0",
        vec!["layer_plan".into(), "selected_targets".into()],
        views,
    )
}

pub fn binding(leg: Leg) -> FrozenPreparationBinding {
    let declaration = Some(declaration(vec![VIEW.into()]));
    match leg {
        Leg::Native => freeze_native(
            MODULE.into(),
            declaration,
            NativePreparationArtifact::Prepared {
                entry: PreparationVisualGuest::__slicer_prepared_native_entry(),
                schema: PreparationVisualGuest::__SLICER_PREPARATION_SCHEMA,
            },
        ),
        Leg::Component => {
            let engine = crate::common::wasm_cache::shared_engine()
                .wasmtime_engine()
                .clone();
            let path = artifact_path();
            let component = Component::from_file(&engine, &path)
                .unwrap_or_else(|error| panic!("cannot load {}: {error:#}", path.display()));
            freeze_wasm(MODULE.into(), declaration, engine, component, STAGE.into())
        }
    }
    .expect("freeze actual same-source visual fixture")
}

pub type Prepare = fn(
    &slicer_sdk::preparation::PrintPreparationInput,
    &mut slicer_sdk::preparation::PreparedPlanOutput,
) -> Result<(), slicer_sdk::error::ModuleError>;

/// Supplemental native boundary cases the fixed component fixture cannot express.
/// The output callbacks, transaction, validator and ordinary entry remain production ones.
pub fn native_case(prepare: Prepare, views: Vec<String>) -> FrozenPreparationBinding {
    let mut entry = PreparationVisualGuest::__slicer_prepared_native_entry();
    entry.prepare = prepare;
    freeze_native(
        MODULE.into(),
        Some(declaration(views)),
        NativePreparationArtifact::Prepared {
            entry,
            schema: PreparationVisualGuest::__SLICER_PREPARATION_SCHEMA,
        },
    )
    .expect("freeze supplemental native validator case")
}

/// Seed actual committed schedule/selection; config controls are fixture-declared keys.
pub fn input(
    owner: &PreparationOwner,
    mode: &str,
    token: &str,
    two_layers: bool,
) -> PreparationInputData {
    let config = Arc::new(ConfigView::from_map(
        [
            (
                "preparation_visual_mode".into(),
                ConfigValue::String(mode.into()),
            ),
            (
                "preparation_visual_token".into(),
                ConfigValue::String(token.into()),
            ),
        ]
        .into_iter()
        .collect(),
    ));
    let mut region_map = RegionMapIR::default();
    // Model target values come from the resolved region config; invocation
    // config supplies the declared-key filter, not replacement target values.
    let mut resolved = ResolvedConfig::default();
    resolved.extensions.insert(
        "preparation_visual_mode".into(),
        ConfigValue::String(mode.into()),
    );
    resolved.extensions.insert(
        "preparation_visual_token".into(),
        ConfigValue::String(token.into()),
    );
    let config_id = region_map.intern_config(resolved);
    let mut layers = Vec::new();
    let mut slices = Vec::new();
    for (index, z) in if two_layers {
        &[(0, 0.2), (1, 0.4)][..]
    } else {
        &[(0, 0.2)][..]
    } {
        let region = SlicedRegion {
            object_id: "probe-object".into(),
            region_id: 7,
            effective_layer_height: 0.2,
            ..Default::default()
        };
        let key = slicer_wasm_host::selection::region_map_key_for_sliced_region(*index, &region);
        region_map.entries.insert(
            key,
            RegionPlan {
                config: config_id,
                ..Default::default()
            },
        );
        layers.push(GlobalLayer {
            index: *index,
            z: *z,
            ..Default::default()
        });
        slices.push(SliceIR {
            global_layer_index: *index,
            z: *z,
            regions: vec![region],
            ..Default::default()
        });
    }
    let committed = CommittedPreparationInputs::new(
        Arc::new(MeshIR::default()),
        Arc::new(LayerPlanIR {
            global_layers: layers,
            ..Default::default()
        }),
        Arc::new(slices),
        Arc::new(region_map),
        None,
        None,
        None,
        None,
        None,
    )
    .expect("commit literal schedule and selected region");
    let input = PreparationInputData::from_committed(
        owner,
        Arc::new(committed),
        PreparationSelectionContext::new(
            STAGE.into(),
            MODULE.into(),
            vec!["path-optimizer".into()],
            Default::default(),
            config,
        ),
    )
    .expect("bind actual shared selection to frozen owner");
    // Regression control for invocation-only seeds: missing target controls
    // would silently select the fixture's valid/PRB1 defaults in fault tests.
    let targets = input.host_selected_targets(owner, 0, 0, u64::MAX).unwrap();
    assert_eq!(targets.len(), 1);
    assert!(targets[0].invoke);
    assert_eq!(targets[0].stage_id, STAGE);
    assert_eq!(
        targets[0].config.get_string("preparation_visual_mode"),
        Some(mode)
    );
    assert_eq!(
        targets[0].config.get_string("preparation_visual_token"),
        Some(token)
    );
    input
}

pub fn request() -> PreparationProjectionRequest {
    PreparationProjectionRequest::new(vec![VIEW.into()]).expect("literal requested view")
}

pub struct Print {
    pub binding: FrozenPreparationBinding,
    pub store: PreparedOwnerStore,
    pub owner: PreparationOwner,
}

impl Print {
    pub fn new(leg: Leg) -> Self {
        let binding = binding(leg);
        let store = PreparedOwnerStore::new();
        let owner = store
            .create_owner(&binding)
            .expect("create real print owner");
        Self {
            binding,
            store,
            owner,
        }
    }

    pub fn prepare(
        &self,
        mode: &str,
        token: &str,
        two_layers: bool,
    ) -> Result<(), PreparationFailure> {
        self.prepare_validated(mode, token, two_layers, |_| Ok(()))
    }

    pub fn prepare_validated(
        &self,
        mode: &str,
        token: &str,
        two_layers: bool,
        validate: impl FnOnce(&StagedPreparation) -> Result<(), PreparationFailure>,
    ) -> Result<(), PreparationFailure> {
        self.binding.prepare_with_diagnostics(
            &self.owner,
            input(&self.owner, mode, token, two_layers),
            request(),
            validate,
        )
    }

    /// Observe the real whole-call guard, on both actual adapter families.
    pub fn with_active_call(&self, observe: impl FnOnce()) {
        if self.binding.artifact_kind() == "native" {
            self.binding
                .with_native_plan(&self.owner, |_| {
                    observe();
                    Ok(())
                })
                .expect("active native production scope");
        } else {
            let context = HostExecutionContextBuilder::new(MODULE, 0.2, 0.2).build();
            self.binding
                .with_wasm_plan(&self.owner, context, |_, _| {
                    observe();
                    Ok(())
                })
                .expect("active component production scope");
        }
    }
}
