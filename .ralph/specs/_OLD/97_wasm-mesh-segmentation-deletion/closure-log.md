# Closure Log — Packet 97: WASM mesh-segmentation surface deletion

**Closed:** 2026-06-13 · **Task:** TASK-247 · **Status:** implemented

## Summary

Pure deletion of the dead WASM-guest mesh-segmentation surface. Packet 94
(TASK-244) had already retired the host `PrePass::MeshSegmentation` stage,
kernel, host producer, kernel unit test, and executor test, and explicitly
deferred the WASM-guest infrastructure to P97. This packet removed that
infrastructure in full. The deleted path was dead code (no live producer/
consumer), confirmed by byte-identical g-code (AC-17).

## Packet-doc reconciliation (pre-implementation)

The packet was authored against the original P94 framing (host built-in kept)
and was internally contradictory once packet-94 reality landed. Ground-truth
inventory confirmed:

- ABSENT (already deleted by packet 94): `slicer-core/src/algos/mesh_segmentation.rs`,
  `slicer-core/tests/algo_mesh_segmentation_tdd.rs`,
  `slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs`,
  `slicer-runtime/src/builtins/mesh_segmentation_producer.rs`.
- `slicer-runtime/src/lib.rs` registers no mesh-seg producer;
  `builtin_producers_tdd.rs:60` records "P94r removed mesh_segmentation; expected 7 producers".

Amendments applied (user-approved before any deletion):
- **AC-9** — was "kernel test PRESERVED; executor test REWIRED"; now asserts all
  four kernel/WASM test files ABSENT (no rewire; nothing to preserve — the stage
  is fully retired, not host-built-in).
- **AC-14** — survivor allow-list corrected to narrative/doc-only; the kernel,
  kernel test, executor test, and host producer are NOT survivors (already gone).
- **requirements.md** "KEPT" + Out-of-Scope corrected to packet-94 reality;
  In-Scope expanded with the ~15 sites the original list under-enumerated
  (Step-1 inventory is authoritative per packet intent).

## Scope executed (inventory-driven, disjoint-file parallel workers)

WIT + slicer-schema/lib + slicer-core/stage_io; slicer-ir types + re-exports +
ir_tests; slicer-sdk builder/trait/types/prelude + test; slicer-macros arm +
trait default + binding test; slicer-wasm-host host.rs + dispatch.rs +
builder-validation test; module dir + `sdk-prepass-meshseg-guest` test-guest +
`prepass-guest` cleanup + root `Cargo.toml` workspace-member; slicer-runtime
blackboard/prepass/instrumentation + dispatch_tdd + 2 macro tests deleted +
all-worlds/wit-drift/guest-freshness/e2e/pipeline tests + bench + tests/main.rs
mod decls; slicer-scheduler execution_plan + 2 tests; pnp-cli module_new + test.

Two extras caught by a broadened probe (first inventory regex missed `meshseg`/
hyphen spellings): `manifest_ingestion_tdd.rs:807` (`com.core.mesh-segmentation`
NON_PLACEHOLDER entry) and `guest_fixture_freshness_tdd.rs` (sdk-prepass-meshseg
entries).

W-SDK additionally removed `TrianglePaintMark` + `ObjectMeshModification`
(mesh-seg-only dependents — probe verified zero surviving references).
`PaintValueView` retained (still used by 6 files — wasm-host/macros/sdk/prelude).

## Acceptance results

| AC | Result |
|----|--------|
| AC-1 module dir deleted | PASS (GONE) |
| AC-2 WIT resource/export removed | PASS |
| AC-3 / AC-4 host.rs / dispatch.rs | PASS |
| AC-5 / AC-16 guest rebuild + `--check` | PASS (31 guests, CLEAN) |
| AC-6 blackboard accessor/commit | PASS |
| AC-7 prepass slot | PASS |
| AC-8 IR types | PASS |
| AC-9 (amended) 4 kernel/WASM test files ABSENT | PASS |
| AC-10 dispatch_tdd arms | PASS |
| AC-11 scaffolder + test | PASS |
| AC-12 scheduler canonical-stage tables | PASS |
| AC-13 bench entry | PASS |
| AC-14 (amended) survivor audit | PASS (only allowed `builtin_producers_tdd.rs:60` comment) |
| AC-15 clippy + workspace test | PASS (clippy clean; suite green after 21→20 count fix) |
| AC-17 byte-identical g-code | PASS (wedge + cube MATCH baseline) |
| AC-N1 host-producer count unchanged (7) | PASS (builtin_producers_tdd passes; no producer touched) |
| AC-N2 no surviving WIT consumer | PASS |

## Baselines (pre == post, byte-identical)

- WEDGE `resources/regression_wedge.stl`: `aa4da2faeca139f2c17909051497d6998f71bfb8a2dd9856d286296252ef1e3b`
- CUBE  `resources/cube_4color.3mf`:       `ad0245c3463174606718d13675b1f9b4f1c09b6af5fdf13f3c2ec791dab54ebf`

## Gate evidence

- `cargo check --workspace --all-targets` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean (only pre-existing
  nom v3.2.1 / quick-xml v0.22.0 future-incompat notice, documented since TASK-244).
- `cargo xtask build-guests && cargo xtask build-guests --check` — 31 guests, CLEAN.
- `cargo test --workspace` — run once; one failure:
  `manifest_ingestion_tdd::core_modules_directory_is_discoverable_and_all_load`
  asserted a hardcoded 21 core modules; deleting the mesh-segmentation module made
  it 20. Fixed (21→20, comment updated). `scheduler_integration` bucket re-run green
  (52/52). Post-audit, the full `cargo clippy --workspace --all-targets -- -D warnings`
  + `cargo test --workspace` were re-run on the final tree (pipefail-gated): clippy
  clean, **192 test buckets `ok`, 0 failed** — final-state confirmed green.

## Net change

Core-module count: **21 → 20** (mesh-segmentation removed).
Host built-in producer count: **7** (unchanged — packet 94 already removed mesh_segmentation).

## Doc cleanup (done in P97, not deferred to P5c)

Per the production-readiness directive, all stale mesh-segmentation references in the
canonical docs were corrected here rather than deferred to packet 99:

- `docs/01_system_architecture.md` — removed the retired `PrePass::MeshSegmentation` from the PrePass tier description, the Stage I/O contract table, and the cross-stage dependency matrix; noted sub-facet normalization now happens at model-load (`split_triangle_strokes`).
- `docs/02_ir_schemas.md` — corrected `MeshIR`/`PaintLayer` comments (normalization is load-time, not a stage).
- `docs/03_wit_and_manifest.md` — removed the `mesh-segmentation-output` resource + `run-mesh-segmentation` export from the `world-prepass` WIT example.
- `docs/04_host_scheduler.md` — removed `StageId::PrePassMeshSegmentation` from `STAGE_ORDER`, rewrote the modifier-routing prose to the per-layer subtract stage, and dropped the stage from the pipeline diagram.
- `docs/05_module_sdk.md` — removed the `run_mesh_segmentation`/`MeshSegmentationOutput` table row, trait-method, and prelude export; repointed the test-guest exemplar to the real `crates/slicer-wasm-host/test-guests/sdk-prepass-guest/`.
- `docs/10_scenario_traces.md` — rewrote the execution-trace step to load-time normalization.

`docs/07_implementation_status.md` historical task-ledger entries (TASK-244, TASK-247, and older) intentionally retain mesh-segmentation references as the audit trail. Verified: zero stale mesh-segmentation references remain outside `docs/07` history and `docs/specs/` roadmaps.
