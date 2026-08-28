# Requirements: 239b-anchored-wit-contract

## Packet Metadata

- Grouped task IDs: `TASK-508`, `TASK-509`, `TASK-510`, `TASK-511`, `TASK-512`, `TASK-513`,
  `TASK-514`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`
- Supersedes: `239-support-independent-layer-z` (jointly with `239a-anchored-host-seams` and
  `239c-support-layer-height-producer`)

Ledger note: the task-id high-water mark, the next free `G-` gap row, and the next free `DEV-`
id are **mutable shared state**. `TASK-507` and `DEV-157` were the values read at split time.
**The next-free `G-` row is actively CONTESTED** — independent readers of
`docs/specs/support-parity-gap-register.md` have reported the highest existing row as both `G-26`
and `G-19`, and an earlier draft of this packet froze `G-27` on that basis. Treat no figure as
settled: derive the row from the file at the moment you append. Do
not quote any of these values — re-derive each one at the moment you edit the ledger
(e.g. `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`). Another packet may
have claimed them since.

## Problem Statement

The anchored-event substrate is **production-dead at the component boundary**. Packet 239
claimed the substrate "already carries everything needed"; a swarm run measured that claim false
and split 239 into three packets. This packet owns finding **F7**, the one the split plan marked
CRITICAL, plus the transport half of **F6**:

- **F7 — the WIT records are orphaned.** `crates/slicer-schema/wit/deps/ir-types.wit` (package
  `slicer:ir-handles`, interface `ir-handles`) declares five anchored records —
  `anchored-entity`, `anchored-geometry-contract` (variant `planar(s64)` |
  `z-spanning(tuple<s64, s64>)`), `anchored-entity-provenance`, `anchored-event-runtime-hooks`,
  and `ordered-event-collection` (fields `anchor-global-layer-index: u32`,
  `events: list<anchored-entity>`, `runtime-hooks`). A grep across the whole `wit/` tree found
  them referenced by **zero** interfaces, **zero** worlds, and **zero** function signatures.
  `crates/slicer-macros/src/lib.rs` and `crates/slicer-wasm-host/src/` contain **zero** lift or
  lower glue for them. A guest module cannot transmit anchored work at all today.
- **F6 — the module-commit path is dead on the guest side.** The SDK helpers
  `LayerCollectionBuilder::set_anchored_event_collection` and
  `LayerCollectionBuilder::anchored_proposal` (`crates/slicer-sdk/src/layer_collection_builder.rs`)
  exist, store a proposal in guest memory, and are drained by nothing. The proposal never leaves
  the guest.

This is one coherent slice because a transport contract is only meaningful end to end: a record
that is declared but not exported by any world, a world with no host lift, a lift with no
producer arm, and a producer arm with no guest drain are each individually untestable. The
falsifiable unit is a byte-for-byte round trip.

**What this packet does NOT do, stated because it changes the packet's shape.**
`LayerStageCommit::AnchoredEvents(Vec<OrderedEventCollection>)` **already exists** in
`crates/slicer-ir/src/stage_io.rs`, already maps to the stage-id string `"Layer::AnchoredEvents"`
via `LayerStageCommit::stage_id`, and is **already handled apply-side** in
`crates/slicer-runtime/src/layer_executor.rs`. What is missing is only that no host arm ever
*constructs* it and no guest can *produce* one. This packet therefore adds a **producer path**;
it does not add a commit variant.

## In Scope

- **New WIT stage package** `crates/slicer-schema/wit/deps/layer-anchored-events/layer-anchored-events.wit`
  declaring package `slicer:layer-anchored-events@1.0.0`, interface `anchored-events` with a
  single `run: func(...) -> result<_, module-error>` export, and world `anchored-events-module`
  importing `slicer:common/host-services`, `slicer:common/profiling`,
  `slicer:config/config-types`, `slicer:ir-handles/ir-handles` and exporting `anchored-events` —
  structurally identical to `deps/layer-support/layer-support.wit`.
- **One new method on the existing `resource layer-collection-builder`** in
  `crates/slicer-schema/wit/deps/ir-types.wit`:
  `set-anchored-event-collection: func(collection: ordered-event-collection) -> result<_, string>`.
  This is the reference that de-orphans all five records: `ordered-event-collection` transitively
  pulls `anchored-entity`, `anchored-geometry-contract`, `anchored-entity-provenance`, and
  `anchored-event-runtime-hooks` into the world.
- **Canonical stage registration**: a ninth `world-layer` `StageSpec` row in
  `slicer_schema::STAGES` (`method: "run_anchored_events"`, `stage_id: "Layer::AnchoredEvents"`,
  `wit_export: "run"`, `tier_id: TIER_LAYER`, `trait_name: "LayerModule"`,
  `wit_dir: "layer-anchored-events"`, `wit_package: "slicer:layer-anchored-events@1.0.0"`,
  `wit_interface: "anchored-events"`, `wit_world: "anchored-events-module"`); the matching entry
  in `slicer_schema::VALID_STAGES`; and the matching entry in
  `slicer_scheduler::execution_plan::STAGE_ORDER`, placed among the `Layer::*` stages.
- **Declaration-model reconciliation** for the 16th per-stage WIT file: a new
  `emit_world_preamble("anchored-events-module", "anchored_events", ...)` call site with its own
  `include_str!` in `crates/slicer-macros/src/lib.rs`, the matching `rerun-if-changed` path in
  `crates/slicer-macros/build.rs`, and the three hard-coded `20` expectations in
  `xtask/src/wit_verify.rs`'s test module raised to `21`. These three surfaces are cross-checked
  against each other by that module's audit tests, so they must move together in one step.
- **ADR-0020 drift gate update**: `production_variants_match_world_layer_stages_exactly`
  (`crates/slicer-runtime/tests/contract/layer_stage_commit_stages_tdd.rs`) gains
  `LayerStageCommit::AnchoredEvents(Vec::new())` in its `production` array and its
  `expected.len()` assertion moves from `8` to `9`. Today the variant is silently absent from
  that array while its `stage_id()` returns a string that is in no `STAGES` row — registration
  closes that hole rather than opening one.
- **SDK trait surface**: `run_anchored_events` added to the `LayerModule` trait
  (`crates/slicer-sdk/src/traits.rs`) with a default no-op body so no existing module is broken.
- **Host lift glue**, mirroring the `support-output-builder` pattern end to end: an
  `AnchoredEventsCollected` accumulator in `crates/slicer-wasm-host/src/marshal/accumulators.rs`,
  a corresponding field on `HostExecutionContext` (`crates/slicer-wasm-host/src/host.rs`) with
  its `_mut` accessor, the resource-method implementation on
  `impl ir::HostLayerCollectionBuilder for HostExecutionContext`, and a
  `convert_anchored_events` converter in `crates/slicer-wasm-host/src/marshal/out.rs` re-exported
  from `crates/slicer-wasm-host/src/marshal/mod.rs`.
- **Guest invocation** — the surface that makes the new world actually callable, and the one a
  declaration-only packet silently omits: a `pub mod layer_anchored_events` wrapping a
  `wasmtime::component::bindgen!` for world `anchored-events-module` in
  `crates/slicer-wasm-host/src/host.rs` (exemplars in the same file: `pub mod layer_support`,
  `pub mod layer_support_postprocess`), its sibling
  `pub use layer_anchored_events::LayerModule as LayerAnchoredEventsModule;` alias, and a
  `"Layer::AnchoredEvents"` arm at the **layer-tier linker/instantiate/call**
  `match stage_id.as_str()` in `crates/slicer-wasm-host/src/dispatch.rs` — the one bound as
  `let (call_result, mut store, mem_initial_bytes) = ...`, whose arms perform
  `add_wasi_to_linker`, `<world>::LayerModule::add_to_linker`, `HostExecutionContextBuilder`
  store construction, and `<world>::LayerModule::instantiate`. The **prepass-tier**
  `match stage_id.as_str()` in the same file (bound as `let (call_result, mut store) = ...`) is
  out of scope and gains no arm.
- **Producer arms on BOTH legs, in one step**: a `"Layer::AnchoredEvents"` arm in
  `deconstruct_layer_ctx` (`crates/slicer-wasm-host/src/dispatch.rs`) — a **different** `match`
  from the invocation site above, running after the guest returns — returning `Ok(None)` on
  empty output, plus the twin arm in the `match stage_export` inside
  `commit_native_layer_response` (`crates/slicer-wasm-host/src/marshal/native.rs`) — note that
  match scrutinises `stage_export`, not `stage_id`. The repo's both-legs guard forbids landing one
  without the other.
- **A `slicer-wasm-host`-local geometry validator**,
  `validate_anchored_entity_geometry` in `crates/slicer-wasm-host/src/marshal/out.rs`,
  duplicating the planar and z-span checks and their message literals from
  `validate_anchored_entity` (`crates/slicer-runtime/src/layer_executor.rs`). This is a
  duplication forced by the crate graph, not a reuse; see `design.md` §Architecture Constraints.
- **SDK drain glue**: the generated guest shim reads
  `LayerCollectionBuilder::anchored_proposal` at the end of a `run_anchored_events` dispatch and
  forwards it through the new WIT method; the native mirror (`crates/slicer-sdk/src/native.rs`)
  and the test capture sink (`crates/slicer-sdk/src/test_support/capture.rs`) gain the same
  path. `set_anchored_event_collection`'s existing double-call rejection is preserved and its
  message updated to name the stage it now serves.
- **New test guest** `crates/slicer-wasm-host/test-guests/anchored-events-roundtrip-guest/`,
  modelled on `finalization-mutation-roundtrip-guest`: one binary, config-parameterized
  (`anchored_event_count`, `emit_malformed_geometry`, `duplicate_proposal`) so it serves every
  positive and negative fixture in this packet. `emit_malformed_geometry` is tri-valued:
  `0` = well-formed, `1` = planar mismatch (AC-N1), `2` = Z-spanning out-of-range (AC-N4).
- **Test registration, not just test authoring.** Both new aggregated test files must be added to
  their binary's `mod` list or they never compile and their ACs report zero tests:
  `mod anchored_events_roundtrip_tdd;` in `crates/slicer-runtime/tests/executor/main.rs`, and
  `mod anchored_events_both_legs_tdd;` in `crates/slicer-wasm-host/tests/contract/main.rs`.
  AC-4's test is appended to the standalone target
  `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` (which today holds only
  `export_for_stage_id_is_total_over_stages_and_rejects_unknown`) and needs no registration line.
- **Doc extension**: `docs/02_ir_schemas.md` §`### anchored entity IR (additive)` gains a
  transport subsection; docs/07 registration; gap-register row; split-plan queue update.

