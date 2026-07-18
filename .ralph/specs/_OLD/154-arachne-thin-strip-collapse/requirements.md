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

> **Refined 2026-07-14.** The original candidate set rested on three false premises, now recorded
> in `design.md` §Falsified Premises: `connectJunctions` lives in `SkeletalTrapezoidation.cpp:1934`
> (not `SkeletalTrapezoidationGraph.cpp`); `getNextUnconnected` is a chain-end lookup, not a
> domain traversal; and none of `connect_junctions` / `get_next_unconnected` / `BeadingPropagation`
> exists in the PnP tree at all. The verified canonical facts are locked in `design.md`
> §Canonical Facts (C-1…C-8) and must be read before Step 1.

- **IR-1.** Confirm which of the **revised** candidates is the responsible mechanism (full
  definitions in `design.md` §Controlling Code Paths):
  - **A′** — the quad chain walk (PnP's inlined `connectJunctions`) in
    `crates/slicer-core/src/arachne/generate_toolpaths.rs`: `resolve_to_vertex`,
    `quad_peak_position`, `chain_junctions_for_bead`, `emit_chain_lines`. D-105D's own symptom
    statement — every emitted edge sharing one `to` peak vertex — is a claim about these functions.
    Prime suspect.
  - **B′** — rib topology in `crates/slicer-core/src/skeletal_trapezoidation/rib.rs`
    (`build_quad_rib_topology`) vs canonical `makeRib()`. Per C-5, a thin strip's junctions can
    only come from R-varying rib/pointy-end edges — canonical emits **none** on the flat spine.
  - **C′** — `generate_local_maxima_single_beads` vs canonical `generateLocalMaximaSingleBeads`.
    **Demoted by C-7**: canonical's guard excludes *central* nodes and needs an isolated local
    maximum; a flat central spine ridge is neither. Check only whether PnP's guard wrongly admits it.
  - **D′** — OrcaSlicer behaves identically (tests wrong). **Effectively killed by C-5 + C-8**:
    canonical has a concrete spine-spanning mechanism, and its `removeSmallLines` drops only
    `is_odd && !is_closed && short` lines. Formal escape hatch only; requires positive contradicting
    `file:line` evidence.
- **IR-2.** Document the evidence: failing-test output for the 4 thin-strip tests + the G4 test,
  plus the local A′ peak-vertex trace and the B′ rib count/R-range. The OrcaSlicer case analysis is
  **already done** (C-1…C-8) and must not be re-dispatched.
- **IR-3.** Choose a faithful fix traceable to a specific OrcaSlicer `file:line` — explicitly NOT
  the reverted spine subdivision, NOT junction emission on flat edges (canonical skips them,
  C-3/C-4), and NOT the addition of interior nodes to the canonical two-node seg-seg spine (C-2).
- **IR-4.** File the `discretize_edge` branch-1/branch-3 conflation (`design.md` F-4) as its own
  new deviation-log row. It is a real parity gap — PnP never subdivides point-point VD edges — but
  it is **not** the thin-strip root cause and must not be absorbed silently.

## Correctness Requirements (what the fix MUST satisfy)

- **CR-1.** All 4 thin-strip parity tests (AC-2, AC-3, AC-4) plus the G4 test (AC-5) are GREEN.
- **CR-2.** The mechanism does not add any edge-subdivision pass over `!is_curved` edges longer
  than `2 * optimal_width` (AC-N1 — the reverted fabrication is forbidden).
- **CR-3.** The mechanism is faithful to OrcaSlicer per ADR-0034: traceable to a specific
  OrcaSlicer `file:line` (AC-N3). It must not emit junctions on flat/equal-R edges (AC-N2) and
  must not add interior nodes to the canonical two-node seg-seg spine (AC-N3).
- **CR-4.** D′ (re-blessing goldens to a zero-length loop) is **effectively closed off** by C-5 and
  C-8: canonical stitches rib-quad segments into a real spine-length `ExtrusionLine`
  (`addToolpathSegment`, `SkeletalTrapezoidation.cpp:1887-1932`), and its `removeSmallLines`
  (`WallToolPaths.cpp:693`) drops only `is_odd && !is_closed && short` lines — so a thin strip's
  wall is never dropped canonically. A golden may be re-blessed to a degenerate loop **only** on
  positive OrcaSlicer `file:line` evidence contradicting C-5/C-8.

## In Scope

- Diagnosis of the thin-strip medial-axis collapse (Step 1), using `/diagnose` and delegated
  OrcaSlicer reads.
- A faithful fix chosen from the Step 1 diagnosis (Steps 2-4) in exactly one of:
  `generate_toolpaths.rs` (A′ — the quad chain walk; or C′ — `generate_local_maxima_single_beads`),
  `rib.rs` (B′ — `build_quad_rib_topology`), or golden re-blessing (D′ — only with positive
  OrcaSlicer evidence).
- Filing the `discretize_edge` branch-1/branch-3 conflation as a new deviation-log row (IR-4/AC-7).
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
- **Fixing** the `discretize_edge` branch-1/branch-3 conflation (`design.md` F-4) — this packet
  only *files* it (AC-7). It is a real parity gap but provably not the thin-strip root cause (C-2).
- `propagation.rs` and `graph.rs::discretize_edge` as edit targets — the draft's Candidates B and C,
  retired by F-3/F-4.
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

**The canonical reads for this packet are already done.** `design.md` §Canonical Facts (C-1…C-8)
carries verified `file:line` answers for `discretize()`'s three branches, a rectangle spine's
branch, `generateJunctions()`'s flat-edge skip, `collapseSmallEdges`'s `snap_dist` (correctly
converted in PnP), and the full stage order. **Do not re-dispatch them.** §Falsified Premises
(F-1…F-4) records the draft's wrong references — notably `connectJunctions()` is in
`SkeletalTrapezoidation.cpp:1934`, **not** `SkeletalTrapezoidationGraph.cpp`.

The `connectJunctions` chaining rule is also pinned (C-5), as are the local-maxima guard (C-7) and
`removeSmallLines` (C-8). **Step 1 needs zero OrcaSlicer dispatches.**

Remaining conditional read:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp` `makeRib()` —
  dispatch only if Step 1's verdict is B′ and the faithful rib construction must be ported.

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (root-cause `verdict:` line written), `AC-2` (thin-wall flag test GREEN),
  `AC-3` (thin-wall loop-type test GREEN), `AC-4` (2 thin-strip `slicer-runtime` tests GREEN),
  `AC-5` (G4 Flow-spacing test GREEN), `AC-6` (D-105D closed + symbol list corrected), `AC-7`
  (`discretize_edge` gap filed as a new Open row).
- Negative cases: `AC-N1` (no fabricated spine subdivision), `AC-N2` (no junction emission on
  flat/equal-R edges — canonical skips them), `AC-N3` (no interior nodes added to the canonical
  two-node spine; mechanism cites an OrcaSlicer `file:line`, ADR-0034).
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
