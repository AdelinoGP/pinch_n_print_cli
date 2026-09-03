# Implementation Plan: 240-support-raft

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs
  (TASK-409..TASK-418 only).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled
  independently; never write "see Step N".
- Guest-facing steps: run `cargo xtask build-guests --check` before attributing
  any failure; a step that creates or edits guest code rebuilds guests in-step.
- WIT-editing steps end with `cargo build --tests`.
- All test commands tee to `target/test-output.log`; read results from the file.

## Steps

### Step 1: Author signed-index + raft-fill IR tests (red)

- Task IDs: `TASK-409`
- Objective: create the failing tests that pin AC-1 and AC-2's Rust half:
  `crates/slicer-ir/tests/signed_layer_indices_tdd.rs` (i32 field types via a
  compile-time assertion helper, serde round-trip of
  `SliceIR { global_layer_index: -2, .. }`, negative-index ordering vs 0) and
  `crates/slicer-ir/tests/sliced_region_raft_fill_tdd.rs`
  (`SlicedRegion.raft_fill` defaults empty, serde-default backward compat,
  determinism of two identical syntheses).
- Precondition: clean tree; 236 forward-dep acknowledged (tests must compile
  against current u32 fields as RED).
- Postcondition: both new test files exist and fail to compile / fail asserts
  for exactly the intended reasons.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - lines 1020-1200, 1760-1840
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/tests/signed_layer_indices_tdd.rs` (new)
  - `crates/slicer-ir/tests/sliced_region_raft_fill_tdd.rs` (new)
  - `crates/slicer-ir/Cargo.toml` (register test targets if auto-discovery off)
- Files explicitly out of bounds:
  - everything else under `crates/`, all modules
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 brief 240
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-ir --test signed_layer_indices_tdd -- signed_layer_indices_round_trip --exact 2>&1 | tee target/test-output.log; grep -q 'error\[' target/test-output.log || grep -q 'FAILED\|panicked' target/test-output.log` - FACT: RED confirmed
- Exit condition: log shows the new tests red for missing i32 types /
  missing `raft_fill` field — nothing else broken.

### Step 2: Signed-index migration u32→i32 (green Step 1's AC-1)

- Task IDs: `TASK-410`
- Objective: retype the six fields + `LayerModule::run_infill` parameter to
  i32 per design.md §Migration Table; fix every blast-radius site from the
  dispatched LOCATIONS list; keep unrelated u32 fields untouched.
- Precondition: Step 1 red; **LOCATIONS sweep dispatched and its result pasted
  into this step's working notes before editing** (question verbatim in
  design.md). If the sweep exceeds ~20 affected files, STOP and split this
  step into 2a (crates) / 2b (modules+tests) with fresh IDs from the free range.
- Postcondition: `cargo check --workspace --all-targets` green;
  AC-1 command green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - ranged reads around each hit only
  - LOCATIONS sweep output (working notes)
- Files allowed to edit (at most 3 primaries; sweep fallout is owned here):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-macros/src/lib.rs`
  plus LOCATIONS-listed call-site/test files (`views.rs`,
  `marshal/{in_,out,native}.rs`, `blackboard.rs`, `layer_executor.rs`,
  gcode consumers, macro test files) edited strictly to restore compilation —
  no behavior changes beyond the sign semantics.
- Files explicitly out of bounds:
  - `modules/core-modules/tree-support-planner/src/lib.rs` beyond the
    `push_raft_plan` range; planner/renderer algorithm files (238b/238c)
- Blast-radius discipline: struct literals of `GlobalLayer`, `ObjectLayerRef`,
  `SliceIR`, `InfillIR`, `SupportIR` in TEST code gain `..` rest or an
  `// exhaustive: <reason>` waiver (literal gate); production literals stay
  exhaustive. Tests hard-asserting u32 wrap/ordering get sign-correct updates,
  never deleted.
- Expected sub-agent dispatches:
  - LOCATIONS blast-radius sweep; scope `crates/ modules/`; return LOCATIONS
    (per-file aggregated counts); purpose: edit list
