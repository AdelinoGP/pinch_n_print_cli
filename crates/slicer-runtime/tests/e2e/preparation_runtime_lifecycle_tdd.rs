//! Real-slice lifecycle regression driver. Registration supplies adapters, not plans.
//!
//! The preparation-only entry point is deliberately exercised separately from
//! ordinary decoding: metadata capture must never become an opaque-payload reader.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex};

use preparation_lifecycle_anchored_guest::PreparationLifecycleAnchored;
use preparation_lifecycle_guest::PreparationLifecycleGuest;
use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredGeometryContract, ConfigValue, ExtrusionRole,
    Point3WithWidth,
};
use slicer_runtime::preparation_lifecycle::{
    AnchoredHostClosureStatus, LayerArenaLifecycleStatus, PreparationCapture,
    PreparationLifecycleObserver, PreparationLifecyclePhase, PreparationLifecycleRecord,
    PreparationLifecycleStatus, SliceRuntimeExtensions,
};
use slicer_runtime::{
    run_slice, run_slice_preparation_capture, run_slice_with_runtime_extensions, SliceRunOptions,
};
use slicer_scheduler::IntegratedModuleRegistration;
use slicer_sdk::preparation::PreparationTargetSource;
use slicer_wasm_host::preparation::PreparationFailure;
use tempfile::TempDir;

const PATH_ID: &str = "com.test.preparation-lifecycle-guest";
const ANCHORED_ID: &str = "com.test.preparation-lifecycle-anchored-guest";
const PATH_MANIFEST: &str =
    include_str!("../../../slicer-wasm-host/test-guests/preparation-lifecycle-guest/module.toml");
const ANCHORED_MANIFEST: &str = include_str!(
    "../../../slicer-wasm-host/test-guests/preparation-lifecycle-anchored-guest/module.toml"
);
const NATIVE_MARKER: &str = "PREPARATION-LIFECYCLE-NATIVE";
const WASM_MARKER: &str = "PREPARATION-LIFECYCLE-WASM";

// Compile witnesses only: these four stages deliberately do not claim driver
// coverage. Each expansion is a distinct, single-stage module.
mod compile_shapes {
    use slicer_ir::{ConfigView, InfillRegion};
    use slicer_sdk::builders::SlicePostprocessBuilder;
    use slicer_sdk::builders::{InfillOutputBuilder, PerimeterOutputBuilder, SupportOutputBuilder};
    use slicer_sdk::error::ModuleError;
    use slicer_sdk::preparation::{LayerPreparation, PreparedPlanOutput, PrintPreparationInput};
    use slicer_sdk::traits::LayerModule;
    use slicer_sdk::views::{PerimeterRegionView, SliceRegionView};
    use slicer_sdk::PaintRegionLayerView;

    macro_rules! pair {
        ($prepared:ident, $plain:ident, $method:ident, $($arg:ident: $ty:ty),*) => {
            pub struct $prepared;
            pub struct $plain;
            impl LayerPreparation for $prepared {
                fn prepare_print(_: &PrintPreparationInput, _: &mut PreparedPlanOutput) -> Result<(), ModuleError> { Ok(()) }
            }
            #[slicer_sdk::slicer_module(preparation)]
            impl LayerModule for $prepared {
                fn from_config(_: &ConfigView) -> Result<Self, ModuleError> { Ok(Self) }
                fn $method(&self, $($arg: $ty),*) -> Result<(), ModuleError> { Ok(()) }
            }
            #[slicer_sdk::slicer_module]
            impl LayerModule for $plain {
                fn from_config(_: &ConfigView) -> Result<Self, ModuleError> { Ok(Self) }
                fn $method(&self, $($arg: $ty),*) -> Result<(), ModuleError> { Ok(()) }
            }
        };
    }
    pair!(SlicePrepared, SlicePlain, run_slice_postprocess,
        _index: u32, _regions: &[SliceRegionView], _paint: &PaintRegionLayerView,
        _out: &mut SlicePostprocessBuilder, _config: &ConfigView);
    pair!(WallPrepared, WallPlain, run_wall_postprocess,
        _index: u32, _regions: &[PerimeterRegionView],
        _out: &mut PerimeterOutputBuilder, _config: &ConfigView);
    pair!(InfillPrepared, InfillPlain, run_infill_postprocess,
        _index: u32, _regions: &[PerimeterRegionView], _prior: &[InfillRegion],
        _out: &mut InfillOutputBuilder, _config: &ConfigView);
    pair!(SupportPrepared, SupportPlain, run_support_postprocess,
        _index: u32, _regions: &[SliceRegionView],
        _out: &mut SupportOutputBuilder, _config: &ConfigView);

    pub fn stages() -> std::collections::BTreeSet<&'static str> {
        let prepared = [
            SlicePrepared::__slicer_stage_name(),
            WallPrepared::__slicer_stage_name(),
            InfillPrepared::__slicer_stage_name(),
            SupportPrepared::__slicer_stage_name(),
        ];
        let plain = [
            SlicePlain::__slicer_stage_name(),
            WallPlain::__slicer_stage_name(),
            InfillPlain::__slicer_stage_name(),
            SupportPlain::__slicer_stage_name(),
        ];
        // These typed values force the prepared native sidecars to compile too.
        let entries = [
            SlicePrepared::__slicer_prepared_native_entry(),
            WallPrepared::__slicer_prepared_native_entry(),
            InfillPrepared::__slicer_prepared_native_entry(),
            SupportPrepared::__slicer_prepared_native_entry(),
        ];
        assert_eq!(entries.len(), prepared.len());
        assert_eq!(prepared, plain);
        prepared.into_iter().collect()
    }
}

#[derive(Debug)]
enum Event {
    Lifecycle(PreparationLifecycleRecord),
    Arena(u32, LayerArenaLifecycleStatus),
    Host(AnchoredHostClosureStatus),
}

#[derive(Default)]
struct Trace {
    events: Mutex<Vec<Event>>,
    // Cooperative checkpoints can be triggered by a real observation, without
    // injecting Ready or substituting a dispatcher/producer backing.
    cancel: Option<(Arc<AtomicBool>, PreparationLifecyclePhase)>,
    cancel_on_ready: bool,
}

impl PreparationLifecycleObserver for Trace {
    fn on_preparation_lifecycle(&self, record: &PreparationLifecycleRecord) {
        if let Some((flag, phase)) = &self.cancel {
            if std::mem::discriminant(&record.lifecycle_phase) == std::mem::discriminant(phase)
                && record.status
                    == if self.cancel_on_ready {
                        PreparationLifecycleStatus::Ready
                    } else {
                        PreparationLifecycleStatus::Start
                    }
            {
                flag.store(true, Ordering::SeqCst);
            }
        }
        self.events
            .lock()
            .unwrap()
            .push(Event::Lifecycle(record.clone()));
    }

    fn on_layer_arena(&self, index: u32, status: LayerArenaLifecycleStatus) {
        self.events
            .lock()
            .unwrap()
            .push(Event::Arena(index, status));
    }

    fn on_anchored_host_closure(&self, status: AnchoredHostClosureStatus) {
        self.events.lock().unwrap().push(Event::Host(status));
    }
}

fn workspace_root() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo test");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

struct Fixture {
    staging: TempDir,
    external: bool,
    anchored: bool,
    trace: Arc<Trace>,
}

impl Fixture {
    fn new(external: bool, anchored: bool) -> Self {
        let fixture = Self {
            staging: TempDir::new().expect("external fixture staging"),
            external,
            anchored,
            trace: Arc::new(Trace::default()),
        };
        if external {
            fixture.stage_component("preparation-lifecycle-guest", PATH_MANIFEST);
            if anchored {
                fixture.stage_component("preparation-lifecycle-anchored-guest", ANCHORED_MANIFEST);
            }
        }
        fixture
    }

    fn stage_component(&self, stem: &str, manifest: &str) {
        let component = workspace_root()
            .join("crates/slicer-wasm-host/test-guests")
            .join(format!("{stem}.component.wasm"));
        assert!(
            component.is_file(),
            "required staged component absent: {}",
            component.display()
        );
        std::fs::write(self.staging.path().join(format!("{stem}.toml")), manifest)
            .expect("write real external manifest");
        std::fs::copy(&component, self.staging.path().join(format!("{stem}.wasm")))
            .expect("stage real external component");
    }

