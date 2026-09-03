# Implementation Plan: 240b-support-raft-module

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs
  (`TASK-414`..`TASK-418`, `TASK-535` only).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled
  independently; never write "see Step N".
- Guest-facing steps: run `cargo xtask build-guests --check` and judge by its
  exit code before attributing any failure; the step that creates the guest
  rebuilds it in-step (drop `--check`).
- This packet edits no WIT. If a step needs a WIT change, STOP — that is a 240a
  defect to route back, not scope to absorb.
- All test commands tee to `target/test-output.log`; read results from the file,
  never re-run for more output.
- A new test file under an aggregated `slicer-runtime` binary gets its `mod`
  registration in the SAME step, or it compiles to zero tests and reports a
  false pass.

## Steps

### Step 1: Verify the 240a substrate, then create the module skeleton

- Task IDs: `TASK-414`
- Objective: (a) verify every row of `design.md` §Substrate Consumed From 240a
  exists with the promised shape — this is a hard gate, not a formality;
  (b) create `modules/core-modules/raft-default/` with `Cargo.toml` mirroring
  `rectilinear-infill/Cargo.toml`, manifest `raft-default.toml` declaring id
  `com.core.raft-default`, stage `Layer::Infill`,
  `[claims] holds = ["claim:raft-fill"]`,
  `[ir-access] reads = ["SliceIR", "LayerPlanIR", "SupportPlanIR"]`,
  `writes = ["SliceIR"]`, `wit-guest/` binding the existing
  `slicer:layer-infill` world, and a guest `src/lib.rs` implementing
  `LayerModule::run_infill(layer_index: i32, ...)` that reads `raft_plan` via
  the paint view and returns `Ok(())` — geometry deferred to Step 3, so this
  step proves only that the module loads, the claim resolves, and the read path
  reaches the guest. Rebuild guests IN-STEP (new artifact).
- Precondition: 240a's AC-1..AC-7 green (verify with a dispatched FACT; do not
  assume). If any FORWARD-DEP row is missing or differently shaped, STOP and
  route it back to 240a.
- Postcondition: `cargo xtask build-guests --check` exit 0 including the new
  artifact; the host discovers the module; the claim registry contains exactly
  one `claim:raft-fill` holder (typo-guard: exact string).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/Cargo.toml` (full; small)
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` (full; small)
  - `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit` (full; 20 lines)
- Files allowed to edit (at most 3; `wit-guest/` rides with `src/` as part of
  the new directory):
  - `modules/core-modules/raft-default/Cargo.toml` (new)
  - `modules/core-modules/raft-default/raft-default.toml` (new)
  - `modules/core-modules/raft-default/src/lib.rs` (new)
- Files explicitly out of bounds:
  - existing support/infill module manifests (Step 5); everything in 240a's
    change surface; the scheduler
- Expected sub-agent dispatches:
  - FACT per row of `design.md` §Substrate Consumed From 240a; scope `crates/`;
    return FACT
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - manifest section; delegated SUMMARY
  - `docs/adr/0009-raft-as-layer-infill-role.md` - direct read (93 lines)
- OrcaSlicer refs: none this step
- Verification:
  - `cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
  - `rg -q 'id = "com.core.raft-default"' modules/core-modules/raft-default/raft-default.toml && rg -q 'claim:raft-fill' modules/core-modules/raft-default/raft-default.toml && rg -q 'Layer::Infill' modules/core-modules/raft-default/raft-default.toml` - AC-1 manifest half
- Exit condition: fresh guest artifact; manifest greps pass; every substrate
  FACT returned EXISTS.

### Step 2: Claim resolution and single-holder proof

- Task IDs: `TASK-415`
- Objective: prove the claim machinery resolves `com.core.raft-default` as the
  sole `claim:raft-fill` holder and that `should_emit(ExtrusionRole::RaftInfill)`
  is true for its held-claim set, reusing the existing SDK test case
  `ac4_raft_fill_claim_emits_raft_infill` (verified present in
  `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs`). Author the
  scheduler double-holder negative in a new
  `crates/slicer-scheduler/tests/raft_claim_conflict_tdd.rs`, asserting a
  `SchedulerError::ClaimConflict` whose `module_a` / `module_b` name both ids.
- Precondition: Step 1 green.
- Postcondition: AC-2 and AC-N1 green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/validation.rs` - the `ClaimConflict` variant
    definition only (locate with `rg -n 'ClaimConflict'`)
  - `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs` (full; small)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/raft_claim_conflict_tdd.rs` (new;
    auto-discovered — `slicer-scheduler` sets no `autotests = false`, though its
    sibling flat `*_tdd.rs` files do carry explicit `[[test]]` entries, so add
    one to match convention if the bare run does not pick the file up)
  - `crates/slicer-scheduler/Cargo.toml` (only if the `[[test]]` entry is needed)
  - `modules/core-modules/raft-default/raft-default.toml` (only if the claim
    string needs correcting)
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/validation.rs` (236-owned; test its observable
    contract only)
