---
status: implemented
packet: 239b-anchored-wit-contract
supersedes: 239-support-independent-layer-z
task_ids:
  - TASK-508
  - TASK-509
  - TASK-510
  - TASK-511
  - TASK-512
  - TASK-513
  - TASK-514
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 239b-anchored-wit-contract

## Goal

Wire the five orphaned anchored records in `crates/slicer-schema/wit/deps/ir-types.wit` into a
real WIT interface, world, and registered stage — with host lift glue, a `deconstruct_layer_ctx`
producer arm and its native twin, and SDK drain glue — so a guest module can transmit an
`ordered-event-collection` across the component boundary and the host receives it as
`LayerStageCommit::AnchoredEvents`.

## Scope Boundaries

Transport contract only: the WIT declaration for the `Layer::AnchoredEvents` stage, its
registration in the canonical stage tables, the host accumulator/converter pair, the producer
arm on both dispatch legs, and the SDK drain that makes the pre-existing
`LayerCollectionBuilder::set_anchored_event_collection` proposal actually cross the boundary —
**plus** the approved widening of `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`'s
`run` with a second builder parameter, `collection: layer-collection-builder`, which is what makes
the drain reachable from a `Layer::Support` guest (see `design.md` §Open Questions).
This packet **adds a producer path**; it does not add a commit variant, does not change how the
executor consumes anchored collections, and does not make any production module emit anchored
work — the two support modules gain the new parameter without using it. Host-side row synthesis and the `execute_per_layer*` call-site switch are
`239a-anchored-host-seams`; support-specific Z semantics, `independent_support_layer_height`,
and a real production producer are `239c-support-layer-height-producer`.

## Prerequisites and Blockers

- Depends on: none. Independent of `239a-anchored-host-seams`; both are implementable against
  the tree as it stands.
- Unblocks: `239c-support-layer-height-producer` (guest-side transmission of anchored work),
  unconditionally — the support-stage reachability seam is now settled; see below.
- **Activation blockers: none open.** The former `[BLOCK]` — that
  `set-anchored-event-collection` on `layer-collection-builder` was unreachable from a
  `Layer::Support` guest — is **resolved** in `design.md` §Open Questions by an approved
  decision: `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`'s `run` gains a
  second builder parameter, `collection: layer-collection-builder`, exactly as
  `crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit`'s `run`
  already takes both `output: gcode-output-builder` and `collection: layer-collection-builder`.
  The three facts that framed the question stand as recorded (one `stage.id` per manifest via
  `required_stage`, `crates/slicer-scheduler/src/manifest.rs`; `layer-support.wit`'s `run`
  received only a `support-output-builder`; `layer-path-optimization.wit` was the sole world
  receiving a `layer-collection-builder`) — the decision is what closes the gap they describe.
  239c records the same resolution. This packet owns the widening and its full blast radius in
  Step 5c and proves support-stage reachability in Step 5d (AC-8). The packet reached
  `status: implemented` after passing its acceptance ceremony (both counter repairs and the
  review-time repairs in `implementation-plan.md` §Implementation Deviations 11-13). `[FWD]`
  questions still live in `design.md` §Open Questions.