    fn options(&self) -> SliceRunOptions {
        let root = workspace_root();
        let model = root.join("resources").join("regression_wedge.stl");
        let module_dir = root.join("modules").join("core-modules");
        assert!(model.exists(), "wedge missing at {}", model.display());
        assert!(
            module_dir.exists(),
            "core modules missing at {}",
            module_dir.display()
        );
        // Selection is manifest-first: do not discover the core implementation
        // of a stage owned by a lifecycle fixture. Keep every other wedge
        // module (including all fill holders) available unchanged.
        let mut module_dirs = vec![self.staging.path().to_owned()];
        for entry in std::fs::read_dir(&module_dir).expect("read wedge module roots") {
            let path = entry.expect("core module directory").path();
            if !path.is_dir() {
                continue;
            }
            let stem = path.file_name().expect("core module name");
            let manifest_path = path.join(stem).with_extension("toml");
            let manifest: toml::Value = std::fs::read_to_string(&manifest_path)
                .unwrap_or_else(|error| panic!("read {}: {error}", manifest_path.display()))
                .parse()
                .expect("parse core module manifest");
            let stage = manifest["stage"]["id"].as_str().expect("core stage");
            if stage != "Layer::PathOptimization"
                && !(self.anchored && stage == "Layer::AnchoredEvents")
            {
                module_dirs.push(path);
            }
        }
        module_dirs.sort();
        let mesh = Arc::new(slicer_model_io::load_model(&model).expect("model load must succeed"));
        let mut opts = SliceRunOptions {
            mesh,
            model_label: model.to_string_lossy().into_owned(),
            module_dirs,
            no_default_module_paths: true,
            ..Default::default()
        };
        opts.config_overrides.extend([
            (
                "preparation_probe_mode".into(),
                ConfigValue::String("ready".into()),
            ),
            (
                "preparation_probe_token".into(),
                ConfigValue::String("probe".into()),
            ),
            ("preparation_probe_length".into(), ConfigValue::Int(4)),
            (
                "probe_start_mm".into(),
                ConfigValue::Int(if self.external { 5 } else { 3 }),
            ),
            (
                "probe_end_mm".into(),
                ConfigValue::Int(if self.external { 6 } else { 4 }),
            ),
            ("probe_mode".into(), ConfigValue::String("standard".into())),
        ]);
        opts
    }

    fn extensions(&self, host_entities: Vec<AnchoredEntity>) -> SliceRuntimeExtensions {
        let mut integrated = vec![IntegratedModuleRegistration {
            manifest_toml: PATH_MANIFEST,
            origin_label: "lifecycle-native-path",
        }];
        let mut prepared = vec![(
            PATH_ID.into(),
            PreparationLifecycleGuest::__slicer_prepared_native_entry(),
            PreparationLifecycleGuest::__SLICER_PREPARATION_SCHEMA,
        )];
        if self.anchored {
            integrated.push(IntegratedModuleRegistration {
                manifest_toml: ANCHORED_MANIFEST,
                origin_label: "lifecycle-native-anchored",
            });
            prepared.push((
                ANCHORED_ID.into(),
                PreparationLifecycleAnchored::__slicer_prepared_native_entry(),
                PreparationLifecycleAnchored::__SLICER_PREPARATION_SCHEMA,
            ));
        }
        SliceRuntimeExtensions::new(
            integrated,
            prepared,
            host_entities,
            Some(self.trace.clone()),
        )
    }

    fn capture(&self, length: i64, token: &str) -> PreparationCapture {
        let mut opts = self.options();
        opts.config_overrides
            .insert("preparation_probe_length".into(), ConfigValue::Int(length));
        opts.config_overrides.insert(
            "preparation_probe_token".into(),
            ConfigValue::String(token.into()),
        );
        run_slice_preparation_capture(opts, self.extensions(vec![]))
            .expect("real production capture prefix")
    }

    fn run(&self) -> String {
        run_slice_with_runtime_extensions(self.options(), self.extensions(vec![]))
            .expect("real prepared wedge slice")
            .gcode_text
    }

    fn marker(&self) -> &'static str {
        if self.external {
            WASM_MARKER
        } else {
            NATIVE_MARKER
        }
    }
}

fn records(events: &[Event]) -> Vec<&PreparationLifecycleRecord> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Lifecycle(record) => Some(record),
            _ => None,
        })
        .collect()
}

fn assert_disposed(events: &[Event], id: &str) {
    let disposed = records(events)
        .into_iter()
        .filter(|record| {
            record.module_id.as_str() == id
                && matches!(record.status, PreparationLifecycleStatus::Disposed)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        disposed.len(),
        1,
        "owner must be disposed exactly once: {id}"
    );
    assert_eq!(disposed[0].retained_bytes, 0);
    assert!(matches!(
        disposed[0].lifecycle_phase,
        PreparationLifecyclePhase::Disposal
    ));
}

fn assert_no_consumer(events: &[Event]) {
    assert!(!records(events).iter().any(|record| matches!(
        record.lifecycle_phase,
        PreparationLifecyclePhase::Consumption
    )));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Event::Arena(_, _) | Event::Host(_))));
}

#[test]
fn activation_order_and_final_inputs() {
    for external in [false, true] {
        let fixture = Fixture::new(external, true);
        let output = fixture.run();
        assert!(output.contains(fixture.marker()));
        let events = fixture.trace.events.lock().unwrap();
        let starts: Vec<_> = records(&events)
            .into_iter()
            .filter(|record| {
                matches!(
                    record.lifecycle_phase,
                    PreparationLifecyclePhase::Preparation
                ) && matches!(record.status, PreparationLifecycleStatus::Start)
            })
            .collect();
        assert_eq!(
            starts
                .iter()
                .map(|r| r.module_id.to_string())
                .collect::<Vec<_>>(),
            [ANCHORED_ID, PATH_ID]
        );
        let first_arena = events
            .iter()
            .position(|e| matches!(e, Event::Arena(_, LayerArenaLifecycleStatus::Created)))
            .expect("real layer arena creation");
        let first_consumer = events.iter().position(|e| matches!(e, Event::Lifecycle(r) if matches!(r.lifecycle_phase, PreparationLifecyclePhase::Consumption)))
            .expect("real ordinary prepared consumption");
        for id in [ANCHORED_ID, PATH_ID] {
            let ready: Vec<_> = events.iter().enumerate().filter(|(_, event)| {
                matches!(event, Event::Lifecycle(r) if r.module_id.as_str() == id && matches!(r.status, PreparationLifecycleStatus::Ready))
            }).collect();
            assert_eq!(ready.len(), 1);
            assert!(ready[0].0 < first_arena && ready[0].0 < first_consumer);
            assert_disposed(&events, id);
        }
    }
}

#[test]
fn external_override_dual_leg() {
    for external in [false, true] {
        let fixture = Fixture::new(external, false);
        let output = fixture.run();
        assert!(output.contains(fixture.marker()));
        assert!(!output.contains(if external { NATIVE_MARKER } else { WASM_MARKER }));
        let events = fixture.trace.events.lock().unwrap();
        let owner_records: Vec<_> = records(&events)
            .into_iter()
            .filter(|r| r.module_id.as_str() == PATH_ID)
            .collect();
        assert!(!owner_records.is_empty());
        let identity = &owner_records[0].artifact_identity;
        assert!(!identity.is_empty());
        for record in owner_records {
            assert_eq!(record.module_version, "0.1.0");
            assert_eq!(&record.artifact_identity, identity);
            assert_eq!(record.consumer_stage.to_string(), "Layer::PathOptimization");
        }
        assert_disposed(&events, PATH_ID);
    }
}