## Out of Scope

- **Any production module emitting anchored work.** No `modules/core-modules/*` edit. A real
  producer is `239c-support-layer-height-producer`.
- **Host-side row synthesis, `PipelineConfig` seams, and the `execute_per_layer*` call-site
  switch** — `239a-anchored-host-seams` owns all of it. This packet must not touch
  `crates/slicer-runtime/src/pipeline.rs` or `crates/pnp-cli/src/visual_debug.rs`.
- **Promoting `validate_anchored_entity` to a shared crate.** Considered and rejected here: the
  symbol lives in `crates/slicer-runtime/src/layer_executor.rs`, which this packet marks
  read-only, and moving it would change apply-side behaviour owned by
  `239a-anchored-host-seams`. This packet instead **duplicates** the two geometry checks and
  their message literals into `slicer-wasm-host` (see In Scope), because the dependency edge runs
  `slicer-runtime` → `slicer-wasm-host` and the producer arm cannot call the original. De-duping
  the two copies behind one shared implementation is deliberately deferred and is not owned by
  this packet.
- **Executor consumption semantics.** `execute_anchored_event_collections`,
  `append_same_z_entities`, and `is_same_z_entity` in
  `crates/slicer-runtime/src/layer_executor.rs` are read-only here; the apply side already
  handles `LayerStageCommit::AnchoredEvents`.