Honest limitation, stated up front and repeated in `requirements.md`: every acceptance criterion
below is driven by a purpose-built test guest, not by a production module — **including AC-8**,
which uses a `Layer::Support` test guest to prove the widened world carries anchored content.
`modules/core-modules/tree-support` and `modules/core-modules/traditional-support` gain the new
`run_support` parameter here but do not use it; no shipped module calls
`set_anchored_event_collection` after this packet, and no fixture slice produces an anchored
commit. Closure must not rest on any slice artifact.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1 (round-trip content).** **Given** the new `anchored-events-roundtrip-guest` configured
  with `anchored_event_count = 2`, **when** the host dispatches `Layer::AnchoredEvents` for
  global layer 7, **then** `deconstruct_layer_ctx` returns
  `Some(LayerStageCommit::AnchoredEvents(v))` with `v.len() == 1`,
  `v[0].anchor_global_layer_index == 7`, `v[0].events.len() == 2`,
  `v[0].events[0].geometry == AnchoredGeometryContract::Planar { z: 3000 }` and
  `v[0].events[1].geometry == AnchoredGeometryContract::ZSpanning { min_z: 3000, max_z: 5000 }`
  — the `s64` values compared for exact integer equality, never through an `f32` hop. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::anchored_event_collection_round_trips_with_exact_canonical_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-2 (runtime hooks fidelity).** **Given** the same guest emitting
  `anchored-event-runtime-hooks { optimize-paths: false, account-cooling: true,
  account-time: false }` (a triple distinguishable from `Default::default()`), **when** the
  collection is lifted, **then** the received `AnchoredEventRuntimeHooks` carries exactly
  `optimize_paths == false`, `account_cooling == true`, `account_time == false`. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::anchored_runtime_hooks_survive_the_boundary_unaltered --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-3 (provenance and capability lists).** **Given** the guest emitting an `anchored-entity`
  with `provenance { requesting-feature: "same-z-support", source-plan-entry: "plan-entry-4" }`,
  `input-capabilities = ["support.plan"]` and `output-capabilities = ["extrusion.paths",
  "cooling.account"]`, **when** the collection is lifted, **then** both strings compare equal and
  both capability vectors compare equal **in order** (`vec!["extrusion.paths",
  "cooling.account"]`, not a set comparison). |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::anchored_provenance_and_capability_order_preserved --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-4 (stage is declared, not orphaned).** **Given** the new WIT package, **when** the schema
  tables are queried, **then** `slicer_schema::stage_by_id("Layer::AnchoredEvents")` is `Some`,
  `wit_world_for_stage_id("Layer::AnchoredEvents") == Some("anchored-events-module")`,
  `interface_for_stage_id` returns `Some("anchored-events")`,
  `package_for_stage_id` returns `Some("slicer:layer-anchored-events@1.0.0")`, and
  `qualified_export_for_stage_id` resolves to a non-empty export string. |
  `mkdir -p target && cargo test -p slicer-schema --test export_for_stage_id_tdd -- anchored_events_stage_is_fully_declared --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5 (ADR-0020 drift gate holds at nine).** **Given** the ninth `world-layer` row in
  `STAGES`, **when** the meta-test runs, **then**
  `production_variants_match_world_layer_stages_exactly` passes with
  `LayerStageCommit::AnchoredEvents` present in its production array and the expected-count
  assertion reading `9` — proving the enum and the canonical stage table did not drift. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- layer_stage_commit_stages_tdd::production_variants_match_world_layer_stages_exactly --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-6 (both legs agree).** **Given** identical anchored input, **when** the wasm leg
  (`deconstruct_layer_ctx`, `crates/slicer-wasm-host/src/dispatch.rs`) and the native leg (the
  `match stage_export` inside `commit_native_layer_response`,
  `crates/slicer-wasm-host/src/marshal/native.rs`) each produce a
  commit, **then** the two `LayerStageCommit::AnchoredEvents` values compare equal under
  `PartialEq` — no field is populated on one leg and defaulted on the other. |
  `mkdir -p target && cargo test -p slicer-wasm-host --test contract -- anchored_events_both_legs_tdd::anchored_events_native_and_wasm_legs_agree --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-7 (guest artifacts fresh after a WIT change).** **Given** this packet edits
  `crates/slicer-schema/wit/`, **when** `cargo xtask build-guests` has rebuilt every guest,
  **then** the freshness check exits `0` and prints `FRESH` — decided by exit code, never by
  grepping for `STALE:`. |
  `cargo xtask build-guests --check && echo FRESH`