#[test]
fn unused_nondefault_and_plain_empty() {
    // Baseline plain execution is an independent control, not a fabricated Ready
    // observation. Region-selected/empty-target variants still need extension.
    let fixture = Fixture::new(false, false);
    let mut plain_options = fixture.options();
    plain_options.module_dirs = vec![workspace_root().join("modules/core-modules")];
    let plain = run_slice(plain_options).expect("ordinary wedge control");
    assert!(!plain.gcode_text.is_empty());
    assert!(!plain.gcode_text.contains(NATIVE_MARKER));
    assert!(fixture.trace.events.lock().unwrap().is_empty());
    assert!(fixture.run().contains(NATIVE_MARKER));

    use selection_controls::{CALLS, PREPARES};
    use slicer_sdk::native::{NativePreparedStageEntry, NativeStageEntry};
    for counter in CALLS.iter().chain(PREPARES.iter()) {
        counter.store(0, Ordering::SeqCst);
    }
    let manifests = [
        selection_controls::manifest("com.test.selection-plain", false, "traditional"),
        selection_controls::manifest("com.test.selection-region", true, "tree"),
        selection_controls::manifest("com.test.selection-unused", true, "unused"),
    ];
    fixture.stage_component("sdk-layer-pathopt-guest", manifests[0]);
    let extensions = || {
        SliceRuntimeExtensions::new(
            manifests
                .iter()
                .skip(1)
                .map(|manifest| IntegratedModuleRegistration {
                    manifest_toml: manifest,
                    origin_label: "selection-control",
                })
                .collect(),
            [selection_controls::region, selection_controls::unused]
                .into_iter()
                .enumerate()
                .map(|(index, stage)| {
                    (
                        ["com.test.selection-region", "com.test.selection-unused"][index].into(),
                        NativePreparedStageEntry {
                            stage: NativeStageEntry::Layer(stage),
                            prepare: [
                                selection_controls::prepare_region,
                                selection_controls::prepare_unused,
                            ][index],
                        },
                        PreparationLifecycleGuest::__SLICER_PREPARATION_SCHEMA,
                    )
                })
                .collect(),
            vec![],
            None,
        )
    };
    let mut options = fixture.options();
    options.config_overrides.insert(
        "support_type".into(),
        ConfigValue::String("normal(auto)".into()),
    );
    let object = &mut Arc::make_mut(&mut options.mesh).objects[0];
    object.id = "selection-region-object".into();
    object.config.data.insert(
        "support_type".into(),
        ConfigValue::String("tree(auto)".into()),
    );
    object
        .config
        .data
        .insert("line_width".into(), ConfigValue::Float(0.9));
    let capture = run_slice_preparation_capture(options.clone(), extensions())
        .expect("nondefault region capture");
    assert_eq!(
        PREPARES[0].load(Ordering::SeqCst),
        0,
        "plain has no preparation export"
    );
    assert_eq!(PREPARES[1].load(Ordering::SeqCst), 1);
    assert_eq!(
        PREPARES[2].load(Ordering::SeqCst),
        0,
        "owned but unselected"
    );
    assert!(CALLS
        .iter()
        .all(|counter| counter.load(Ordering::SeqCst) == 0));
    assert_eq!(capture.owners.len(), 1);
    let owner = &capture.owners[0];
    assert_eq!(owner.module_id.as_str(), "com.test.selection-region");
    assert!(!owner.selected_targets.is_empty());
    for target in &owner.selected_targets {
        assert_eq!(
            target.source,
            PreparationTargetSource::Model {
                key: slicer_sdk::preparation::PreparationRegionKey {
                    global_layer_index: target.global_layer_index,
                    object_id: "selection-region-object".into(),
                    region_id: 0,
                    variant_chain: vec![],
                },
                members: vec![slicer_sdk::preparation::PreparationRegionKey {
                    global_layer_index: target.global_layer_index,
                    object_id: "selection-region-object".into(),
                    region_id: 0,
                    variant_chain: vec![],
                }],
            }
        );
        assert_eq!(target.config.get_float("line_width"), Some(0.9));
        assert_eq!(target.held_claims, Vec::<String>::new());
    }
    PREPARES[1].store(0, Ordering::SeqCst);
    run_slice_with_runtime_extensions(options, extensions()).expect("nondefault ordinary dispatch");
    assert_eq!(PREPARES[1].load(Ordering::SeqCst), 1);
    assert!(CALLS[1].load(Ordering::SeqCst) > 0);
    assert_eq!(PREPARES[2].load(Ordering::SeqCst), 0);
    assert_eq!(CALLS[2].load(Ordering::SeqCst), 0);

    // A vertical zero-volume sheet has a real schedule but no closed model regions.
    // Both artifacts are paint-transparent and receive identical empty input.
    for prepared in [false, true] {
        let fixture = Fixture::new(false, false);
        let id = "com.test.selection-empty";
        let manifest = selection_controls::manifest(id, prepared, "");
        if !prepared {
            fixture.stage_component("sdk-layer-pathopt-guest", manifest);
        }
        let mut options = fixture.options();
        options.no_integrated_modules = true;
        options.profile = true;
        options.mesh = Arc::new(elevated_support_mesh());
        options
            .module_dirs
            .retain(|path| path.file_name().is_none_or(|name| name != "seam-placer"));
        let object = &mut Arc::make_mut(&mut options.mesh).objects[0];
        object.modifier_volumes.clear();
        object
            .mesh
            .vertices
            .iter_mut()
            .for_each(|point| point.x = 0.0);
        PREPARES[1].store(0, Ordering::SeqCst);
        CALLS[1].store(0, Ordering::SeqCst);
        let extensions = SliceRuntimeExtensions::new(
            if prepared {
                vec![IntegratedModuleRegistration {
                    manifest_toml: manifest,
                    origin_label: "empty-prepared",
                }]
            } else {
                vec![]
            },
            if prepared {
                vec![(
                    id.into(),
                    NativePreparedStageEntry {
                        stage: NativeStageEntry::Layer(selection_controls::region),
                        prepare: selection_controls::prepare_region,
                    },
                    PreparationLifecycleGuest::__SLICER_PREPARATION_SCHEMA,
                )]
            } else {
                vec![]
            },
            vec![],
            Some(fixture.trace.clone()),
        );
        let output = run_slice_with_runtime_extensions(options, extensions)
            .expect("empty-target production slice");
        assert_eq!(PREPARES[1].load(Ordering::SeqCst), 0);
        assert_eq!(CALLS[1].load(Ordering::SeqCst), 0);
        if prepared {
            assert!(!output.gcode_text.contains(NATIVE_MARKER));
            assert!(records(&fixture.trace.events.lock().unwrap())
                .iter()
                .all(|record| record.module_id.as_str() != id));
        } else {
            assert!(
                output.profile.as_ref().expect("ordinary dispatch profile").modules.iter().any(|row| row.module_id == id && row.calls > 0),
                "plain empty-target ordinary export must execute (empty-layer G-code annotations are not emitted)"
            );
        }
    }
}

mod selection_controls {
    use super::*;
    use slicer_sdk::error::ModuleError;
    use slicer_sdk::native::{NativeLayerRequest, NativeLayerResponse, NativeStageEntry};
    use slicer_sdk::preparation::{PreparedPlanOutput, PrintPreparationInput};
    use std::sync::atomic::AtomicUsize;

    pub static CALLS: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
    pub static PREPARES: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];

    macro_rules! control {
        ($call:ident, $prepare:ident, $index:expr) => {
            pub fn $call(request: &NativeLayerRequest) -> Result<NativeLayerResponse, ModuleError> {
                CALLS[$index].fetch_add(1, Ordering::SeqCst);
                let NativeStageEntry::Layer(entry) =
                    PreparationLifecycleGuest::__slicer_prepared_native_entry().stage
                else {
                    unreachable!()
                };
                entry(request)
            }
            pub fn $prepare(
                input: &PrintPreparationInput,
                output: &mut PreparedPlanOutput,
            ) -> Result<(), ModuleError> {
                PREPARES[$index].fetch_add(1, Ordering::SeqCst);
                (PreparationLifecycleGuest::__slicer_prepared_native_entry().prepare)(input, output)
            }
        };
    }
    control!(region, prepare_region, 1);
    control!(unused, prepare_unused, 2);

    pub fn manifest(id: &str, prepared: bool, family: &str) -> &'static str {
        let mut manifest: toml::Value = PATH_MANIFEST.parse().expect("control manifest");
        manifest["module"]["id"] = id.into();
        manifest["claims"]["holds"] = toml::Value::Array(if family.is_empty() {
            vec!["path-optimizer".into()]
        } else {
            vec![format!("support-family:{family}").into()]
        });
        if !prepared {
            manifest.as_table_mut().unwrap().remove("preparation");
        }
        manifest["config"]["schema"].as_table_mut().unwrap().insert(
            "line_width".into(),
            toml::Value::Table(toml::map::Map::from_iter([
                ("type".into(), "float".into()),
                ("default".into(), 0.8.into()),
            ])),
        );
        Box::leak(toml::to_string(&manifest).unwrap().into_boxed_str())
    }
}

