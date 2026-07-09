# Implementation Plan: 149-pipeline-config-wiring

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Manifest entries — 8 new keys in arachne, 7 in classic (with `min_width_top_surface` default verified)

- Task IDs:
  - none
- Objective: register the 8 new config keys in the arachne manifest and the 7 in the classic manifest, with byte-for-byte default values matching `docs/ORCA_CONFIG_REFERENCE.md`. The `min_width_top_surface` default is verified via a sub-agent dispatch BEFORE commit (the spec's guess of 1.2mm is unverified).
- Precondition: `parity/arachne` is checked out; packet 148 has landed (so the arachne module compiles with the new `slicer-core` dep).
- Postcondition: `cargo check -p arachne-perimeters --all-targets` and `cargo check -p classic-perimeters --all-targets` are green; the 4 arachne-keys red test (AC-1) and 2 keys-arachne/2 keys-classic red tests (AC-2 partial) are green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/ORCA_CONFIG_REFERENCE.md` lines 135, 161, 165-168, 178, 1327, 1941 (the canonical defaults; the sub-agent dispatch reads these)
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` lines 45-50 (the existing `extra_perimeters_on_overhangs` to re-publish)
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (204 lines, full)
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` (197 lines, full)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/**` (no edits this step)
  - the source `lib.rs` files (no edits this step)
- Expected sub-agent dispatches:
  - "Read `docs/ORCA_CONFIG_REFERENCE.md` lines 135, 161, 165-168, 178, 1327, 1941; return SNIPPETS (verbatim, ≤ 30 lines) of the canonical OrcaSlicer defaults for the 8 new keys, and confirm whether `min_width_top_surface` is a percent or mm, and the resolved mm value for a 0.4mm nozzle." — purpose: confirm the canonical default values BEFORE the manifest is committed.
  - "Run `cargo check -p arachne-perimeters --all-targets 2>&1 | tee target/check.log`; return FACT (pass) or SNIPPETS (first 20 lines of error)." — purpose: confirm the manifest parses.
  - "Run `cargo xtask build-guests --check 2>&1 | tee target/guest-check.log`; return FACT (Fresh/STALE)." — purpose: confirm the manifest change is non-stale.
- Context cost: S
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` (delegate the `[config.schema]` format summary)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:5003-5066, 1491-1511, 5059-5066, 1658, 1327, 1941`
- Verification:
  - `cargo check -p arachne-perimeters --all-targets 2>&1 | tee target/check.log` — pass
  - `cargo check -p classic-perimeters --all-targets 2>&1 | tee target/check.log` — pass
  - `cargo xtask build-guests --check` — Fresh
- Exit condition: both `cargo check` exit 0; `xtask build-guests --check` exits 0 with `Fresh:` in the last 5 lines.

### Step 2: AC-1 + AC-2 — manifest-presence red tests go green (D1 + D2 close at manifest layer)

- Task IDs:
  - none
- Objective: confirm that after Step 1, the audit's `arachne_parity_pipeline_overhang_reverse_config_keys` and `arachne_parity_pipeline_only_one_wall_top_vs_min_width_top_surface` red tests are green. No code changes; this is a verification step.
- Precondition: Step 1 complete.
- Postcondition: AC-1 and AC-2 are green.
- Files allowed to read:
  - `crates/slicer-runtime/tests/arachne_parity.rs` lines 260-290 (AC-1) and 376-395 (AC-2) — the red test predicates (read-only, to confirm the test's grep assertions match the manifest entries).
- Files allowed to edit:
  - none (no code or doc edits this step)
- Files explicitly out-of-bounds for this step:
  - all source files (read-only)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_pipeline_overhang_reverse_config_keys 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-1 green.
  - "Run `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_pipeline_only_one_wall_top_vs_min_width_top_surface 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-2 green.
- Context cost: S
- Authoritative docs:
  - none
- OrcaSlicer refs:
  - none (the test predicates are manifest greps)
- Verification:
  - AC-1 + AC-2 green
- Exit condition: full `arachne_parity` count is now 12 passed (3 packet-1 + 7 packet-148 + 2 this step), 3 red (2 packet-149 D3+D4 + 1 D-104f).

### Step 3: AC-3 — `alternate_extra_wall` behavior (`max_bead_count` bump on odd layers)

- Task IDs:
  - none
- Objective: thread `alternate_extra_wall` through `arachne_params_from_config` and apply the **max_bead_count bump** in `run_perimeters` (mirrors OrcaSlicer's `loop_number++` → `max_bead_count` beading-stack mechanism, NOT a wall-count mutation). The bump fires on odd layers (`layer_index % 2 == 1`) when `alternate_extra_wall=true && !spiral_vase && sparse_infill_density > 0`. Add a unit test in `arachne-perimeters/tests/alternate_extra_wall_tdd.rs` (NEW) that drives `ArachnePerimeters::run_perimeters` natively and verifies the alternating wall count (3 on odd, 2 on even).
- Precondition: Step 2 complete; packet 148 has landed.
- Postcondition: `arachne_parity_pipeline_alternate_extra_wall_not_registered` is green; the new unit test is green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` lines 100-200 (arachne_params_from_config), 230-353 (run_perimeters)
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1227` (classic) and `:2133` (arachne) — the canonical `loop_number++` site; delegate via SUMMARY
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/tests/alternate_extra_wall_tdd.rs` (new file)
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/classic-perimeters/src/lib.rs` (out of scope; classic's D3 behavior is also needed but is a separate change; the audit's red test only asserts the arachne path; for parity, the implementer may mirror the change to classic, but the unit test is arachne-only)
  - `crates/slicer-core/src/**` (no flow math this step)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_pipeline_alternate_extra_wall_not_registered 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-3 integration test green.
  - "Run `cargo test -p arachne-perimeters --test alternate_extra_wall_tdd 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-3 unit test green.
- Context cost: S
- Authoritative docs:
  - `docs/ORCA_CONFIG_REFERENCE.md` line 178 (canonical `alternate_extra_wall` default and group)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1227` (classic) and `:2133` (arachne) — the `alternate_extra_wall` consumption site
- Verification:
  - AC-3 integration + unit green
- Exit condition: full `arachne_parity` count is now 13 passed (12 + 1 D3), 2 red (D4 + D-104f).

### Step 4: AC-4 — `bridging_flow()` helper + D4 bridge flow (real OrcaSlicer ratio)

- Task IDs:
  - none
- Objective: add `pub fn bridging_flow(bridge_flow_ratio: f32, thick_bridges: bool) -> f32` to `crates/slicer-core/src/flow.rs`; apply the bridge flow factor reduction in BOTH `arachne-perimeters/src/lib.rs` and `classic-perimeters/src/lib.rs` for any `path.points[i]` with `feature_flags[i].is_bridge == true`; add a unit test in `arachne-perimeters/tests/bridge_flow_factor_tdd.rs` (NEW) that drives `run_perimeters` natively with `bridge_areas` non-empty and `bridge_flow = 0.7`, asserting `flow_factor == 0.7` for bridge vertices. The helper implements the real OrcaSlicer formula (ratio-based, NOT constant 0.85); the per-vertex `flow_factor` model diverges from OrcaSlicer's per-path `Flow` model (D-104g documents this).
- Precondition: Step 3 complete; packet 148 has landed (`is_bridge` flag is set per-vertex).
- Postcondition: `arachne_parity_pipeline_bridge_flow_factor_on_overhang` is green; the new unit test is green; the classic path also applies the bridge flow (verified by the classic path's existing bridge fixture tests, which should still pass).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/flow.rs` (122 lines, full) — the existing flow math
  - `modules/core-modules/arachne-perimeters/src/lib.rs` lines 280-310 (the construction loop where flow_factor is set)
  - `modules/core-modules/classic-perimeters/src/lib.rs` lines 670-720 (the per-vertex bridge flag and wall loop construction)
- Files allowed to edit (≤ 4):
  - `crates/slicer-core/src/flow.rs`
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/tests/bridge_flow_factor_tdd.rs` (new file)
  - `modules/core-modules/classic-perimeters/src/lib.rs` (one-line edit; allowed in addition to the ≤ 3 limit because the change is symmetric and trivially small)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-gcode/src/**` (the G-code emit consumes `flow_factor` but is out of scope)
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp:135`; return SUMMARY (≤ 200 words) of the real `bridging_flow(FlowRole, bool)` formula (it is a ratio, not a constant)." — purpose: confirm the real formula.
  - "Run `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_pipeline_bridge_flow_factor_on_overhang 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-4 integration test green.
  - "Run `cargo test -p arachne-perimeters --test bridge_flow_factor_tdd 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-4 unit test green.
  - "Run `cargo test -p classic-perimeters --lib 2>&1 | tee target/test-output.log`; return FACT (pass) — confirm the classic path's existing tests still pass." — purpose: parity check.
- Context cost: M (touches both perimeter modules + a new `slicer-core` helper)
- Authoritative docs:
  - `docs/02_ir_schemas.md` §1542-1558 (`Point3WithWidth.flow_factor`) — delegate SUMMARY
  - `docs/ORCA_CONFIG_REFERENCE.md` line 150 (`thick_bridges` default), `:1327` (`bridge_flow` default)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp:135` — `bridging_flow(FlowRole, bool)` (real formula)
- Verification:
  - AC-4 integration + unit green; classic path tests still pass
- Exit condition: full `arachne_parity` count is now 14 passed (13 + 1 D4), 1 red (D-104f only).

### Step 5: AC-5 + AC-6 + AC-N1 + AC-N2 — deviation log + audit history + config keys reference + manifest-drift guard

- Task IDs:
  - none
- Objective: add 6 new deviation rows to `docs/DEVIATION_LOG.md` (D-104b, D-104c, D-104d, D-104e with `Status: Closed — 2026-07-09: packet 149`, D-104f with `Status: Open — deferred to follow-up workstream` and `Target Close: — (deferred; follow-up workstream TBD)`, D-104g with `Status: Open` documenting the per-vertex `flow_factor` vs OrcaSlicer's per-path `Flow` model divergence); append 6 rows to `docs/14_deviation_audit_history.md`; append 8 rows to `docs/15_config_keys_reference.md`; flip D-104's status (closed at packet 148) is unchanged here.
- Precondition: Steps 1-4 complete (all 4 red tests for D1/D2/D3/D4 are green).
- Postcondition: 6 Doc Impact greps pass; `cargo xtask check-deviations` regenerates the Open Deviation Map without errors.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/DEVIATION_LOG.md` lines 12-50 (the table format)
  - `docs/14_deviation_audit_history.md` (whole file, ≤ 100 lines)
  - `docs/15_config_keys_reference.md` (whole file)
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/14_deviation_audit_history.md`
  - `docs/15_config_keys_reference.md`
- Files explicitly out-of-bounds for this step:
  - `docs/07_implementation_status.md` (NOT edited directly; regenerated by `cargo xtask check-deviations`)
  - the manifest and source files (no edits this step)
- Expected sub-agent dispatches:
  - "Run `rg -q 'D-104b-OVERHANG-FLOW-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104c-OVERHANG-REVERSE-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104d-MIN-WIDTH-TOP-SURFACE-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104e-ALTERNATE-EXTRA-WALL-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104f-CONCENTRIC-INFILL-NO-ARACHNE' docs/DEVIATION_LOG.md && rg -q 'D-104g-FLOW-FACTOR-PERVERTEX-DIVERGENCE' docs/DEVIATION_LOG.md; echo $?`; return FACT (exit 0 = pass for AC-5)." — purpose: Doc Impact grep 1.
  - "Run `rg -q 'config\.schema\.(precise_outer_wall|seam_candidate_angle_threshold_deg)' modules/core-modules/arachne-perimeters/arachne-perimeters.toml; echo $?`; return FACT (exit code 1 = pass for AC-N1)." — purpose: manifest-drift guard.
  - "Run `rg -A1 'D-104f-CONCENTRIC-INFILL-NO-ARACHNE' docs/DEVIATION_LOG.md | head -5`; return SNIPPETS." — purpose: AC-N2.
  - "Run `rg -q 'alternate_extra_wall' docs/15_config_keys_reference.md && rg -q 'detect_overhang_wall' docs/15_config_keys_reference.md && rg -q 'min_width_top_surface' docs/15_config_keys_reference.md && rg -q 'bridge_flow' docs/15_config_keys_reference.md && rg -q 'thick_bridges' docs/15_config_keys_reference.md; echo $?`; return FACT (exit 0 = pass)." — purpose: Doc Impact grep 2.
  - "Run `cargo xtask check-deviations 2>&1 | tee target/check-deviations.log`; return FACT (pass) or SNIPPETS (first 20 lines of error)." — purpose: regenerate the Open Deviation Map.
- Context cost: S
- Authoritative docs:
  - `docs/14_deviation_audit_history.md` — read for the existing row format
- OrcaSlicer refs:
  - none
- Verification:
  - All Doc Impact greps pass
  - `cargo xtask check-deviations` exits 0
  - AC-6 (full `arachne_parity` count is 14 passed, 1 red — the D-104f test)
- Exit condition: 6 deviation rows present, 6 audit-history rows present, 8 config-keys rows present, manifest-drift guard passes, D-104f row's `Target Close` does not name a fabricated packet.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Manifest edits (8 new keys arachne, 7 new keys classic), no source-code changes; `min_width_top_surface` default verified via sub-agent dispatch |
| Step 2 | S | Verification only (no code edits) |
| Step 3 | S | One new config read + one `max_bead_count` bump branch + one new test file |
| Step 4 | M | New `slicer-core` helper (real OrcaSlicer ratio) + two perimeter modules + one new test file |
| Step 5 | S | Doc edits (6 deviation rows + 6 audit-history rows + 8 config-keys rows), no code changes |

Aggregate: M. No step is L. The packet does not need to split.

## Packet Completion Gate

- All 5 steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green: AC-1 through AC-6, AC-N1, AC-N2.
- The arachne guest is rebuilt: `cargo xtask build-guests --check` is Fresh.
- `docs/DEVIATION_LOG.md` has 6 new rows; `docs/14_deviation_audit_history.md` has 6 new rows; `docs/15_config_keys_reference.md` has 8 new rows.
- `cargo xtask check-deviations` regenerates the Open Deviation Map without errors.
- D-104f's `arachne_parity_pipeline_concentric_infill_uses_arachne` red test STAYS RED (the explicit success criterion for the deviation registration).
- D-104g's deviation row documents the per-vertex `flow_factor` vs OrcaSlicer's per-path `Flow` model divergence; the row's `Status: Open` reflects the limited divergence (the `bridge_flow` ratio is correctly modelable per-vertex; only the `thick_bridges` branch is the realization site).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-6, AC-N1, AC-N2).
- Confirm packet-level verification commands are green: `cargo test -p slicer-runtime --test arachne_parity` shows 14 passed, 1 red (D-104f only).
- Confirm `cargo test -p arachne-perimeters --tests` is clean (AC-3 + AC-4 unit tests green).
- Confirm `cargo test -p classic-perimeters --lib` is clean (parity check after D4 bridge flow added to both perimeter modules).
- Confirm `cargo clippy -p slicer-runtime --test arachne_parity -- -D warnings` is clean.
- Confirm `cargo xtask build-guests --check` is Fresh.
- Confirm `cargo xtask check-deviations` exits 0.
- Record any remaining packet-local risk explicitly before moving to `status: implemented` (the largest residual risk is the D-104f deferral's "deferred to follow-up workstream" framing; flagged in `design.md` §Risks).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.