- **AC-8 (a `Layer::Support` guest actually reaches the drain — content, not artifacts).** This
  is the criterion that proves the §Open Questions resolution did what it was chosen for; it must
  not be satisfied by any file-existence or `rg` check. **Given** the new
  `crates/slicer-wasm-host/test-guests/support-anchored-reach-guest/`, which implements
  `LayerModule::run_support` under `#[slicer_module]` — the first test-guest to implement that
  stage method — with the **test** (not a manifest) supplying the `"Layer::Support"` stage string,
  as the `stage_id` argument to `LayerStageRunner::run_stage`
  (`crates/slicer-wasm-host/src/traits.rs`) and as the `stage` argument to
  `LoadedModuleBuilder::new` (`crates/slicer-scheduler/src/manifest.rs`). There are no
  module-manifest TOMLs under `crates/slicer-wasm-host/test-guests/` and
  `xtask/src/build_guests.rs` sets `stage_id: None` unconditionally for its `GuestTree::TestGuest`
  branch, so manifest-bound `stage.id` (`parse_stage_id_from_module_manifest`) is a
  `modules/core-modules/<name>/<name>.toml` mechanism only and is deliberately not used here. The
  guest's `run_support` calls
  `LayerCollectionBuilder::set_anchored_event_collection` with a one-event
  `ordered-event-collection` whose sole `anchored-entity` carries the canonical `s64` anchor Z
  `1_234_567` units (deliberately **not** representable exactly as `f32`, so any float hop
  corrupts it), **when** the host dispatches `Layer::Support` for that guest at global layer 7,
  **then** the host-side accumulator converts to `LayerStageCommit::AnchoredEvents(v)` with
  `v.len() == 1`, `v[0].anchor_global_layer_index == 7`, `v[0].events.len() == 1`, and the
  entity's anchor Z equal to **exactly** `1_234_567` under `assert_eq!` on the `i64`/`s64` value
  — never an epsilon comparison, and never merely "an anchored commit was produced". |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- support_anchored_reach_tdd::support_stage_guest_reaches_anchored_drain_with_exact_canonical_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

Every AC names exact fields, values, or lookup results and ends with its own runnable command.
Each command names one test with `--exact`, tees to `target/test-output.log`, and asserts a
non-zero matched count so a zero-match run can never read green.

