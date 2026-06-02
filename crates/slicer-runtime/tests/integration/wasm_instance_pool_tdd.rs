//! Red tests for TASK-024 WASM instance pool planning and leasing.

use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use slicer_ir::SemVer;
use slicer_runtime::{
    build_wasm_instance_pool, InstancePoolError, InstancePoolMode, LoadedModule,
    LoadedModuleBuilder, WasmArtifactMetadata,
};

#[test]
fn parallel_safe_modules_use_requested_host_parallelism_as_pool_size() {
    let module = loaded_module(
        "com.example.parallel",
        "Layer::Perimeters",
        true,
        "slicer:world-layer@1.0.0",
    );

    let pool = build_wasm_instance_pool(
        module.id(),
        module.stage(),
        module.layer_parallel_safe(),
        6,
        artifact(false),
    )
    .expect("parallel-safe layer modules should build a pool");

    assert_eq!(pool.mode(), InstancePoolMode::Parallel);
    assert_eq!(pool.size(), 6);
}

#[test]
fn non_parallel_safe_modules_are_forced_to_a_single_serialized_slot() {
    let module = loaded_module(
        "com.example.serial",
        "Layer::Infill",
        false,
        "slicer:world-layer@1.0.0",
    );

    let pool = build_wasm_instance_pool(
        module.id(),
        module.stage(),
        module.layer_parallel_safe(),
        8,
        artifact(false),
    )
    .expect("non-parallel-safe modules should still produce a serialized pool");

    assert_eq!(pool.mode(), InstancePoolMode::Serialized);
    assert_eq!(pool.size(), 1);
}

#[test]
fn finalization_stage_is_always_serialized_even_when_manifest_claims_parallel_safety() {
    let module = loaded_module(
        "com.example.finalizer",
        "PostPass::LayerFinalization",
        true,
        "slicer:world-finalization@1.0.0",
    );

    let pool = build_wasm_instance_pool(
        module.id(),
        module.stage(),
        module.layer_parallel_safe(),
        16,
        artifact(false),
    )
    .expect("finalization modules should still build a serialized pool");

    assert_eq!(pool.mode(), InstancePoolMode::Serialized);
    assert_eq!(pool.size(), 1);
}

#[test]
fn shared_memory_artifacts_are_rejected_when_parallel_safety_is_declared() {
    let module = loaded_module(
        "com.example.shared-memory",
        "Layer::Support",
        true,
        "slicer:world-layer@1.0.0",
    );

    let error = build_wasm_instance_pool(
        module.id(),
        module.stage(),
        module.layer_parallel_safe(),
        4,
        artifact(true),
    )
    .expect_err("shared-memory artifacts must be rejected for parallel-safe modules");

    assert_eq!(
        error,
        InstancePoolError::SharedMemoryRejected {
            module_id: String::from("com.example.shared-memory"),
            stage: String::from("Layer::Support"),
        }
    );
}

#[test]
fn parallel_pools_hand_out_distinct_slots_until_exhausted_then_reuse_released_slot() {
    let module = loaded_module(
        "com.example.parallel-leases",
        "Layer::SlicePostProcess",
        true,
        "slicer:world-layer@1.0.0",
    );

    let pool = build_wasm_instance_pool(
        module.id(),
        module.stage(),
        module.layer_parallel_safe(),
        2,
        artifact(false),
    )
    .expect("parallel-safe modules should build a pool");
    let lease_a = pool.acquire();
    let lease_b = pool.acquire();

    assert_eq!(lease_a.slot_index(), 0);
    assert_eq!(lease_b.slot_index(), 1);

    drop(lease_a);

    let lease_c = pool.acquire();
    assert_eq!(lease_c.slot_index(), 0);
}

#[test]
fn serialized_pools_only_ever_hand_out_slot_zero() {
    let module = loaded_module(
        "com.example.serial-leases",
        "Layer::PerimetersPostProcess",
        false,
        "slicer:world-layer@1.0.0",
    );

    let pool = build_wasm_instance_pool(
        module.id(),
        module.stage(),
        module.layer_parallel_safe(),
        8,
        artifact(false),
    )
    .expect("serialized modules should still build a pool");
    let first = pool.acquire();

    assert_eq!(first.slot_index(), 0);

    drop(first);

    let second = pool.acquire();
    assert_eq!(second.slot_index(), 0);
}

#[test]
fn serialized_pools_block_other_leasers_until_release() {
    let module = loaded_module(
        "com.example.serial-contention",
        "Layer::PerimetersPostProcess",
        false,
        "slicer:world-layer@1.0.0",
    );

    let pool = Arc::new(
        build_wasm_instance_pool(
            module.id(),
            module.stage(),
            module.layer_parallel_safe(),
            8,
            artifact(false),
        )
        .expect("serialized modules should still build a pool"),
    );
    let first = pool.acquire();
    let (tx, rx) = mpsc::channel();
    let pool_for_thread = Arc::clone(&pool);

    let handle = thread::spawn(move || {
        let lease = pool_for_thread.acquire();
        tx.send(lease.slot_index())
            .expect("blocked acquire should eventually succeed");
    });

    assert!(
        rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "second acquire should remain blocked while the only serialized slot is leased"
    );

    drop(first);

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1))
            .expect("second acquire should unblock after release"),
        0
    );
    handle
        .join()
        .expect("contention thread should exit cleanly");
}

fn artifact(uses_shared_memory: bool) -> WasmArtifactMetadata {
    WasmArtifactMetadata { uses_shared_memory }
}

fn loaded_module(
    id: &str,
    stage: &str,
    layer_parallel_safe: bool,
    wit_world: &str,
) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        wit_world,
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(layer_parallel_safe)
    .build()
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}