- **Any IR schema version bump.** No anchored type carries a `schema_version` field and no
  `ANCHORED_*_SCHEMA_VERSION` constant exists (verified across `slice_ir.rs`), so there is
  nothing to bump and no test asserts an anchored version.
  `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` is explicitly not disturbed.
- **`independent_support_layer_height`, support Z decoupling, and the `height_delta` verdict** —
  `239c-support-layer-height-producer`.
- **`crates/slicer-gcode/src/emit.rs` and the `GCodeEmitter` impls** — unchanged; anchored work
  does not reach G-code in this packet.
- **Config-key declarations.** The test guest's three config keys are test-fixture parameters
  read via `ConfigView::get_int`; no manifest `[config.schema]` entry and no
  `docs/15_config_keys_reference.md` regeneration.

## Authoritative Docs

- `docs/specs/support-independent-layer-z-split-plan.md` - short; direct ranged read. Findings
  F5/F6/F7 and the queue table.
- `docs/02_ir_schemas.md` - long; ranged read of `### anchored entity IR (additive)` (under
  `## IR 10 — LayerCollectionIR`) and `## IR Versioning Contract` only. Never full-read.
- `docs/03_wit_and_manifest.md` - over 300 lines; delegate a `SUMMARY` for the world-membership and
  interface-declaration conventions.
