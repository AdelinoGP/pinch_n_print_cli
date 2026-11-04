# Requirements: lightning-migration

## Packet Metadata

- Grouped task: `TASK-577`; backlog authority `docs/07_implementation_status.md`; status **draft**; context cost M per bounded slice (aggregate is large — see `implementation-plan.md` `## Budget and Completion`).
- Reworked premise: this packet is a **behaviour-preserving ownership migration**. Its oracle is the live producer's own recorded output, captured before deletion. It is not blocked on Gate A/B.

## Problem Statement

Lightning planning currently runs in the host as a PrePass builtin: `LIGHTNING_TREE_PRODUCER` commits a public `LightningTreeIR` into the Blackboard, and the consuming module reads it back through the `lightning-tree-segments` WIT method and the `PaintRegionLayerView::lightning_tree_segments_for` accessor. That splits one algorithm's ownership across a host producer, a public IR, a scheduler stage and a guest accessor, and it makes the plan's lifetime and identity unrelated to the artifact that consumes it.

ADR-0066 already decides that preparation-owning artifacts plan and publish privately, with the host owning transport/access/lifetime only, and [ADR-0074](../../adr/0074-lightning-migration-on-recorded-equivalence.md) records that the ownership move is authorized by recorded equivalence while a strategy change still needs the gates. Rows 01–06 built that capability. What remains is the mechanical part: relocate the existing planner into the consuming artifact behind preparation, switch the consumer to private pieces, publish a typed projection, and delete the legacy contract.

Two earlier attempts to author this packet failed for the same reason: they made the migration's correctness depend on a *new scientific result* (Gate A/B). That dependency is unsatisfiable in this tree — row 07 recorded `Gate A outcome: fail`, and row 09's `feasibility-kernel` contains empty `planner.rs`/`tree.rs` stubs, so Gate B never had a planner to execute. The rework replaces that dependency with behavioural equivalence, which is falsifiable now and is the property a refactor actually needs.

## In Scope

