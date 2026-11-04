//! Packet 06 Step 14: real runtime gate for typed preparation visual capture.
//!
//! AC-3 runs two full wedge slices through the production facade — one with no
//! preparation view requested and one requesting the generated fixture's
//! `probe_xy` view — and compares emitted G-code byte-for-byte, then inspects
//! the same staged component's preparation-only visual capture on the
//! unrequested and requested paths.
//!
//! AC-4 drives the preparation-only visual facade and observes that the shared
//! startup prefix ran (real frozen external artifact, committed schedule and
//! Ready owner) while no Layer arena and no ordinary consumer was entered.
//!
//! The fixture is the actual generated `preparation-visual-guest` component
//! staged beside its manifest through `module_dirs`; nothing here injects a
//! Ready plan, a test-local store, or a test-local resolver.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use slicer_ir::ConfigValue;
use slicer_runtime::{
    run_slice_preparation_visual_capture, run_slice_with_runtime_extensions,
    AnchoredHostClosureStatus, CapturedIr, LayerArenaLifecycleStatus, PreparationCapture,
    PreparationLifecycleObserver, PreparationLifecyclePhase, PreparationLifecycleRecord,
    PreparationLifecycleStatus, PreparationOwnerCapture, PreparationViewRequest,
    PreparationVisualCapture, PreparationVisualCaptureError, SliceRunOptions,
    SliceRuntimeExtensions, StageCapture,
};
use slicer_sdk::preparation::PreparationTargetSource;
use slicer_wasm_host::preparation::CommittedPreparationProjection;
use tempfile::TempDir;

const MODULE_ID: &str = "test.preparation-visual";
const VIEW: &str = "probe_xy";
const MODULE_STEM: &str = "preparation-visual-guest";
const MODULE_MANIFEST: &str =
    include_str!("../../../slicer-wasm-host/test-guests/preparation-visual-guest/module.toml");

// Ordinary output the fixture's own `run_path_optimization` emits after its
// private probe codec accepted the four-byte plan piece. These are pinned from
// the fixture source, never read back from a capture run.
const PROBE_COMMENT: &str = "PREPARATION-VISUAL-PROBE";
const TOKEN_COMMENT: &str = "PREPARATION-VISUAL-TOKEN:PRB1";
const PROBE_NAME: &str = "probe";
const PROBE_BYTES: u64 = 4;
/// `"probe".len() as u64`; the owner's ordinary name accounting.
const PROBE_NAME_BYTES: u64 = 5;

/// Logical diagnostic bytes of the fixture's five-primitive `probe_xy` scene.
///
/// Independent UTF-8/geometry inventory (1 unit = 100 nm, `docs/08`):
/// strings `final`(5) + `probe_xy`(8) + classes `anchors`/`domains`/`paths`/
/// `remote`(25) + ids `anchor_a`/`path_a`/`domain_a`/`outline_a`/`remote_a`(39)
/// + labels(28) = 137; geometry `Point`(17) + `Polyline`(1+2*16=33) +
///   filled `Polygon`(2+8*16=130) + outline `Polygon`(2+4*16=66) + `Point`(17)
///   = 263; plus 4 bytes of layer index per primitive = 20. Total 420.
const SCENE_BYTES: u64 = 420;

fn workspace_root() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo test");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn view_request() -> PreparationViewRequest {
    PreparationViewRequest {
        module_id: MODULE_ID.into(),
        view: VIEW.into(),
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
}

impl Trace {
    fn lifecycle(&self) -> Vec<PreparationLifecycleRecord> {
        self.events
            .lock()
            .expect("runtime observation trace poisoned")
            .iter()
            .filter_map(|event| match event {
                Event::Lifecycle(record) => Some(record.clone()),
                Event::Arena(_, _) | Event::Host(_) => None,
            })
            .collect()
    }

    /// Real Layer arena events observed through the runtime observer.
    fn arena_events(&self) -> Vec<(u32, LayerArenaLifecycleStatus)> {
        self.events
            .lock()
            .expect("runtime observation trace poisoned")
            .iter()
            .filter_map(|event| match event {
                Event::Arena(index, status) => Some((*index, *status)),
                Event::Lifecycle(_) | Event::Host(_) => None,
            })
            .collect()
    }

