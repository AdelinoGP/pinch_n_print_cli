---
status: draft
packet: layer-module-preparation_08_lightning-migration
task_ids:
  - TASK-577
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: lightning-migration

## Goal

Move lightning planning out of the host PrePass and into the selected `com.core.lightning-infill` artifact. The planner and its two plan types relocate into the module (`modules/core-modules/lightning-infill/src/kernel/`), preparation plans the trees from the same committed inputs the host producer reads today, fresh ordinary `Layer::Infill` calls consume module-private pieces, a typed `planning_geometry` projection publishes the final plan, and the legacy host producer, public `LightningTreeIR`/`LightningTreeEntry`/`PrePass::LightningTreeGen`, the `lightning-tree-segments` accessor and the `slicer:lightning-queries` WIT package are deleted. Correctness is **recorded behavioural equivalence to the live producer, captured before removal**: this packet moves ownership and changes no output.

## Scope Boundaries

This is a **behaviour-preserving ownership migration**, not a planning improvement. The planning algorithm itself is relocated, not rewritten: the code that `crates/slicer-core/src/algos/lightning/` executes today moves into the module's kernel, with **planning decisions frozen** and only file location, container types, plan-type home and input adaptation permitted. The equivalence oracle is the current producer's own output. Every acceptance criterion below is therefore falsifiable today, without a scientific gate.

The ADR conflict this creates is resolved by dated in-place amendments to ADR-0066 and ADR-0029 (both 2026-10-20), which quote the superseded gating text and separate *ownership move* from *strategy change*.

Two consequences are load-bearing and must not be elided:

- **No new parity claim.** Equivalence preserves whatever the live producer does, including any existing deviation from OrcaSlicer. A green equivalence run says "ownership changed, output did not", never "the plan is canonical". Row 07's recorded `Gate A outcome: fail` and row 09's unexecuted Gate B remain recorded as-is; this packet neither resolves nor re-litigates them.
- **Strategy work is out of scope.** Replacing the current strategy with a gate-verified one is row 09/07 forward work. Nothing here is evidence about it.

The packet is a blocked-draft successor: the earlier revision of this directory made Gate A `pass` and Gate B `pass` entry conditions, which was unsatisfiable (Gate A is recorded `fail`, Gate B has no planner to run). That premise is withdrawn. See `design.md` `## Decisions That Replace the Former Blockers` (D1–D5).

## Prerequisites and Blockers

- Rows 01–06 are implemented in the tree (`layer-module-preparation_01..06` are `status: implemented`) and supply the capability, transport, whole-print inputs, lifecycle and typed projection this packet consumes. Re-derive their status at activation; do not quote it.
- The five Classic fixture inputs used as case material exist at `crates/slicer-wasm-host/tests/fixtures/lightning_gate/` (`inputs.json`, `canonical.json`, `manifest.json`). They are **input material only**; their `canonical.json` is not this packet's oracle.
- The current host producer is live and is the oracle: `crates/slicer-core/src/algos/lightning/` (1729 lines, `generate_lightning_trees` returning `Arc<LightningTreeIR>`), driven by `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs::commit_lightning_tree_ir_builtin`. Because the algorithm's return type *is* the public IR, the planner and the plan types move together; the packet cannot delete one without the other.
- ADR-0066 and ADR-0029 carry dated amendments (2026-10-20) authorizing the ownership move on recorded equivalence while reserving strategy changes for the gates. Re-derive their presence at activation rather than trusting this line.
- **No scientific gate is an entry condition.** Gate A/B remain row 07/09 outcomes and are recorded unchanged.
- No `[BLOCK]` remains. D1–D5 in `design.md` resolve the former B1–B5.
- The former non-zero `python -c ... sys.exit('BLOCKED packet08 ...')` hold is **deleted**. Its replacement is a real, executable baseline validator (AC-1) that fails when the baseline is absent, stale or non-deterministic. Removing that hold required the equivalence oracle, not a directory existing.

## Acceptance Criteria

All tests below are **NET-NEW** and live in `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs`, registered from `crates/slicer-runtime/tests/e2e/main.rs`. `run_slice_against_wedge_returns_nonempty_gcode` (`crates/slicer-runtime/tests/e2e/run_slice_api_tdd.rs`) grounds the production driver this binary reuses; its success-only assertion is not the oracle.

Commands use Bash, preserve Cargo/xtask exits, and require exactly one executed test. `xtask` owns the combined `target/test-output.log`; the separate command log preserves freshness/preflight failures. Read/archive both before overwrite, then remove the old test log under `set -euo pipefail` before every xtask test invocation. A missing fresh log must fail the count guard; it must never reuse a stale pass.

