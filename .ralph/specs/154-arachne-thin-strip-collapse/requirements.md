# Requirements: 154-arachne-thin-strip-collapse

## Packet Metadata

- Grouped task IDs: **none** — this investigation is tracked by `docs/DEVIATION_LOG.md` `D-105D`
  (line 27), not a `docs/07_implementation_status.md` `TASK-###`.
- Backlog source: `docs/DEVIATION_LOG.md` `D-105D` (line 27); the `/diagnose` session
  2026-07-13 that reverted the fabricated spine subdivision; `docs/18_arachne_parity_audit.md`
  §G4 (lines 87-101).
- Packet status: `active` (explicit authoring request — no other packet is assumed to hold the
  active slot for this work).
- Aggregate context cost: `M` (Step 1 diagnosis is the heavy dispatch; the fix itself is sized
  S/M pending the diagnosis).

## Problem Statement

A faithful-port audit of the D-105 beading fix verified one fix and surfaced one open root-cause
question: the **thin-strip medial-axis collapse**. Symptom (per D-105D): a thin strip's medial
axis collapses to a single two-node edge whose `to` peak vertex every emitted edge shares, so
every junction snaps to that one point and the wall loop has zero length, then
`remove_small_lines` drops it. The D-105 beading fix is faithful but does NOT address this — it
is a graph-construction / topology issue governed by ADR-0034 (faithful port, no fabricated
subdivisions). A prior session tried to mask the symptom with a fabricated
`from_polygons_with_beading` spine subdivision in `graph.rs` (subdividing all `!is_curved` edges
longer than `2 * optimal_width`). That was verified as NOT a faithful port — OrcaSlicer's
`discretize` returns `{start, end}` for the seg-seg edges that make up a thin strip's spine. The
fabrication was reverted; the thin-strip parity tests are now RED. This packet investigates the
true root cause and fixes it faithfully, closing D-105D.

## Investigation Requirements (what the packet MUST determine)

- **IR-1.** Confirm which candidate location is the responsible mechanism:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — `connectJunctions` /
    `getNextUnconnected` traversal may not correctly walk a single-edge spine domain.
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — `BeadingPropagation` may
    assign a degenerate bead count to a single-edge domain, collapsing all junctions to one
    vertex.
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — `discretize_edge` currently
    returns `{start, end}` for ALL `!is_curved` edges; OrcaSlicer `discretize` distinguishes
    three cases (seg-seg → `{start,end}`, point-segment → parabola, point-point → subdivided by
    `discretization_step_size` with marking vertices); the missing point-point case 3 subdivision
    may be needed — BUT a rectangle's medial-axis spine is seg-seg, NOT point-point, so this is
    unlikely to be the root cause for a thin strip (must be verified during investigation).
  - Whether OrcaSlicer's actual thin-strip behavior is identical to PnP's broken behavior (in
    which case the tests are testing the wrong thing) — verified by reading OrcaSlicer's
    `WallToolPaths.cpp` for thin-strip special cases.
- **IR-2.** Document the evidence: failing-test output for the 4 thin-strip tests + the G4 test,
  and a delegated OrcaSlicer `discretize`/`WallToolPaths` case analysis showing which behavior is
  correct.
- **IR-3.** Choose the faithful fix mechanism traceable to a specific OrcaSlicer
  `discretize`/`WallToolPaths`/`connectJunctions` case — explicitly NOT the reverted fabricated
  spine subdivision.

## Correctness Requirements (what the fix MUST satisfy)

- **CR-1.** All 4 thin-strip parity tests (AC-2, AC-3, AC-4) plus the G4 test (AC-5) are GREEN.
- **CR-2.** The mechanism does not add any edge-subdivision pass over `!is_curved` edges longer
  than `2 * optimal_width` (AC-N1 — the reverted fabrication is forbidden).
- **CR-3.** The mechanism is faithful to OrcaSlicer per ADR-0034: traceable to a specific
  OrcaSlicer case with file:line provenance (AC-N2).