    /// Counts of real Layer arena `Created`, `Committed` and `Dropped` events.
    fn arena_counts(&self) -> (usize, usize, usize) {
        let events = self.arena_events();
        let count = |status| events.iter().filter(|(_, seen)| *seen == status).count();
        (
            count(LayerArenaLifecycleStatus::Created),
            count(LayerArenaLifecycleStatus::Committed),
            count(LayerArenaLifecycleStatus::Dropped),
        )
    }

    fn anchored_host_closures(&self) -> Vec<AnchoredHostClosureStatus> {
        self.events
            .lock()
            .expect("runtime observation trace poisoned")
            .iter()
            .filter_map(|event| match event {
                Event::Host(status) => Some(*status),
                Event::Lifecycle(_) | Event::Arena(_, _) => None,
            })
            .collect()
    }

    fn consumption_starts(&self, module_id: &str) -> usize {
        self.lifecycle()
            .iter()
            .filter(|record| {
                record.module_id.as_str() == module_id
                    && record.lifecycle_phase == PreparationLifecyclePhase::Consumption
                    && record.status == PreparationLifecycleStatus::Start
            })
            .count()
    }
}

impl PreparationLifecycleObserver for Trace {
    fn on_preparation_lifecycle(&self, record: &PreparationLifecycleRecord) {
        self.events
            .lock()
            .expect("runtime observation trace poisoned")
            .push(Event::Lifecycle(record.clone()));
    }

    fn on_layer_arena(&self, global_layer_index: u32, status: LayerArenaLifecycleStatus) {
        self.events
            .lock()
            .expect("runtime observation trace poisoned")
            .push(Event::Arena(global_layer_index, status));
    }

    fn on_anchored_host_closure(&self, status: AnchoredHostClosureStatus) {
        self.events
            .lock()
            .expect("runtime observation trace poisoned")
            .push(Event::Host(status));
    }
}

fn records_in(
    trace: &Trace,
    module_id: &str,
    status: PreparationLifecycleStatus,
) -> Vec<PreparationLifecycleRecord> {
    trace
        .lifecycle()
        .into_iter()
        .filter(|record| record.module_id.as_str() == module_id && record.status == status)
        .collect()
}

fn ready_record(trace: &Trace, module_id: &str) -> PreparationLifecycleRecord {
    let mut ready = records_in(trace, module_id, PreparationLifecycleStatus::Ready);
    assert_eq!(
        ready.len(),
        1,
        "exactly one atomic Ready publication for {module_id}: {ready:?}"
    );
    ready.remove(0)
}

fn disposed_record(trace: &Trace, module_id: &str) -> PreparationLifecycleRecord {
    let mut disposed = records_in(trace, module_id, PreparationLifecycleStatus::Disposed);
    assert_eq!(
        disposed.len(),
        1,
        "exactly one disposal for {module_id}: {disposed:?}"
    );
    disposed.remove(0)
}