// Closed solids, unlike the prepass-only open plate in
// crates/slicer-runtime/tests/executor/prepass_support_geometry_tdd.rs::overhang_plate_mesh.
// The roof's source is absent below Z=2, leaving real support-only carriers.
fn elevated_support_mesh() -> slicer_ir::MeshIR {
    fn cuboid(id: &str, xy: f32, bottom: f32, top: f32) -> slicer_ir::ObjectMesh {
        slicer_ir::ObjectMesh {
            id: id.into(),
            world_z_extent: Some((bottom, top)),
            mesh: slicer_ir::IndexedTriangleSet {
                vertices: [bottom, top]
                    .into_iter()
                    .flat_map(|z| {
                        [(0.0, 0.0), (xy, 0.0), (xy, xy), (0.0, xy)]
                            .into_iter()
                            .map(move |(x, y)| slicer_ir::Point3 { x, y, z })
                    })
                    .collect(),
                indices: vec![
                    0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3,
                    7, 2, 7, 6, 3, 0, 4, 3, 4, 7,
                ],
            },
            transform: slicer_ir::Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            ..Default::default()
        }
    }
    let mut object = cuboid("step9-source", 2.0, 0.0, 0.8);
    let roof = cuboid("roof", 8.0, 2.0, 2.8);
    let offset = object.mesh.vertices.len() as u32;
    object
        .mesh
        .vertices
        .extend(roof.mesh.vertices.iter().copied());
    object
        .mesh
        .indices
        .extend(roof.mesh.indices.iter().map(|i| i + offset));
    object.world_z_extent = Some((0.0, 2.8));
    object.modifier_volumes.push(slicer_ir::ModifierVolume::new(
        "step9-roof".into(),
        roof.mesh,
        slicer_ir::ConfigDelta {
            fields: std::collections::HashMap::from([(
                "support_type".into(),
                ConfigValue::String("normal(auto)".into()),
            )]),
        },
        0,
        slicer_ir::ModifierKind::ParameterModifier,
    ));
    slicer_ir::MeshIR {
        objects: vec![object],
        ..Default::default()
    }
}

fn corpus_stage_drivers(external: bool) -> std::collections::BTreeSet<String> {
    use whole_print_input_infill::WholePrintInputInfill;
    use whole_print_input_perimeters::WholePrintInputPerimeters;
    use whole_print_input_support::WholePrintInputSupport;
    let cases = [
        (
            "whole-print-input-perimeters",
            "com.test.whole-print-input-perimeters",
            include_str!(
                "../../../slicer-wasm-host/test-guests/whole-print-input-perimeters/module.toml"
            ),
            WholePrintInputPerimeters::__slicer_prepared_native_entry(),
            WholePrintInputPerimeters::__SLICER_PREPARATION_SCHEMA,
        ),
        (
            "whole-print-input-infill",
            "com.test.whole-print-input-infill",
            include_str!(
                "../../../slicer-wasm-host/test-guests/whole-print-input-infill/module.toml"
            ),
            WholePrintInputInfill::__slicer_prepared_native_entry(),
            WholePrintInputInfill::__SLICER_PREPARATION_SCHEMA,
        ),
        (
            "whole-print-input-support",
            "com.test.whole-print-input-support",
            include_str!(
                "../../../slicer-wasm-host/test-guests/whole-print-input-support/module.toml"
            ),
            WholePrintInputSupport::__slicer_prepared_native_entry(),
            WholePrintInputSupport::__SLICER_PREPARATION_SCHEMA,
        ),
    ];
    let mut called = std::collections::BTreeSet::new();
    for (stem, id, manifest, entry, schema) in cases {
        let fixture = Fixture::new(external, false);
        // Own the whole Infill stage, not just its original top/sparse subset.
        let manifest: &'static str = if stem == "whole-print-input-infill" {
            Box::leak(manifest.replace("\"claim:top-fill\", \"claim:sparse-fill\"",
                "\"claim:top-fill\", \"claim:bottom-fill\", \"claim:solid-fill\", \"claim:sparse-fill\"").into_boxed_str())
        } else {
            manifest
        };
        let parsed: toml::Value = manifest.parse().expect("corpus manifest");
        let stage = parsed["stage"]["id"].as_str().expect("fixture stage");
        if external {
            fixture.stage_component(stem, manifest);
        }
        let mut options = fixture.options();
        options.module_dirs.retain(|path| {
            if path == fixture.staging.path() {
                return true;
            }
            let name = path.file_name().expect("core stem");
            let text = std::fs::read_to_string(path.join(name).with_extension("toml"))
                .expect("core manifest");
            let value: toml::Value = text.parse().expect("core manifest syntax");
            value["stage"]["id"].as_str() != Some(stage)
        });
        Arc::make_mut(&mut options.mesh).objects[0].id = "step9-source".into();
        options
            .config_overrides
            .insert("line_width".into(), ConfigValue::Float(0.8));
        if stage == "Layer::Support" {
            options.mesh = Arc::new(elevated_support_mesh());
            options.model_label = "step9-elevated-support".into();
            options.no_integrated_modules = true;
            // Seam placement is unrelated to Support delivery; this disconnected
            // fixture has empty source slices in the gap and needs no wall seam.
            options
                .module_dirs
                .retain(|path| path.file_name().is_none_or(|name| name != "seam-placer"));
            options.config_overrides.insert(
                "support_type".into(),
                ConfigValue::String("normal(auto)".into()),
            );
            options
                .config_overrides
                .insert("enable_support".into(), ConfigValue::Bool(true));
            options
                .config_overrides
                .insert("support_raft_layers".into(), ConfigValue::Int(1));
            options
                .config_overrides
                .insert("base_raft_layers".into(), ConfigValue::Int(0));
            options
                .config_overrides
                .insert("interface_raft_layers".into(), ConfigValue::Int(0));
        }
        let extensions = || {
            SliceRuntimeExtensions::new(
                vec![
                    IntegratedModuleRegistration {
                        manifest_toml: PATH_MANIFEST,
                        origin_label: "step9-path",
                    },
                    IntegratedModuleRegistration {
                        manifest_toml: manifest,
                        origin_label: "step9-corpus",
                    },
                ],
                vec![
                    (
                        PATH_ID.into(),
                        PreparationLifecycleGuest::__slicer_prepared_native_entry(),
                        PreparationLifecycleGuest::__SLICER_PREPARATION_SCHEMA,
                    ),
                    (id.into(), entry, schema),
                ],
                vec![],
                Some(fixture.trace.clone()),
            )
        };
        let capture =
            run_slice_preparation_capture(options.clone(), extensions()).expect("corpus capture");
        let owner = capture
            .owners
            .iter()
            .find(|owner| owner.module_id.as_str() == id)
            .expect("corpus owner");
        assert!(
            !owner.selected_targets.is_empty(),
            "{stage} needs a nonempty selected control"
        );
        assert!(owner
            .selected_targets
            .iter()
            .all(|target| target.stage_id == stage && target.invoke));
        assert!(owner
            .selected_targets
            .iter()
            .all(|target| target.config.get_float("line_width") == Some(0.8)));
        if stage == "Layer::Support" {
            assert!(
                owner.selected_targets.iter().any(|target| matches!(
                    target.source,
                    PreparationTargetSource::Raft {
                        global_layer_index: 0
                    }
                )),
                "one authored raft layer must be selected"
            );
            assert!(
                owner.selected_targets.iter().any(|target| matches!(
                    target.source,
                    PreparationTargetSource::SupportCarrier { .. }
                )),
                "elevated roof must supply a real carrier below its model range"
            );
        }
        for target in &owner.selected_targets {
            if let PreparationTargetSource::Model { key, members } = &target.source {
                assert!(
                    key.object_id == "step9-source"
                        || (stage == "Layer::Support" && key.object_id == "step9-base")
                );
                assert!(key.variant_chain.is_empty());
                assert!(!members.is_empty());
                assert!(members
                    .iter()
                    .all(|member| member.object_id == key.object_id
                        && member.variant_chain.is_empty()));
            }
        }
        fixture.trace.events.lock().unwrap().clear();
        run_slice_with_runtime_extensions(options, extensions())
            .expect("actual corpus stage dispatch");
        let events = fixture.trace.events.lock().unwrap();
        let observed = records(&events);
        assert_eq!(
            observed
                .iter()
                .filter(
                    |r| r.module_id.as_str() == id && r.status == PreparationLifecycleStatus::Ready
                )
                .count(),
            1,
            "{stage} prepares once"
        );
        let consumers: Vec<_> = observed
            .iter()
            .filter(|r| {
                r.module_id.as_str() == id
                    && r.lifecycle_phase == PreparationLifecyclePhase::Consumption
                    && r.status == PreparationLifecycleStatus::Start
            })
            .collect();
        assert!(
            !consumers.is_empty(),
            "{stage} must actually dispatch, not just capture"
        );
        called.insert(stage.to_owned());
        assert_disposed(&events, id);
    }
    called
}