- Context cost: `M` (split trigger above)
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated SUMMARY of the layer-index sections
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tail -5 | tee target/test-output.log` - FACT pass/fail
  - `cargo test -p slicer-ir --test signed_layer_indices_tdd -- signed_layer_indices_round_trip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-1 green
- Exit condition: AC-1 command passes with non-zero count; workspace check green.

### Step 3: SlicedRegion.raft_fill + WIT accessor + schema bump

- Task IDs: `TASK-411`
- Objective: add `pub raft_fill: Vec<ExPolygon>` (serde default) to
  `SlicedRegion`; add `raft-fill: func() -> list<ex-polygon>` to
  `slice-region-view` in `crates/slicer-schema/wit/deps/ir-types.wit`;
  minor-bump `CURRENT_SLICE_IR_SCHEMA_VERSION` with a version-history doc
  comment line; project the field through BOTH marshal legs
  (`in_.rs` resource construction + `native.rs` native request).
- Precondition: Step 2 green.
- Postcondition: AC-2 green; `cargo build --tests` green after the WIT edit;
  SliceIR schema-version assertions updated in the same step.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - region-projection range only
  - `crates/slicer-wasm-host/src/marshal/native.rs` - same
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-wasm-host/src/marshal/in_.rs` (+ `native.rs` as the
    second-leg mirror; T9 discipline: both legs or neither)
- Files explicitly out of bounds:
  - `modules/core-modules/**` (guests rebuilt in later steps), scheduler
- Blast-radius discipline: schema-bump fallout — grep
  `CURRENT_SLICE_IR_SCHEMA_VERSION` assertions; every test hard-asserting the
  old value is updated in THIS step (bump + fallout together).
- Expected sub-agent dispatches:
  - FACT: locate schema-version assert sites; scope `crates/`; return LOCATIONS ≤20
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - SliceIR section; delegated SUMMARY before editing
- OrcaSlicer refs: none this step
- Verification:
  - `cargo build --tests 2>&1 | tail -3` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-ir --test sliced_region_raft_fill_tdd -- raft_fill_defaults_empty_and_deterministic --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-2 Rust half green
- Exit condition: AC-2 Rust-half green; both legs project the new field.

### Step 4: New guest module com.core.raft-default (manifest + skeleton)

- Task IDs: `TASK-412`, `TASK-413`
- Objective: create `modules/core-modules/raft-default/`: `Cargo.toml`
  mirroring `rectilinear-infill/Cargo.toml`; manifest
  `raft-default.toml` with id `com.core.raft-default`, stage
  `Layer::Infill`, `[claims] holds = ["claim:raft-fill"]`,
  `[ir-access] reads = ["SliceIR", "LayerPlanIR", "SupportPlanIR"]`,
  `writes = ["SliceIR"]`, `[config.schema]` declaring
  `raft_contact_distance` (float, default 0.1, min 0.0),
  `raft_expansion` (float, default 1.5, min 0.0),
  `raft_first_layer_expansion` (float, default 2.0, min 0.0);
  `wit-guest/` binding the existing `slicer:layer-infill` world; guest src
  implementing `LayerModule::run_infill(layer_index: i32, ...)` that reads
  `SupportPlanIR.raft_plan` and returns Ok(()) with TODO-free synthesis
  scaffolding deferred to Step 5 (this step: module loads, claims resolve).
  Rebuild guests IN-STEP (new artifact).
- Precondition: Steps 1-3 green.
- Postcondition: `cargo xtask build-guests --check` exit 0 including the new
  artifact; host discovers the module; claim registry contains exactly one
  `claim:raft-fill` holder (typo-guard: exact string `claim:raft-fill`);
  `should_emit_raft_fill_claim_tdd` green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/Cargo.toml` (full; small)
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` (full; small)
  - `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit` (full; 20 lines)
- Files allowed to edit (at most 3):
  - `modules/core-modules/raft-default/Cargo.toml` (new)
  - `modules/core-modules/raft-default/raft-default.toml` (new)
  - `modules/core-modules/raft-default/src/lib.rs` (new)
  (`wit-guest/` copy rides with src; counts as part of the new directory)
- Files explicitly out of bounds:
  - existing support/infill modules' manifests (Step 6a), scheduler
- Expected sub-agent dispatches:
  - FACT: confirm `slicer:layer-infill` world import set suffices; scope
    `crates/slicer-schema/wit/deps/layer-infill/`; return FACT
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - manifest section; delegated SUMMARY if >300 lines
  - `docs/adr/0009-raft-as-layer-infill-role.md` - direct read
- OrcaSlicer refs: none this step
- Verification:
  - `cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
  - `mkdir -p target && cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd -- raft_infill_claim_emits --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-3 claim half green
  - `rg -q 'claim:raft-fill' modules/core-modules/raft-default/raft-default.toml` - FACT
- Exit condition: fresh guest artifact; single-holder claim resolves; AC-N3
  grep passes for the three keys.

### Step 5: Raft geometry synthesis (deterministic polygons)

- Task IDs: `TASK-414`, `TASK-415`
- Objective: implement the port inside `run_infill`: given
  `Some(raft_plan)` on layers with negative indices, synthesize
  object-independent raft footprint POLYGONS for the declared band
  (`raft_layers` = first + `base_raft_layers` + `interface_raft_layers`),
  apply `raft_expansion` inflation staged in multiple steps, first printed
  layer expanded by `raft_first_layer_expansion`, interface-band footprints at
  contact-distance-derived spacing — deterministic pure geometry into
  `SlicedRegion.raft_fill`. Polygons only: no scan-line pattern math, no
  extrusion paths in this module (design.md §ADR-0009 Reconciliation; the
  claim-holder pattern stage consumes the polygons downstream). All mm
  constants ÷100 at the unit boundary (coord-system bullet). No anchored
  entities anywhere.
- Precondition: Step 4 green.
- Postcondition: integration cases green (ordering, no-anchored); determinism
  assert in `sliced_region_raft_fill_tdd` extended and green; guests rebuilt.
- Files allowed to read, with ranges when over 300 lines:
  - delegated Orca SUMMARY of `generate_raft_base` staging (working notes)
- Files allowed to edit (at most 3):
  - `modules/core-modules/raft-default/src/lib.rs`
  - `crates/slicer-runtime/tests/integration/main.rs` (register two cases)
  - `crates/slicer-runtime/tests/integration/raft_prefix_layers.rs` (new; holds
    `raft_prefix_orders_before_model_layers` + `raft_mints_no_anchored_entities`)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**` (delegate); `rectilinear-infill` src and other
    pattern modules stay UNTOUCHED this packet (pattern dispatch rides the
    existing claim machinery; any needed holder-side wiring is a follow-up
    recorded in design.md §ADR-0009 Reconciliation, not silent scope growth)
- Blast-radius discipline: none (no struct-field change this step).
- Expected sub-agent dispatches:
  - OrcaSlicer SUMMARY: `generate_raft_base` staging order + multi-step
    inflation + classic-vs-organic branch selection relevant to PnP's v1
    rectilinear path; return SUMMARY
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - porting checklist; delegated SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - delegate;
    never load
- Verification:
  - `cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_prefix_orders_before_model_layers --exact --nocapture && cargo test -p slicer-runtime --test integration -- raft_mints_no_anchored_entities --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-4 green
- Exit condition: AC-4 green with non-zero count; two identical runs produce
  byte-identical `raft_fill`.

### Step 6a: Keys wiring-or-record + claim-conflict negative

- Task IDs: `TASK-416`
- Objective: (a) write wire-or-record rows for each dead raft key in the four
  support-module manifests — declare-and-wire where the module genuinely
  consumes it, otherwise a TOML comment recording why it stays (each row names
  the decision owner); (b) author scheduler test
  `raft_fill_double_holder_conflicts` proving a second `claim:raft-fill`
  holder surfaces as structured `ClaimConflict` naming both module ids;
  (c) regenerate `docs/15_config_keys_reference.md` via the gen-config-docs
  gate (T8).
- Precondition: Step 5 green.
- Postcondition: AC-5 and AC-N1 green.
- Files allowed to read, with ranges when over 300 lines:
  - four support-module manifests (small; full read fine)
- Files allowed to edit (at most 3):
  - `modules/core-modules/{tree-support-planner,traditional-support-planner,tree-support,traditional-support}/*.toml` (annotation edits across these four counts as one logical surface)
  - `crates/slicer-scheduler/tests/validation_tdd.rs`
  - `docs/15_config_keys_reference.md` (regenerated output only)
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/validation.rs` (G-21 validator shape is
    236-owned; this packet only tests its observable contract)
- Expected sub-agent dispatches:
  - FACT: current validator advisory shape for duplicate claims (post-G-21
    expectation); scope `crates/slicer-scheduler/`; return SNIPPETS ≤20 lines
- Context cost: `M`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - regenerated output only
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate: confirm
    the three key defaults/mins; return FACT
- Verification:
  - `mkdir -p target && cargo test -p slicer-scheduler --test validation_tdd -- raft_fill_double_holder_conflicts --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N1
  - `rg -q 'raft_contact_distance' modules/core-modules/raft-default/raft-default.toml && rg -q 'raft_first_layer_expansion' modules/core-modules/raft-default/raft-default.toml && rg -q 'raft_contact_distance' docs/15_config_keys_reference.md` - AC-N3 + doc regen
- Exit condition: both commands green; every dead-key decision recorded.

### Step 6b: Raft bounds negative case

- Task IDs: `TASK-417`
- Objective: author `crates/slicer-runtime/tests/contract/raft_bounds_tdd.rs`
  with case `raft_index_outside_band_rejected`, registering
  `mod raft_bounds_tdd;` in `crates/slicer-runtime/tests/contract/main.rs`;
  plus the AC-5 keys-consumption case `raft_keys_declared_and_wired` in the
  same new file. Registration is part of this step — an unregistered file
  compiles to zero tests (T2 blindness).
- Precondition: Step 6a green.
- Postcondition: AC-N2 and AC-5 green; binary count for `--test contract`
  unchanged-or-grown with the two cases visible in output.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/main.rs` (registration list only)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/raft_bounds_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs` (add `mod raft_bounds_tdd;`)
  - `modules/core-modules/raft-default/raft-default.toml` (only if the
    bounds case exposes a schema-range gap; otherwise untouched)
- Files explicitly out of bounds:
  - all other contract test files; scheduler internals
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §13 T2/T8 traps
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_index_outside_band_rejected --exact --nocapture && cargo test -p slicer-runtime --test contract -- raft_keys_declared_and_wired --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N2 + AC-5
  - `grep -q 'mod raft_bounds_tdd;' crates/slicer-runtime/tests/contract/main.rs` - FACT registration present
- Exit condition: both cases pass with non-zero count; registration grep passes.

### Step 7: DEV-124 verify-record + formal ADR-0009 amendment + docs

- Task IDs: `TASK-418`
- Objective: run the DEV-124 protocol (AC-6 commands) and write the verdict +
  residual note into requirements.md §DEV-124 Verify-Record; execute a FORMAL
  AMENDMENT of ADR-0009 inside its own document (Amendments section
  convention, status `Proposed` → `Accepted`), recording that Decision 5's
  claim assignment is superseded by the support-families completion plan
  (`docs/specs/support-families-anchored-entities-plan.md` §12 240 brief):
  the amendment QUOTES the original Decision 5 clause ("Pattern variety is
  provided by whichever `Layer::Infill` module(s) declare `claim:raft-fill`
  in their manifest. v1 ships with `rectilinear-infill` declaring the
  claim…"), states the reassignment of `claim:raft-fill` to
  `com.core.raft-default`, and preserves Decision 4 / the zero-pattern-
  algorithm clause / the "Do not re-suggest making `raft-default` a renderer"
  Future-Reviewer Note UNCHANGED (the module still writes polygons only; no
  extrusion-path/flow/speed rendering moves into it). Also update
  `docs/02_ir_schemas.md` (SliceIR section: raft_fill + signed indices +
  bump). The ADR's inline Decision-5 text itself stays verbatim — the
  amendment is additive, per ADR immutability convention.
- Precondition: Steps 1-6a/6b green.
- Postcondition: AC-6 green; doc greps in packet.spec.md §Doc Impact Statement
  all pass; the amendment grep below passes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - SliceIR section only
- Files allowed to edit (at most 3):
  - `docs/spec_packets/240-support-raft/requirements.md`
  - `docs/adr/0009-raft-as-layer-infill-role.md` (additive Amendments
    section + status flip only)
  - `docs/02_ir_schemas.md`
- Files explicitly out of bounds:
  - `docs/DEVIATION_LOG.md` unless a NEW DEV-124 finding forces a row (then
    re-derive next free ID at write time; run `cargo xtask check-deviations`)
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0009-raft-as-layer-infill-role.md` - direct read (<110 lines)
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 240 brief
    only (amendment citation source); direct range read
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture && cargo test -p slicer-runtime --test contract -- classic_clamp_unchanged_when_no_raft_configured --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-6
  - `rg -q 'raft_fill' docs/02_ir_schemas.md && rg -q 'Accepted' docs/adr/0009-raft-as-layer-infill-role.md && rg -q 'Amendments' docs/adr/0009-raft-as-layer-infill-role.md && rg -q 'claim:raft-fill' docs/adr/0009-raft-as-layer-infill-role.md && rg -q 'com\.core\.raft-default' docs/adr/0009-raft-as-layer-infill-role.md` - FACT: amendment present with reassignment
  - `rg -q 'rectilinear-infill. declaring the' docs/adr/0009-raft-as-layer-infill-role.md` - FACT: original Decision-5 clause quoted verbatim in the amendment
- Exit condition: verdict recorded; all greps pass; Decision 4 text unchanged
  (`git diff` on the ADR shows additions only, no modifications to existing
  Decision/Future-Reviewer lines).

### Step 8: Acceptance gates + Human Validation Gate execution

- Task IDs: `TASK-418`
- Objective: run packet-level gates; produce human-gate artifacts
  (tmp/p240-* G-code + visual-debug bundle); record checklist verdicts;
  leave sign-off to the human.
- Precondition: Step 7 green; §9 raft-enabled Orca references exist under
  `tmp/p240-orca-*-raft.gcode` (human-owned precondition — if absent, the
  gate stays open and the packet reports blocked-on-human, not done).
- Postcondition: gates green; checklist written in packet.spec.md
  §Human Validation Gate.
- Files allowed to read, with ranges when over 300 lines: n/a
- Files allowed to edit (at most 3):
  - `docs/spec_packets/240-support-raft/packet.spec.md` (checklist verdicts only)
  - `tmp/p240-profile.json` (matched-profile copy with `support_raft_layers >= 2`)
  - `tmp/p240-vd-raft.json` (visual-debug request)
- Files explicitly out of bounds:
  - anything under `docs/spec_packets/` other than this packet
- Expected sub-agent dispatches:
  - cargo runs delegated; visual-debug bundle generation via pnp_cli
- Context cost: `S`
- Authoritative docs:
  - `docs/19_visual_debug.md` - bundle format; delegated SUMMARY
- OrcaSlicer refs: comparison against regenerated references only
- Verification:
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask check-literals --report` - FACT: violation count unchanged from G-15 baseline (61 inherited; no new ones from this packet)
- Exit condition: gates green; artifacts present; checklist filled except the
  human sign-off line.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | red tests, IR-only |
| Step 2 | M | split trigger documented (LOCATIONS > ~20 files → 2a/2b) |
| Step 3 | M | WIT + both marshal legs + schema bump |
| Step 4 | M | new guest dir + first rebuild |
| Step 5 | M | geometry port + integration cases |
| Step 6a | M | keys decisions + claim-conflict + doc regen |
| Step 6b | S | bounds negative + contract registration |
| Step 7 | S | records + ADR + IR doc |
| Step 8 | S | gates + human-gate artifacts |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a
  full backlog read.
- Reconcile superseded transitions: stub-support-raft.md consumed at authoring
  (done); G-06 destination rerouted (done); 215 deletion belongs to 236.
- `packet.spec.md` is ready for `status: implemented` only after the Human
  Validation Gate sign-off line is dated.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (leg-skew on future transports; pattern
  takeover by another claim holder).
- Confirm context stayed at or below 150k standard, or at/below 300k only with
  a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and
verification commands use `--all-targets` where applicable so test, bench, and
example targets compile.