- Expected sub-agent dispatches:
  - FACT: does a bare `cargo test -p slicer-scheduler --test raft_claim_conflict_tdd`
    discover the new file, or is an explicit `[[test]]` entry required? scope
    `crates/slicer-scheduler/`; return FACT
- Context cost: `S`
- Authoritative docs: `docs/03_wit_and_manifest.md` - claims section; delegated SUMMARY
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd -- ac4_raft_fill_claim_emits_raft_infill --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-2
  - `test "$(rg -l 'claim:raft-fill' modules/core-modules/*/[a-z-]*.toml | wc -l)" -eq 1` - AC-2 single holder
  - `mkdir -p target && cargo test -p slicer-scheduler --test raft_claim_conflict_tdd -- raft_fill_double_holder_conflicts --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N1
- Exit condition: AC-2 and AC-N1 green with non-zero counts; the double-holder
  test genuinely observes a `ClaimConflict`, not an absence of error.

### Step 3: Raft geometry synthesis (deterministic polygons)

- Task IDs: `TASK-416`
- Objective: implement the port inside `run_infill` — given `Some(raft_plan)`
  on a layer with a negative global index, synthesize object-independent raft
  footprint POLYGONS for the declared band (`raft_layers` = first +
  `base_raft_layers` + `interface_raft_layers`), apply `raft_expansion`
  inflation staged as iterated offsets, expand the first printed raft layer by
  `raft_first_layer_expansion`, and derive interface-band footprints at
  contact-distance spacing — deterministic pure geometry into
  `SlicedRegion.raft_fill`. Polygons only: no scan-line pattern math, no
  extrusion paths (design.md §ADR-0009 Reconciliation). All mm constants ÷100
  at the unit boundary. No anchored entities anywhere. Author the four
  integration cases and register their `mod` line.
- Precondition: Step 2 green.
- Postcondition: AC-3, AC-4, AC-5 green; guests rebuilt.
- Files allowed to read, with ranges when over 300 lines:
  - delegated Orca SUMMARY of `generate_raft_base` staging (working notes)
- Files allowed to edit (at most 3):
  - `modules/core-modules/raft-default/src/lib.rs`
  - `crates/slicer-runtime/tests/integration/raft_geometry.rs` (new; holds
    `raft_fill_is_deterministic_across_two_runs`,
    `raft_first_layer_expansion_exceeds_upper_layers`,
    `raft_geometry_orders_before_model_layers`,
    `raft_mints_no_anchored_entities`)
  - `crates/slicer-runtime/tests/integration/main.rs` (add `mod raft_geometry;`)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**` (delegate); `rectilinear-infill/src/**` and other
    pattern modules; everything in 240a's change surface
- Blast-radius discipline: none (no struct-field change this step).
- Expected sub-agent dispatches:
  - OrcaSlicer SUMMARY: `generate_raft_base` staging order, multi-step
    inflation, base-vs-interface loop structure, classic-vs-organic branch
    selection relevant to PnP's v1 path; return SUMMARY
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` - porting checklist; delegated SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - delegate; never load
- Verification:
  - `cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_fill_is_deterministic_across_two_runs --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-3
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_first_layer_expansion_exceeds_upper_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-4
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry_orders_before_model_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test integration -- raft_mints_no_anchored_entities --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-5
  - `grep -q 'mod raft_geometry;' crates/slicer-runtime/tests/integration/main.rs` - FACT registration present