- `docs/04_host_scheduler.md` - over 300 lines; delegate a `SUMMARY` for where a ninth
  `Layer::*` stage sits in `STAGE_ORDER`. This file owns the canonical `STAGE_ORDER` listing.
- `docs/05_module_sdk.md` - over 300 lines; delegate a `SUMMARY` for what a `STAGES` row must
  declare (stage method name, trait, world / interface / package / export).
  Path audit note: `docs/03_wit_contracts.md` and `docs/04_module_stages.md` were cited by an
  earlier draft and **do not exist**; the real files are the three named here plus
  `docs/03_wit_and_manifest.md`. Verify any doc path with `ls docs/` before citing it — a
  citation to a non-existent file passes every line-number check trivially.
- `docs/adr/0059-support-families-and-anchored-entities.md` (`0059-support-families-and-anchored-entities`,
  accepted) - the governing decision for anchored entities. Ranged read of the two anchored-entity
  paragraphs only. Conformance obligations are itemised in `design.md` §Architecture Constraints;
  the operative clause here is that path validation follows each entity's **declared planar or
  Z-spanning contract**, which is why AC-N1 and AC-N4 are both mandatory.
- `CLAUDE.md` §"WIT/Type Changes Checklist", §"Guest WASM Staleness", §"Config Key Naming
  Convention" - direct read; all three bind every step.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- **None, and the reason is structural.** OrcaSlicer is a monolithic C++ binary with no
  component-model boundary, no WIT, and no guest/host split, so it has no equivalent of an
  `ordered-event-collection` transport contract and no function to port or deliberately diverge
  from. Parity applies to this packet only weakly.
- **Where the real parity obligation lives:** support-row Z semantics, the object/support row
  merge, and per-support-layer flow are canonical concerns owned by
  `239c-support-layer-height-producer` — `GCode::collect_layers_to_print` (`GCode.cpp`),
  `generate_support_layers` (`Support/SupportCommon.cpp`),
  `PrintObjectSupportMaterial::bottom_contact_layer` (`Support/SupportMaterial.cpp`), and
  `Flow::mm3_per_mm` (`Flow.cpp`). Do not restate those obligations here and do not implement
  against them in this packet.
