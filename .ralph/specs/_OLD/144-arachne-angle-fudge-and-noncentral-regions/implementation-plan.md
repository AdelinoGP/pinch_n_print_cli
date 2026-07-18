# Implementation Plan: 144-arachne-angle-fudge-and-noncentral-regions

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (`none` — provenanced by the audit + red tests at `b2ea52b7`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Delete π hack + 0.1× filter-dist fudge + thread configured `wall_transition_angle`

- Task IDs:
  - `none` (N5 — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N5)
- Objective: Delete the π-cap workaround (`pipeline.rs:325-334`) and the 0.1× filter-dist fudge (`pipeline.rs:272-277`), and thread the configured `wall_transition_angle` (already on the `BeadingStrategy` trait at `beading/mod.rs:93`, threaded via `BeadingFactoryParams` at `factory.rs:92,157,192`) through `filter_central` at `pipeline.rs:335-339`. This is the N5 fix — the π hack is load-bearing for A1's centrality-gated scheme until A1/A2 land; C removes it strictly after A2.
- Precondition: A2 (`142`) is `status: implemented` — the canonical junction scheme is in place; the π hack is no longer load-bearing.
- Postcondition: AC-N1 passes — `rg -q 'std::f64::consts::PI' crates/slicer-core/src/arachne/pipeline.rs` returns no match (exit 1), and `rg -q '\* 0\.1' crates/slicer-core/src/arachne/pipeline.rs` returns no match in `to_centrality_params`. AC-1 stays green (N1 red tests pass with the configured angle). N2, N3, N4 red tests stay GREEN. `centrality_*.json` fixtures NOT yet re-baselined (Step 2 owns the re-baseline + the new `filter_noncentral_regions` that drifts them).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/pipeline.rs` — lines `:260-340` (`to_centrality_params` + the π hack + `filter_central` call); do NOT read `:384-390` (A2's deleted `assign_perimeter_indices`) or `:340-360` (B's stage wiring, already done).
  - `crates/slicer-core/src/beading/mod.rs` — full (108 lines); `wall_transition_angle()` at `:93` (read-only — C does NOT edit `beading/`).
  - `crates/slicer-core/src/beading/factory.rs` — lines `:90-100, 150-200`; `BeadingFactoryParams::wall_transition_angle` + `create_stack` threading (read-only).
  - `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — full (202 lines); AC-1 oracle.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/pipeline.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (Step 2's scope — `filter_noncentral_regions`)
  - `crates/slicer-core/src/beading/*` (B's scope; C reads `mod.rs`/`factory.rs` read-only but does NOT edit)
  - `crates/slicer-core/tests/arachne_filter_noncentral_regions.rs` (Step 2's NEW test)
  - `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` (Step 2's re-baseline)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:716-730` dead `filterCentral` — ask for the self-contradictory condition explicitly (to confirm PNP's `centrality.rs:263-389` helpers correctly mirror dead code, NOT to wire them); return ≤ 200 words" — purpose: confirm the gotcha (C must NOT wire the dissolve).
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-1 (N1 stays green with configured angle).
  - "Run `rg -q 'std::f64::consts::PI' crates/slicer-core/src/arachne/pipeline.rs; test $? -eq 1`; return FACT pass (exit 1 = no match)" — purpose: validate AC-N1 (π hack gone).
  - "Run `rg -q '\* 0\.1' crates/slicer-core/src/arachne/pipeline.rs; test $? -eq 1`; return FACT pass (exit 1)" — purpose: validate AC-N1 (0.1× fudge gone).
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — N2/N4/N3 stay green)" — purpose: gate C didn't regress A2/B.
  - "Find all callers of `filter_central`; return LOCATIONS" — purpose: confirm the angle-threading call-site update is complete.
- Context cost: `M`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` §"Arachne beading strategy stack" (lines ~479-521) — `wall_transition_angle` default 10.0°.
  - `docs/DEVIATION_LOG.md` `D-141-JUNCTION-BANDS` entry — addendum target (Step 2 writes the addendum; Step 1 confirms the target exists).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.h:78` — delegate (canonical `getTransitioningAngle` default 60°).
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategyFactory.hpp:49` — delegate (factory default π/4).
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:716-730` — delegate (dead `filterCentral` — confirm the gotcha).
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-c-step1-ac1.log` — FACT pass (AC-1).
  - `rg -q 'std::f64::consts::PI' crates/slicer-core/src/arachne/pipeline.rs; test $? -eq 1` — FACT pass (AC-N1, π gone).
  - `rg -q '\* 0\.1' crates/slicer-core/src/arachne/pipeline.rs; test $? -eq 1` — FACT pass (AC-N1, 0.1× gone).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-c-step1-stays-green.log` — FACT pass (N2/N4/N3 stay green).
  - `cargo check -p slicer-core --all-targets` — FACT pass.
- Exit condition: AC-1 stays green; AC-N1 passes (π + 0.1× gone); N2/N4/N3 stay green; `cargo check -p slicer-core --all-targets` passes. `centrality_*.json` fixtures may be transiently red (Step 2 re-baselines them with `filter_noncentral_regions`).

### Step 2: Port `filter_noncentral_regions` + dumbbell test + fixture re-baseline + deviation log

- Task IDs:
  - `none` (N6 — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N6)
- Objective: Port `filterNoncentralRegions` (`SkeletalTrapezoidation.cpp:811-862`) as `filter_noncentral_regions` in `centrality.rs` (promote non-central gaps between same/±1-bead-count central regions within 0.4 mm = 4000 units back to central; copy bead counts across); call it unconditionally after `assign_bead_counts` in `pipeline.rs`. Write the dumbbell test (`arachne_filter_noncentral_regions.rs`, NEW). Re-baseline `centrality_*.json` fixtures. Add the `D-144-ANGLE-FUDGE-NONCENTRAL` deviation-log entry + `D-141-JUNCTION-BANDS` addendum.
- Precondition: Step 1 is green (π hack + 0.1× fudge gone; configured angle threaded; N1 stays green).
- Postcondition: AC-2 (dumbbell single central region) passes. N1, N2, N3, N4 stay GREEN. `centrality` regression green (fixtures re-baselined). `D-144-ANGLE-FUDGE-NONCENTRAL` present.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — lines `:100-200` (`filter_central` + `updateIsCentral` predicate — the convention C's new function mirrors) and `:260-390` (the un-wired whisker-dissolve helpers — read-only confirmation they mirror dead code, do NOT wire).
  - `crates/slicer-core/src/arachne/pipeline.rs` — lines `:340-345` (the `assign_bead_counts` call site — insert `filter_noncentral_regions` after).
  - `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — full (202 lines); the `run_arachne_pipeline` + `inset0_lines` helper pattern the dumbbell test mirrors.
  - `docs/08_coordinate_system.md` §"Constant Conversion Table" (~30 lines) — 0.4 mm = 4000 units.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs`
  - `crates/slicer-core/tests/arachne_filter_noncentral_regions.rs` (NEW)
  - `docs/DEVIATION_LOG.md` (addendum only — new `D-144-ANGLE-FUDGE-NONCENTRAL` + one-line addendum on `D-141-JUNCTION-BANDS`; no in-place edits)
- (Secondary edit not counted against the ≤ 3: `crates/slicer-core/src/arachne/pipeline.rs:343` for the `filter_noncentral_regions` call insertion — a one-line surgical insert, not a primary edit surface.)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/pipeline.rs:325-339` (Step 1's scope — already done)
  - `crates/slicer-core/src/beading/*` (B's scope)
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs:263-389` (the un-wired whisker-dissolve helpers — read-only, do NOT wire)
  - `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` (re-record via self-capture; never read directly)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:811-862` `filterNoncentralRegions` — explicitly ask for the promote-back condition (same/±1-bead-count within 0.4 mm) + the bead-count copy rule; return ≤ 200 words, no code unless asked" — purpose: confirm Step 2's port.
  - "SUMMARY of `SkeletalTrapezoidation.cpp:633` call site — confirm `filterNoncentralRegions` is called unconditionally after `updateBeadCount`; return FACT (≤ 5 lines)" — purpose: confirm call-site ordering.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_filter_noncentral_regions --nocapture`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-2 (dumbbell).
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT pass (expected — N1 stays green)" — purpose: gate Step 2 didn't regress Step 1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — N2/N4/N3 stay green)" — purpose: gate scope.
  - "Run `cargo test -p slicer-core --features host-algos --test centrality 2>&1`; return FACT pass/fail (fixtures re-baselined)" — purpose: regression gate.
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/c-cube4color.gcode && cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture`; return FACT + the `failures.len()/total_checked` summary line — purpose: record the e2e closure delta (record-only per cross-cutting policy; C does NOT block on green)" — purpose: record delta for commit message.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` §"Constant Conversion Table" (~30 lines) — 0.4 mm = 4000 units.
  - `docs/DEVIATION_LOG.md` `D-141-JUNCTION-BANDS` entry — addendum target.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:811-862` — delegate (`filterNoncentralRegions`).
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:633` — delegate (call site).
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_filter_noncentral_regions --nocapture 2>&1 | tee target/test-output-c-step2-ac2.log` — FACT pass (AC-2).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-c-step2-n1-green.log` — FACT pass (N1 stays green).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-c-step2-stays-green.log` — FACT pass (N2/N4/N3 stay green).
  - `cargo test -p slicer-core --features host-algos --test centrality 2>&1 | tee target/test-output-c-step2-regression.log` — FACT pass (fixtures re-baselined).
  - `rg -q 'D-144-ANGLE-FUDGE-NONCENTRAL' docs/DEVIATION_LOG.md` — FACT pass.
- Exit condition: AC-2 passes; N1/N2/N3/N4 stay green; `centrality` regression green (fixtures re-baselined); `D-144-ANGLE-FUDGE-NONCENTRAL` present; `cargo check -p slicer-core --all-targets` + `cargo clippy -p slicer-core --all-targets -- -D warnings` pass; e2e closure delta recorded (record-only).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (N5: delete π hack + 0.1× fudge + thread angle) | M | Heaviest dispatch: dead `filterCentral` SUMMARY (gotcha confirmation). |
| Step 2 (N6: `filter_noncentral_regions` + dumbbell test + fixtures + deviation log) | M | Heaviest dispatch: `filterNoncentralRegions` SUMMARY + centrality regression. |

Aggregate: M + M = M (Step 2 shares Step 1's `pipeline.rs`/`centrality.rs` context). If the sum exceeds M aggregate in practice, hand off after Step 1.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1, AC-2, AC-N1 dispatched and returned PASS).
- N1, N2, N3, N4 stay GREEN (scope boundary gates).
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask build-guests --check` returns clean (C's surface is `slicer-core`-internal).
- `D-144-ANGLE-FUDGE-NONCENTRAL` present in `docs/DEVIATION_LOG.md` with addendum on `D-141-JUNCTION-BANDS`.
- Affected `centrality_*.json` fixtures re-baselined with rationale in commit messages.
- e2e closure delta recorded (record-only — Packet F blocks on green).
- `docs/07_implementation_status.md` updated (via worker dispatch).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1, AC-2, AC-N1).
- Confirm packet-level verification commands are green.
- Confirm N1/N2/N3/N4 "stays green" commands returned as expected.
- Record the e2e closure delta explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.