- Exit condition: AC-3/AC-4/AC-5 green with non-zero counts; registration grep
  passes; two identical runs produce byte-identical `raft_fill`. If the
  claim-holder emit path turns out not to convert `raft_fill` to paths, record
  it as a follow-up per `design.md` §Open Questions — do not add a renderer here.

### Step 4: Raft config keys declared and wired

- Task IDs: `TASK-417`
- Objective: declare `raft_contact_distance` (float, default 0.1, min 0.0),
  `raft_expansion` (float, default 1.5, min 0.0), and
  `raft_first_layer_expansion` (float, default 2.0, min 0.0) in
  `raft-default.toml`'s `[config.schema]` with min/max/display/group matching
  sibling modules, and confirm each is actually read by the geometry it
  controls. Author `crates/slicer-runtime/tests/contract/raft_bounds_tdd.rs`
  with `raft_keys_declared_and_wired`, `raft_index_outside_band_rejected`, and
  `undeclared_raft_key_is_rejected_not_defaulted`, registering
  `mod raft_bounds_tdd;` in `crates/slicer-runtime/tests/contract/main.rs`.
  AC-N3 must exercise the rejection path (a consumed key absent from the
  schema), never a manifest presence grep.
- Precondition: Step 3 green.
- Postcondition: AC-6, AC-N2, AC-N3 green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/main.rs` - registration list only
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` - the
    `[config.schema]` block, as the formatting template
- Files allowed to edit (at most 3):
  - `modules/core-modules/raft-default/raft-default.toml`
  - `crates/slicer-runtime/tests/contract/raft_bounds_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs` (add `mod raft_bounds_tdd;`)
- Files explicitly out of bounds:
  - all other contract test files; the four support manifests (Step 5);
    scheduler internals
- Expected sub-agent dispatches:
  - OrcaSlicer FACT: `PrintConfig.cpp::init_fff_params` defaults and any
    declared minima for the three raft keys; return FACT
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §13 traps T2/T8
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_keys_declared_and_wired --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-6
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_index_outside_band_rejected --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N2
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- undeclared_raft_key_is_rejected_not_defaulted --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N3
  - `grep -q 'mod raft_bounds_tdd;' crates/slicer-runtime/tests/contract/main.rs` - FACT registration present
- Exit condition: all three cases pass with non-zero counts; registration grep
  passes.

### Step 5: Four-manifest wire-or-record + config doc regeneration

- Task IDs: `TASK-418`
- Objective: enumerate every raft key declared in the four support-module
  manifests (`tree-support-planner`, `traditional-support-planner`,
  `tree-support`, `traditional-support`), decide wire-or-record for each, apply
  the manifest edit where the verdict is `wired`, and write every (key,
  manifest) pair into `requirements.md` §Wire-or-Record Decisions with a
  verdict and a reason naming the decision owner. Then regenerate
  `docs/15_config_keys_reference.md` with `cargo xtask gen-config-docs` (T8).
  The scaffold table has four rows as AC-7's minimum — expand it to the real
  key set the dispatched grep returns.
- Precondition: Step 4 green.
- Postcondition: AC-7 green; the regenerated config doc contains the three new
  keys.
- Files allowed to read, with ranges when over 300 lines:
  - the four support-module manifests (small; full read fine)
- Files allowed to edit (at most 3 logical surfaces):
  - `modules/core-modules/{tree-support-planner,traditional-support-planner,tree-support,traditional-support}/*.toml` (annotation / declaration edits across these four count as one logical surface; no logic changes)
  - `docs/spec_packets/240b-support-raft-module/requirements.md` (the table)
  - `docs/15_config_keys_reference.md` (regenerated output only — never hand-edited)
- Files explicitly out of bounds:
  - the support modules' `src/**` (238b/238c surface); `raft-default.toml`