- **The surviving obligation:** before closure, one delegated dispatch must actively confirm the
  absence rather than assume it. Contract: return `FACT` — "no component/serialization boundary
  for support or anchored events exists in `OrcaSlicerDocumented/`". Record the FACT in the
  Step 7 notes. Never fabricate a canonical citation to populate this section.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-8`.
  - Measurable refinements not restated in the Given/When/Then text: AC-1's `s64` comparison must
    be against `i64` values in Rust (`AnchoredGeometryContract::Planar { z: 3000 }` is 0.3 mm at
    1 unit = 100 nm) and the test must fail if any implementation routes the value through `f32`;
    AC-3's capability comparison is `assert_eq!` on `Vec<String>`, never on a `HashSet`;
    AC-6's equality is derived `PartialEq` on `LayerStageCommit`, not a field-by-field spot check;
    AC-8's `1_234_567`-unit anchor Z is likewise compared with `assert_eq!` on the `i64` value and
    is deliberately not exactly representable in `f32`, so an `f32` hop fails it. AC-8 asserts
    **content** — commit variant, collection length, anchor layer index, and the exact Z — and is
    never satisfied by a file-existence or `rg` check.
- Negative: `AC-N1` through `AC-N4`. Negatives are **mandatory** for this packet because it is a
  contract-boundary change: a transport that accepts malformed payloads, silently commits on
  empty output, or keeps a superseded proposal is worse than no transport. AC-N1 and AC-N4 are a
  **pair**, not a redundancy: ADR-0059
  (`docs/adr/0059-support-families-and-anchored-entities.md`) requires that path validation
  follow *each* entity's declared contract, so the planar case (AC-N1) and the Z-spanning case
  (AC-N4) must both reject. Both substrings are **duplicated literals** inside `slicer-wasm-host`
  — `crates/slicer-runtime/src/layer_executor.rs`'s `validate_anchored_entity` is unreachable
  from the producer arm because the dependency edge runs `slicer-runtime` → `slicer-wasm-host`.
- Cross-packet impact: `239c-support-layer-height-producer` consumes this contract and must not
  re-declare the stage, the WIT package, or the builder method. **The former `[BLOCK]` is
  resolved** (`design.md` §Open Questions, approved decision): the `layer-support` world's `run`
  gains a second builder parameter, `collection: layer-collection-builder`, matching
  `layer-path-optimization.wit`'s existing two-builder `run`, so a support-stage guest reaches
  the drain. 239c records the same resolution and consumes the two-builder signature.
  This packet additionally owns the breaking change's full blast radius (Step 5c) and the
  support-stage reachability proof (Step 5d / AC-8). `239a-anchored-host-seams` is
  independent and touches no file in this packet's edit list — the two are safe to run
  concurrently, but both register rows in `docs/07_implementation_status.md`, so the docs/07
  edit must re-derive the tail rather than assume a line offset.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands. Every
`cargo test` row tees to `target/test-output.log` and appends the non-zero-match guard
`test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` so a zero-match run cannot read
green.

**Every `--exact` filter below is module-qualified, and must stay that way.**
`crates/slicer-runtime/tests/executor/main.rs`, `crates/slicer-runtime/tests/contract/main.rs`,
and `crates/slicer-wasm-host/tests/contract/main.rs` are `mod <file>;` aggregators, so libtest
names each test `<mod_name>::<fn_name>` and a bare function name under `--exact` matches **zero**
tests while still printing `test result: ok`. Measured against this tree:
`cargo test -p slicer-runtime --test contract -- production_variants_match_world_layer_stages_exactly --exact`
printed `0 passed; ...; 295 filtered out`, while `-- --list` printed module-qualified names such
as `wit_single_source_tdd::no_flat_copies: test`. Prefixes in use:
`anchored_events_roundtrip_tdd::` (slicer-runtime `executor`),
`layer_stage_commit_stages_tdd::` (slicer-runtime `contract`), and
`anchored_events_both_legs_tdd::` (slicer-wasm-host `contract`). The AC-4 row is deliberately
unprefixed: `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` is a standalone target and
its `--list` output is unqualified. Re-derive any prefix from the owning `main.rs` `mod` list
before running, rather than trusting the value quoted here.

The `cargo xtask build-guests --check` row is **exempt** from both the `--exact` name and the
matched-count guard: it is not a `cargo test` invocation, prints no `test result:` line, and is
judged solely by exit code.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check && echo FRESH` | Guest freshness by **exit code** (0 fresh / 1 stale / 3 `wasm-tools` missing). Run FIRST before attributing any guest, component, or dispatch failure. | FACT fresh/stale/infra |
| `cargo build --tests` | The `CLAUDE.md` WIT/Type-Changes-Checklist gate; run immediately after every `.wit` edit | FACT pass/fail |
| `cargo check --workspace --all-targets` | Type gate covering the `STAGES` / `STAGE_ORDER` / `VALID_STAGES` blast radius | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required before committing | FACT pass/fail |
| `cargo xtask check-literals` | Required before committing; the new test guest and round-trip fixtures create watched-type struct literals | FACT pass/fail |
| `cargo test -p slicer-schema --test export_for_stage_id_tdd -- anchored_events_stage_is_fully_declared --exact` | AC-4: stage is declared in `STAGES` with a resolvable world/interface/package/export | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- layer_stage_commit_stages_tdd::production_variants_match_world_layer_stages_exactly --exact` | AC-5: ADR-0020 enum↔`STAGES` drift gate at nine | FACT pass/fail |
| `cargo test -p xtask --bin xtask wit_verify` (xtask has no `[lib]`; its tests live in the `xtask` binary target) | Declaration-model audit: macro `include_str!` set, `build.rs` watch set, and the count expectation must all agree at 21 | FACT pass/fail; SNIPPETS ≤20 lines |
| `cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::anchored_event_collection_round_trips_with_exact_canonical_z --exact` | AC-1: the primary round-trip proof | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::anchored_runtime_hooks_survive_the_boundary_unaltered --exact` | AC-2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::anchored_provenance_and_capability_order_preserved --exact` | AC-3 | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test contract -- anchored_events_both_legs_tdd::anchored_events_native_and_wasm_legs_agree --exact` | AC-6: both-legs guard | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::malformed_anchored_geometry_is_rejected_as_fatal --exact` | AC-N1 | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::zspanning_anchored_geometry_out_of_range_is_rejected_as_fatal --exact` | AC-N4: the ADR-0059 Z-spanning half of "validation follows each entity's declared contract" | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::guest_emitting_no_anchored_events_produces_no_commit --exact` | AC-N2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::duplicate_anchored_proposal_is_rejected_and_commits_nothing --exact` | AC-N3 | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- support_anchored_reach_tdd::support_stage_guest_reaches_anchored_drain_with_exact_canonical_z --exact` | AC-8: a `Layer::Support` guest reaches the drain and its `s64` anchor Z survives intact — the proof that the two-builder widening did its job | FACT pass/fail |
| `cargo test -p slicer-scheduler --test contract -- stage_list_consistency_tdd:: 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | Proves the new `STAGE_ORDER` entry was classified as user-targetable (`VALID_STAGES`) rather than host-only. `crates/slicer-scheduler/tests/contract/main.rs` is **also** a `mod` aggregator (`mod stage_list_consistency_tdd;`), so a bare test name under `--exact` matches zero here too. This row deliberately uses the module prefix as a **substring** filter (no `--exact`) so it runs every test in that file — at authoring time those were `valid_stages_is_subset_of_stage_order` and `host_only_stages_partition_stage_order_into_valid_stages`, but re-derive the current set from the file rather than trusting those names | FACT pass/fail |
| `cargo xtask test --summary --workspace --no-fail-fast` | Closure ceremony ONLY, per `CLAUDE.md` Test Discipline; dispatch to a sub-agent with a `FACT pass/fail` return and never absorb the full output | FACT pass/fail |