**Module-qualified `--exact` filters (mandatory).** `crates/slicer-runtime/tests/executor/main.rs`,
`crates/slicer-runtime/tests/contract/main.rs`, and `crates/slicer-wasm-host/tests/contract/main.rs`
are `mod <file>;` aggregators, so libtest reports every test as `<mod_name>::<fn_name>`. A bare
function name under `--exact` therefore matches **zero** tests and the run still prints
`test result: ok` — the exact false-green this packet's guard exists to catch. Measured against
this tree: `cargo test -p slicer-runtime --test contract -- --list` prints
`wit_single_source_tdd::no_flat_copies: test` (module-qualified), and
`cargo test -p slicer-runtime --test contract -- production_variants_match_world_layer_stages_exactly --exact`
printed `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 295 filtered out`. Every AC
command above is therefore written with its binary's `mod` prefix
(`anchored_events_roundtrip_tdd::`, `layer_stage_commit_stages_tdd::`,
`anchored_events_both_legs_tdd::`, and AC-8's `support_anchored_reach_tdd::`). AC-8's prefix was
verified the same way at authoring time: `crates/slicer-runtime/tests/executor/main.rs` mounts
every test file as a bare `mod <file>;` and declares **no** top-level `#[test]` wrapper functions,
so libtest reports `<file>::<fn>`. Top-level wrappers are *not* unique to one bucket: both
`crates/slicer-runtime/tests/integration/main.rs` (22 `#[test]` attributes) and
`crates/slicer-wasm-host/tests/contract/main.rs` (4 — `exact_z_support_query`,
`support_decline_contract`, `support_plan_validation`, `support_plan_structural_contract`) declare
them, and any name so wrapped is reported **unqualified**. That does not change AC-6's filter:
`anchored_events_both_legs_tdd::anchored_events_native_and_wasm_legs_agree` is a new test in a new
`mod`, not one of those four wrappers, so it is reported module-qualified. The rule is per-test,
not per-bucket — check the owning `main.rs` for a wrapper of *your* test name rather than
generalising from the bucket. AC-4 alone carries no prefix, and correctly so:
`crates/slicer-schema/tests/export_for_stage_id_tdd.rs` is a standalone integration-test target
whose `--list` output is unqualified. Re-derive each prefix from the owning `main.rs` `mod` list
at edit time if a file is renamed.

**AC-7 guard exemption.** AC-7's command (`cargo xtask build-guests --check && echo FRESH`) is
**not** a `cargo test` invocation: it runs no libtest binary, emits no `test result:` line, and
therefore carries neither an `--exact` test name nor the matched-count guard. It is judged
**solely by exit code** (`0` fresh / `1` stale / `3` `wasm-tools` missing), per `CLAUDE.md`
§"Guest WASM Staleness". Never grep its output for `STALE:`.

`slicer-schema`,
`slicer-runtime`, and `slicer-wasm-host` declare no `required-features` on any test target and
no non-default feature that gates a test file, so the feature-gated-blindness rule
(`CLAUDE.md` §"Feature-gated test files report green when they don't compile") does not apply to
this packet's suite — verified against their `Cargo.toml` files at authoring time; re-confirm
before relying on it.

## Negative Test Cases

- **AC-N1 (malformed collection is rejected, not silently committed).** **Given** the guest
  configured with `emit_malformed_geometry = 1`, emitting an `anchored-entity` whose
  `geometry` is `planar(3000)` but whose `path-points` contain a point at `z = 9000` (beyond
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`), **when** the producer arm runs,
  **then** it returns `Err(LayerStageError::FatalModule { .. })` whose `message` contains the
  substring `anchored entity planar z mismatch`, and **no** `LayerStageCommit` is committed.
  **Implementation note (crate-graph constraint, not a preference):** the producer arm lives in
  `crates/slicer-wasm-host/src/dispatch.rs` and **cannot** call
  `validate_anchored_entity` (`crates/slicer-runtime/src/layer_executor.rs`) — the dependency edge
  runs `slicer-runtime` → `slicer-wasm-host` and not the reverse. The geometry check is therefore
  **re-implemented inside `slicer-wasm-host`** and the substring is a **duplicated string
  literal**, not a reuse. See `implementation-plan.md` Step 5b for the owning symbol and file. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::malformed_anchored_geometry_is_rejected_as_fatal --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2 (silence produces no commit).** **Given** the guest configured with
  `anchored_event_count = 0` so it never calls `set_anchored_event_collection`, **when** the
  host dispatches `Layer::AnchoredEvents`, **then** `deconstruct_layer_ctx` returns `Ok(None)`
  — matching the `Ok(None)`-on-empty-output convention of the `Layer::Support` and
  `Layer::Infill` arms — and the arena is byte-identical to the pre-dispatch snapshot. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::guest_emitting_no_anchored_events_produces_no_commit --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N3 (atomicity: one proposal per dispatch).** **Given** the guest configured with
  `duplicate_proposal = 1` so it calls `set_anchored_event_collection` twice within one
  dispatch, **when** the second call executes, **then** it returns `Err` to the guest, the guest
  surfaces a `ModuleError`, and the host commits **zero** `AnchoredEvents` collections — the
  first proposal is not silently kept. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::duplicate_anchored_proposal_is_rejected_and_commits_nothing --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N4 (Z-spanning contract is validated too, not just planar).** ADR-0059
  (`docs/adr/0059-support-families-and-anchored-entities.md`) requires that "path validation
  follows each entity's declared planar or Z-spanning contract"; AC-N1 exercises only the planar
  half. **Given** the guest configured with `emit_malformed_geometry = 2`, emitting an
  `anchored-entity` whose `geometry` is `z-spanning((3000, 5000))` but whose `path-points`
  contain a point at `z = 9000` (above `max_z` by more than
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`), **when** the producer arm runs,
  **then** it returns `Err(LayerStageError::FatalModule { .. })` whose `message` contains the
  substring `anchored entity z-span violation`, and **no** `LayerStageCommit` is committed. The
  same crate-graph constraint recorded on AC-N1 applies: this substring is likewise a duplicated
  literal inside `slicer-wasm-host`, and both branches are re-implemented by the single helper
  named in `implementation-plan.md` Step 5b. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::zspanning_anchored_geometry_out_of_range_is_rejected_as_fatal --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check && echo FRESH`
- Primary targeted proof: AC-1's command.

## Authoritative Docs

- `docs/specs/support-independent-layer-z-split-plan.md` - the split plan of record; findings
  F5, F6, F7 are this packet's problem statement and the queue table is its dependency
  authority. Direct ranged read (the file is short).
- `docs/02_ir_schemas.md` - section `### anchored entity IR (additive)` under
  `## IR 10 — LayerCollectionIR`, plus `## IR Versioning Contract`. Ranged reads of those two
  sections only; the file is long and must not be full-read.
- `docs/03_wit_and_manifest.md` - consulted by delegated SUMMARY for the world-membership and
  interface-declaration conventions; never full-read.
- `docs/05_module_sdk.md` - consulted by delegated SUMMARY for what a `STAGES` row must declare
  (stage method, trait, world/interface/package/export); never full-read.
- `docs/04_host_scheduler.md` - consulted by delegated SUMMARY for `STAGE_ORDER` placement of a
  ninth `Layer::*` stage; never full-read. (Path audit: `docs/03_wit_contracts.md` and
  `docs/04_module_stages.md` do not exist in this tree and were mis-cited in an earlier draft.
  Confirm any doc path against `ls docs/` before citing it.)
- `docs/adr/0059-support-families-and-anchored-entities.md` (`0059-support-families-and-anchored-entities`,
  Status: accepted) - the governing decision for anchored entities. Its clauses "each worker
  returns ordered event collections" and "path validation follows each entity's declared planar
  or Z-spanning contract instead of the model-layer Z envelope" are the reason this packet
  transports an `ordered-event-collection` rather than a flat entity list, and the reason AC-N1
  and AC-N4 cover **both** declared contracts. Its "anchored to the upper global layer" clause is
  what the `anchor-global-layer-index: u32` field carries. Direct ranged read of the two
  anchored-entity paragraphs only; do not full-read the ADR.
- `CLAUDE.md` §"WIT/Type Changes Checklist" and §"Guest WASM Staleness" - both apply verbatim to
  every step of this packet.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` - the existing section `### anchored entity IR (additive)` is
  **extended** (it exists today and describes the types at IR level only) with a WIT-transport
  subsection naming the `Layer::AnchoredEvents` stage, the `slicer:layer-anchored-events@1.0.0`
  package, and the `s64`-preserving lift rule -
  `rg -q 'layer-anchored-events' docs/02_ir_schemas.md && rg -q 'anchored entity IR \(additive\)' docs/02_ir_schemas.md`
- `docs/07_implementation_status.md` - `TASK-508`..`TASK-514` registered at packet-owned closure
  (Step 7) - `rg -q 'TASK-508' docs/07_implementation_status.md && rg -q 'TASK-514' docs/07_implementation_status.md`
- `docs/specs/support-parity-gap-register.md` - a new row recording that the anchored-event
  substrate had no WIT transport (F7), with this packet as its destination -
  `rg -q '239b-anchored-wit-contract' docs/specs/support-parity-gap-register.md`
- `docs/specs/support-independent-layer-z-split-plan.md` - queue row 2's `status`/`packet dir`
  columns updated - `rg -q '239b-anchored-wit-contract' docs/specs/support-independent-layer-z-split-plan.md`
- **No IR schema version bump.** Verified at authoring time across
  `AnchoredGeometryContract`, `AnchoredEntityProvenance`, `AnchoredEntity`,
  `AnchoredEventRuntimeHooks`, and `OrderedEventCollection` in `crates/slicer-ir/src/slice_ir.rs`:
  none carries a `schema_version` field and no `ANCHORED_*_SCHEMA_VERSION` constant exists, so
  zero tests hard-assert an anchored version and there is no version-bump fallout.
  `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` stays at its documented value and must not be
  disturbed — this packet adds transport, not IR shape.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- **None, and the reason is structural.** OrcaSlicer is a monolithic C++ binary with no
  component-model boundary, no WIT, and no guest/host split. It therefore has no equivalent of
  an `ordered-event-collection` transport contract, and there is no canonical function to port
  or diverge from. Parity applies to this packet only weakly: the *payload* semantics
  (support-row Z, layer merging, flow per support layer height) are canonical concerns, and they
  are owned by `239c-support-layer-height-producer`, whose `requirements.md` carries the
  `GCode::collect_layers_to_print` / `generate_support_layers` / `bottom_contact_layer`
  obligations. This packet moves bytes across a boundary that canonical does not have.
- The one obligation that survives: **before closure, a delegated dispatch must confirm there is
  no canonical equivalent** rather than assuming it. Dispatch contract: return `FACT` — "no
  component/serialization boundary for support events exists in `OrcaSlicerDocumented/`". Do not
  invent canonical citations to fill this section, and do not restate 239c's obligations here.

Citation policy (E7): canonical behaviour is cited by file + function only, never line number,
and only what a delegated dispatch actually returned.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