- Expected sub-agent dispatches:
  - LOCATIONS: every raft key declared in the four support-module manifests;
    scope `modules/core-modules/`; return LOCATIONS; purpose: the real key set
- Context cost: `M`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - regenerated output only, not read in bulk
- OrcaSlicer refs: none this step
- Verification:
  - `cargo xtask gen-config-docs && rg -q 'raft_contact_distance' docs/15_config_keys_reference.md && rg -q 'raft_first_layer_expansion' docs/15_config_keys_reference.md` - FACT
  - `test "$(rg -c '^\| .raft_[a-z_]+. \|' docs/spec_packets/240b-support-raft-module/requirements.md)" -ge 4 && ! rg -q 'pending Step 5' docs/spec_packets/240b-support-raft-module/requirements.md && rg -q 'tree-support-planner' docs/spec_packets/240b-support-raft-module/requirements.md && rg -q 'traditional-support' docs/spec_packets/240b-support-raft-module/requirements.md` - AC-7
  - `git diff --stat docs/15_config_keys_reference.md` - FACT: regenerated, not hand-edited
- Exit condition: AC-7 green; every enumerated key has a written verdict; the
  config doc regenerates cleanly.

### Step 6: Formal ADR-0009 amendment + deviation row

- Task IDs: `TASK-535`
- Objective: execute the ADR-0009 amendment per `design.md` §ADR-0009
  Reconciliation "Amendment mechanics" — flip the Status line from
  `Proposed (lands with docs/specs/raft-default-module.md)` to `Accepted`
  (dropping the parenthetical, since that file does not exist), and add an
  additive `## Amendment — <date> (packet 240b)` section that QUOTES the
  original Decision-5 clause verbatim and records the reassignment of
  `claim:raft-fill` to `com.core.raft-default`. Decision 4, the
  zero-pattern-algorithm clause, and the "Do not re-suggest making
  `raft-default` a renderer" Future-Reviewer Note stay UNCHANGED. Then file the
  `D-<pkt>-ADR-0009-AMENDED` deviation row — re-derive the free ID space at
  write time, do not trust an ID written in this packet. Also add
  `com.core.raft-default` to the module inventory in
  `docs/03_wit_and_manifest.md`.
- Precondition: Steps 1-5 green.
- Postcondition: every `packet.spec.md` §Doc Impact Statement grep passes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0009-raft-as-layer-infill-role.md` - direct read (93 lines)
  - `docs/DEVIATION_LOG.md` - the header row and the two `*-ADR-*-AMENDED` rows
    only, as the format template
- Files allowed to edit (at most 3):
  - `docs/adr/0009-raft-as-layer-infill-role.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/03_wit_and_manifest.md`
- Files explicitly out of bounds:
  - `docs/specs/support-families-anchored-entities-plan.md` (the plan is the
    governing authority being cited, not edited)
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0009-raft-as-layer-infill-role.md` - direct read
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 brief only,
    as the amendment's citation source; direct range read
- OrcaSlicer refs: none this step
- Verification:
  - ``rg -A2 '^## Status' docs/adr/0009-raft-as-layer-infill-role.md | rg -q 'Accepted' && rg -q '^## Amendment' docs/adr/0009-raft-as-layer-infill-role.md && rg -q 'com\.core\.raft-default' docs/adr/0009-raft-as-layer-infill-role.md`` - FACT: amendment present with reassignment
  - ``test "$(rg -c 'rectilinear-infill` declaring the claim' docs/adr/0009-raft-as-layer-infill-role.md)" -ge 2`` - FACT: the clause appears TWICE (original Decision 5 + the verbatim quote inside the Amendment). One occurrence means the amendment did not quote it; a single-hit `rg -q` would pass against the unamended file and prove nothing.
  - `rg -q 'raft-default-module\.md' docs/adr/0009-raft-as-layer-infill-role.md && exit 1 || true` - FACT: the dangling reference is gone
  - `rg -q 'ADR-0009-AMENDED' docs/DEVIATION_LOG.md && cargo xtask check-deviations; echo EXIT:$?` - FACT exit 0