Feature-gating note: `slicer-schema`, `slicer-runtime` (`default = ["report"]`), and
`slicer-wasm-host` declare no `required-features` on any test target and no `#![cfg(feature ...)]`
gate on the files above, so a narrow `-p` run compiles them. This was checked against their
`Cargo.toml` files at authoring time; re-check before trusting a narrow green.

## Step Completion Expectations

- **AC-1 is red from Step 3 until Step 6, by design.** The round-trip test and guest land in
  Step 3 and stay red through Steps 4 and 5 because the guest's proposal has no drain until
  Step 6 wires it. Steps 4 and 5 therefore verify with type gates plus their own leg-local
  assertions, not with AC-1. Do not "fix" the red by weakening the assertion.
- **Steps 4, 5a, and 5b must not be merged or reordered.** Step 4 introduces both the accumulator
  that the producer arm reads and the `bindgen!` world module that Step 5a's invocation arm links
  against; Step 5a makes the guest run; Step 5b's two producer arms (wasm + native) must land
  together under the both-legs guard. Steps 5a and 5b share `TASK-512` and are one verification
  atom — 5a alone runs a guest whose output nothing consumes, and 5b alone is unreachable code.
- **AC-N4 is red until Step 5b, alongside AC-N1**, and both are proved by the same
  `slicer-wasm-host`-local validator. Do not close 5b with only the planar branch implemented:
  ADR-0059 requires validation per declared contract, and an arm that skips the Z-spanning branch
  passes AC-N1 while being non-conformant.
