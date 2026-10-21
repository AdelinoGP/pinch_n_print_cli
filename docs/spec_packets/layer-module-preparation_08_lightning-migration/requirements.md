# Requirements: lightning-migration

## Packet Metadata

- Grouped task: `TASK-577`; backlog authority `docs/07_implementation_status.md`; status **draft**; context cost M per bounded slice (aggregate is large — see `implementation-plan.md` `## Budget and Completion`).
- Reworked premise: this packet is a **behaviour-preserving ownership migration**. Its oracle is the live producer's own recorded output, captured before deletion. It is not blocked on Gate A/B.

## Problem Statement

Lightning planning currently runs in the host as a PrePass builtin: `LIGHTNING_TREE_PRODUCER` commits a public `LightningTreeIR` into the Blackboard, and the consuming module reads it back through the `lightning-tree-segments` WIT method and the `PaintRegionLayerView::lightning_tree_segments_for` accessor. That splits one algorithm's ownership across a host producer, a public IR, a scheduler stage and a guest accessor, and it makes the plan's lifetime and identity unrelated to the artifact that consumes it.

ADR-0066 already decides that preparation-owning artifacts plan and publish privately, with the host owning transport/access/lifetime only. Rows 01–06 built that capability. What remains is the mechanical part: relocate the existing planner into the consuming artifact behind preparation, switch the consumer to private pieces, publish a typed projection, and delete the legacy contract.

Two earlier attempts to author this packet failed for the same reason: they made the migration's correctness depend on a *new scientific result* (Gate A/B). That dependency is unsatisfiable in this tree — row 07 recorded `Gate A outcome: fail`, and row 09's `feasibility-kernel` contains empty `planner.rs`/`tree.rs` stubs, so Gate B never had a planner to execute. The rework replaces that dependency with behavioural equivalence, which is falsifiable now and is the property a refactor actually needs.

## In Scope

- **Baseline oracle.** Record, from the live host producer and before any deletion, a committed baseline containing per case: the `(global_layer_index, object_id, region_id, ordered variant_chain)` plan entries with directed edge multisets and grounding/root witnesses, the final caller-attributed segment multisets, and the sha256 of both the plan and the inputs it was derived from.
- **Relocation, not rewrite.** The planner and its two plan types move into `modules/core-modules/lightning-infill/src/kernel/`, so the guest owns them and `slicer-core` stops owning lightning at all. Grounded fact: `crates/slicer-core/src/algos/lightning/` is std-only, thread-free and host-facility-free (`std::sync::Arc`, `std::rc::{Rc, Weak}`, `std::cell::{Cell, RefCell}`, `HashMap`/`BTreeMap`; no `rayon`/`std::thread`/`std::fs`/`std::env`), so it compiles for wasm unchanged apart from container types and the input adapter. **Planning decisions are frozen**: only file location, container types, plan-type home and input adaptation may change. This also resolves the earlier compile-fatal hole — the algorithm's return type *was* the public IR, so moving the code and relocating the types are one decision.
- **Same-artifact preparation.** `com.core.lightning-infill` implements `slicer_sdk::preparation::LayerPreparation::prepare_print`, plans from the typed committed inputs, and publishes immutable bounded private pieces in one transaction with the optional typed projection.
- **Fresh private consumers.** Ordinary `Layer::Infill` calls obtain a fresh `PreparedPlanView` and use `PreparedPlanView::require(name)` (inherited `ModuleError` code `57401`, fatal, `required preparation piece missing: {name}`) instead of the legacy accessor. Piece decode, partition and bounds checks are module-owned.
- **Typed projection.** One module-defined view `planning_geometry`, scene id `lightning-plan`, classes `planning_domain` (Polygon with holes), `tree_branch` (open Polyline), `tree_root` and `grounding` (Point), committed atomically with the pieces through the row-06 typed scene API only.
- **Direct retirement.** Delete the host producer and its direct prepass wiring, the public plan IR (`LightningTreeIR`, `LightningTreeEntry`, `CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION`), the `PrePass::LightningTreeGen` stage (including `STAGE_ORDER`), the Blackboard slot/commit/byte-estimate path, the whole `slicer:lightning-queries` WIT package (its only member is the retired resource) and its imports, the `lightning_tree_segments_for` SDK accessor with its field/builders/guest glue, the wasm-host binding/dispatch/host/marshal plumbing, the macro glue and watch entry, and the verifier expectations that hardcode the package. No deprecated alias, accessor shim or hollow interface.
- **ADR reconciliation.** Record dated in-place amendments to `docs/adr/0066-private-layer-preparation-capability.md` and `docs/adr/0029-lightning-prepass-tree-generator.md`, each quoting the gating clause it supersedes and separating the ownership move (equivalence) from a strategy change (gates). ADR-0073 is not amended.
- **Affected tests.** Retarget tests whose *subject* survives (export/import linkage, per-region keying, stage-order consistency) and retire only the scaffolding that exists to carry the legacy tree. Never narrow a surviving contract's assertion population to obtain green.
- **Docs.** Replace live legacy guidance in `docs/01`, `02`, `03`, `04`, `05`, `19` under the headings listed in `packet.spec.md` `## Verification`. Historical packet prose and ADR-0029's record stay untouched.
- **Ledger reconciliation.** One re-scope note in the source plan's queue row 08 and one `TASK-577` line in `docs/07_implementation_status.md`, both stating that migration correctness rests on recorded equivalence and that Gate A/B are unchanged and not entry conditions. Re-derive every mutable value at edit time.