- Exit condition: all greps pass; `git diff` on the ADR shows additions plus the
  single Status-line change, and no modification to any Decision or
  Future-Reviewer line.

### Step 7: DEV-124 re-verification, acceptance gates, Human Validation Gate

- Task IDs: `TASK-535`
- Objective: run the AC-8 commands under a raft-configured config view and
  write the outcome into `requirements.md` §DEV-124 Re-verification — pass, or
  the corrected predicate plus the fix. 240a filed the reopen row on the
  grounds that the clamp is index-convention-dependent; this is where it is
  tested against real behaviour. Never widen or weaken the assertions. Then run
  the packet-level gates and produce the human-gate artifacts (`tmp/p240b-*`
  G-code + visual-debug bundle), recording checklist verdicts and leaving
  sign-off to the human.
- Precondition: Step 6 green; the §9 raft-enabled Orca references exist at
  `tmp/p240b-orca-*-raft.gcode` (human-owned). If absent, the gate stays open
  and the packet reports blocked-on-human, not done.
- Postcondition: gates green; the checklist is written in `packet.spec.md`
  §Human Validation Gate except the sign-off line.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs` -
    the two clamp cases only
- Files allowed to edit (at most 3):
  - `docs/spec_packets/240b-support-raft-module/requirements.md` (§DEV-124 Re-verification)
  - `docs/spec_packets/240b-support-raft-module/packet.spec.md` (checklist verdicts only)
  - `tmp/p240b-profile.json` + `tmp/p240b-vd-raft.json` (artifact inputs; count
    as one surface)
- Files explicitly out of bounds:
  - the perimeter generators — if AC-8 fails, the fix goes through the reopened
    deviation's routing, and the failure is recorded here before any code moves
  - anything under `docs/spec_packets/` other than this packet
- Expected sub-agent dispatches:
  - cargo runs delegated under the FACT contract; visual-debug bundle
    generation via `pnp_cli`
- Context cost: `S`
- Authoritative docs:
  - `docs/19_visual_debug.md` - bundle format; delegated SUMMARY
  - `docs/17_agent_debugging.md` - delegated SUMMARY
- OrcaSlicer refs: comparison against the regenerated references only
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test contract -- classic_clamp_unchanged_when_no_raft_configured --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-8
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask check-literals --report` - FACT: violation count equal to the count recorded on a clean tree immediately BEFORE this packet's first edit (re-derive it then; do not trust any number written here). Requirement: this packet adds zero new violations.
- Exit condition: AC-8 outcome recorded with evidence; gates green; artifacts
  present; checklist filled except the human sign-off line.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | substrate verification + new guest dir + first rebuild |
| Step 2 | S | claim resolution + double-holder negative |
| Step 3 | M | geometry port + four integration cases |
| Step 4 | M | keys + bounds + undeclared-key negative |
| Step 5 | M | wire-or-record across four manifests + doc regen |
| Step 6 | S | ADR amendment + deviation row |
| Step 7 | S | DEV-124 re-verification + gates + human gate |

Aggregate is `M`; no row is `L`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo xtask check-literals` exit 0, with no new violations relative to the
  pre-packet baseline re-derived on a clean tree.
- Update `docs/07_implementation_status.md` with `TASK-414`..`TASK-418` and
  `TASK-535` through a worker dispatch, never a full backlog read.
- Reconcile superseded transitions: G-06 closed; 215-raft-geometry absorption
  recorded (the directory was already deleted by 236 AC-10); plan §11 queue
  row #7 marked done across both 240a and 240b.
- `packet.spec.md` is ready for `status: implemented` only after the Human
  Validation Gate sign-off line is dated.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run `cargo xtask test --summary --workspace` once, dispatched to a sub-agent
  under the FACT contract, since this packet activates a code path that was
  dead across the whole pipeline.
- Record remaining packet-local risk (pattern takeover by another claim holder;
  downstream `raft_fill`→path conversion if Step 3 found it missing).
- Confirm context stayed at or below the standard band, or at/below the
  extended band only with a logged swarm ESCALATION; otherwise record a
  packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and
verification commands use `--all-targets` where applicable so test, bench, and
example targets compile.