- **Steps 5c and 5d also share `TASK-512` and must not be reordered.** 5c is the breaking
  `layer-support` `run` widening plus every co-moving surface in one commit — the workspace does
  not compile green partway through it. 5d adds the `Layer::Support` test guest and AC-8. AC-8 is
  red until 5d lands and must not be "fixed" by asserting anything weaker than the exact `s64` Z.
  No task id outside `TASK-508`..`TASK-514` is minted by either.
- **A signature-only WIT edit does not move the `20`→`21` counts.** `xtask/src/wit_verify.rs`
  counts distinct `.wit` **file paths** in three places; Step 5c edits a file already in every
  set, so it changes none of them. The 20→21 change belongs solely to Step 2's new
  `layer-anchored-events.wit`. Conflating the two produces a count of 22 and a red audit.
- **The three declaration-model surfaces move as one atom.** `crates/slicer-macros/src/lib.rs`'s
  `include_str!` set, `crates/slicer-macros/build.rs`'s `rerun-if-changed` set, and
  `xtask/src/wit_verify.rs`'s count expectation are cross-checked against each other by that
  module's audit tests. Editing any one alone leaves the workspace red.
- **Guest freshness gates every step after Step 2.** Once the `.wit` tree changes, every guest
  artifact is stale. `cargo xtask build-guests --check` must run (and be rebuilt from) before any
  failure in Steps 3-7 is attributed to that step's code.
- Shared scratch state: none. No step writes to `tmp/` or shares a fixture file with another.

## Context Discipline Notes

- `crates/slicer-wasm-host/src/host.rs` and `crates/slicer-wasm-host/src/dispatch.rs` are both
  multi-thousand-line files. **Ranged reads only**, anchored on symbol names
  (`impl ir::HostLayerCollectionBuilder for HostExecutionContext`, `push_layer_collection_builder`,
  `deconstruct_layer_ctx`, `HostExecutionContext`'s output-collector block). Never full-read
  either one.
- `crates/slicer-macros/src/lib.rs` is large and contains 15 near-identical
  `emit_world_preamble` call sites. Read **one** exemplar (the `support-module` site) and
  pattern-match; do not scan all fifteen.
- `xtask/src/wit_verify.rs` is long. Only its `#[cfg(test)]` module's three `20` expectations and
  `macro_embedded_wit_files` / `parse_macro_include_str_wit_paths` are in play. Delegate a
  `LOCATIONS` dispatch for the three literals rather than reading the file.
- Tempting reads to skip: `crates/slicer-runtime/src/layer_executor.rs` (over 2400 lines) — the
  apply side is already done and is out of scope. Read only `validate_anchored_entity` for the
  AC-N1 error wording, by delegated `SNIPPETS` ≤ 20 lines.
- Heavy-dispatch return limits: the Step 1 inventory dispatch returns `LOCATIONS` capped at 20
  entries; the whole-suite closure run returns `FACT pass/fail` only.
