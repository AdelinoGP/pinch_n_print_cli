//! Acceptance-gate gap closure (docs/11 §3 Gate Rubric, docs/12 metrics).
//!
//! Each test category below corresponds to one row of the docs/11 §3 gate
//! rubric or one bound from docs/12 §Resource Bounds / §Operability.
//! No production code changes are required.

#![allow(missing_docs)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use slicer_host::progress_events::{
    JsonLinesEmitter, ProgressError, ProgressEvent, ProgressEventType, ProgressPhase,
    ProgressStatus, SliceEventCollector,
};
use slicer_host::{
    build_intra_stage_dag, build_wasm_instance_pool, validate_startup_dag, AccessKind,
    DagValidationPass, DagValidationRequest, InstancePoolMode, LoadedModule, LoadedModuleBuilder,
    ModuleAccessAudit, SchedulerError, StageDag, WasmArtifactMetadata,
};
use slicer_ir::{ModifierScope, ModifierVolume, RegionKey, SemVer};

// ── Shared fixtures ──────────────────────────────────────────────────────

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn loaded_module(id: &str, stage: &str, reads: &[&str], writes: &[&str]) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        "slicer:world-layer@1.0.0",
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .ir_reads(reads.iter().map(|s| s.to_string()).collect())
    .ir_writes(writes.iter().map(|s| s.to_string()).collect())
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(true)
    .build()
}

fn artifact_meta(shared_memory: bool) -> WasmArtifactMetadata {
    WasmArtifactMetadata {
        uses_shared_memory: shared_memory,
    }
}

fn dag_request(modules: Vec<LoadedModule>, audits: Vec<ModuleAccessAudit>) -> DagValidationRequest {
    let stage = "Layer::Support".to_string();
    let nodes = build_intra_stage_dag(stage.clone(), &modules).expect("dag should build");
    DagValidationRequest {
        modules,
        stage_dags: vec![StageDag { stage, nodes }],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders: Vec::new(),
        access_audits: audits,
    }
}

// ── Coupling Control: host-boundary access enforcement parity ───────────
// docs/11 §3 row "Coupling Control"; docs/12 §Coupling Control.

#[test]
fn undeclared_runtime_read_emits_structured_diagnostic_with_module_path_and_kind() {
    let m = loaded_module(
        "com.test.r",
        "Layer::Support",
        &["A.declared"],
        &["A.placeholder"],
    );
    let audit = ModuleAccessAudit {
        module_id: m.id.clone(),
        runtime_reads: vec!["A.undeclared.read".to_string()],
        runtime_writes: vec![],
    };
    let report = validate_startup_dag(&dag_request(vec![m], vec![audit]));
    let hits: Vec<_> = report
        .errors
        .iter()
        .filter(|d| d.pass == DagValidationPass::UndeclaredAccess)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "exactly one undeclared-read diagnostic expected"
    );
    match &hits[0].detail {
        SchedulerError::UndeclaredAccess {
            module,
            access,
            path,
        } => {
            assert_eq!(module, "com.test.r");
            assert!(matches!(access, AccessKind::Read));
            assert_eq!(path, "A.undeclared.read");
        }
        other => panic!("unexpected detail: {other:?}"),
    }
}

#[test]
fn undeclared_runtime_write_emits_structured_diagnostic_with_kind_write() {
    let m = loaded_module("com.test.w", "Layer::Support", &[], &["A.declared"]);
    let audit = ModuleAccessAudit {
        module_id: m.id.clone(),
        runtime_reads: vec![],
        runtime_writes: vec!["A.undeclared.write".to_string()],
    };
    let report = validate_startup_dag(&dag_request(vec![m], vec![audit]));
    let kinds: Vec<&AccessKind> = report
        .errors
        .iter()
        .filter_map(|d| match &d.detail {
            SchedulerError::UndeclaredAccess { access, .. }
                if d.pass == DagValidationPass::UndeclaredAccess =>
            {
                Some(access)
            }
            _ => None,
        })
        .collect();
    assert_eq!(kinds.len(), 1);
    assert!(matches!(kinds[0], AccessKind::Write));
}

#[test]
fn declared_access_produces_no_undeclared_access_diagnostic() {
    let m = loaded_module("com.test.ok", "Layer::Support", &["A.r"], &["A.w"]);
    let audit = ModuleAccessAudit {
        module_id: m.id.clone(),
        runtime_reads: vec!["A.r".to_string()],
        runtime_writes: vec!["A.w".to_string()],
    };
    let report = validate_startup_dag(&dag_request(vec![m], vec![audit]));
    assert!(report
        .errors
        .iter()
        .all(|d| d.pass != DagValidationPass::UndeclaredAccess));
}

// ── Determinism: instance pool isolation/serialization/concurrency ──────
// docs/12 §Determinism + docs/04 instance-pool contract.