/// Stage the actual generated component beside its manifest, as `module_dirs`
/// discovery expects (`{stem}.toml` + `{stem}.wasm`).
struct Fixture {
    staging: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let staging = TempDir::new().expect("stage the generated visual fixture");
        let component = workspace_root()
            .join("crates/slicer-wasm-host/test-guests")
            .join(format!("{MODULE_STEM}.component.wasm"));
        assert!(
            component.is_file(),
            "required generated preparation visual component absent: {} \
             (run `cargo xtask build-guests`; a silent skip or substitute is not permitted)",
            component.display()
        );
        let manifest_path = staging.path().join(format!("{MODULE_STEM}.toml"));
        let component_path = staging.path().join(format!("{MODULE_STEM}.wasm"));
        std::fs::write(&manifest_path, MODULE_MANIFEST).unwrap_or_else(|error| {
            panic!("stage manifest at {}: {error}", manifest_path.display())
        });
        std::fs::copy(&component, &component_path).unwrap_or_else(|error| {
            panic!(
                "stage component {} -> {}: {error}",
                component.display(),
                component_path.display()
            )
        });
        Self { staging }
    }

    /// Real wedge model plus every core module except the core path-optimizer,
    /// which the staged fixture must win through the real claim arbitration.
    ///
    /// Integrated modules are disabled so the external component (not the
    /// in-process native pair) is the selected `Layer::PathOptimization`
    /// artifact; this is the exact external-WASM leg this gate owns.
    fn options(&self) -> SliceRunOptions {
        let root = workspace_root();
        let model = root.join("resources").join("regression_wedge.stl");
        let module_dir = root.join("modules").join("core-modules");
        assert!(
            model.is_file(),
            "required regression model absent: {}",
            model.display()
        );
        assert!(
            module_dir.is_dir(),
            "core module root absent: {}",
            module_dir.display()
        );
        let mut module_dirs = vec![self.staging.path().to_owned()];
        for entry in std::fs::read_dir(&module_dir).expect("read core module roots") {
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
            if manifest["stage"]["id"].as_str() == Some("Layer::PathOptimization") {
                continue;
            }
            module_dirs.push(path);
        }
        module_dirs.sort();
        let mesh = Arc::new(
            slicer_model_io::load_model(&model).expect("regression wedge model must load"),
        );
        // The transport schedule this gate pins independently: layer 0 at
        // first_layer_height 0.2, layer 1 at 0.2 + layer_height 0.2 = 0.4.
        let mut options = SliceRunOptions {
            mesh,
            model_label: model.to_string_lossy().into_owned(),
            module_dirs,
            no_default_module_paths: true,
            no_integrated_modules: true,
            ..Default::default()
        };
        options.config_overrides.extend([
            (
                "preparation_visual_mode".into(),
                ConfigValue::String("valid".into()),
            ),
            (
                "preparation_visual_token".into(),
                ConfigValue::String("PRB1".into()),
            ),
            ("layer_height".into(), ConfigValue::Float(0.2)),
            ("first_layer_height".into(), ConfigValue::Float(0.2)),
        ]);
        options
    }
}

fn extensions(views: Vec<PreparationViewRequest>, trace: Arc<Trace>) -> SliceRuntimeExtensions {
    SliceRuntimeExtensions::new(Vec::new(), Vec::new(), Vec::new(), Some(trace))
        .with_preparation_views(views)
}

fn owner(capture: &PreparationVisualCapture) -> &PreparationOwnerCapture {
    assert_eq!(
        capture.metadata().owners.len(),
        1,
        "exactly one eligible prepared owner in the fixture print"
    );
    &capture.metadata().owners[0]
}

fn projection_capture(capture: &PreparationVisualCapture) -> &StageCapture {
    assert_eq!(
        capture.captures().len(),
        1,
        "one rendered layer and one requested view produce exactly one typed capture"
    );
    &capture.captures()[0]
}

fn projection_snapshot(capture: &PreparationVisualCapture) -> &Arc<CommittedPreparationProjection> {
    let stage = projection_capture(capture);
    let CapturedIr::PreparationProjection(snapshot) = &stage.ir else {
        panic!(
            "expected a committed preparation projection capture, got {:?}",
            stage.ir
        );
    };
    snapshot
}

fn assert_z(actual: f32, expected: f32, label: &str) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "{label}: expected Z {expected}, got {actual}"
    );
}

fn assert_probe_pieces(capture: &PreparationCapture) {
    let owner = &capture.owners[0];
    assert_eq!(owner.pieces.len(), 1, "one private piece: {owner:?}");
    assert_eq!(owner.pieces[0].name, PROBE_NAME);
    assert_eq!(owner.pieces[0].length, PROBE_BYTES);
}

