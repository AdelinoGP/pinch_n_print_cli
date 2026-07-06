# Implementation Plan: 116b-arachne-canonical-connectjunctions-emission

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (`none` — provenanced by the audit + red tests at `b2ea52b7`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Canonical `connectJunctions` per-quad emission + `perimeter_index = bead_idx`

- Task IDs:
  - `none` (N2 — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N2)
- Objective: Rewrite `chain_junctions_for_bead`/`emit_chain_lines`/`generate_toolpaths` (`generate_toolpaths.rs:401-758`) to the canonical per-quad `connectJunctions` scheme — from/to pairing, `perimeter_index` pop-back merge, `addToolpathSegment` line growth, `new_domain_start` flag. Set `perimeter_index = junction_idx` (bead/inset index) at junction generation (`:315,326`). Delete `assign_perimeter_indices` (`pipeline.rs:384-390`) + its call site (`:373`). Update `arachne_pipeline.rs:122` in place to assert `perimeter_index == line.inset_idx`.
- Precondition: A1 (`116a`) is `status: implemented` — A1's upward-half-edge junction fans exist and the N1 red tests are green.
- Postcondition: AC-1 (N2 red test) passes — every junction carries `perimeter_index == line.inset_idx`. AC-N1 (`arachne_pipeline.rs:122` updated) passes. N1 red tests stay GREEN. N3, N4 red tests stay RED (Step 2 owns N4).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — full-read for this step (primary edit target; A1 already full-read it, but A2's implementer may re-read).
  - `crates/slicer-core/src/arachne/pipeline.rs` — lines `:360-390` (the `assign_perimeter_indices` deletion + call site).
  - `crates/slicer-core/tests/arachne_pipeline.rs` — lines `:120-150` (the in-place update target).
  - `crates/slicer-core/tests/arachne_parity_red_perimeter_index.rs` — full (157 lines); AC-1 oracle.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs`
  - `crates/slicer-core/src/arachne/pipeline.rs`
  - `crates/slicer-core/tests/arachne_pipeline.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs:632` (`is_odd` — Step 2's scope)
  - `crates/slicer-core/src/arachne/pipeline.rs:334` and `:272-277` (Packet C's π hack / fudge)
  - `crates/slicer-sdk/src/host.rs:717` and `crates/slicer-wasm-host/src/host.rs:1814` (wire-type-transparent; NOT edited)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:2283-2327` `connectJunctions` — explicitly ask for the per-quad from/to pairing + `perimeter_index` pop-back merge; return ≤ 200 words, no code unless asked" — purpose: confirm emission rewrite.
  - "SUMMARY of `SkeletalTrapezoidation.cpp:2198-2234` `addToolpathSegment` — extend-vs-new-line (10 µm tolerance, same width, `new_domain_start`); return ≤ 200 words" — purpose: confirm line-growth.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index -- n2_junction_perimeter_index_is_bead_index --nocapture`; return FACT (pass) or SNIPPETS (fail + ≤ 20 lines)" — purpose: validate AC-1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_perimeter_index_is_sequential_per_line --nocapture`; return FACT pass/fail" — purpose: validate AC-N1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT pass (expected — N1 stays green)" — purpose: gate A2 didn't regress A1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT fail (expected — N4 stays red, Step 2 owns it)" — purpose: gate scope.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT fail (expected — N3 stays red)" — purpose: gate scope.
  - "Find all callers of `assign_perimeter_indices`; return LOCATIONS" — purpose: confirm no orphan call sites.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"Arachne extrusion-line geometry" (lines ~1091-1150) — `ExtrusionJunction`/`ExtrusionLine` field shapes.
  - `docs/DEVIATION_LOG.md` `D-116A-JUNCTION-BANDS` entry — addendum target.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2283-2327` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2198-2234` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2064-2077` — delegate (`perimeter_index = junction_idx`).
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index -- n2_junction_perimeter_index_is_bead_index --nocapture 2>&1 | tee target/test-output-a2-step1-ac1.log` — FACT pass.
  - `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_perimeter_index_is_sequential_per_line --nocapture 2>&1 | tee target/test-output-a2-step1-neg1.log` — FACT pass.
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-a2-step1-n1-green.log` — FACT pass (N1 stays green).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-a2-step1-stays-red.log` — FACT fail (expected — N4/N3 stay red).
- Exit condition: AC-1 + AC-N1 pass; N1 stays green; N4/N3 stay red; `cargo check -p slicer-core --all-targets` passes.

### Step 2: Canonical `is_odd` per-segment + `passed_odd_edges` + fixture re-baseline + deviation log

- Task IDs:
  - `none` (N4 — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N4)
- Objective: Replace `is_odd: bead_idx % 2 == 1` (`generate_toolpaths.rs:632`) with the canonical per-segment rule (`bead_count % 2 == 1`, `transition_ratio == 0`, innermost junction, endpoint proximity 0.005 mm to peak node). Rework `passed_odd_edges` to key on the physical edge. Re-baseline affected fixtures (`toolpaths_tapered_wedge.json`, `stitch_*.json` if they exist). Add the `D-116B-CONNECTJUNCTIONS-EMISSION` deviation-log entry + `D-116A-JUNCTION-BANDS` addendum.
- Precondition: Step 1 is green (canonical `connectJunctions` emission + `perimeter_index = bead_idx` land first; `is_odd` is computed per segment during the `connectJunctions` walk, so it depends on Step 1's quad structure).
- Postcondition: AC-2 (even bead count → no `is_odd`) and AC-3 (inset-1 survives `remove_small_lines`) pass. N1, N2 stay GREEN. N3 stays RED. Affected fixtures re-baselined. `D-116B-CONNECTJUNCTIONS-EMISSION` present.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — lines `:620-640` (the `is_odd` site) + the `passed_odd_edges` site (range-read; A2's Step 1 already touched this file).
  - `crates/slicer-core/tests/arachne_parity_red_is_odd_semantics.rs` — full (194 lines); AC-2 + AC-3 oracle + `FixedBeadingStrategy`/`two_bead_single_edge_graph` fixture.
  - `crates/slicer-core/src/arachne/stitch.rs` — line `:83` (the `is_odd` grouping key — read-only confirmation).
  - `crates/slicer-core/src/arachne/remove_small.rs` — line `:57` (the `is_odd && !is_closed` gate — read-only confirmation).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs`
  - `docs/DEVIATION_LOG.md` (addendum only — new `D-116B-CONNECTJUNCTIONS-EMISSION` + one-line addendum on `D-116A-JUNCTION-BANDS`; no in-place edits)
  - `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` (re-record via self-capture; never read directly)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/stitch.rs` and `remove_small.rs` (A2 changes the `is_odd` *producer*, not the consumers — read-only confirmations only)
  - `crates/slicer-core/src/arachne/pipeline.rs:334` and `:272-277` (Packet C)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:2344-2354` canonical `is_odd` — ask for the four conditions (`bead_count % 2 == 1`, `transition_ratio == 0`, innermost, endpoint proximity 0.005 mm) and the `passed_odd_edges` physical-edge key (`:2355-2361`); return ≤ 200 words" — purpose: confirm `is_odd` rewrite.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-2 + AC-3.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --no-fail-fast`; return FACT pass (expected — N1/N2 stay green)" — purpose: gate A2 didn't regress A1/Step 1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT fail (expected — N3 stays red)" — purpose: gate scope.
  - "Run `cargo test -p slicer-core --features host-algos --test generate_toolpaths --test stitch --test remove_small 2>&1`; return FACT pass/fail (fixtures re-baselined)" — purpose: regression gate.
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/a2-cube4color.gcode && cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture`; return FACT + the `failures.len()/total_checked` summary line — purpose: record the e2e closure delta (record-only; A2 does NOT block on green)" — purpose: record delta for commit message.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"Arachne extrusion-line geometry" — `ExtrusionLine::is_odd` field shape.
  - `docs/DEVIATION_LOG.md` `D-116A-JUNCTION-BANDS` entry — addendum target.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2344-2354` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2355-2361` — delegate (`passed_odd_edges`).
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.hpp:62-70` — delegate (`is_odd` semantics).
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:838-856` — delegate (`removeSmallLines` gate).
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast 2>&1 | tee target/test-output-a2-step2-ac.log` — FACT pass (AC-2 + AC-3).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --no-fail-fast 2>&1 | tee target/test-output-a2-step2-stays-green.log` — FACT pass (N1/N2 stay green).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-a2-step2-n3-red.log` — FACT fail (expected — N3 stays red).
  - `cargo test -p slicer-core --features host-algos --test generate_toolpaths --test stitch --test remove_small 2>&1 | tee target/test-output-a2-step2-regression.log` — FACT pass (fixtures re-baselined).
  - `rg -q 'D-116B-CONNECTJUNCTIONS-EMISSION' docs/DEVIATION_LOG.md` — FACT pass.
- Exit condition: AC-2, AC-3 pass; N1/N2 stay green; N3 stays red; generate_toolpaths/stitch/remove_small regression green; `D-116B-CONNECTJUNCTIONS-EMISSION` present; `cargo check -p slicer-core --all-targets` and `cargo clippy -p slicer-core --all-targets -- -D warnings` pass; e2e closure delta recorded (record-only).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (N2 connectJunctions emission + perimeter_index) | M | Heaviest dispatch: `connectJunctions` SUMMARY. |
| Step 2 (N4 is_odd + passed_odd_edges + fixtures + deviation log) | M | Heaviest dispatch: `is_odd` SUMMARY + regression suite. |

Aggregate: M + M = M (Step 2 shares Step 1's `generate_toolpaths.rs` context). If the sum exceeds M aggregate in practice, hand off after Step 1.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1, AC-2, AC-3, AC-N1 dispatched and returned PASS).
- N1, N2 stay GREEN; N3 stays RED (scope boundary gates).
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask build-guests --check` returns clean.
- `D-116B-CONNECTJUNCTIONS-EMISSION` present in `docs/DEVIATION_LOG.md` with addendum on `D-116A-JUNCTION-BANDS`.
- Affected `slicer-core` fixtures re-baselined with rationale in commit messages.
- e2e closure delta recorded (record-only — Packet F blocks on green).
- `docs/07_implementation_status.md` updated (via worker dispatch).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1, AC-2, AC-3, AC-N1).
- Confirm packet-level verification commands are green.
- Confirm N1/N2 "stays green" and N3 "stays red" commands returned as expected.
- Record the e2e closure delta explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.