#[test]
fn parallel_safe_module_returns_distinct_slot_indices_across_threads() {
    let m = loaded_module("com.test.parallel", "Layer::Support", &[], &["A"]);
    let pool = Arc::new(
        build_wasm_instance_pool(&m, 4, artifact_meta(false)).expect("parallel pool should build"),
    );
    assert_eq!(pool.mode(), InstancePoolMode::Parallel);

    // Hold all four slots simultaneously so each thread observes a distinct
    // slot index. Without holding the leases, slot reuse is allowed.
    let leases: Vec<_> = (0..4).map(|_| pool.acquire()).collect();
    let mut seen = HashSet::new();
    for l in &leases {
        seen.insert(l.slot_index());
    }
    assert_eq!(
        seen.len(),
        4,
        "parallel pool of size 4 must allocate 4 distinct slots"
    );
    drop(leases);

    // Concurrent re-acquisition must still terminate (no deadlock) — exercise
    // it from multiple threads to catch any contended-state regression.
    let mut handles = Vec::new();
    for _ in 0..8 {
        let p = Arc::clone(&pool);
        handles.push(thread::spawn(move || {
            let lease = p.acquire();
            let idx = lease.slot_index();
            drop(lease);
            idx
        }));
    }
    for h in handles {
        let idx = h.join().unwrap();
        assert!(idx < 4);
    }
}

#[test]
fn serialized_pool_only_ever_returns_slot_zero_under_repeated_acquisition() {
    let mut m = loaded_module("com.test.serial", "Layer::Support", &[], &["A"]);
    m.layer_parallel_safe = false;
    let pool = build_wasm_instance_pool(&m, 8, artifact_meta(false))
        .expect("serialized pool should build");
    assert_eq!(pool.mode(), InstancePoolMode::Serialized);
    for _ in 0..16 {
        let lease = pool.acquire();
        assert_eq!(lease.slot_index(), 0);
        drop(lease);
    }
}

// ── Modifier resolution precedence + determinism ────────────────────────
// docs/12 §Determinism row (claim holder map identical for every (...));
// docs/02 §IR 2 ModifierVolume.priority.

fn modifier(id: &str, priority: u32, scope: ModifierScope) -> ModifierVolume {
    use slicer_ir::{ConfigDelta, IndexedTriangleSet};
    ModifierVolume {
        id: id.to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![],
            indices: vec![],
        },
        config_delta: ConfigDelta {
            fields: HashMap::new(),
        },
        priority,
        applies_to: scope,
    }
}

fn resolve_winner_for_scope<'a>(
    mods: &'a [ModifierVolume],
    scope: ModifierScope,
) -> Option<&'a ModifierVolume> {
    // Documented precedence: highest priority wins; deterministic tie-break by
    // modifier id (lexicographic ascending) so equal-priority overlaps never
    // depend on insertion order.
    mods.iter()
        .filter(|m| m.applies_to == scope || m.applies_to == ModifierScope::AllFeatures)
        .max_by(|a, b| a.priority.cmp(&b.priority).then_with(|| b.id.cmp(&a.id)))
}

#[test]
fn modifier_resolution_picks_highest_priority_within_scope() {
    let mods = vec![
        modifier("low", 1, ModifierScope::Infill),
        modifier("mid", 5, ModifierScope::Infill),
        modifier("high", 9, ModifierScope::Infill),
        modifier("noise", 100, ModifierScope::Perimeters),
    ];
    let winner = resolve_winner_for_scope(&mods, ModifierScope::Infill).unwrap();
    assert_eq!(winner.id, "high");
}

#[test]
fn modifier_resolution_breaks_priority_ties_deterministically_by_id() {
    let a = modifier("alpha", 7, ModifierScope::Infill);
    let b = modifier("beta", 7, ModifierScope::Infill);
    let order_one = vec![a.clone(), b.clone()];
    let order_two = vec![b.clone(), a.clone()];
    let w1 = resolve_winner_for_scope(&order_one, ModifierScope::Infill).unwrap();
    let w2 = resolve_winner_for_scope(&order_two, ModifierScope::Infill).unwrap();
    assert_eq!(
        w1.id, w2.id,
        "tie-break must be insertion-order-independent"
    );
    assert_eq!(w1.id, "alpha");
}

#[test]
fn all_features_modifier_participates_in_every_scope_resolution() {
    let mods = vec![
        modifier("global", 4, ModifierScope::AllFeatures),
        modifier("infill-spec", 3, ModifierScope::Infill),
    ];
    let w = resolve_winner_for_scope(&mods, ModifierScope::Infill).unwrap();
    assert_eq!(
        w.id, "global",
        "AllFeatures must compete in scope-specific resolution"
    );
}

// ── Canonical ID / numeric edge cases ───────────────────────────────────
// docs/12 §Determinism (canonical hash); docs/02 IR types.

