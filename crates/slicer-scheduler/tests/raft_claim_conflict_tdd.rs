#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

#[test]
fn raft_fill_double_holder_conflicts() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("modules/core-modules/raft-default");
    let temp = tempfile::tempdir().expect("create duplicate-manifest directory");
    let manifest_a = temp.path().join("raft-default.toml");
    let manifest_b = temp.path().join("raft-duplicate.toml");
    let wasm_a = temp.path().join("raft-default.wasm");
    let wasm_b = temp.path().join("raft-duplicate.wasm");
    let source_toml = fs::read_to_string(source.join("raft-default.toml"))
        .expect("raft-default manifest must exist");
    fs::write(&manifest_a, &source_toml).expect("stage first manifest");
    fs::write(
        &manifest_b,
        source_toml.replace("com.core.raft-default", "com.test.raft-duplicate"),
    )
    .expect("stage second manifest");
    fs::copy(source.join("raft-default.wasm"), &wasm_a).expect("stage first guest");
    fs::copy(source.join("raft-default.wasm"), &wasm_b).expect("stage second guest");
    let loaded_a = slicer_scheduler::load_module_from_paths(&manifest_a, &wasm_a)
        .expect("first Layer::Infill manifest loads");
    let loaded_b = slicer_scheduler::load_module_from_paths(&manifest_b, &wasm_b)
        .expect("second Layer::Infill manifest loads");

    // exhaustive: this contract fixture initializes every validation request field.
    let result = slicer_scheduler::validate_startup_dag(&slicer_scheduler::DagValidationRequest {
        access_audits: Vec::new(),
        claim_holders: vec![
            slicer_scheduler::ClaimHolder {
                claim: "claim:raft-fill".to_string(),
                module_id: "com.core.raft-default".into(),
                scope: slicer_scheduler::ConflictScope::Global,
            },
            slicer_scheduler::ClaimHolder {
                claim: "claim:raft-fill".to_string(),
                module_id: "com.test.raft-duplicate".into(),
                scope: slicer_scheduler::ConflictScope::Global,
            },
        ],
        host_ir_schema_version: Default::default(),
        host_version: Default::default(),
        modules: vec![loaded_a, loaded_b],
        stage_dags: Vec::new(),
    });

    let conflicts: Vec<_> = result
        .errors
        .iter()
        .filter_map(|diagnostic| match &diagnostic.detail {
            slicer_scheduler::SchedulerError::ClaimConflict {
                claim,
                module_a,
                module_b,
                scope,
            } => Some((claim, module_a, module_b, scope)),
            _ => None,
        })
        .collect();
    assert_eq!(
        conflicts.len(),
        1,
        "expected exactly one raft-fill conflict"
    );
    let (claim, module_a, module_b, scope) = conflicts[0];
    assert_eq!(claim, "claim:raft-fill");
    assert_eq!(module_a, "com.core.raft-default");
    assert_eq!(module_b, "com.test.raft-duplicate");
    // Observed scope: the two global claim holders conflict globally.
    assert_eq!(scope, &slicer_scheduler::ConflictScope::Global);
}