#[test]
fn capture_is_semantically_neutral() {
    let fixture = Fixture::new();

    // Two real full slices through the production facade. The only difference
    // is the requested view: the baseline extension requests none.
    let baseline_trace = Arc::new(Trace::default());
    let baseline = run_slice_with_runtime_extensions(
        fixture.options(),
        extensions(Vec::new(), Arc::clone(&baseline_trace)),
    )
    .expect("full wedge slice without probe_xy capture");

    let capture_trace = Arc::new(Trace::default());
    let captured = run_slice_with_runtime_extensions(
        fixture.options(),
        extensions(vec![view_request()], Arc::clone(&capture_trace)),
    )
    .expect("full wedge slice with probe_xy capture");

    // Differential oracle: the entire emitted G-code is byte-identical. This
    // covers ordinary probe comments, the module-owned plan result and every
    // other emitted command, without trusting either run as the other's baseline.
    assert_eq!(
        captured.gcode_text, baseline.gcode_text,
        "requesting a typed preparation view must not perturb emitted G-code"
    );
    assert_eq!(captured.layer_count, baseline.layer_count);

    for (label, gcode) in [
        ("baseline", &baseline.gcode_text),
        ("capture", &captured.gcode_text),
    ] {
        // Independently pinned ordinary comment literals.
        assert!(
            gcode.contains(PROBE_COMMENT),
            "{label} must emit the fixture's ordinary probe comment"
        );
        assert!(
            gcode.contains(TOKEN_COMMENT),
            "{label} must emit the module-decoded four-byte token"
        );
        // Negative control: diagnostic identities never leak into G-code.
        assert!(
            !gcode.contains("anchor_a") && !gcode.contains("domain_a"),
            "{label} must not emit preparation diagnostic identities"
        );
    }
    let probe_comments = baseline.gcode_text.matches(PROBE_COMMENT).count();
    assert!(probe_comments > 0, "the wedge must actually consume layers");
    assert_eq!(
        captured.gcode_text.matches(PROBE_COMMENT).count(),
        probe_comments
    );
    assert_eq!(
        captured.gcode_text.matches(TOKEN_COMMENT).count(),
        probe_comments
    );

    // Private piece names/lengths and the module-owned plan result agree
    // between the two runs, and the Ready accounting excludes diagnostics.
    let baseline_ready = ready_record(&baseline_trace, MODULE_ID);
    let capture_ready = ready_record(&capture_trace, MODULE_ID);
    assert_eq!(
        baseline_ready.pieces,
        vec![(PROBE_NAME.to_owned(), PROBE_BYTES)]
    );
    assert_eq!(capture_ready.pieces, baseline_ready.pieces);
    assert_eq!(
        baseline_ready.retained_bytes,
        PROBE_BYTES + PROBE_NAME_BYTES
    );
    assert_eq!(
        capture_ready.retained_bytes, baseline_ready.retained_bytes,
        "diagnostic bytes must never be folded into ordinary retained accounting"
    );

    let baseline_starts = baseline_trace.consumption_starts(MODULE_ID);
    assert!(
        baseline_starts > 0,
        "ordinary consumers must actually decode the plan"
    );
    assert_eq!(capture_trace.consumption_starts(MODULE_ID), baseline_starts);
    let baseline_disposed = disposed_record(&baseline_trace, MODULE_ID);
    let capture_disposed = disposed_record(&capture_trace, MODULE_ID);
    assert_eq!(
        capture_disposed.transferred_bytes, baseline_disposed.transferred_bytes,
        "capture must not change ordinary piece-read accounting"
    );
    assert_eq!(
        baseline_disposed.transferred_bytes,
        baseline_starts as u64 * PROBE_BYTES,
        "each ordinary consumer returns exactly the four-byte probe"
    );

    // Unrequested path through the same production visual facade: no view is
    // requested, so no projection publication exists and no diagnostic
    // accounting is retained. (Store `staged_bytes` is only live inside the
    // prepare transaction; the runtime companion exposes the post-commit
    // retained count, asserted here as zero.)
    let unrequested = run_slice_preparation_visual_capture(
        fixture.options(),
        extensions(Vec::new(), Arc::new(Trace::default())),
        vec![0],
        Vec::new(),
    )
    .expect("preparation-only visual capture with no requested view");
    assert!(
        unrequested.captures().is_empty(),
        "unrequested path must publish zero projection captures: {:?}",
        unrequested.captures()
    );
    assert!(
        unrequested.diagnostic_accounting().is_empty(),
        "unrequested path must retain zero diagnostic bytes: {:?}",
        unrequested.diagnostic_accounting()
    );
    assert_eq!(unrequested.rendered_layer_indices(), &[0]);
    assert!(unrequested.executed_layer_indices().is_empty());
    let unrequested_owner = owner(&unrequested);
    assert_probe_pieces(unrequested.metadata());
    assert_eq!(
        unrequested_owner
            .accounting_before_disposal
            .retained_payload_bytes,
        PROBE_BYTES
    );
    assert_eq!(
        unrequested_owner
            .accounting_before_disposal
            .retained_name_bytes,
        PROBE_NAME_BYTES
    );
    assert_eq!(
        unrequested_owner
            .accounting_before_disposal
            .returned_read_bytes,
        0
    );
    assert_eq!(unrequested.metadata().retained_bytes, 0);
    assert_eq!(unrequested.metadata().transferred_bytes, 0);

    // Requested path: a nonempty committed typed snapshot and actual
    // diagnostic accounting, separate from the owner's ordinary pieces.
    let requested = run_slice_preparation_visual_capture(
        fixture.options(),
        extensions(vec![view_request()], Arc::new(Trace::default())),
        vec![0],
        Vec::new(),
    )
    .expect("preparation-only visual capture with probe_xy requested");
    let stage = projection_capture(&requested);
    assert_eq!(stage.stage_id, "Layer::PathOptimization");
    assert_eq!(stage.layer_index, 0);
    assert_z(stage.layer_z, 0.2, "requested capture layer Z");
    let snapshot = projection_snapshot(&requested);
    let scene = snapshot.scene();
    assert_eq!(snapshot.owner_module_id(), MODULE_ID);
    assert_eq!(snapshot.view(), VIEW);
    assert_eq!(scene.scene_id, "final");
    assert_eq!(scene.view, VIEW);
    assert_eq!(scene.classes, ["anchors", "domains", "paths", "remote"]);
    assert_eq!(
        scene
            .primitives
            .iter()
            .map(|primitive| primitive.id.as_str())
            .collect::<Vec<_>>(),
        ["anchor_a", "path_a", "domain_a", "outline_a", "remote_a"]
    );
    assert_eq!(snapshot.diagnostic_bytes(), SCENE_BYTES);

    assert_eq!(
        requested.diagnostic_accounting(),
        &[(MODULE_ID.to_owned(), SCENE_BYTES)],
        "actual retained diagnostic accounting, pinned by an independent inventory"
    );
    assert_probe_pieces(requested.metadata());
    assert_eq!(
        requested.metadata().owners[0]
            .accounting_before_disposal
            .retained_payload_bytes,
        PROBE_BYTES,
        "diagnostics are not opaque piece payload"
    );
    assert_eq!(
        requested.metadata().owners[0]
            .accounting_before_disposal
            .retained_name_bytes,
        PROBE_NAME_BYTES
    );
    // Disposal cleared ordinary retention; typed snapshots remain usable.
    assert_eq!(requested.metadata().retained_bytes, 0);
    assert_eq!(requested.metadata().transferred_bytes, 0);
}