- **CR-4.** If the investigation concludes OrcaSlicer itself drops/zero-length-loops the thin
  strip, the goldens are re-blessed to that documented behavior with the rationale recorded —
  the fix is then "the tests were wrong", not a code change (still satisfies CR-1 via re-blessed
  goldens).

## In Scope

- Diagnosis of the thin-strip medial-axis collapse (Step 1), using `/diagnose` and delegated
  OrcaSlicer reads.
- A faithful fix chosen from the Step 1 diagnosis (Steps 2-4) in exactly one of:
  `generate_toolpaths.rs` (`connectJunctions`/`getNextUnconnected`), `propagation.rs`
  (`BeadingPropagation`), `graph.rs` (`discretize_edge` — only if case 3 is genuinely needed), or
  golden re-blessing (if OrcaSlicer agrees with the current PnP behavior).
- Re-blessing the 6 stale goldens (4 thin-strip + the G4 test) against verified OrcaSlicer-parity
  behavior (Step 5).
- Closing `D-105D` in `docs/DEVIATION_LOG.md` with the verified root cause and faithful mechanism
  (Step 5).
- Updating `docs/18_arachne_parity_audit.md` §G4 with the investigation outcome (Step 5).

## Out of Scope

- The D-105 beading fix (already in place, faithful) — confirmed via `D-105` row.
- The D-105B/C/E sentinel fixes (already in place, faithful) — confirmed via their rows.
- The D-105 deviation-log entry itself (its content is fixed; this packet only closes D-105D).
- Any new WIT record changes — no host-service interface changes expected.
- The fabricated spine-subdivision mechanism (`from_polygons_with_beading` subdividing all
  `!is_curved` edges > `2 * optimal_width`) — explicitly forbidden (AC-N1).
- Classic-perimeters (M1, frozen); spiral-vase; non-planar.

## Authoritative Docs

- `docs/DEVIATION_LOG.md` — read `D-105D` (line 27) in full; read `D-105B`/`D-105C`/`D-105E` to
  confirm out-of-scope. Purpose: this packet is D-105D's named tracker.
- `docs/adr/0034-arachne-faithful-graph-construction.md` — read full. Purpose: the architectural
  constraint this packet MUST NOT violate (faithful port, no fabricated subdivisions).
- `docs/18_arachne_parity_audit.md` — read §G4 (lines 87-101). Purpose: the G4 closure the
  collapse masks on thin strips.
- `docs/08_coordinate_system.md` — range-read §"Constant Conversion Table" only (unit conversion
  if the fix touches mm↔unit boundaries).

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into
the implementer's own context. Default dispatch contract: return `LOCATIONS` (file:line + 1-line
context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns
are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` — `discretize()` /
  `discretize_edge()` case analysis (seg-seg → `{start,end}`; point-segment → parabola;
  point-point → subdivided by `discretization_step_size` with marking vertices). The missing
  faithful port of case 3 is a candidate, evaluated against the fact that a rectangle's
  medial-axis spine is seg-seg, not point-point.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` — thin-strip special cases: does
  OrcaSlicer itself produce a zero-length loop on a thin strip, or does it emit a real wall?
  Determines whether PnP's broken behavior is identical-to-OrcaSlicer (tests wrong) or a PnP
  defect (tests correct, fix required).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp` —
  `connectJunctions()` / `getNextUnconnected()` — the single-edge-domain traversal candidate.

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (root-cause diagnosis written), `AC-2` (thin-wall flag test GREEN),
  `AC-3` (thin-wall loop-type test GREEN), `AC-4` (2 thin-strip `slicer-runtime` tests GREEN),
  `AC-5` (G4 Flow-spacing test GREEN), `AC-6` (D-105D closed).
- Negative cases: `AC-N1` (no fabricated spine subdivision), `AC-N2` (mechanism faithful to
  OrcaSlicer, ADR-0034).
- Refinements not captured in Given/When/Then:
  - The G4 test (`..._wall_gap_uses_flow_spacing_not_width`) lives in
    `slicer-runtime --test arachne_parity_gaps` (file
    `crates/slicer-runtime/tests/arachne_parity_gaps.rs:290`), NOT a binary of that name.
  - Step 1 is the single point of failure: Steps 2-4 MUST NOT begin until Step 1's findings
    exist and name the responsible mechanism.
  - The 6 stale goldens are: `arachne_parity_is_thin_wall_flag_tdd`,
    `arachne_parity_thin_wall_loop_type_tdd`, the 2 thin-strip tests in `slicer-runtime --test
    arachne_parity`, and the G4 test. All were RED under the reverted fabrication.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'Step 1 Findings' .ralph/specs/154-arachne-thin-strip-collapse/design.md` | AC-1: diagnosis written | FACT pass/fail |