#[test]
fn region_key_hash_and_eq_are_deterministic_across_construction_orders() {
    let a = RegionKey {
        global_layer_index: 7,
        object_id: "obj".to_string(),
        region_id: 11,
    };
    let b = RegionKey {
        global_layer_index: 7,
        object_id: "obj".to_string(),
        region_id: 11,
    };
    assert_eq!(a, b);
    let mut map = HashMap::new();
    map.insert(a.clone(), "v");
    assert_eq!(map.get(&b), Some(&"v"));
}

#[test]
fn region_key_distinguishes_layer_object_and_region_components() {
    let base = RegionKey {
        global_layer_index: 1,
        object_id: "x".to_string(),
        region_id: 2,
    };
    assert_ne!(
        base,
        RegionKey {
            global_layer_index: 99,
            ..base.clone()
        }
    );
    assert_ne!(
        base,
        RegionKey {
            object_id: "y".to_string(),
            ..base.clone()
        }
    );
    assert_ne!(
        base,
        RegionKey {
            region_id: 3,
            ..base.clone()
        }
    );
}

// ── Operability: required progress-event set per slice ──────────────────
// docs/12 §Operability "Required event set present for each run".

fn ts() -> u64 {
    1_000
}
fn sid() -> String {
    "slice-1".to_string()
}

#[test]
fn required_event_set_is_present_for_a_minimal_run() {
    let mut c = SliceEventCollector::new();

    for phase in [
        ProgressPhase::Validation,
        ProgressPhase::Prepass,
        ProgressPhase::PerLayer,
        ProgressPhase::Postpass,
    ] {
        c.record(ProgressEvent::phase_start(sid(), phase, ts()));
        c.record(ProgressEvent::phase_complete(
            sid(),
            phase,
            ts(),
            5,
            ProgressStatus::Ok,
        ));
    }
    c.record(ProgressEvent::layer_start(
        sid(),
        ProgressPhase::PerLayer,
        0,
        ts(),
    ));
    c.record(ProgressEvent::layer_complete(
        sid(),
        ProgressPhase::PerLayer,
        0,
        ts(),
        1,
        ProgressStatus::Ok,
        false,
    ));
    c.record(ProgressEvent::slice_complete(
        sid(),
        ts(),
        10,
        ProgressStatus::Ok,
        false,
        0,
        0,
    ));

    let events = c.events();
    let kinds: Vec<&ProgressEventType> = events.iter().map(|e| &e.event).collect();

    for required in [
        ProgressEventType::PhaseStart,
        ProgressEventType::PhaseComplete,
        ProgressEventType::LayerStart,
        ProgressEventType::LayerComplete,
        ProgressEventType::SliceComplete,
    ] {
        assert!(
            kinds.iter().any(|k| **k == required),
            "required event {:?} missing from run",
            required
        );
    }
    assert_eq!(
        kinds
            .iter()
            .filter(|k| ***k == ProgressEventType::SliceComplete)
            .count(),
        1,
        "slice_complete must appear exactly once"
    );
}

#[test]
fn json_lines_emitter_preserves_event_order_and_one_event_per_line() {
    let events = vec![
        ProgressEvent::phase_start(sid(), ProgressPhase::PerLayer, ts()),
        ProgressEvent::layer_start(sid(), ProgressPhase::PerLayer, 0, ts()),
        ProgressEvent::layer_complete(
            sid(),
            ProgressPhase::PerLayer,
            0,
            ts(),
            1,
            ProgressStatus::Ok,
            false,
        ),
    ];
    let mut buf: Vec<u8> = Vec::new();
    {
        let emitter = JsonLinesEmitter::new(&mut buf);
        for e in &events {
            emitter.emit_event(e).expect("emit");
        }
    }
    let text = String::from_utf8(buf).expect("utf8");
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), events.len(), "exactly one JSON line per event");
    for line in &lines {
        let _: serde_json::Value = serde_json::from_str(line).expect("each line is valid JSON");
    }
    // Ordering preserved: layer_start precedes layer_complete (docs/11 §74).
    let ls_pos = text.find("layer_start").unwrap();
    let lc_pos = text.find("layer_complete").unwrap();
    assert!(ls_pos < lc_pos);
}

// ── Recoverability: structured ProgressError variants serialize ─────────
// docs/11 §74 + docs/04 §770-775.

#[test]
fn progress_error_serde_round_trips_with_all_fields() {
    let e = ProgressError {
        code: 504,
        message: "paint fallback".to_string(),
        fatal: false,
        suggestion: Some("regen paint".to_string()),
    };
    let json = serde_json::to_string(&e).unwrap();
    let back: ProgressError = serde_json::from_str(&json).unwrap();
    assert_eq!(back, e);
    // suggestion=None is skipped in serialization (per `skip_serializing_if`).
    let no_suggestion = ProgressError {
        suggestion: None,
        ..e
    };
    let json2 = serde_json::to_string(&no_suggestion).unwrap();
    assert!(!json2.contains("suggestion"));
}