#[test]
fn preparation_only_uses_production_prefix() {
    // Schema `1.4.0`'s model-source tap
    // `{"preparation":{"module_id":"test.preparation-visual","view":"probe_xy"}}`
    // resolves to exactly this runtime request; the CLI schema decoder is
    // Step 15/16 and this gate owns the runtime facade it resolves to.
    let fixture = Fixture::new();
    let trace = Arc::new(Trace::default());
    let capture = run_slice_preparation_visual_capture(
        fixture.options(),
        extensions(vec![view_request()], Arc::clone(&trace)),
        vec![0],
        Vec::new(),
    )
    .expect("preparation-only visual capture through the shared startup prefix");

    // The public facade is the only entry: `prepare_prepass_context` is never
    // called here. Shared-prefix evidence is observable startup state — a
    // frozen external artifact identity, the real committed schedule and a
    // Ready owner produced from committed selection.
    let owner = owner(&capture);
    assert_eq!(owner.module_id.as_str(), MODULE_ID);
    assert_eq!(owner.module_version, "0.1.0");
    assert!(
        owner.artifact_identity.starts_with("wasm:"),
        "staged external component must be the selected artifact: {}",
        owner.artifact_identity
    );
    assert_eq!(owner.consumer_stage.as_str(), "Layer::PathOptimization");
    assert!(!owner.selected_targets.is_empty());
    assert!(owner.selected_targets.iter().all(|target| {
        target.stage_id == "Layer::PathOptimization"
            && target.invoke
            && matches!(target.source, PreparationTargetSource::Model { .. })
    }));
    assert!(
        owner
            .selected_targets
            .iter()
            .any(|target| target.global_layer_index == 0),
        "selected layer 0 must be a real committed target"
    );

    let ready = ready_record(&trace, MODULE_ID);
    assert_eq!(ready.status, PreparationLifecycleStatus::Ready);
    assert_eq!(
        ready.lifecycle_phase,
        PreparationLifecyclePhase::Preparation
    );
    assert_eq!(
        ready.global_layer_index, None,
        "whole-print preparation has no consuming layer"
    );
    assert_eq!(ready.pieces, vec![(PROBE_NAME.to_owned(), PROBE_BYTES)]);

    // No Layer arena and no ordinary consumer call is entered.
    assert_eq!(trace.arena_counts(), (0, 0, 0));
    assert!(
        trace.anchored_host_closures().is_empty(),
        "no synthetic anchored host closure may run: {:?}",
        trace.anchored_host_closures()
    );
    assert_eq!(trace.consumption_starts(MODULE_ID), 0);
    assert!(capture.executed_stage_ids().is_empty());
    assert!(capture.executed_layer_indices().is_empty());
    assert_eq!(capture.rendered_layer_indices(), &[0]);

    // Snapshot Z is the real committed schedule Z, independently pinned by the
    // transport config: 0/Z0.2 and 1/Z0.4.
    let schedule = capture.schedule();
    assert!(
        schedule.len() > 1,
        "the wedge schedules more than one layer"
    );
    assert_eq!(schedule[0].index, 0);
    assert_z(schedule[0].z, 0.2, "committed layer 0");
    assert_eq!(schedule[1].index, 1);
    assert_z(schedule[1].z, 0.4, "committed layer 1");

    let stage = projection_capture(&capture);
    assert_eq!(stage.layer_index, 0);
    assert_z(stage.layer_z, 0.2, "capture layer Z");
    assert_z(
        stage.layer_z,
        schedule[0].z,
        "capture Z equals the real schedule Z",
    );
    let snapshot = projection_snapshot(&capture);
    assert_eq!(snapshot.layer_z(0), Some(schedule[0].z));
    assert_eq!(
        snapshot.layer_z(1),
        Some(schedule[1].z),
        "the whole-print scene carries the real second schedule layer"
    );
    assert_eq!(
        capture.diagnostic_accounting(),
        &[(MODULE_ID.to_owned(), SCENE_BYTES)]
    );

    // Metadata-only accounting still reports actual transfer and post-disposal
    // retention, both zero because no consumer read the private piece.
    assert_eq!(capture.metadata().transferred_bytes, 0);
    assert_eq!(capture.metadata().retained_bytes, 0);
    let disposed = disposed_record(&trace, MODULE_ID);
    assert_eq!(
        disposed.lifecycle_phase,
        PreparationLifecyclePhase::Disposal
    );
    assert_eq!(disposed.retained_bytes, 0);
}