#[test]
fn shared_selection_all_target_kinds() {
    let compiled = compile_shapes::stages();
    let derived: std::collections::BTreeSet<_> = slicer_schema::STAGES
        .iter()
        .filter(|stage| stage.tier_id == slicer_schema::TIER_LAYER && !stage.wit_package.is_empty())
        .map(|stage| stage.stage_id)
        .collect();
    // This first real capture pins the two lifecycle fixtures' target kinds.
    // Painted/member/support/raft fixtures must extend this driver, not infer
    // their coverage from this unpainted wedge or the pilot compile roster.
    for external in [false, true] {
        let mut delivered = corpus_stage_drivers(external);
        let fixture = Fixture::new(external, true);
        let capture = fixture.capture(4, "probe");
        let path = capture
            .owners
            .iter()
            .find(|o| o.module_id.as_str() == PATH_ID)
            .expect("selected path owner");
        assert!(!path.selected_targets.is_empty());
        for target in &path.selected_targets {
            assert_eq!(target.stage_id, "Layer::PathOptimization");
            assert!(target.invoke);
            assert!(target.held_claims.is_empty());
            assert!(matches!(
                target.source,
                PreparationTargetSource::Model { .. }
            ));
            assert!(!target.authored_coloring);
        }
        let anchored = capture
            .owners
            .iter()
            .find(|o| o.module_id.as_str() == ANCHORED_ID)
            .expect("selected anchored owner");
        assert!(!anchored.selected_targets.is_empty());
        assert!(anchored.selected_targets.iter().any(|target| {
            matches!(target.source, PreparationTargetSource::AnchoredStage { anchor_layer_index } if anchor_layer_index == target.global_layer_index)
        }), "non-region anchored delivery must be nonempty");
        for target in &anchored.selected_targets {
            match &target.source {
                PreparationTargetSource::AnchoredStage { anchor_layer_index } => {
                    assert_eq!(*anchor_layer_index, target.global_layer_index)
                }
                PreparationTargetSource::Model { key, members } => {
                    assert_eq!(key.global_layer_index, target.global_layer_index);
                    assert!(!members.is_empty());
                    assert!(key.variant_chain.is_empty());
                }
                other => panic!("unexpected anchored fixture provenance: {other:?}"),
            }
        }
        fixture.trace.events.lock().unwrap().clear();
        assert!(fixture.run().contains(fixture.marker()));
        let events = fixture.trace.events.lock().unwrap();
        for id in [PATH_ID, ANCHORED_ID] {
            let calls: Vec<_> = records(&events)
                .into_iter()
                .filter(|r| {
                    r.module_id.as_str() == id
                        && r.lifecycle_phase == PreparationLifecyclePhase::Consumption
                        && r.status == PreparationLifecycleStatus::Start
                })
                .collect();
            assert!(
                !calls.is_empty(),
                "selected module {id} must actually execute"
            );
            delivered.insert(calls[0].consumer_stage.to_string());
        }
        let delivered: std::collections::BTreeSet<_> =
            delivered.iter().map(String::as_str).collect();
        assert!(compiled.is_disjoint(&delivered));
        assert_eq!(
            compiled
                .union(&delivered)
                .copied()
                .collect::<std::collections::BTreeSet<_>>(),
            derived
        );
    }
}

#[test]
fn non_infill_fresh_concurrent_reads() {
    for external in [false, true] {
        let fixture = Fixture::new(external, false);
        let output = fixture.run();
        assert!(
            output.matches(fixture.marker()).count() > 1,
            "multiple ordinary consumers must decode independently"
        );
        let events = fixture.trace.events.lock().unwrap();
        let observed = records(&events);
        assert_eq!(
            observed
                .iter()
                .filter(|r| matches!(r.status, PreparationLifecycleStatus::Ready))
                .count(),
            1
        );
        let consumers: Vec<_> = observed
            .iter()
            .filter(|r| {
                matches!(r.lifecycle_phase, PreparationLifecyclePhase::Consumption)
                    && matches!(r.status, PreparationLifecycleStatus::Start)
            })
            .collect();
        assert!(consumers.len() > 1);
        assert!(consumers.iter().all(|r| r.global_layer_index.is_some()));
        assert_eq!(output.matches(fixture.marker()).count(), consumers.len());
        assert_eq!(
            output
                .matches("PREPARATION-LIFECYCLE-OPTIONAL-NONE")
                .count(),
            consumers.len()
        );
        let disposed = observed
            .iter()
            .find(|r| r.status == PreparationLifecycleStatus::Disposed)
            .expect("quiescent read accounting");
        assert_eq!(
            disposed.transferred_bytes,
            8 * consumers.len() as u64,
            "each fresh consumer reads the four-byte probe and independent three/one-byte offsets"
        );
        assert_disposed(&events, PATH_ID);
    }
}

#[test]
fn ready_empty_optional_required_decoder() {
    for external in [false, true] {
        let fixture = Fixture::new(external, false);
        let mut empty = fixture.options();
        empty.config_overrides.insert(
            "preparation_probe_mode".into(),
            ConfigValue::String("ready_empty".into()),
        );
        let result = run_slice_with_runtime_extensions(empty, fixture.extensions(vec![]))
            .expect("Ready-empty is not unready");
        assert!(result
            .gcode_text
            .contains("PREPARATION-LIFECYCLE-READY-EMPTY"));
        assert!(result
            .gcode_text
            .contains("PREPARATION-LIFECYCLE-OPTIONAL-NONE"));
        let events = fixture.trace.events.lock().unwrap();
        let ready = records(&events)
            .into_iter()
            .find(|r| matches!(r.status, PreparationLifecycleStatus::Ready))
            .expect("empty publication is Ready");
        assert!(ready.pieces.is_empty());
        drop(events);
        for (token, length, code, detail) in [
            ("changed_probe", 4, "57401", "probe"),
            ("probe", 2, "57403", "lifecycle fixture malformed payload"),
        ] {
            let fixture = Fixture::new(external, false);
            let mut opts = fixture.options();
            opts.config_overrides.insert(
                "preparation_probe_token".into(),
                ConfigValue::String(token.into()),
            );
            opts.config_overrides
                .insert("preparation_probe_length".into(), ConfigValue::Int(length));
            let error = run_slice_with_runtime_extensions(opts, fixture.extensions(vec![]))
                .expect_err("required/decoder control must fail");
            let text = format!("{error:?}");
            assert!(
                text.contains(code) && text.contains(detail),
                "original decoder error: {text}"
            );
            let events = fixture.trace.events.lock().unwrap();
            let failures: Vec<_> = records(&events)
                .into_iter()
                .filter(|r| {
                    r.lifecycle_phase == PreparationLifecyclePhase::Consumption
                        && r.status == PreparationLifecycleStatus::Failure
                })
                .collect();
            assert!(
                !failures.is_empty(),
                "decoder failure must originate in an actual consumer"
            );
            assert_disposed(&events, PATH_ID);
        }
    }
}