- **Baseline oracle.** Record, from the live host producer and before any deletion, a committed baseline containing per case: the `(global_layer_index, object_id, region_id, ordered variant_chain)` plan entries with directed edge multisets and grounding/root witnesses, the final caller-attributed segment multisets, and the sha256 of both the plan and the inputs it was derived from. The recorded case material is the fixture geometry **as transformed by the recorder**: `geometry_scale = 0.25` on the recorded XY geometry (a measured bound on planning cost in the debug profile), plus a synthetic material-paint patch on one `hole_void_classic` top-face triangle so the ordered `variant_chain` identity axis is exercised rather than compared empty-to-empty. Both transformations are recorded in `derivation`, and the recorder's caller dedup keys on the full identity including the ordered chain.
- **Relocation, not rewrite.** The planner and its two plan types move into `modules/core-modules/lightning-infill/`: the algorithm into `src/kernel/`, the two plan types into a NET-NEW `src/kernel/plan.rs` as private `KernelPlan`/`PlanEntry` (no `serde`, no schema-version constant). `slicer-core` stops owning lightning planning at all. Grounded fact: `crates/slicer-core/src/algos/lightning/` is std-only, thread-free and host-facility-free (`std::sync::Arc`, `std::rc::{Rc, Weak}`, `std::cell::{Cell, RefCell}`, `HashMap`/`BTreeMap`; no `rayon`/`std::thread`/`std::fs`/`std::env`), and its `slicer-core` helpers (`polygon_ops`, `flow`, `geometry`, `difference`/`offset`) are ungated modules of a crate the consumer already depends on, so the relocation needs only a `crate::` → `slicer_core::` path rewrite. **Planning numeric logic is frozen**: `generator.rs`, `layer.rs`, `tree_node.rs`, `distance_field.rs`, `error.rs` and the computation in `mod.rs` keep every decision, including the `sparse_fill_holder` branch. The permitted differences are enumerated — file location, the plan-type import, those helper paths, the error type's path, and a NET-NEW additive adapter that supplies inputs only. AC-2/AC-3 prove the list was respected; a sixth difference stops the step. This also resolves the earlier compile-fatal hole: the algorithm's return type *was* the public IR, so the types move with it and the public copies are deleted only after their last consumer (the host copy) is gone.
- **Module-side input adapter.** A NET-NEW `modules/core-modules/lightning-infill/src/planning_input.rs` builds the planner's inputs from the typed preparation records: `PreparationRegion` → `SliceIR` layer entries, grouped one layer per distinct `global_layer_index` and built with `SliceIR::default()`/`SlicedRegion::default()`, regions carrying `object_id`, `region_id`, `polygons` verbatim and `variant_chain: region.variant_chain.iter().map(|(k, v)| (k.clone(), slicer_ir::PaintValue::from(v))).collect()` (the conversion `slicer_ir::PaintValue`'s `impl From<&PreparationPaintValue>` supplies over `PreparationVariantChain = Vec<(String, PreparationPaintValue)>`); `ConfigView`/`effective_layer_height` → the five `ResolvedConfig` fields the planner reads (`sparse_fill_holder` pinned to `lightning-infill` because selection is now the gate, plus `infill_density`, `layer_height`, `infill_angle`, `line_width`), with `line_width == 0.0` recognised as the host's automatic sentinel and resolved before the kernel sees it. The five scalars must agree across every committed region config and `layer_height` must agree with `effective_layer_height`, else preparation fails with `57412` rather than silently choosing a source; `lightning-infill.toml` therefore also declares `layer_height` and `infill_angle`.
- **Same-artifact preparation.** `com.core.lightning-infill` implements `slicer_sdk::preparation::LayerPreparation::prepare_print`, plans from the typed committed inputs, and publishes immutable bounded private pieces in one transaction with the optional typed projection.
- **Native prepared-pair export.** `slicer-integrated-modules` exposes `prepared_native_entries()`, returning the generated `NativePreparedStageEntry` and `PreparationExportSchema` for every enabled integrated module that declares preparation (today: the lightning module). AC-2's native leg uses it so the native and WASM arms are each the module's real paired artifact; it is an export/access addition, not a new runtime selection path.
- **Module-owned failure-path trigger.** The module declares an advanced, default-`none` `prepared_plan_diagnostic` enum and applies it only after planning: `missing_piece` omits the topmost planned layer's piece, `malformed_piece` and `unsupported_version` stage bodies its own decoder rejects, and `empty_plan` publishes empty plan pieces plus an empty scene. It never reaches the kernel, never changes a planning scalar, and is inert on `none`; it exists because the module owns the `require`/decoder path AC-N2 protects, and a host-side injector would fake that boundary. The foreign/disposed-owner `AccessDenied` leg uses the already-public `PreparedOwnerStore`/`freeze_native` authority contract and needs no new API.
- **Fresh private consumers.** Ordinary `Layer::Infill` calls obtain a fresh `PreparedPlanView` and use `PreparedPlanView::require(name)` (inherited `ModuleError` code `57401`, fatal, `required preparation piece missing: {name}`) instead of the legacy accessor. Piece decode, partition and bounds checks are module-owned.
- **Typed projection.** One module-defined view `planning_geometry`, scene id `lightning-plan`, classes `planning_domain` (Polygon with holes), `tree_branch` (open Polyline), `tree_root` and `grounding` (Point), committed atomically with the pieces through the row-06 typed scene API only.
- **Direct retirement.** Delete the host producer and its direct prepass wiring, the public plan IR (`LightningTreeIR`, `LightningTreeEntry`, `CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION`), the `PrePass::LightningTreeGen` stage (including `STAGE_ORDER`), the Blackboard slot/commit/byte-estimate path, the whole `slicer:lightning-queries` WIT package (its only member is the retired resource) and its imports, the `lightning_tree_segments_for` SDK accessor with its field/builders/guest glue, the wasm-host binding/dispatch/host/marshal plumbing, the macro glue and watch entry, and the verifier expectations that hardcode the package. No deprecated alias, accessor shim or hollow interface.
- **ADR reconciliation.** The packet is governed by `docs/adr/0074-lightning-migration-on-recorded-equivalence.md`, which supersedes ADR-0029's host-ownership decision and amends ADR-0066's retirement precondition, quoting both contested clauses by text and ADR slot; the two dated `## Amendment — 2026-10-20` sections in those ADRs carry the clause-by-clause replacement, and its registered deviations are `D-LMP08-ADR-0066-RETIREMENT-PRECONDITION` and `D-LMP08-ADR-0029-HOST-OWNERSHIP` in `docs/DEVIATION_LOG.md`. ADR-0029's Future-Reviewer Note is not superseded (preparation is whole-print), and ADR-0073 is not amended.
- **Affected tests.** Retarget tests whose *subject* survives (export/import linkage, per-region keying, stage-order consistency) and retire only the scaffolding that exists to carry the legacy tree. Never narrow a surviving contract's assertion population to obtain green.
- **Docs.** Replace live legacy guidance in `docs/01`, `02`, `03`, `04`, `05`, `19` under the headings listed in `packet.spec.md` `## Verification`; each replaced section states that the legacy path is retired. The docs are deliberately outside AC-5's absence greps, because describing a retirement requires naming the retired symbols. Historical packet prose and ADR-0029's record stay untouched.
- **Ledger reconciliation.** **Confirm** the source plan's existing `**Row 08 re-scope (2026-10-20).**` note is accurate and re-derive it (do not re-add it), make the blocking paragraph that follows it agree with it — equivalence is row 08's criterion, Gate A/B stay recorded and unchanged, and rows 01–06 are re-derived from each packet's `status:` frontmatter — and update the `TASK-577` line in `docs/07_implementation_status.md`. Re-derive every mutable value at edit time.

## Out of Scope