/// Count actual preparation entries (phase `Preparation`, status `Start`).
/// `NoWork`/`Disposed` records are lifecycle observations, not invocations.
fn preparation_starts(trace: &Trace) -> usize {
    trace
        .lifecycle()
        .iter()
        .filter(|record| {
            record.lifecycle_phase == PreparationLifecyclePhase::Preparation
                && record.status == PreparationLifecycleStatus::Start
        })
        .count()
}

#[test]
fn multi_layer_capture_shares_one_scene_and_one_accounting_entry() {
    let fixture = Fixture::new();
    let capture = run_slice_preparation_visual_capture(
        fixture.options(),
        extensions(vec![view_request()], Arc::new(Trace::default())),
        vec![0, 1],
        Vec::new(),
    )
    .expect("two-layer preparation visual capture");

    assert_eq!(capture.rendered_layer_indices(), &[0, 1]);
    assert_eq!(
        capture.captures().len(),
        2,
        "one typed capture per rendered layer"
    );
    let schedule = capture.schedule();
    assert!(
        schedule.len() >= 2,
        "the wedge schedules more than one layer"
    );
    let mut snapshots = Vec::new();
    for (capture_index, stage) in capture.captures().iter().enumerate() {
        assert_eq!(
            stage.layer_index,
            capture.rendered_layer_indices()[capture_index],
            "capture order follows the rendered layer selection"
        );
        assert_z(
            stage.layer_z,
            schedule[capture_index].z,
            "capture Z equals the real schedule Z",
        );
        let CapturedIr::PreparationProjection(snapshot) = &stage.ir else {
            panic!(
                "layer {} capture is not a committed preparation projection: {:?}",
                stage.layer_index, stage.ir
            );
        };
        snapshots.push(snapshot);
    }
    assert!(
        Arc::ptr_eq(snapshots[0], snapshots[1]),
        "both rendered layers must share one immutable whole-print scene Arc"
    );
    assert_eq!(snapshots[0].scene(), snapshots[1].scene());
    assert_eq!(
        capture.diagnostic_accounting(),
        &[(MODULE_ID.to_owned(), SCENE_BYTES)],
        "one accounting entry per owner/view, never once per rendered layer"
    );
    assert_eq!(capture.metadata().retained_bytes, 0);
    assert_eq!(capture.metadata().transferred_bytes, 0);
}

