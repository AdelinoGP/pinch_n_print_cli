# Implementation Plan: internal-bridge-over-infill

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Pop `stash@{0}`, triage salvage, restore a green guest baseline

- Task IDs: `ISSUE-82`
- Objective: per plan D4/D10, pop `stash@{0}` and triage its ~1338 added lines across 19 files: KEEP flag threading/routing shape, module routing + TOML schema, gcode label mapping, and the false-site gating direction; DISCARD the orientation heuristic and the contour-band expansion approximation (`INTERNAL_BRIDGE_EXPANSION_MULTIPLIER` in the stashed `slice_postprocess_prepass.rs`) — Steps 4/5 replace them. Rebuild guests and re-establish a compiling, guest-fresh baseline before any packet edit.
- Precondition: packet activated; working tree otherwise clean; human confirmation obtained for the git mutation (`git stash pop`).
- Postcondition: stash popped and triaged; discarded hunks removed; `cargo xtask build-guests --check` exits 0 after a rebuild; `cargo check --workspace --all-targets` green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/bridge-parity-plan.md` - §5 stash disposition only
  - stash diff - file-by-file per salvage map, never wholesale
- Files allowed to edit (at most 3):
  - whichever stashed files carry discarded hunks (triage output; typically `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, `modules/core-modules/rectilinear-infill/src/lib.rs`, plus one more as triage finds)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/`, `crates/slicer-core/src/algos/mesh_analysis.rs`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct/schema additions in this step. Triage must not delete `is_internal_bridge` threading yet — Step 2 retires it (deleting it now would break the stashed module dispatch before the variant exists).
- Expected sub-agent dispatches:
  - Question: which stash files carry the discarded orientation heuristic and contour-band expansion; scope: `git stash show -p stash@{0}`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/bridge-parity-plan.md` - §0 D4/D10, §5
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check; echo EXIT=$?` — if 1, run `cargo xtask build-guests`, re-check until 0; exit 3 = missing `wasm-tools` infra error, stop and report
  - `cargo check --workspace --all-targets` - FACT pass/fail
- Exit condition: guests fresh (exit 0), workspace compiles, salvage map applied, discarded code gone from the working tree.

### Step 2: Add `ExtrusionRole::InternalBridgeInfill` through IR/WIT/host/marshal/gcode; retire the string tag

- Task IDs: `ISSUE-82`
- Objective: D7/F8 — add the `InternalBridgeInfill` variant to `ExtrusionRole` (`crates/slicer-ir/src/slice_ir.rs`), the WIT `extrusion-role` enum (`crates/slicer-schema/wit/`), host marshal, and gcode emission (variant → `internal_bridge_speed`, default 37.5 in `crates/slicer-ir/src/feedrate.rs`; label `Internal Bridge`); retire `Custom("InternalBridge")`, the `"InternalBridge"` string arm in `crates/slicer-gcode/src/emit.rs`, and the stash's `is_internal_bridge` flag (AC-N1).
- Precondition: Step 1 exit met (green, guest-fresh baseline).
- Postcondition: variant compiles end-to-end; AC-1/AC-2/AC-N1 greps and unit test pass; guests rebuilt fresh; schema-version bump (if required) and its test fallout landed in this step.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - `ExtrusionRole` enum + impls only
  - `crates/slicer-gcode/src/emit.rs` - feedrate/label mapping only
  - `crates/slicer-ir/src/feedrate.rs` - `internal_bridge_speed` field/default/key only
- Files allowed to edit (at most 3 primary + test files):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-schema/wit/` (the file defining `extrusion-role`)
  - `crates/slicer-gcode/src/emit.rs` + `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs` (AC-2 test — the mapping/label change and its test are one gcode-emission unit; the test file is listed here so AC-2 names a real binary)
  - plus match/marshal sites enumerated by the LOCATIONS dispatch (host, macros, SDK, partition, report) — edited in place, each one listed in the step record
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/`, `modules/core-modules/` (module emission switches to the variant in Step 5)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - A new `ExtrusionRole` variant breaks every exhaustive `match` workspace-wide. Dispatch a `LOCATIONS` worker BEFORE editing; cite the returned site list inline in the step record.
  - Dispatch a `FACT + LOCATIONS` worker: does committed SliceIR serialize `ExtrusionRole`, and is there an IR schema-version constant? If yes, bump the constant in this step and fix every test hard-asserting the old value (run that test binary here, not at the ceremony).
  - Watched-type test literals broken by the variant get `..` rest or `// exhaustive:` waivers in this step; `cargo xtask check-literals` is part of verification.