#[test]
fn concurrent_print_isolation_and_cleanup() {
    let start = Arc::new(Barrier::new(2));
    let handles: Vec<_> = [(4, "probe"), (2, "isolated_probe")]
        .into_iter()
        .map(|(length, token)| {
            let start = start.clone();
            std::thread::spawn(move || {
                let fixture = Fixture::new(false, false);
                start.wait();
                let capture = fixture.capture(length, token);
                let owner = capture
                    .owners
                    .iter()
                    .find(|o| o.module_id.as_str() == PATH_ID)
                    .expect("per-print owner");
                assert_eq!(owner.pieces.len(), 1);
                assert_eq!(owner.pieces[0].name, token);
                assert_eq!(owner.pieces[0].length, length as u64);
                assert_eq!(capture.retained_bytes, 0);
                assert_eq!(capture.transferred_bytes, 0);
                assert_disposed(&fixture.trace.events.lock().unwrap(), PATH_ID);
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("concurrent real capture");
    }
}

#[test]
fn preparation_only_capture_shared_prefix() {
    for external in [false, true] {
        for (length, token) in [(4, "probe"), (2, "probe"), (4, "changed_probe")] {
            let fixture = Fixture::new(external, false);
            let capture = fixture.capture(length, token);
            let owner = capture
                .owners
                .iter()
                .find(|o| o.module_id.as_str() == PATH_ID)
                .expect("captured selected owner");
            assert_eq!(owner.pieces.len(), 1);
            assert_eq!(owner.pieces[0].name, token);
            assert_eq!(owner.pieces[0].length, length as u64);
            assert_eq!(
                owner.accounting_before_disposal.retained_payload_bytes,
                length as u64
            );
            assert_eq!(owner.accounting_before_disposal.returned_read_bytes, 0);
            assert_eq!(capture.retained_bytes, 0);
            assert_eq!(capture.transferred_bytes, 0);
            assert!(!capture.lifecycle.is_empty());
            assert_no_consumer(&fixture.trace.events.lock().unwrap());
            assert_disposed(&fixture.trace.events.lock().unwrap(), PATH_ID);
        }
        // Independently authored model provenance, not expectations copied from
        // the resolver/captured DTOs. Change both the source identity and the
        // module-declared per-leg config; no old source may survive the change.
        for (source, absent, mode) in [
            ("capture-source-a", "capture-source-b", "ready"),
            ("capture-source-b", "capture-source-a", "required_decoder"),
        ] {
            let fixture = Fixture::new(external, false);
            let options = || {
                let mut opts = fixture.options();
                assert_eq!(opts.mesh.objects.len(), 1, "single authored model source");
                Arc::make_mut(&mut opts.mesh).objects[0].id = source.into();
                opts.config_overrides.insert(
                    "preparation_probe_mode".into(),
                    ConfigValue::String(mode.into()),
                );
                opts
            };
            let capture =
                run_slice_preparation_capture(options(), fixture.extensions(vec![host_entity()]))
                    .expect("capture authored source provenance");
            let owner = capture
                .owners
                .iter()
                .find(|owner| owner.module_id.as_str() == PATH_ID)
                .expect("selected source owner survives store disposal");
            assert!(
                !owner.selected_targets.is_empty(),
                "typed targets must exist"
            );
            assert!(
                owner
                    .selected_targets
                    .iter()
                    .any(|target| target.global_layer_index == 0),
                "literal first-layer target"
            );
            for target in &owner.selected_targets {
                assert_eq!(target.stage_id, "Layer::PathOptimization");
                assert!(target.invoke);
                // resolve_held_claims (crates/slicer-scheduler/src/validation.rs)
                // retains fill-role claims, not the path-optimizer capability.
                assert_eq!(target.held_claims, Vec::<String>::new());
                assert!(!target.authored_coloring);
                assert_eq!(
                    target.config.get_string("preparation_probe_mode"),
                    Some(mode)
                );
                let PreparationTargetSource::Model { key, members } = &target.source else {
                    panic!("expected literal model provenance, got {:?}", target.source);
                };
                assert_eq!(key.object_id, source);
                assert_ne!(key.object_id, absent, "old source must be absent");
                assert_eq!(key.global_layer_index, target.global_layer_index);
                assert!(key.variant_chain.is_empty(), "unpainted literal variant");
                assert!(
                    !members.is_empty(),
                    "full model members must survive capture"
                );
                for member in members {
                    assert_eq!(member.object_id, source);
                    assert_ne!(member.object_id, absent, "no stale member provenance");
                    assert!(member.variant_chain.is_empty());
                }
            }
            assert_eq!(owner.pieces.len(), 1);
            assert_eq!(owner.pieces[0].name, "probe");
            assert_eq!(owner.pieces[0].length, 4);
            assert_eq!(owner.accounting_before_disposal.retained_payload_bytes, 4);
            assert_eq!(owner.accounting_before_disposal.retained_name_bytes, 5);
            assert_eq!(owner.accounting_before_disposal.returned_read_bytes, 0);
            assert_eq!(capture.retained_bytes, 0);
            assert_eq!(capture.transferred_bytes, 0);
            assert_no_consumer(&fixture.trace.events.lock().unwrap());
            assert_disposed(&fixture.trace.events.lock().unwrap(), PATH_ID);

            // The same authored leg separately runs ordinary module-owned
            // decoding. Capture itself never obtains the opaque probe bytes.
            let full = run_slice_with_runtime_extensions(options(), fixture.extensions(vec![]))
                .expect("ordinary module decodes the same prepared leg");
            assert!(full.gcode_text.contains(fixture.marker()));
        }
        let full = Fixture::new(external, false);
        assert!(full.run().contains(full.marker()));
    }
}

#[test]
fn truthful_lifecycle_metadata() {
    for instrumented in [false, true] {
        let fixture = Fixture::new(false, false);
        let mut opts = fixture.options();
        opts.instrument_stderr = instrumented;
        let capture = run_slice_preparation_capture(opts, fixture.extensions(vec![]))
            .expect("observed capture");
        assert!(!capture.lifecycle.is_empty());
        for record in &capture.lifecycle {
            assert_eq!(record.module_version, "0.1.0");
            assert!(!record.artifact_identity.is_empty());
            assert_eq!(record.consumer_stage.to_string(), "Layer::PathOptimization");
            assert!(record.global_layer_index.is_none());
            if !instrumented {
                assert!(record.elapsed.is_none());
            }
            if matches!(record.status, PreparationLifecycleStatus::Ready) {
                assert_eq!(record.pieces, [("probe".into(), 4)]);
                if instrumented {
                    assert!(record.elapsed.is_some());
                }
            }
            assert_eq!(record.transferred_bytes, 0);
        }
    }
}

fn host_entity() -> AnchoredEntity {
    // exhaustive: synthetic-host control has independent identity and endpoints
    AnchoredEntity {
        local_id: 574,
        anchor_global_layer_index: 0,
        geometry: AnchoredGeometryContract::Planar { z: 0 },
        input_capabilities: vec![],
        output_capabilities: vec![],
        provenance: AnchoredEntityProvenance {
            requesting_feature: "lifecycle_host_control".into(),
            source_plan_entry: "synthetic_not_a_probe_reader".into(),
        },
        path_points: [7.0, 8.0]
            .into_iter()
            .map(|x| Point3WithWidth {
                x,
                y: 0.0,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            })
            .collect(),
        role: ExtrusionRole::SupportMaterial,
    }
}

fn has_x(gcode: &str, x: f64) -> bool {
    gcode
        .lines()
        .filter(|line| line.starts_with("G1 "))
        .any(|line| {
            line.split_whitespace()
                .filter_map(|word| word.strip_prefix('X'))
                .filter_map(|word| word.parse::<f64>().ok())
                .any(|value| value == x)
        })
}

#[test]
fn prepared_anchored_module_reads_and_host_closure_lifetime() {
    for external in [false, true] {
        for (start, end) in if external {
            [(5, 6), (9, 10)]
        } else {
            [(3, 4), (9, 10)]
        } {
            let fixture = Fixture::new(external, true);
            let mut opts = fixture.options();
            opts.config_overrides
                .insert("probe_start_mm".into(), ConfigValue::Int(start));
            opts.config_overrides
                .insert("probe_end_mm".into(), ConfigValue::Int(end));
            let output = run_slice_with_runtime_extensions(opts, fixture.extensions(vec![]))
                .expect("actual anchored-module reader");
            assert!(has_x(&output.gcode_text, start as f64));
            assert!(has_x(&output.gcode_text, end as f64));
            let events = fixture.trace.events.lock().unwrap();
            let reads: Vec<_> = records(&events)
                .into_iter()
                .filter(|r| {
                    r.module_id.as_str() == ANCHORED_ID
                        && matches!(r.lifecycle_phase, PreparationLifecyclePhase::Consumption)
                })
                .collect();
            assert!(
                !reads.is_empty(),
                "synthetic-host activity cannot stand in for a module read"
            );
            assert!(reads.iter().any(|r| r.transferred_bytes >= 4));
            assert_disposed(&events, ANCHORED_ID);
        }
        // Separate host-only control: do not register the anchored fixture.
        let fixture = Fixture::new(external, false);
        let output = run_slice_with_runtime_extensions(
            fixture.options(),
            fixture.extensions(vec![host_entity()]),
        )
        .expect("synthetic host closure");
        assert!(has_x(&output.gcode_text, 7.0) && has_x(&output.gcode_text, 8.0));
        let events = fixture.trace.events.lock().unwrap();
        let start = events
            .iter()
            .position(|e| matches!(e, Event::Host(AnchoredHostClosureStatus::Start)))
            .expect("host start");
        let finish = events
            .iter()
            .position(|e| matches!(e, Event::Host(AnchoredHostClosureStatus::Finished)))
            .expect("host finished");
        let dispose = events.iter().position(|e| matches!(e, Event::Lifecycle(r) if matches!(r.status, PreparationLifecycleStatus::Disposed))).expect("owner disposed");
        assert!(start < finish && finish < dispose);
        assert!(!records(&events)
            .iter()
            .any(|r| r.module_id.as_str() == ANCHORED_ID));
        assert!(!events[start..=finish].iter().any(|e| matches!(e, Event::Lifecycle(r) if matches!(r.lifecycle_phase, PreparationLifecyclePhase::Consumption))));
        assert_disposed(&events, PATH_ID);
    }
}

#[test]
fn required_module_error_preserves_fields() {
    for external in [false, true] {
        let fixture = Fixture::new(external, false);
        let mut opts = fixture.options();
        opts.config_overrides.insert(
            "preparation_probe_mode".into(),
            ConfigValue::String("module_error".into()),
        );
        let _error = run_slice_with_runtime_extensions(opts, fixture.extensions(vec![]))
            .expect_err("required preparation must abort despite fatal=false");
        let events = fixture.trace.events.lock().unwrap();
        assert_no_consumer(&events);
        let observed = records(&events);
        let failure = observed
            .iter()
            .find(|r| matches!(r.status, PreparationLifecycleStatus::Failure))
            .expect("typed failure observation");
        match failure
            .failure
            .as_deref()
            .expect("preserved original failure")
        {
            PreparationFailure::Module(error) => {
                assert_eq!(error.code, 57451);
                assert_eq!(error.message, "prepare rejected");
                assert!(!error.fatal);
            }
            other => panic!("module failure rewritten: {other:?}"),
        }
        assert!(!observed
            .iter()
            .any(|r| matches!(r.status, PreparationLifecycleStatus::Ready)));
        assert_disposed(&events, PATH_ID);
    }
}

#[test]
fn trap_poison_and_failure_cleanup() {
    for external in [false, true] {
        for mode in [if external { "trap" } else { "panic" }, "duplicate_put"] {
            let fixture = Fixture::new(external, false);
            let mut opts = fixture.options();
            opts.config_overrides.insert(
                "preparation_probe_mode".into(),
                ConfigValue::String(mode.into()),
            );
            let error = run_slice_with_runtime_extensions(opts, fixture.extensions(vec![]))
                .expect_err("trap or poisoned output cannot publish");
            let events = fixture.trace.events.lock().unwrap();
            assert_no_consumer(&events);
            let observed = records(&events);
            let failure = observed
                .iter()
                .find(|r| matches!(r.status, PreparationLifecycleStatus::Failure))
                .expect("failure event");
            if mode != "duplicate_put" {
                let Some(PreparationFailure::Trap { detail }) = failure.failure.as_deref() else {
                    panic!("original trap rewritten: {:?}", failure.failure);
                };
                // Wasmtime's original trap is its execution backtrace; the
                // guest panic text is logged separately, not the trap payload.
                if external {
                    assert!(detail.contains("wasm backtrace"), "{detail}");
                } else {
                    assert_eq!(detail, "lifecycle fixture preparation trap");
                }
                assert!(
                    error.0.contains(detail),
                    "slice lost original trap: {error:?}"
                );
            } else {
                assert!(
                    matches!(
                        failure.failure.as_deref(),
                        Some(PreparationFailure::Transport(
                            slicer_schema::preparation::PreparationTransportError::PoisonedOutput
                        ))
                    ),
                    "caught duplicate-put must reject publication: {:?}",
                    failure.failure
                );
            }
            assert!(!observed
                .iter()
                .any(|r| matches!(r.status, PreparationLifecycleStatus::Ready)));
            assert_disposed(&events, PATH_ID);
        }
    }

    // Call the real module producer first, then exercise poison completion
    // choices inside the same native adapter and active transaction.
    for (prepare, expected) in [
        (caught_poison as FixturePrepare, 0),
        (module_after_poison, 1),
        (panic_after_poison, 2),
    ] {
        let fixture = Fixture::new(false, false);
        let mut entry = PreparationLifecycleGuest::__slicer_prepared_native_entry();
        entry.prepare = prepare;
        let extensions = SliceRuntimeExtensions::new(
            vec![IntegratedModuleRegistration {
                manifest_toml: PATH_MANIFEST,
                origin_label: "lifecycle-poison-native",
            }],
            vec![(
                PATH_ID.into(),
                entry,
                PreparationLifecycleGuest::__SLICER_PREPARATION_SCHEMA,
            )],
            vec![],
            Some(fixture.trace.clone()),
        );
        run_slice_with_runtime_extensions(fixture.options(), extensions)
            .expect_err("poison cannot publish");
        let events = fixture.trace.events.lock().unwrap();
        assert_no_consumer(&events);
        let observed = records(&events);
        let failure = observed
            .iter()
            .find(|r| r.status == PreparationLifecycleStatus::Failure)
            .expect("typed poison completion")
            .failure
            .as_deref()
            .expect("failure payload");
        match (expected, failure) {
            (
                0,
                PreparationFailure::Transport(
                    slicer_schema::preparation::PreparationTransportError::PoisonedOutput,
                ),
            ) => {}
            (1, PreparationFailure::Module(error)) => {
                assert_eq!(error.code, 57451);
                assert_eq!(error.message, "prepare rejected");
                assert!(!error.fatal);
            }
            (2, PreparationFailure::Trap { detail }) => {
                assert_eq!(detail, "lifecycle fixture preparation trap after poison")
            }
            _ => panic!("original error/poison precedence changed: {failure:?}"),
        }
        assert!(!observed
            .iter()
            .any(|r| r.status == PreparationLifecycleStatus::Ready));
        assert_disposed(&events, PATH_ID);
    }
}

type FixturePrepare = fn(
    &slicer_sdk::preparation::PrintPreparationInput,
    &mut slicer_sdk::preparation::PreparedPlanOutput,
) -> Result<(), slicer_sdk::error::ModuleError>;

fn caught_poison(
    input: &slicer_sdk::preparation::PrintPreparationInput,
    output: &mut slicer_sdk::preparation::PreparedPlanOutput,
) -> Result<(), slicer_sdk::error::ModuleError> {
    (PreparationLifecycleGuest::__slicer_prepared_native_entry().prepare)(input, output)?;
    assert!(
        output.put("probe", b"again").is_err(),
        "duplicate must poison writer"
    );
    Ok(())
}

fn module_after_poison(
    input: &slicer_sdk::preparation::PrintPreparationInput,
    output: &mut slicer_sdk::preparation::PreparedPlanOutput,
) -> Result<(), slicer_sdk::error::ModuleError> {
    caught_poison(input, output)?;
    Err(slicer_sdk::error::ModuleError {
        code: 57451,
        message: "prepare rejected".into(),
        fatal: false,
    })
}

fn panic_after_poison(
    input: &slicer_sdk::preparation::PrintPreparationInput,
    output: &mut slicer_sdk::preparation::PreparedPlanOutput,
) -> Result<(), slicer_sdk::error::ModuleError> {
    caught_poison(input, output)?;
    panic!("lifecycle fixture preparation trap after poison")
}

#[test]
fn cancellation_waits_for_inflight_cleanup() {
    // Before preparation (the ordinary pre-set cancellation boundary).
    let fixture = Fixture::new(false, false);
    let mut opts = fixture.options();
    opts.cancel_flag = Some(Arc::new(AtomicBool::new(true)));
    let error = run_slice_with_runtime_extensions(opts, fixture.extensions(vec![]))
        .expect_err("pre-set cancellation");
    assert!(error.0.contains("cancelled"), "{error:?}");
    assert!(fixture.trace.events.lock().unwrap().is_empty());

    // Between identity-sorted owners: anchored is Ready, path must not start.
    let flag = Arc::new(AtomicBool::new(false));
    let mut fixture = Fixture::new(false, true);
    fixture.trace = Arc::new(Trace {
        cancel: Some((flag.clone(), PreparationLifecyclePhase::Preparation)),
        cancel_on_ready: true,
        ..Default::default()
    });
    let mut opts = fixture.options();
    opts.cancel_flag = Some(flag);
    let error = run_slice_with_runtime_extensions(opts, fixture.extensions(vec![]))
        .expect_err("between-owner cancellation");
    assert!(error.0.contains("cancelled"), "{error:?}");
    let events = fixture.trace.events.lock().unwrap();
    assert_no_consumer(&events);
    assert!(records(&events)
        .iter()
        .any(|r| r.module_id.as_str() == ANCHORED_ID
            && r.status == PreparationLifecycleStatus::Ready));
    assert!(!records(&events)
        .iter()
        .any(|r| r.module_id.as_str() == PATH_ID && r.status == PreparationLifecycleStatus::Start));
    assert_disposed(&events, ANCHORED_ID);
    drop(events);

    // The real adapter holds its active guard while this native entry blocks.
    // Both preparation staging and ordinary reads must be allowed to return.
    for (phase, anchored) in [
        (PreparationLifecyclePhase::Preparation, false),
        (PreparationLifecyclePhase::Consumption, false),
        (PreparationLifecyclePhase::Consumption, true),
    ] {
        let flag = Arc::new(AtomicBool::new(false));
        let fixture = Fixture::new(false, anchored);
        let block = Arc::new(InFlightCall {
            entered: Barrier::new(2),
            release: Barrier::new(2),
            returned: AtomicBool::new(false),
            claimed: AtomicBool::new(false),
        });
        *IN_FLIGHT_CALL.lock().unwrap() = Some(block.clone());
        let mut entry = PreparationLifecycleGuest::__slicer_prepared_native_entry();
        if phase == PreparationLifecyclePhase::Preparation {
            entry.prepare = blocking_prepare;
        } else {
            entry.stage = slicer_sdk::native::NativeStageEntry::Layer(blocking_consume);
        }
        let mut integrated = vec![IntegratedModuleRegistration {
            manifest_toml: PATH_MANIFEST,
            origin_label: "lifecycle-blocking-native",
        }];
        let mut prepared = vec![(
            PATH_ID.into(),
            entry,
            PreparationLifecycleGuest::__SLICER_PREPARATION_SCHEMA,
        )];
        if anchored {
            integrated.push(IntegratedModuleRegistration {
                manifest_toml: ANCHORED_MANIFEST,
                origin_label: "lifecycle-blocking-anchored",
            });
            prepared[0].1 = PreparationLifecycleGuest::__slicer_prepared_native_entry();
            let mut entry = PreparationLifecycleAnchored::__slicer_prepared_native_entry();
            entry.stage = slicer_sdk::native::NativeStageEntry::Layer(blocking_anchored_consume);
            prepared.push((
                ANCHORED_ID.into(),
                entry,
                PreparationLifecycleAnchored::__SLICER_PREPARATION_SCHEMA,
            ));
        }
        let extensions =
            SliceRuntimeExtensions::new(integrated, prepared, vec![], Some(fixture.trace.clone()));
        let mut opts = fixture.options();
        opts.cancel_flag = Some(flag.clone());
        let result = std::thread::scope(|scope| {
            let worker = scope.spawn(move || run_slice_with_runtime_extensions(opts, extensions));
            block.entered.wait();
            flag.store(true, Ordering::Release);
            let events = fixture.trace.events.lock().unwrap();
            assert!(!block.returned.load(Ordering::Acquire));
            assert!(!records(&events)
                .iter()
                .any(|r| r.status == PreparationLifecycleStatus::Disposed));
            if phase == PreparationLifecyclePhase::Preparation {
                assert_no_consumer(&events);
                assert!(!records(&events)
                    .iter()
                    .any(|r| r.status == PreparationLifecycleStatus::Ready));
            }
            drop(events);
            block.release.wait();
            worker.join().expect("slice thread must return, not panic")
        });
        *IN_FLIGHT_CALL.lock().unwrap() = None;
        assert!(
            block.returned.load(Ordering::Acquire),
            "active call was interrupted"
        );
        let error = result.expect_err("cancelled slice must not succeed");
        assert!(error.0.contains("layer execution cancelled"), "{error:?}");
        let events = fixture.trace.events.lock().unwrap();
        if phase == PreparationLifecyclePhase::Preparation {
            assert_no_consumer(&events);
            assert!(records(&events)
                .iter()
                .any(|r| r.status == PreparationLifecycleStatus::Cancelled));
            assert!(!records(&events)
                .iter()
                .any(|r| r.status == PreparationLifecycleStatus::Ready));
        }
        assert_disposed(&events, PATH_ID);
        if anchored {
            assert_disposed(&events, ANCHORED_ID);
        }
    }
}

struct InFlightCall {
    entered: Barrier,
    release: Barrier,
    returned: AtomicBool,
    claimed: AtomicBool,
}

static IN_FLIGHT_CALL: Mutex<Option<Arc<InFlightCall>>> = Mutex::new(None);

fn block_active_call() {
    let block = IN_FLIGHT_CALL
        .lock()
        .unwrap()
        .as_ref()
        .expect("blocking fixture installed")
        .clone();
    // A parallel-safe ordinary fixture can have other calls already in flight.
    // Only one participates in the controller handshake; none is interrupted.
    if !block.claimed.swap(true, Ordering::AcqRel) {
        block.entered.wait();
        block.release.wait();
        block.returned.store(true, Ordering::Release);
    }
}

fn blocking_prepare(
    input: &slicer_sdk::preparation::PrintPreparationInput,
    output: &mut slicer_sdk::preparation::PreparedPlanOutput,
) -> Result<(), slicer_sdk::error::ModuleError> {
    (PreparationLifecycleGuest::__slicer_prepared_native_entry().prepare)(input, output)?;
    block_active_call();
    Ok(())
}

fn blocking_consume(
    input: &slicer_sdk::native::NativeLayerRequest,
) -> Result<slicer_sdk::native::NativeLayerResponse, slicer_sdk::error::ModuleError> {
    let slicer_sdk::native::NativeStageEntry::Layer(call) =
        PreparationLifecycleGuest::__slicer_prepared_native_entry().stage
    else {
        panic!("fixture must expose Layer entry")
    };
    let result = call(input);
    block_active_call();
    result
}

fn blocking_anchored_consume(
    input: &slicer_sdk::native::NativeLayerRequest,
) -> Result<slicer_sdk::native::NativeLayerResponse, slicer_sdk::error::ModuleError> {
    let slicer_sdk::native::NativeStageEntry::Layer(call) =
        PreparationLifecycleAnchored::__slicer_prepared_native_entry().stage
    else {
        panic!("anchored fixture must expose Layer entry")
    };
    let result = call(input);
    block_active_call();
    result
}

#[test]
fn arena_run_to_completion_on_failure() {
    let fixture = Fixture::new(false, false);
    let mut opts = fixture.options();
    opts.config_overrides
        .insert("preparation_probe_length".into(), ConfigValue::Int(2));
    let error = run_slice_with_runtime_extensions(opts, fixture.extensions(vec![]))
        .expect_err("ordinary decoder fails after preparation");
    assert!(format!("{error:?}").contains("57403"));
    let events = fixture.trace.events.lock().unwrap();
    let mut arena_states: BTreeMap<u32, (usize, usize, usize)> = BTreeMap::new();
    for event in events.iter() {
        if let Event::Arena(index, status) = event {
            let state = arena_states.entry(*index).or_default();
            match status {
                LayerArenaLifecycleStatus::Created => {
                    assert_eq!(state.0, 0);
                    state.0 += 1;
                }
                LayerArenaLifecycleStatus::Committed => {
                    assert_eq!(state.0, 1);
                    assert_eq!(state.2, 0);
                    state.1 += 1;
                }
                LayerArenaLifecycleStatus::Dropped => {
                    assert_eq!(state.0, 1);
                    state.2 += 1;
                }
            }
        }
    }
    assert!(
        arena_states.len() > 1,
        "must exercise a real multilayer failure"
    );
    assert!(arena_states
        .values()
        .all(|(created, committed, dropped)| *created == 1 && *committed <= 1 && *dropped == 1));
    let disposal = events.iter().position(|e| matches!(e, Event::Lifecycle(r) if matches!(r.status, PreparationLifecycleStatus::Disposed))).expect("cleanup observed");
    assert!(!events[disposal + 1..]
        .iter()
        .any(|e| matches!(e, Event::Arena(_, _))));
    assert_disposed(&events, PATH_ID);
}