#[test]
fn pre_execution_rejections_never_invoke_preparation() {
    let fixture = Fixture::new();

    // Unknown owner: rejected before any owner can enter preparation.
    let trace = Arc::new(Trace::default());
    let error = match run_slice_preparation_visual_capture(
        fixture.options(),
        extensions(
            vec![PreparationViewRequest {
                module_id: "test.unknown".into(),
                view: VIEW.into(),
            }],
            Arc::clone(&trace),
        ),
        vec![0],
        Vec::new(),
    ) {
        Ok(capture) => {
            panic!(
                "unknown owner must fail closed; rendered {:?}",
                capture.rendered_layer_indices()
            )
        }
        Err(error) => error,
    };
    assert!(
        matches!(
            error,
            PreparationVisualCaptureError::UnknownOwner { ref module_id }
                if module_id.as_str() == "test.unknown"
        ),
        "unknown owner: {error:?}"
    );
    assert_eq!(
        preparation_starts(&trace),
        0,
        "unknown owner must not invoke preparation"
    );
    assert_eq!(trace.arena_counts(), (0, 0, 0));

    // Unknown view on the declared, eligible owner.
    let trace = Arc::new(Trace::default());
    let error = match run_slice_preparation_visual_capture(
        fixture.options(),
        extensions(
            vec![PreparationViewRequest {
                module_id: MODULE_ID.into(),
                view: "undeclared_xy".into(),
            }],
            Arc::clone(&trace),
        ),
        vec![0],
        Vec::new(),
    ) {
        Ok(capture) => {
            panic!(
                "unknown view must fail closed; rendered {:?}",
                capture.rendered_layer_indices()
            )
        }
        Err(error) => error,
    };
    assert!(
        matches!(
            error,
            PreparationVisualCaptureError::UnknownView { ref module_id, ref view }
                if module_id.as_str() == MODULE_ID && view == "undeclared_xy"
        ),
        "unknown view: {error:?}"
    );
    assert_eq!(
        preparation_starts(&trace),
        0,
        "unknown view must not invoke preparation"
    );

    // Layer outside the committed schedule (the 0.2 mm wedge schedules
    // 0..=199): rejected before preparation.
    let trace = Arc::new(Trace::default());
    let error = match run_slice_preparation_visual_capture(
        fixture.options(),
        extensions(vec![view_request()], Arc::clone(&trace)),
        vec![9999],
        Vec::new(),
    ) {
        Ok(capture) => {
            panic!(
                "unknown layer must fail closed; rendered {:?}",
                capture.rendered_layer_indices()
            )
        }
        Err(error) => error,
    };
    assert!(
        matches!(
            error,
            PreparationVisualCaptureError::UnknownLayer {
                global_layer_index: 9999
            }
        ),
        "unknown layer: {error:?}"
    );
    assert_eq!(
        preparation_starts(&trace),
        0,
        "unknown layer must not invoke preparation"
    );
    assert_eq!(trace.arena_counts(), (0, 0, 0));
    assert!(trace.anchored_host_closures().is_empty());
}