- Expected sub-agent dispatches:
  - Question: all exhaustive `match` on `ExtrusionRole` + construction/marshal sites; scope: `crates/`, `modules/core-modules/`; return: `LOCATIONS` (≤20/crate, state if more)
  - Question: SliceIR serialization of `ExtrusionRole` + schema-version constant; scope: `crates/slicer-ir/`, `crates/slicer-runtime/`; return: `FACT + LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY of the `extrusion-role` contract section
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.cpp` - delegate; `erInternalBridgeInfill` label text `Internal Bridge`
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
  - `cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd -- internal_bridge 2>&1 | tee target/test-output.log` - AC-2 (packet-added test)
  - `cargo xtask build-guests --check` then rebuild; guest-touching re-runs via `cargo xtask test --summary`
  - If schema version bumped: the test binary asserting the constant - FACT pass/fail
- Exit condition: AC-1, AC-2, AC-N1 commands all pass; workspace green; guests fresh.

### Step 3: Canonicalize `bridging_flow` (F5)

- Task IDs: `ISSUE-82`
- Objective: `crates/slicer-core/src/flow.rs` `bridging_flow` gains canonical thread-diameter selection (`bridge_line_width` if set else `nozzle_diameter`) and `bridge_extrusion_spacing(dmr) = dmr + BRIDGE_EXTRA_SPACING` where `BRIDGE_EXTRA_SPACING = 0.05 mm`; retire the stash's module-level +0.05 mm shim in `run_infill` (AC-N3). Do NOT touch `resolve_role_width` — it already consumes `RoleWidthContext.bridge_line_width` separately.
- Precondition: Step 2 exit met.
- Postcondition: AC-8/AC-N3 pass; all `bridging_flow` callers updated to the new signature.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/flow.rs` - `bridging_flow` and spacing helpers only
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/flow.rs`
  - `modules/core-modules/rectilinear-infill/src/lib.rs` (shim removal + call-site update)
  - `crates/slicer-core/tests/bridge_over_infill_tdd.rs` (NEW — created here with the AC-8 `bridging_flow` tests; Step 4 extends the same file with AC-3/AC-4 angle/polygon tests)
- Files explicitly out of bounds:
  - `resolve_role_width` and its callers
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Signature change only: dispatch `LOCATIONS` for `bridging_flow` callers before editing; cite inline. No schema constants touched.
- Expected sub-agent dispatches:
  - Question: all `bridging_flow` call sites; scope: `crates/`, `modules/core-modules/`; return: `LOCATIONS`
  - Question: canonical `LayerRegion::bridging_flow` + `Flow::bridge_extrusion_spacing` exact semantics; scope: `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp`, `OrcaSlicerDocumented/src/libslic3r/Flow.hpp`, `OrcaSlicerDocumented/src/libslic3r/Flow.cpp`; return: `SUMMARY` ≤200 words
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - porting checklist (0.05 mm → units)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.hpp` + `Flow.cpp` - delegate
- Verification:
  - `cargo test -p slicer-core --test bridge_over_infill_tdd -- bridging_flow 2>&1 | tee target/test-output.log` - AC-8 (packet-added tests: width selection, spacing = dmr + 0.05 mm)
  - `cargo check --workspace --all-targets` - FACT pass/fail
- Exit condition: AC-8, AC-N3 pass; callers compile.

### Step 4: Introduce the internal-bridge decision at the InfillPostProcess seam + port the geometry

- Task IDs: `ISSUE-82`
- Objective: create `crates/slicer-core/src/algos/bridge_over_infill.rs` (standard OrcaSlicer porting header) with `determine_bridging_angle` (length-weighted mean over ±18° sliding window of nearest-anchor orientations; `internal_bridge_angle > 0` override; equal-cost → smallest quantized angle per ADR-0061) and `construct_anchored_polygon` (scan lines every `bridging_flow.scaled_spacing()`, clipped to anchors/walls), plus deterministic anchor clustering above voids. Wire the pass into the `LayerStageCommit::InfillPostProcess(ir)` arm in `crates/slicer-runtime/src/layer_executor.rs`: consume committed sparse-infill polylines as anchors (canonical `generate_sparse_infill_polylines_for_anchoring` equivalent), emit `InternalBridgeInfill` regions, subtract from sparse infill. This INTRODUCES the decision at the seam — at HEAD `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) contains zero internal-bridge logic, so nothing is removed from it; AC-N2 is a post-implementation guard (grep only, no edit to the prepass file).
- Precondition: Steps 2 and 3 exit met (variant + canonical flow exist).
- Postcondition: AC-3/AC-4/AC-5/AC-N2 pass; internal bridges are decided post-surface with anchored polygons.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - ranged around `LayerStageCommit::InfillPostProcess` and the `"Layer::Infill" | "Layer::InfillPostProcess"` grouping
  - `crates/slicer-runtime/src/region_partition.rs` - ranged around the "Shared with the Layer::InfillPostProcess dispatch arm's wall-source" comment
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - ranged around `commit_shell_classification_builtin`
- Files allowed to edit (at most 3 primary + named test files):
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` (new) + `crates/slicer-core/src/algos/mod.rs` (register `pub mod bridge_over_infill;` — 1-line fallout of the new module)
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/tests/integration/region_partition_tdd.rs` (AC-5 I6 disjointness test — already declared in `tests/integration/main.rs`, so no new `mod` registration)
  - test file: `crates/slicer-core/tests/bridge_over_infill_tdd.rs` (extend with AC-3/AC-4 angle/polygon tests; created in Step 3)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/prepass_slice.rs` (`assemble_bridge_areas` — packet 234), `crates/slicer-core/src/algos/mesh_analysis.rs` (packet 235), `modules/core-modules/`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - New stage-blackboard data (anchor clusters, internal-bridge regions) follows the existing commit/rollback pattern of `ShellClassificationError`-style pass plumbing; any new pub struct with ≥5 fields triggers the watched-type rule for its test literals (`..` rest or waiver) — enumerate those literals when writing the packet-added tests.
- Expected sub-agent dispatches:
  - Question: pseudocode of canonical `determine_bridging_angle` + `construct_anchored_polygon` lambdas and `bridge_over_infill` clustering loop; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SUMMARY` ≤200 words + snippet ≤30 lines
  - Question: how the `InfillPostProcess` arm currently exposes committed sparse-infill polylines/wall sources; scope: `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-runtime/src/region_partition.rs`; return: `FACT + LOCATIONS`
