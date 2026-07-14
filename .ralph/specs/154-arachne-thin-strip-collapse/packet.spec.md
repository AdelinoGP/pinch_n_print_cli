---
status: active
packet: 154-arachne-thin-strip-collapse
task_ids: []
backlog_source: docs/DEVIATION_LOG.md D-105D (line 27); /diagnose session 2026-07-13 on the reverted spine-subdivision fix; docs/18_arachne_parity_audit.md (G4)
context_cost_estimate: M
---

# Packet Contract: 154-arachne-thin-strip-collapse

## Goal

Determine and fix the root cause of the thin-strip medial-axis collapse so the 4 thin-strip
parity tests plus the G4 Flow-spacing test go GREEN with a *faithful* mechanism — never with a
fabricated spine subdivision that violates ADR-0034. This packet is an **investigation first**:
it prescribes no solution; its sole deliverable is a verified root cause and a faithful fix that
survives the OrcaSlicer `discretize` case analysis.

## Scope Boundaries

This packet owns: a disciplined diagnosis of which candidate location is responsible for the
thin-strip collapse (Step 1, using `/diagnose`); a faithful fix chosen from the diagnosis (Steps
2-4, mechanism TBD pending Step 1); re-blessing the 6 stale goldens (4 thin-strip + G4 from the
reverted session) against verified OrcaSlicer-parity behavior (Step 5); and closing D-105D in the
deviation log (Step 5). The packet explicitly does NOT prescribe a solution up front — the design
depends on Step 1's findings. Out of scope: the D-105 beading fix, the D-105B/C/E sentinel fixes,
and the D-105 deviation-log entry itself (all already in place and faithful).

## Prerequisites and Blockers

- Depends on: packet 150 (`status: implemented`) for the G4 Flow-spacing wiring that the D-105
  beading fix relies on; packet 153 (`status: implemented`) or the active arachne topology work
  that left `connectJunctions`/`getNextUnconnected` current; D-105D in `docs/DEVIATION_LOG.md`
  (the open entry this packet closes).
- Activation: `status: active` per the packet authoring request — this is an explicit
  investigation packet and no other packet is assumed to hold the active slot for this work.
- No activation blocker. The investigation is self-contained.

## Acceptance Criteria

- **AC-1.** A written root-cause diagnosis exists in `design.md` (§Step 1 Findings) naming exactly
  one of the candidate locations (`connectJunctions`/`getNextUnconnected` traversal in
  `generate_toolpaths.rs`; `BeadingPropagation` in `propagation.rs`; `discretize_edge` case-3
  port in `graph.rs`; or "OrcaSlicer behaves identically — tests are wrong") as the responsible
  mechanism, with the evidence (test output + delegated OrcaSlicer `discretize`/`WallToolPaths`
  case analysis) cited. |
  `rg -q 'Step 1 Findings' .ralph/specs/154-arachne-thin-strip-collapse/design.md`
- **AC-2.** `arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd` is GREEN. |
  `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd 2>&1 | tee target/test-output-thin-flag.log | tail -5; grep -q '^test result: ok' target/test-output-thin-flag.log`
- **AC-3.** `arachne-perimeters --test arachne_parity_thin_wall_loop_type_tdd` is GREEN. |
  `cargo test -p arachne-perimeters --test arachne_parity_thin_wall_loop_type_tdd 2>&1 | tee target/test-output-thin-loop.log | tail -5; grep -q '^test result: ok' target/test-output-thin-loop.log`
- **AC-4.** `slicer-runtime --test arachne_parity` — the 2 thin-strip tests are GREEN. |
  `cargo test -p slicer-runtime --test arachne_parity 2>&1 | tee target/test-output-runtime-thin.log | tail -5; grep -q '^test result: ok' target/test-output-runtime-thin.log`
- **AC-5.** `slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width` is GREEN (the G4 test the D-105 architectural fix targets). |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width --exact 2>&1 | tee target/test-output-g4.log | tail -5; grep -q '^test result: ok' target/test-output-g4.log`
- **AC-6.** D-105D in `docs/DEVIATION_LOG.md` is closed by this packet with the verified
  root cause and the faithful mechanism recorded. |
  `rg -q 'D-105D' docs/DEVIATION_LOG.md && rg -q 'Closed' docs/DEVIATION_LOG.md`

## Negative Test Cases

- **AC-N1.** The fix does NOT add any spine-subdivision / edge-subdivision pass that subdivides
  all `!is_curved` edges longer than `2 * optimal_width` (the reverted fabricated
  `from_polygons_with_beading` mechanism). A grep for the reverted fabrication marker returns
  nothing. | `rg -L 'from_polygons_with_beading|subdivide.*2 \* optimal_width' crates/slicer-core/src/skeletal_trapezoidation/graph.rs || echo CLEAN`
- **AC-N2.** The fix preserves ADR-0034 faithfulness: the implemented mechanism is traceable to a
  specific OrcaSlicer `discretize` / `WallToolPaths` case (seg-seg / point-segment / point-point),
  not to an invented PnP-only subdivision. The mechanism is documented in `design.md` with the
  OrcaSlicer file:line it ports. | `rg -q 'OrcaSlicer' .ralph/specs/154-arachne-thin-strip-collapse/design.md`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (CLEAN — this packet edits `slicer-core` internals and/or
  `arachne-perimeters` host-side; no WIT changes expected)
- `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd --test arachne_parity_thin_wall_loop_type_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test arachne_parity --test arachne_parity_gaps 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/DEVIATION_LOG.md` — read `D-105D` (line 27) in full; this packet is the spec packet D-105D
  names as its tracker. Read `D-105B`/`D-105C`/`D-105E` to confirm those fixes are out of scope.
- `docs/adr/0034-arachne-faithful-graph-construction.md` — read full; the architectural decision
  this packet MUST NOT violate (faithful port, no fabricated subdivisions).
- `docs/18_arachne_parity_audit.md` — read §G4 (lines 87-101) for the Flow-spacing closure the G4
  test asserts and why the collapse masks it on thin strips.
- `docs/08_coordinate_system.md` — range-read §"Constant Conversion Table" only (unit conversion
  at any mm↔unit boundary the fix touches).

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` — close `D-105D` (line 27) with the verified root cause and the faithful
  mechanism, noting the 6 goldens re-blessed (4 thin-strip + G4). |
  `rg -q 'D-105D' docs/DEVIATION_LOG.md && rg -q 'Closed' docs/DEVIATION_LOG.md`
- `docs/18_arachne_parity_audit.md` — record the thin-strip collapse investigation outcome under
  the G4 section (the collapse was masking G4 observability on thin strips). |
  `rg -q 'thin-strip' docs/18_arachne_parity_audit.md`

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

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by
`spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or
reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. Step 1 is
the gating diagnosis; Steps 2-4 are TBD pending its findings.