## Out of Scope

- Any planning-strategy change, canonical-parity repair, or claim that equivalence proves parity. Row 07's `fail` and row 09's unexecuted Gate B stay exactly as recorded.
- Any change to a **planning decision** inside the relocated kernel: layer iteration order, domain/parameter derivation, grounding or clipping. Container types, error type, file location and the input adapter are the only permitted differences.
- Hand-editing generated bindings (the example module's `interface/` output is regenerated via its documented `make bindings`), or editing the stale historical probe under `docs/feasibility-probes/`.
- Re-authoring `feasibility-kernel` (`build_planning_domain`, empty `planner.rs`/`tree.rs`), the row-07 recorder/publisher, or the canonical fixtures.
- New host decoders, a shared plan IR, a generic lightning renderer, a lightning-specific CLI selector or tap, extra module worlds/exports, or a native surrogate for an externally selected WASM artifact.
- Moving preparation later, changing framework rows 01–06 behavior, or giving any framework surface lightning-specific semantics.
- Approximate/derived domains, copying reference values into planning, or repairing unavailable source fields by defaulting.
- Blanket workspace test runs during implementation; unmeasured performance/memory claims.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md`: settled requirements, projection contract, late-PrePass geometry questions, queue rows 07 and 08 (row 09 is the Gate B packet) and the row-08 re-scope record.
- `docs/adr/0066-private-layer-preparation-capability.md`: ownership/atomicity/lifetime/retirement clauses, read together with its `## Amendment — 2026-10-20` section (accepted decision, not implemented evidence).
- `docs/adr/0029-lightning-prepass-tree-generator.md`: host-owned production decision and its Future-Reviewer Notes, read together with its `## Amendment — 2026-10-20` section. The note forbidding per-layer tree generation still stands; preparation is whole-print.
- `docs/adr/0073-full-source-variant-chain-identity-and-versioned-specialist-queries.md`: identity contract, unchanged by this packet.
- `docs/00_project_overview.md`: normative map; then targeted ranges of `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md`.
- `docs/08_coordinate_system.md` (1 unit = 100 nm), `docs/17_agent_debugging.md`, `docs/19_visual_debug.md` (fail-closed capture, truthful instrumentation).
- `docs/21_data_defaults_and_fixtures.md`, `docs/22_test_quality.md`, `docs/ORCASLICER_ATTRIBUTION.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` — `Generator::Generator`, `generateInitialInternalOverhangs`, `generateTrees`: describes which planning surfaces the relocated algorithm consumes; used to explain existing behaviour, not to define this packet's oracle.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::bridge_over_infill`, `prepare_lightning_infill_data`: the committed planning surfaces a late preparation legitimately sees.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `group_fills`, `SurfaceFillParams::operator<`, `Layer::make_fills`: the original caller identity the baseline records.

The canonical checkout is optional explanatory evidence only. This packet's oracle is the repository's own live producer output, recorded and committed.

## Acceptance Summary

Assertions live only in `packet.spec.md`. This table is the index: identifier → NET-NEW test in `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs` → protected surface.

| AC | Test | Protected surface |
| --- | --- | --- |
| AC-1 | `baseline_is_deterministic_and_strict` | Baseline determinism, strict decode, nonempty per-case populations, input+plan digests |
| AC-2 | `module_plan_matches_recorded_baseline` | Native and WASM plans reproduce the baseline independently; key set, edge multiplicity, grounding |
| AC-3 | `fresh_consumers_emit_baseline_segments` | Segment multiset, role/width/variant identity, lifecycle accounting and disposal |
| AC-4 | `typed_projection_is_atomic_and_neutral` | Typed `lightning-plan` scene from the same plan; atomic commit; capture neutrality; named failures |
| AC-5 | `legacy_contract_is_retired` | Live contract absence (stage, builtin, IR, WIT method, accessor) with baseline-equivalent output |
| AC-N1 | `equivalence_rejects_corrupted_baseline` | Comparator discrimination: dropped/reversed/duplicated/swapped entries rejected; empty-population control |
| AC-N2 | `failed_or_malformed_plan_never_falls_back` | `57401`/`57410`/`57411`, NotReady, no partial publication, no consumers, cleanup |
| AC-N3 | `selection_override_empty_and_foreign_owner` | `invoke && !targets.is_empty()`, override precedence, Ready-empty, `AccessDenied` |
| AC-N4 | `retirement_leaves_no_host_fallback` | Removed-symbol compile witnesses; no host-owned plan remains |

Negative controls are covered by AC-N1..AC-N4; a removal-only packet still carries rejection criteria because the retirement can silently fail open.

## Verification Commands

The nine AC commands are the packet's acceptance. They use Bash, `set -euo pipefail`, a removed stale `target/test-output.log`, a separate `target/lightning-migration-command.log`, and an exact `1 passed; 0 failed` count guard — so a tolerated summary-log write failure cannot resurrect a stale pass.

| # | Command/filter | Protected surface | Return |
| --- | --- | --- | --- |
| 1 | `cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::baseline_is_deterministic_and_strict -- --exact` | AC-1 baseline | FACT pass/fail |
| 2 | `…::module_plan_matches_recorded_baseline -- --exact` | AC-2 equivalence | FACT pass/fail + first divergence |
| 3 | `…::fresh_consumers_emit_baseline_segments -- --exact` | AC-3 consumers/lifetime | FACT pass/fail |
| 4 | `…::typed_projection_is_atomic_and_neutral -- --exact` | AC-4 projection | FACT pass/fail |
| 5 | `…::legacy_contract_is_retired -- --exact` | AC-5 retirement | FACT pass/fail |
| 6 | `…::equivalence_rejects_corrupted_baseline -- --exact` | AC-N1 discrimination | FACT pass/fail |
| 7 | `…::failed_or_malformed_plan_never_falls_back -- --exact` | AC-N2 failure paths | FACT pass/fail |
| 8 | `…::selection_override_empty_and_foreign_owner -- --exact` | AC-N3 gating | FACT pass/fail |
| 9 | `…::retirement_leaves_no_host_fallback -- --exact` | AC-N4 no-fallback | FACT pass/fail |
| 10 | aggregate `lightning_migration_tdd:: -- --nocapture` | All nine in one run | FACT `9 passed` |
| 11 | `cargo xtask build-guests --check` | Guest freshness: exit 0 fresh, 1 stale → rebuild → recheck, 3 infrastructure (not clean) | FACT exit |
| 12 | `cargo check --workspace --all-targets` | Whole-tree type/literal fallout | FACT exit |
| 13 | `cargo clippy --workspace --all-targets -- -D warnings` | Lint closure | FACT exit |
| 14 | `cargo xtask check-literals` | FRU/exhaustive discipline | FACT exit |
| 15 | `cargo xtask check-test-quality --report` | Touched-code findings fixed or waived with a named surface | FACT findings |

No workspace test suite is required; if closure later needs it, it runs once through `cargo xtask test`, not plain `cargo test`.

## Step Completion Expectations

Baseline recorded and deterministic **before** the first production edit. Feature split and module planning land while the host producer still exists, so both paths are observable side by side; equivalence is compared adapter-to-baseline in that window. Deletion of the host producer begins only after AC-2/AC-3 pass. Each retirement step owns the full fallout of the symbol it removes (constructors, literals, exhaustive matches, old-value assertions, registrations, docs) — no compiler-led cleanup in a later step. Guest rebuilds happen after any change to a guest fingerprint input. `cargo check --workspace --all-targets` is the type gate; `cargo xtask build-guests --check` is the artifact gate; they answer different questions and both are required.

## Context Discipline Notes

Delegate every cargo run, every cross-crate symbol closure and every authoritative-doc fact check. Never load the baseline fixture JSON wholesale (>1 MB territory), generated bindings, lockfiles or `target/`. Locate first, then read a bounded window. Return FACT or SUMMARY, never full logs.