- Any planning-strategy change, canonical-parity repair, or claim that equivalence proves parity. Row 07's `fail` and row 09's unexecuted Gate B stay exactly as recorded.
- Any change to a **planning decision** inside the relocated kernel: layer iteration order, domain/parameter derivation, grounding, clipping, or the `sparse_fill_holder` branch. File location, the plan-type import, the `crate::` → `slicer_core::` helper paths, the error type's path and the NET-NEW input adapter are the only permitted differences — no container-type rewrite, no re-derivation.
- Hand-editing generated bindings (the example module's `interface/` output is regenerated via its documented `make bindings`), or editing the stale historical probe under `docs/feasibility-probes/`.
- Re-authoring `feasibility-kernel` (`build_planning_domain`, empty `planner.rs`/`tree.rs`), the row-07 recorder/publisher, or the canonical fixtures.
- New host decoders, a shared plan IR, a generic lightning renderer, a lightning-specific CLI selector or tap, extra module worlds/exports, a host-side piece injector that manufactures the AC-N2 failure payloads, or a native surrogate for an externally selected WASM artifact. The diagnostic publication mode is module-owned and inert by default; extending it beyond the named failure outputs is out of scope.
- Moving preparation later, changing framework rows 01–06 behavior, or giving any framework surface lightning-specific semantics.
- Approximate/derived domains, copying reference values into planning, or repairing unavailable source fields by defaulting.
- Blanket workspace test runs during implementation; unmeasured performance/memory claims.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md`: settled requirements, projection contract, late-PrePass geometry questions, queue rows 07 and 08 (the table has rows 01–08; packet 09 has no queue row and is referenced only as Gate B in row 07's packet-dir cell), the recorded Gate A/B outcomes and the row-08 re-scope note.
- `docs/adr/0074-lightning-migration-on-recorded-equivalence.md`: the accepted decision record for this packet — recorded-equivalence authorization, the two superseded/amended clauses quoted by text and slot, and the explicit list of what is *not* superseded.
- `docs/adr/0066-private-layer-preparation-capability.md`: ownership/atomicity/lifetime/retirement clauses, read together with its `## Amendment — 2026-10-20` section and ADR-0074 (accepted decision, not implemented evidence).
- `docs/adr/0029-lightning-prepass-tree-generator.md`: host-owned production decision and its Future-Reviewer Notes, read together with its `## Amendment — 2026-10-20` section and ADR-0074. The note forbidding per-layer tree generation still stands; preparation is whole-print.
- `docs/DEVIATION_LOG.md`: the two registered deviations for those amendments. Re-derive the next free ID at use instead of quoting one.
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
| AC-2 | `module_plan_matches_recorded_baseline` | Native (via `prepared_native_entries()`) and WASM plans reproduce the baseline independently; key set, edge multiplicity, grounding, and the painted `hole_void_classic` chains |
| AC-3 | `fresh_consumers_emit_baseline_segments` | Segment multiset, role/width/ordered variant identity, lifecycle accounting and disposal |
| AC-4 | `typed_projection_is_atomic_and_neutral` | Typed `lightning-plan` scene from the same plan; atomic commit; capture neutrality; `empty_plan` empty scene; named failures |
| AC-5 | `legacy_contract_is_retired` | Live-contract absence over **code + WIT** with an explicit scope and exemption list (stage, builtin, IR, WIT method, accessor; `crates/slicer-core/src/algos/lightning/` exempt), plus baseline-equivalent output. The six docs are **not** absence-grepped: each is checked for a unique replacement heading and the explicit `The legacy path is retired.` sentence |
| AC-N1 | `equivalence_rejects_corrupted_baseline` | Comparator discrimination: dropped/reversed/duplicated/swapped entries rejected; empty-population and length-only controls |
| AC-N2 | `failed_or_malformed_plan_never_falls_back` | Module-scoped `57401`/`57410`/`57411`/`57412` on this module's own errors through the module-owned diagnostic publication mode, NotReady, no partial publication, no consumers, cleanup |
| AC-N3 | `selection_override_empty_and_foreign_owner` | `invoke && !targets.is_empty()`, override precedence, `empty_plan` Ready/no-work, public-store disposed-owner `AccessDenied` |
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

Baseline recorded and deterministic **before** the first production edit, with Step 1a's painted-chain coverage in place before the harness is authored. Feature split and module planning land while the host producer still exists, so both paths are observable side by side; equivalence is compared adapter-to-baseline in that window. Deletion of the host producer begins only after AC-2/AC-3 pass. Each retirement step owns the full fallout of the symbol it removes (constructors, literals, exhaustive matches, old-value assertions, registrations, docs) — no compiler-led cleanup in a later step. Guest rebuilds happen after any change to a guest fingerprint input. `cargo check --workspace --all-targets` is the type gate; `cargo xtask build-guests --check` is the artifact gate; they answer different questions and both are required.

## Context Discipline Notes

Delegate every cargo run, every cross-crate symbol closure and every authoritative-doc fact check. Never load the baseline fixture JSON wholesale (>1 MB territory), generated bindings, lockfiles or `target/`. Locate first, then read a bounded window. Return FACT or SUMMARY, never full logs.
