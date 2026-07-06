# Implementation Plan: 146-arachne-postprocess-order-and-remove-small-simplify

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (`none` — provenanced by the audit + red tests at `b2ea52b7`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`.

## Steps

### Step 1: Post-process order swap + `separate_out_inner_contour` + `remove_empty_toolpaths` (N11)

- Task IDs:
  - `none` (N11 — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N11)
- Objective: Reorder `pipeline.rs:360-375` from `stitch → simplify → remove_small` to `stitch → remove_small → separate_out_inner_contour → simplify → remove_empty`. Add `separate_out_inner_contour` (NEW — inner-surface bookkeeping for infill boundary; delegate the exact responsibility) and `remove_empty_toolpaths` (filter out empty `ExtrusionLine`s after simplify).
- Precondition: D (`145`) is `status: implemented` — E's `removeSmallLines` interacts with D's `is_odd = true` micro-loops; D's canonical `is_odd` semantics must land first.
- Postcondition: AC-1 passes (canonical post-process order). N1, N2, N3, N4 stay GREEN. N12 per-line `min_width` + N13 distance gates NOT yet in place (Steps 2 + 3 own them).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/pipeline.rs` — lines `:360-375` (the post-processing pipeline); do NOT read `:260-340` (A1/A2/B/C's scope).
  - `crates/slicer-core/src/arachne/stitch.rs` — read-only (the stitch stage is unchanged; E only reorders it).
  - `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — full (202 lines); AC-N1 oracle pattern.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/pipeline.rs`
  - `crates/slicer-core/src/arachne/separate_inner_contour.rs` (NEW, or inline in `pipeline.rs` if minimal)
  - `crates/slicer-core/tests/arachne_postprocess_order.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/remove_small.rs` (Step 2's scope)
  - `crates/slicer-core/src/arachne/simplify.rs` (Step 3's scope)
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` (A1/A2/D's scope)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `WallToolPaths.cpp:679-699` canonical post-process order — ask for the exact stage sequence + the `separateOutInnerContour` responsibility; return ≤ 200 words" — purpose: confirm Step 1's order swap + `separate_out_inner_contour`.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_postprocess_order --nocapture`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT pass (expected — AC-N1, N1 stays green)" — purpose: gate E didn't regress A1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — N2/N4/N3 stay green)" — purpose: gate scope.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — `min_length_factor` (0.5).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:679-699` — delegate.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_postprocess_order --nocapture 2>&1 | tee target/test-output-e-step1-ac1.log` — FACT pass (AC-1).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-e-step1-neg1.log` — FACT pass (AC-N1).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-e-step1-stays-green.log` — FACT pass.
  - `cargo check -p slicer-core --all-targets` — FACT pass.
- Exit condition: AC-1 passes; AC-N1 passes; N2/N4/N3 stay green; `cargo check -p slicer-core --all-targets` passes.

### Step 2: Per-line `min_width` in `remove_small_lines` (N12)

- Task IDs:
  - `none` (N12 — provenanced by §N12)
- Objective: Rewrite `remove_small.rs:40-50` to compute `min_width` per line (minimum junction width over the line) + layer-type divisor (`min_width/2` on top/bottom layers via `is_initial_layer`, `min_width * min_length_factor` otherwise), matching `WallToolPaths.cpp:838-856`.
- Precondition: Step 1 is green (canonical post-process order in place; `remove_small` now runs before `simplify`).
- Postcondition: AC-2 passes (per-line `min_width`). N1, N2, N3, N4 stay GREEN.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/remove_small.rs` — full (~57 LOC per the audit; small file).
  - `crates/slicer-core/src/arachne/pipeline.rs` — range-read the `remove_small_lines` call site (now in the canonical position after `stitch`); confirm `is_initial_layer` is available.
  - `crates/slicer-core/tests/arachne_parity_red_is_odd_semantics.rs` — full (194 lines); the `remove_small_lines` call pattern + `is_odd` semantics (N4 fix) E must preserve.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/remove_small.rs`
  - `crates/slicer-core/tests/arachne_remove_small_per_line_min_width.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/simplify.rs` (Step 3's scope)
  - `crates/slicer-core/src/arachne/pipeline.rs:360-375` (Step 1's scope — already done)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `WallToolPaths.cpp:838-856` `removeSmallLines` — ask for the per-line `min_width` computation + the layer-type divisor (`min_width/2` top/bottom, `min_width * min_length_factor` otherwise); return ≤ 200 words" — purpose: confirm Step 2's per-line `min_width`.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_remove_small_per_line_min_width --nocapture`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-2.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT pass (expected — N4 stays green, real walls not mis-removed)" — purpose: gate E didn't regress A2's `is_odd` fix.
  - "Run `cargo test -p slicer-core --features host-algos --test remove_small 2>&1`; return FACT pass/fail (fixtures re-baselined)" — purpose: regression gate.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — `min_length_factor` (0.5).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:838-856` — delegate.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_remove_small_per_line_min_width --nocapture 2>&1 | tee target/test-output-e-step2-ac2.log` — FACT pass (AC-2).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast 2>&1 | tee target/test-output-e-step2-n4-green.log` — FACT pass (N4 stays green).
  - `cargo test -p slicer-core --features host-algos --test remove_small 2>&1 | tee target/test-output-e-step2-regression.log` — FACT pass (fixtures re-baselined).
- Exit condition: AC-2 passes; N4 stays green; `remove_small` regression green; `cargo check -p slicer-core --all-targets` passes.

### Step 3: Simplify distance gates + fixture re-baseline + deviation log (N13)

- Task IDs:
  - `none` (N13 — provenanced by §N13)
- Objective: Replace `simplify.rs:43-121`'s iterative multi-pass area-only sweep with the canonical single linear pass gated by `smallest_line_segment_squared` / `allowed_error_distance_squared` (from `meshfix_maximum_resolution`/`_deviation`); `calculateExtrusionAreaDeviationError` becomes an extra guard on the near-colinear fast path only. Thread the distance-gate config keys (add to `ArachneParams` + the `arachne-params` WIT record if not already registered — surface the WIT change, don't silently absorb). Re-baseline `simplify_*.json`/`stitch_*.json` fixtures. Add `D-146-POSTPROCESS-ORDER` deviation-log entry + `D-112-SIMPLIFY-DP` addendum.
- Precondition: Step 2 is green (per-line `min_width` in place; `remove_small` now runs before `simplify` so the simplify input is clean).
- Postcondition: AC-3 passes (simplify distance gates). N1, N2, N3, N4 stay GREEN. `simplify`/`stitch` regression green (fixtures re-baselined). `D-146-POSTPROCESS-ORDER` present.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/simplify.rs` — range-read `:43-121` (the iterative multi-pass sweep + the area gate).
  - `docs/15_config_keys_reference.md` — confirm whether `meshfix_maximum_resolution`/`_deviation` are registered.
  - `crates/slicer-schema/wit/` — the `arachne-params` WIT record (confirm whether new fields are needed).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/simplify.rs`
  - `crates/slicer-core/src/arachne/pipeline.rs` (for `ArachneParams` fields if the distance-gate config keys are not registered)
  - `docs/DEVIATION_LOG.md` (addendum only — new `D-146-POSTPROCESS-ORDER` + one-line addendum on `D-112-SIMPLIFY-DP`; no in-place edits)
- (Secondary edits not counted against the ≤ 3: `crates/slicer-core/tests/arachne_simplify_distance_gates.rs` (NEW — AC-3), `crates/slicer-core/tests/fixtures/arachne/simplify_*.json`/`stitch_*.json` (re-baseline), `crates/slicer-schema/wit/` + `slicer-sdk`/`slicer-wasm-host` if WIT record fields are added.)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/remove_small.rs` (Step 2's scope — already done)
  - `crates/slicer-core/src/arachne/pipeline.rs:360-375` (Step 1's scope — already done)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `ExtrusionLine.cpp:56-243` `simplifyToolpaths` — ask for the distance-gate thresholds (`smallest_line_segment_squared` / `allowed_error_distance_squared`) + the near-colinear fast-path guard (`calculateExtrusionAreaDeviationError`); return ≤ 200 words" — purpose: confirm Step 3's distance gates.
  - "SUMMARY of `WallToolPaths.cpp:868-872` — ask for the `meshfix_maximum_resolution`/`_deviation` sourcing; return ≤ 200 words" — purpose: confirm the distance-gate config keys.
  - "Run `rg -q 'meshfix_maximum_resolution' docs/15_config_keys_reference.md`; return FACT pass/fail" — purpose: confirm whether the distance-gate config keys are already registered.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_simplify_distance_gates --nocapture`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-3.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT pass (expected — N1 stays green)" — purpose: gate E didn't regress A1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — N2/N4/N3 stay green)" — purpose: gate scope.
  - "Run `cargo test -p slicer-core --features host-algos --test simplify --test stitch 2>&1`; return FACT pass/fail (fixtures re-baselined)" — purpose: regression gate.
  - "Run `cargo xtask build-guests --check`; return FACT clean / STALE list" — purpose: guest WASM coherence (if E added WIT record fields).
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/e-cube4color.gcode && cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture`; return FACT + summary line — purpose: record e2e closure delta (record-only)."
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — `meshfix_maximum_resolution`/`_deviation` (confirm registration).
  - `docs/DEVIATION_LOG.md` `D-112-SIMPLIFY-DP` entry — addendum target.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/ExtrusionLine.cpp:56-243` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:868-872` — delegate.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_simplify_distance_gates --nocapture 2>&1 | tee target/test-output-e-step3-ac3.log` — FACT pass (AC-3).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-e-step3-n1-green.log` — FACT pass (N1 stays green).
  - `cargo test -p slicer-core --features host-algos --test simplify --test stitch 2>&1 | tee target/test-output-e-step3-regression.log` — FACT pass (fixtures re-baselined).
  - `rg -q 'D-146-POSTPROCESS-ORDER' docs/DEVIATION_LOG.md` — FACT pass.
  - `cargo xtask build-guests --check` — FACT clean (if WIT record fields added).
- Exit condition: AC-3 passes; N1/N2/N3/N4 stay green; `simplify`/`stitch` regression green; `D-146-POSTPROCESS-ORDER` present; `cargo check -p slicer-core --all-targets` + `cargo clippy -p slicer-core --all-targets -- -D warnings` pass; `cargo xtask build-guests --check` clean (if WIT changed); e2e closure delta recorded (record-only).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (N11 order swap + separate_out_inner_contour) | S | Heaviest dispatch: `WallToolPaths.cpp:679-699` SUMMARY. |
| Step 2 (N12 per-line min_width) | S | Heaviest dispatch: `removeSmallLines` SUMMARY. |
| Step 3 (N13 simplify distance gates + fixtures + deviation log) | S | Heaviest dispatch: `simplifyToolpaths` SUMMARY + WIT check. |

Aggregate: S + S + S = S (the three steps share the `arachne/` post-processing context). If the sum exceeds S aggregate in practice, hand off after Step 2.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1, AC-2, AC-3, AC-N1 dispatched and returned PASS).
- N1, N2, N3, N4 stay GREEN.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask build-guests --check` returns clean (run unconditionally; mandatory if E added WIT record fields).
- `D-146-POSTPROCESS-ORDER` present in `docs/DEVIATION_LOG.md` with addendum on `D-112-SIMPLIFY-DP`.
- Affected `simplify_*.json`/`stitch_*.json`/`remove_small_*.json` fixtures re-baselined with rationale in commit messages.
- e2e closure delta recorded (record-only — Packet F blocks on green).
- If E added WIT record fields, the WIT change is surfaced in the commit message (not silently absorbed) + threaded through `slicer-sdk`/`slicer-wasm-host`.
- `docs/07_implementation_status.md` updated (via worker dispatch).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1, AC-2, AC-3, AC-N1).
- Confirm packet-level verification commands are green.
- Confirm N1/N2/N3/N4 "stays green" commands returned as expected.
- Record the e2e closure delta explicitly before moving to `status: implemented`.
- If E added WIT record fields, confirm `cargo xtask build-guests --check` is clean and the WIT change is surfaced.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.