- **AC-1. Given** the live host producer and the five Classic case inputs, **when** the baseline recorder runs the real pipeline twice and writes the committed baseline, **then** two runs are byte-identical, the baseline decodes strictly, and it records for every case a nonempty population of `(global_layer_index, object_id, region_id, variant_chain)` plan entries with their directed edge multisets and final caller-attributed segments, plus the sha256 of the plan and of the inputs it was derived from. A missing case, a tampered hash, an unknown key or an absent baseline is an error, never a silent skip. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::baseline_is_deterministic_and_strict -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-2. Given** the recorded baseline and its recorded input digest, **when** the real `com.core.lightning-infill` native and fresh WASM preparation run from the same committed inputs, **then** each independently reproduces the baseline exactly — identical `(global_layer_index, object_id, region_id, ordered variant_chain)` key set, identical directed edge multisets with multiplicity (not set equality), identical grounding/root witness sets — with nonempty asserted populations. The two adapters are each compared to the baseline, never to each other. A mismatch reports whether the input digest or the plan digest diverged first, so an input-plumbing defect is distinguishable from an algorithm difference. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::module_plan_matches_recorded_baseline -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-3. Given** a published plan and multiple layers, **when** fresh ordinary `Layer::Infill` instances consume their private pieces, **then** the emitted sparse-path segment multiset equals the baseline's per case and per caller, with `ExtrusionRole::SparseInfill` and the same widths, and `variant_chain` from the piece reaches the emitted region identity. Lifecycle accounting is truthful: the preparation-only path has `transferred_bytes == 0` and constructs no consumer, full consumption returns nonzero piece bytes, and `retained_bytes == 0` after success, failure and cancellation. Cleanup never waits inside `dispose_owner` and never revokes an active guard. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::fresh_consumers_emit_baseline_segments -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-4. Given** the module declares view `planning_geometry`, **when** production visual capture requests that owner/view on real scheduled layers, **then** one committed scene `lightning-plan` contains typed `planning_domain` (Polygon, holes preserved), `tree_branch` (open Polyline), `tree_root` and `grounding` (Point) primitives derived from the same final plan as the private pieces, with real `global_layer_index`, class/id/label and integer `Point2`/`ExPolygon` geometry. Scene and pieces commit in one transaction; capture-off produces identical semantic plan and attributed infill; an explicit empty scene succeeds; a requested-but-missing scene fails with a named row06 error. No runtime/CLI private-byte decoder, serialized piece payload or lightning-specific renderer is introduced. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::typed_projection_is_atomic_and_neutral -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-5. Given** the migrated module and freshly rebuilt affected guests, **when** real discovery, scheduler planning and dispatch run, **then** no scheduled `PrePass::LightningTreeGen`, registered `host:lightning_tree`, host lightning producer, public `LightningTreeIR`/`LightningTreeEntry`/`CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION`, `lightning-tree-segments` WIT method, `lightning_tree_segments_for` accessor or `slicer:lightning-queries` package remains in the live contract, while native and WASM outputs still satisfy the recorded baseline. The kernel's plan types exist only as `pub(crate)` items inside the module and are not re-exported from `slicer-ir`. A caller of any removed symbol fails on **that** symbol, not on an unrelated compile or fixture error. No deprecated coexistence, accessor shim or host fallback exists. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::legacy_contract_is_retired -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a valid baseline and a required nonempty contributor/caller population, **when** the comparator is fed a poisoned baseline — a dropped entry, a reversed edge, a dropped `variant_chain` element, a duplicated entry, a swapped region identity, or a length-only stub — **then** each mismatch is rejected naming the case and key, and a comparator that would pass on an empty population or on a length-only comparison fails its own control. A digest-only self-comparison is not equivalence. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::equivalence_rejects_corrupted_baseline -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-N2. Given** a selected owner with a missing required piece, a malformed module-private payload or a failed preparation, **when** real adapters admit ordinary consumers, **then** `PreparedPlanView::require(name)` for an absent piece preserves `ModuleError` code `57401`, `fatal == true` and `message == format!("required preparation piece missing: {}", name)`; malformed bytes fail the module decoder with code `57410` and unsupported piece versions with `57411`, never the host producer. Absent, malformed and version-mismatched pieces remain three distinct failures; `57402`/`57403` (the row-03 fixture transport's malformed-payload codes) must not be reused for them. A failed preparation leaves `current()` `NotReady`, publishes neither plan nor scene and admits zero consumers; the original module `code`/`message`/`fatal` survive cleanup, and storage is released. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::failed_or_malformed_plan_never_falls_back -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-N3. Given** no selected lightning targets, an external artifact selected over the integrated one, or an empty successful preparation, **when** the shared projection evaluates `selection.invoke && !selection.targets.is_empty()`, **then** the inactive owner has zero preparation, zero consumption and zero piece reads, and no fabricated `Ready` plan exists; external selection suppresses the native prepared pair; a selected owner with successful empty preparation is `Ready`/no-work, not a missing prerequisite. Default/non-lightning output is unchanged, and foreign/disposed owner resources remain `AccessDenied`. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::selection_override_empty_and_foreign_owner -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-N4. Given** the retired tree, **when** a compile witness is built against each removed symbol and a lightning slice is run, **then** every witness fails with an unresolved-import/unresolved-name diagnostic naming that symbol and no other removed symbol is implicated, while the slice still emits baseline-equivalent infill through the module path only. A witness that fails for an unrelated reason, or a slice that still produces a host-owned plan, fails this criterion. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::retirement_leaves_no_host_fallback -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`

## Completion Semantics

This packet is **not** a record-only investigation like rows 07/09. Its deliverable is a migration, so:

- A recorded **equivalence `fail` blocks retirement** (Steps 10a–13c). AC-2/AC-3 are hard gates on deletion, because deleting a producer whose replacement differs silently would ship a behaviour change disguised as a refactor. Report the divergence, keep the packet open, do not proceed to deletion.
- A recorded **equivalence `pass` plus every negative control** is required before deletion starts; the packet then closes when AC-5/AC-N4 document that nothing host-owned remains.
- Equivalence `pass` is **not** a parity claim and must not be written up as one. The source-plan record and the `docs/07` note must both say so.
- Gate A/B status is neither improved nor worsened by this packet.

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`
- `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd:: -- --nocapture >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 9 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- `cargo xtask check-literals`
- `cargo xtask check-test-quality --report`
- Documentation placement (Step 15a): `rg -q '^### Lightning private preparation migration$' docs/01_system_architecture.md && rg -q '^### Lightning legacy IR retirement$' docs/02_ir_schemas.md && rg -q '^### Lightning accessor retirement$' docs/03_wit_and_manifest.md`
- Documentation placement (Step 15b): `rg -q '^### Lightning selection-driven preparation$' docs/04_host_scheduler.md && rg -q '^### Lightning prepared module consumption$' docs/05_module_sdk.md && rg -q '^### Lightning private preparation migration$' docs/19_visual_debug.md`

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md`: queue rows 07 and 08 (row 09 has no queue row; it appears only as the Gate B pointer in row 07's Source cell), the late-PrePass geometry questions and the row-08 re-scope record.
- `docs/adr/0066-private-layer-preparation-capability.md`: accepted ownership/atomicity/lifetime/retirement decision (not implemented evidence).
- `docs/00_project_overview.md`: normative doc map; targeted `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md` ground the live legacy contracts and native dispatch.
- `docs/08_coordinate_system.md`, `docs/17_agent_debugging.md`, `docs/19_visual_debug.md`: units, truthful instrumented observations and fail-closed capture behavior.
- `docs/21_data_defaults_and_fixtures.md`, `docs/22_test_quality.md`, `docs/ORCASLICER_ATTRIBUTION.md`: literal discipline, independent falsifiable oracles and port attribution.

## Doc Impact Statement

Implementation replaces live legacy guidance — not historical packet prose — in the architecture/IR/WIT/scheduler/SDK docs and documents the module view in visual-debug; exact headings are listable from the two documentation-placement bullets in `## Verification`. ADR-0066 and ADR-0029 are amended **in place** with dated sections that quote the gating clause they supersede and separate the ownership move from a strategy change; ADR-0073's identity contract is not amended. This revision additionally records the re-scope in the source plan's queue row 08 and in `docs/07_implementation_status.md`, both re-derived at edit time.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` — `Generator::Generator`, `generateInitialInternalOverhangs`, `generateTrees`: for verifying that the relocated algorithm's inputs and graph grounding are unchanged by the move.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::bridge_over_infill`, `prepare_lightning_infill_data`: for confirming which committed planning surfaces the producer consumes today.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `group_fills`, `SurfaceFillParams::operator<`, `Layer::make_fills`: for confirming the caller identity the baseline records.

Canonical reference is used here only to *describe existing behaviour*, never as this packet's oracle: the oracle is the live producer's own recorded output. No new translation is performed by this packet.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