- Context cost: `M` (largest step)
- Authoritative docs:
  - `docs/adr/0061-deterministic-bridge-orientation-tie-break.md` - direct read (short)
  - `docs/08_coordinate_system.md` - porting checklist
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` - delegate
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` (`generate_sparse_infill_polylines_for_anchoring`) - delegate
- Verification:
  - `cargo test -p slicer-core --test bridge_over_infill_tdd -- bridging_angle 2>&1 | tee target/test-output.log` - AC-3
  - `cargo test -p slicer-core --test bridge_over_infill_tdd -- anchored_polygon 2>&1 | tee target/test-output.log` - AC-4
  - `cargo test -p slicer-runtime --test integration -- internal_bridge_disjoint 2>&1 | tee target/test-output.log` - AC-5
  - `! rg -q 'INTERNAL_BRIDGE_EXPANSION_MULTIPLIER|[Ii]nternal[Bb]ridge' crates/slicer-runtime/src/slice_postprocess_prepass.rs` - AC-N2 (guard; no edit)
  - `cargo xtask test --summary -p slicer-runtime --test e2e` - guest-consuming regression net
- Exit condition: AC-3, AC-4, AC-5, AC-N2 pass; e2e net green; prepass free of internal-bridge logic.

### Step 5: Module behavior — per-role speeds (F6 + Q1), no odd-layer alternation (D11/F7), owned config keys