| `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd 2>&1 \| tee target/test-output-thin-flag.log \| tail -5; grep -q '^test result: ok' target/test-output-thin-flag.log` | AC-2 | FACT pass/fail |
| `cargo test -p arachne-perimeters --test arachne_parity_thin_wall_loop_type_tdd 2>&1 \| tee target/test-output-thin-loop.log \| tail -5; grep -q '^test result: ok' target/test-output-thin-loop.log` | AC-3 | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity 2>&1 \| tee target/test-output-runtime-thin.log \| tail -5; grep -q '^test result: ok' target/test-output-runtime-thin.log` | AC-4 (2 thin-strip tests) | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width --exact 2>&1 \| tee target/test-output-g4.log \| tail -5; grep -q '^test result: ok' target/test-output-g4.log` | AC-5 (G4) | FACT pass/fail |
| `rg -q 'D-105D' docs/DEVIATION_LOG.md && rg -q 'Closed' docs/DEVIATION_LOG.md` | AC-6: D-105D closed | FACT pass/fail |
| `rg -L 'from_polygons_with_beading\|subdivide.*2 \* optimal_width' crates/slicer-core/src/skeletal_trapezoidation/graph.rs \|\| echo CLEAN` | AC-N1: no fabricated subdivision | FACT CLEAN |
| `rg -q 'OrcaSlicer' .ralph/specs/154-arachne-thin-strip-collapse/design.md` | AC-N2: mechanism has OrcaSlicer provenance | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence | FACT clean / STALE list |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot express:

- **Step 1 (diagnosis) gates Steps 2-4.** The written `design.md` §Step 1 Findings MUST name
  exactly one responsible mechanism before any fix code is written. If the diagnosis concludes
  "OrcaSlicer behaves identically — tests wrong", Steps 2-4 collapse into Step 5
  (golden re-blessing) with no code change beyond the goldens.
- **The fix MUST be faithful (ADR-0034).** No fabricated subdivision. AC-N1 + AC-N2 are the
  faithfulness gates; if either fails, the mechanism is wrong regardless of AC-2..AC-5 green.
- **Step 5 (golden re-blessing) is atomic with D-105D closure.** Re-bless all 6 goldens and close
  D-105D in the same step; record the OrcaSlicer-parity rationale for each re-blessed golden.

## Context Discipline Notes

Packet-specific context hazards:

- `crates/slicer-core/src/arachne/generate_toolpaths.rs` (~975 LOC) — candidate edit target if
  `connectJunctions`/`getNextUnconnected` is the cause; full-read only for the diagnosis step that
  implicates it.
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (632 LOC) — candidate edit
  target if `BeadingPropagation` is the cause.
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (~700 LOC) — candidate edit target if
  `discretize_edge` case 3 is genuinely needed (unlikely for a seg-seg spine).
- `OrcaSlicerDocumented/...` — delegate; never load directly.
- The 4 thin-strip tests + G4 test are the only acceptance surface; no new tests are required by
  this packet (it fixes an existing-RED regression, not adds coverage).

If none apply, write `None packet-specific.`
