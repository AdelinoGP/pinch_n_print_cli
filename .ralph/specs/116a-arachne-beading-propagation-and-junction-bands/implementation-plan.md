# Implementation Plan: 116a-arachne-beading-propagation-and-junction-bands

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (`none` — provenanced by the audit + red tests at `b2ea52b7`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: `BeadingPropagation` side table + `get_beding` + propagation gating

- Task IDs:
  - `none` (N7 — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N7)
- Objective: Add the `BeadingPropagation` side table (full `Beading` per node) to `SkeletalTrapezoidationGraph`, port `getBeading`/`getNearestBeading` (0.1 mm radius in slicer units = 1000 units), drop the centrality gate from `upward_central_edges`/`primary_source_vertices`, and replace `interpolate_bead_counts`'s rounded-integer blend with a width/location blend into the side table. Land N7 first so N1's `generate_junctions` rewrite (Step 2) has a `get_beding` to call.
- Precondition: 113c's graph construction is `status: implemented`; the red suite at `b2ea52b7` is committed and FAIL.
- Postcondition: a new structural test (`arachne_beding_propagation_side_table`) passes — the side table is populated for rib-foot nodes (no `bead_count`), `get_beding` returns the correct `Beading` for known vertices, and the `Beading` invariant (`bead_widths.len() == toolpath_locations.len()`) holds on every entry. N1 red tests still FAIL (Step 2 owns the junction rewrite).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — lines `:120-160` (`upward_central_edges`), `:810-860` (`primary_source_vertices`, `interpolate_bead_counts`), `:980-1100` (`propagate_beadings_downward`); do NOT read `:640-740` (Packet B's `apply_transitions` scope).
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — `SkeletalTrapezoidationGraph` struct def and `from_polygons` (range-read; the file is ~700 lines per 113c).
  - `crates/slicer-core/src/beading/mod.rs` — full (108 lines); `Beading` struct shape.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`
  - `crates/slicer-core/tests/arachne_beding_propagation_side_table.rs` (NEW — structural test)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` (Step 2's scope)
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs:640-740` (Packet B's `apply_transitions`)
  - `crates/slicer-core/src/beading/{distributed,widening,redistribute,outer_wall_inset,limited,factory}.rs` (Packet B's trait extension)
  - `crates/slicer-core/src/arachne/pipeline.rs:334` and `:272-277` (Packet C's π hack / fudge)
  - `OrcaSlicerDocumented/...` (delegate via the contract)
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:2091-2127` `getBeading`/`getNearestBeading` — ask for the 0.1 mm radius constant and the nearest-lookup algorithm; return ≤ 200 words, no code unless asked" — purpose: confirm `get_beding`'s lookup shape.
  - "SUMMARY of `SkeletalTrapezoidation.cpp:1833-1899` `propagateBeadingsDownward` — ask for the `ratio_of_top` blend over bead widths/locations (not integer counts) and the central-edge skip; return ≤ 200 words" — purpose: confirm the interpolation fix.
  - "SUMMARY of `SkeletalTrapezoidation.cpp:1669-1672` `upward_quad_mids` — confirm no centrality filter; return FACT (≤ 5 lines)" — purpose: confirm dropping the centrality gate.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_beding_propagation_side_table --nocapture`; return FACT (pass) or SNIPPETS (fail with assertion + ≤ 20 lines)" — purpose: validate Step 1's structural test.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT fail (expected — confirms N1 still red, Step 2 owns the fix)" — purpose: gate Step 1's scope boundary.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` §"Constant Conversion Table" (~30 lines) — 0.1 mm = 1000 units conversion.
  - `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — substrate.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2091-2127` — delegate; never load.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1833-1899` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1669-1672` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1883-1885` — delegate (`ratio_of_top`).
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_beding_propagation_side_table --nocapture 2>&1 | tee target/test-output-a1-step1.log` — FACT pass/fail.
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-a1-step1-n1-still-red.log` — FACT fail (expected).
- Exit condition: structural test passes; N1 red tests still FAIL (Step 2 owns them); `cargo check -p slicer-core --all-targets` passes.

### Step 2: Canonical `generate_junctions` rewrite + fixture re-baseline + deviation log

- Task IDs:
  - `none` (N1 — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N1)
- Objective: Rewrite `generate_junctions` (`generate_toolpaths.rs:192-334`) to the canonical scheme — iterate ALL edges (no centrality gate, ribs included), skip non-upward half-edges (`from.R > to.R`), skip flat/same-bead-count edges, compute ONE beading at the peak node via `get_beding`, emit ONLY in-band beads (middle-index start, break on `bead_R < end_R`), no clamping, near-`start_R` snap. Add the AC-N1 structural test. Re-baseline the affected `slicer-core` fixtures. Add the `D-116A-JUNCTION-BANDS` deviation-log entry + `D-113C` addendum.
- Precondition: Step 1 is green (the `BeadingPropagation` side table + `get_beding` exist and the structural test passes).
- Postcondition: AC-1 and AC-2 (the N1 red tests) pass **without weakened assertions**. AC-N1 (upward-half-edge-only) passes. N2, N3, N4 red tests stay RED (gated by the "stays red" commands). Affected `slicer-core` fixtures re-baselined. `D-116A-JUNCTION-BANDS` present in `docs/DEVIATION_LOG.md`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — lines `:192-334` (`generate_junctions`); full-read is acceptable for this step only (the file is ~953 lines but this is the primary edit target).
  - `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — full (202 lines); A1's oracle.
  - `crates/slicer-core/tests/arachne_parity_red_transition_ends.rs` — full (217 lines); AC-N1's fixture shape (single central twin-pair edge).
  - `crates/slicer-core/tests/{centrality,bead_count,propagation,generate_toolpaths}.rs` — fixture-record sites; range-read the self-capture logic only (the test files are large; do not full-read).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs`
  - `crates/slicer-core/tests/arachne_junction_upward_half_edge_only.rs` (NEW — AC-N1 structural test)
  - `docs/DEVIATION_LOG.md` (addendum only — new `D-116A-JUNCTION-BANDS` entry + one-line addendum on `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`; no in-place edits to 113c's narrative)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/arachne/pipeline.rs:334` and `:272-277` (Packet C's π hack / fudge)
  - `crates/slicer-core/src/arachne/pipeline.rs:384-390` (`assign_perimeter_indices` — Packet A2 deletes it; A1 leaves it)
  - `crates/slicer-core/tests/arachne_pipeline.rs:122` (Packet A2 updates it in-place; A1 leaves it red)
  - `crates/slicer-core/tests/fixtures/arachne/*.json` (re-record via self-capture; never read directly)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:2013-2079` `generateJunctions` — explicitly ask for the upward-skip / in-band-break / middle-index-start loop structure, NOT just a callee summary; return ≤ 200 words, no code unless asked" — purpose: confirm the rewrite shape.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands -- n1_rectangle_outer_wall_junctions_stay_near_boundary --nocapture`; return FACT (pass) or SNIPPETS (fail with assertion + ≤ 20 lines)" — purpose: validate AC-1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands -- n1_square_outer_wall_junctions_at_outer_bead_radius --nocapture`; return FACT (pass) or SNIPPETS (fail)" — purpose: validate AC-2.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_junction_upward_half_edge_only --nocapture`; return FACT (pass) or SNIPPETS (fail)" — purpose: validate AC-N1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --no-fail-fast`; return FACT fail (expected — confirms N2 stayed red)" — purpose: gate scope.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT fail (expected — confirms N4 stayed red)" — purpose: gate scope.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT fail (expected — confirms N3 stayed red)" — purpose: gate scope.
  - "Run `cargo test -p slicer-core --features host-algos --test centrality --test bead_count --test propagation --test generate_toolpaths 2>&1`; return FACT pass/fail (fixtures re-baselined) or SNIPPETS on failure" — purpose: regression gate.
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/a1-cube4color.gcode && cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture`; return FACT + the `failures.len()/total_checked` summary line — purpose: record the e2e closure delta (record-only per cross-cutting policy; A1 does NOT block on green)" — purpose: record delta for commit message.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"Arachne extrusion-line geometry" (lines ~1091-1150) — `ExtrusionJunction`/`ExtrusionLine` field shapes.
  - `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — addendum target.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2013-2079` — delegate; never load.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2024-2027` — delegate (flat/same-bead-count skip).
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2064-2077` — delegate (in-band bead loop + near-`start_R` snap).
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-a1-step2-ac.log` — FACT pass (AC-1 + AC-2).
  - `cargo test -p slicer-core --features host-algos --test arachne_junction_upward_half_edge_only --nocapture 2>&1 | tee target/test-output-a1-step2-neg.log` — FACT pass (AC-N1).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-a1-step2-stays-red.log` — FACT fail (expected — N2/N3/N4 stay red).
  - `cargo test -p slicer-core --features host-algos --test centrality --test bead_count --test propagation --test generate_toolpaths 2>&1 | tee target/test-output-a1-step2-regression.log` — FACT pass (fixtures re-baselined).
  - `rg -q 'D-116A-JUNCTION-BANDS' docs/DEVIATION_LOG.md` — FACT pass.
- Exit condition: AC-1, AC-2, AC-N1 pass; N2/N3/N4 stay red; centrality/bead_count/propagation/generate_toolpaths regression green (fixtures re-baselined); `D-116A-JUNCTION-BANDS` present in deviation log; `cargo check -p slicer-core --all-targets` and `cargo clippy -p slicer-core --all-targets -- -D warnings` pass; e2e closure delta recorded in commit message (record-only).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (N7 side table + propagation gating) | M | Heaviest dispatch: `getBeading`/`propagateBeadingsDownward` SUMMARYs. |
| Step 2 (N1 junction rewrite + fixtures + deviation log) | M | Heaviest dispatch: `generateJunctions` SUMMARY + the 7 test runs. |

Aggregate: M + M = M (the two steps share the `Beading`/propagation context, so the second step's marginal cost is below a fresh M). If the sum exceeds M aggregate in practice, the implementer should hand off after Step 1 and resume Step 2 on a fresh context.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1, AC-2, AC-N1 dispatched and returned PASS).
- N2, N3, N4 red tests confirmed still RED (scope boundary gate).
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask build-guests --check` returns clean (A1's surface is `slicer-core`-internal, but the gate must run before any guest-test failure is blamed on A1).
- `D-116A-JUNCTION-BANDS` present in `docs/DEVIATION_LOG.md` with addendum on `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`.
- Affected `slicer-core` fixtures re-baselined with rationale in commit messages.
- e2e closure delta recorded in commit message (record-only — Packet F blocks on green).
- `docs/07_implementation_status.md` updated for the packet (via worker dispatch — never edited by loading the full backlog into the implementer's context).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1, AC-2, AC-N1).
- Confirm packet-level verification commands are green (`cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, the N1 red-test gate).
- Confirm N2/N3/N4 "stays red" commands returned FACT fail (expected).
- Record the e2e closure delta explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.