- Task IDs: `ISSUE-82`
- Objective: in `modules/core-modules/rectilinear-infill/src/lib.rs` `run_infill`: (a) delete the shared `speed_factor = self.infill_speed / BASE_SPEED` (BASE_SPEED=50.0) coupling — bridge roles feedrate from `internal_bridge_speed`/`bridge_speed`, solid roles from their own resolved speeds per the Q1 decision (canonical: `erTopSolidInfill → top_surface_speed`, `erSolidInfill → internal_solid_infill_speed`, `erInternalInfill → sparse_infill_speed`); (b) delete the `layer_index.is_multiple_of(2)` +90° odd-layer rotation; (c) emit `InternalBridgeInfill` via the Step 2 variant (retiring the stash's Custom tag emission); (d) manifest gains snake_case `dont_filter_internal_bridges`, `enable_extra_bridge_layer`, `internal_bridge_angle` (and solid-speed keys if absent per Q1).
- Precondition: Step 2 exit met (variant exists); Step 3 exit met.
- Postcondition: AC-7/AC-9 pass; module emits role-correct feedrates and constant rectilinear direction; guests rebuilt.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` - `run_infill` + config plumbing only
- Files allowed to edit (at most 3 primary + named test files):
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
  - `modules/core-modules/rectilinear-infill/` manifest TOML
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs` (AC-7 alternation test — existing flat file, auto-discovered; no aggregator registration needed)
  - plus one host config-plumbing file if the keys need registration (dispatch finds it)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/flow.rs` (Step 3 owns), other core-modules
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - New manifest keys ripple into module config structs: enumerate struct-literal sites of the module config type in module tests; `..` rest or waiver per docs/21; cite the LOCATIONS result inline.
- Expected sub-agent dispatches:
  - Question: module config struct literal sites + host key-registration path; scope: `modules/core-modules/rectilinear-infill/`, `crates/slicer-runtime/`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY of the waiver rule
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` (per-role `role_speed` block) - delegate only if Q1 evidence in design.md is doubted
  - `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.hpp` (`_layer_angle` ≡ 0) - delegate only if doubted
- Verification:
  - `cargo test -p rectilinear-infill --test rectilinear_infill_tdd -- alternation 2>&1 | tee target/test-output.log` - AC-7
  - `rg -q 'dont_filter_internal_bridges' modules/core-modules && rg -q 'enable_extra_bridge_layer' modules/core-modules && rg -q 'internal_bridge_angle' modules/core-modules` - AC-9
  - `cargo xtask build-guests --check` (exit 0 after rebuild) then `cargo xtask test --summary -p slicer-runtime --test e2e` - FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
- Exit condition: AC-7, AC-9 pass; guests fresh; e2e net green.

### Step 6: Nominate the AC-6 model, run the invariant sweep, land doc edits and backlog update

- Task IDs: `ISSUE-82`
- Objective: verify the nominated model `resources/bridge.obj` yields an internal-bridge-over-sparse site (substitute `overhang.obj` then `ipadstand.obj` per the concrete fallback clause in `requirements.md` §Acceptance Summary if it does not, and record the substitution + Z there); create the two AC-6 config fixtures; run the AC-6 I7 reslice pair; land the three doc edits with their greps (`docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/15_config_keys_reference.md`); re-run every pipe-suffixed AC. No `docs/07_implementation_status.md` update — the backlog uses issue files, not `TASK-###` rows (see `task-map.md`).
- Precondition: Steps 1–5 exits met.
- Postcondition: all ACs pass; doc greps green; backlog updated; packet ready for `status: implemented`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - `ExtrusionRole` section only (locate by grep)
  - `docs/03_wit_and_manifest.md` - `extrusion-role` section only
  - `docs/15_config_keys_reference.md` - bridging/infill key section only
- Files allowed to edit (at most 3 primary + new fixtures):
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
  - `docs/15_config_keys_reference.md`
  - new fixtures: `resources/test_config/ac6_infill_40.json` (`{"infill_speed": 40}`) and `resources/test_config/ac6_infill_120.json` (`{"infill_speed": 120}`) — 2-line JSON, created here so AC-6's `--config` paths are real
- Files explicitly out of bounds:
  - `crates/`, `modules/` (no code edits in this step)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - None — docs only.
- Expected sub-agent dispatches:
  - Question: does `resources/bridge.obj` yield an internal-bridge-over-sparse site, and at what Z; scope: `resources/`; return: `FACT model + Z` (reslices MUST pass `--module-dir modules/core-modules`); if no site, repeat for `overhang.obj` then `ipadstand.obj`
- Context cost: `S`
- Authoritative docs:
  - none beyond the edited three
- OrcaSlicer refs:
  - none
- Verification:
  - AC-6 reslice pair + python assertion (see `packet.spec.md`; model `resources/bridge.obj`, configs `resources/test_config/ac6_infill_40.json` / `ac6_infill_120.json`) - FACT pass/fail
  - `rg -q 'InternalBridgeInfill' docs/02_ir_schemas.md && rg -q 'InternalBridgeInfill|internal-bridge-infill' docs/03_wit_and_manifest.md && rg -q 'internal_bridge_angle' docs/15_config_keys_reference.md` - doc greps
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: every AC (positive + negative + doc greps) returns PASS; packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | stash pop + triage + guest rebuild |
| Step 2 | M | enum blast radius; largest dispatch surface |
| Step 3 | S | signature + constant |
| Step 4 | M | largest step: seam introduction + two geometry ports |
| Step 5 | M | module behavior + keys |
| Step 6 | S | nomination, sweep, docs |

Aggregate: M. No step is L; no split required.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check` and `cargo clippy` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile. `cargo test` runs use the narrow `--test <bin>` / `-p <crate>` invocations specified per AC — `--all-targets` does not apply to test runs.
