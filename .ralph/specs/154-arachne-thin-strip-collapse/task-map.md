# Task Map: 154-arachne-thin-strip-collapse

This packet is a focused root-cause investigation of the thin-strip medial-axis collapse, tracked
by `docs/DEVIATION_LOG.md` `D-105D` (line 27). No `TASK-###` entries exist in
`docs/07_implementation_status.md` for this work; the provenance is a `/diagnose` session
(2026-07-13) that reverted a fabricated spine subdivision, plus the D-105D deviation entry.

## Task Crosswalk

| Packet Step | Candidate Investigated | Resolution Path | OrcaSlicer Ref |
|---|---|---|---|
| Step 1: Diagnose root cause | A (`connectJunctions`/`getNextUnconnected`), B (`BeadingPropagation`), C (`discretize_edge` case 3), D (OrcaSlicer identical) | Names the single responsible mechanism in `design.md` §Step 1 Findings | `SkeletalTrapezoidation.cpp` (`discretize`), `WallToolPaths.cpp`, `SkeletalTrapezoidationGraph.cpp` (`connectJunctions`) |
| Step 2: Faithful fix (mechanism-specific) | the mechanism Step 1 named | Code fix traceable to OrcaSlicer case (AC-N2) | per Step 1 verdict |
| Step 3: Validate 4 thin-strip + G4 | (validation) | AC-2, AC-3, AC-4, AC-5 + AC-N1/AC-N2 green | none |
| Step 4: Workspace gate | (verification) | check / clippy / build-guests --check clean | none |
| Step 5: Re-bless goldens + close D-105D | (closure) | 6 goldens re-blessed; D-105D Closed; G4 note | none |

## Acceptance Surface (the 6 goldens)

| # | Test | Binary | Criterion |
|---|---|---|---|
| 1 | `arachne_parity_is_thin_wall_flag_tdd` | `arachne-perimeters` | AC-2 |
| 2 | `arachne_parity_thin_wall_loop_type_tdd` | `arachne-perimeters` | AC-3 |
| 3 | 2 thin-strip tests | `slicer-runtime --test arachne_parity` | AC-4 |
| 4 | `arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width` | `slicer-runtime --test arachne_parity_gaps` | AC-5 (G4) |

All 6 are currently RED under the reverted fabricated spine subdivision. Note the G4 test lives in
`--test arachne_parity_gaps` (file `crates/slicer-runtime/tests/arachne_parity_gaps.rs:290`), not
a binary of that name.

## Deviation Disposition (Post-Packet)

| Deviation | Status Before 154 | Status After 154 | Mechanism |
|---|---|---|---|
| `D-105D` | Open (root cause unknown; tests RED) | **Closed** (Step 5, against verified OrcaSlicer-parity evidence: root cause named + faithful fix OR golden re-bless per OrcaSlicer-identical behavior) | Steps 1-5 |

`D-105`, `D-105B`, `D-105C`, `D-105E` are out of scope — confirmed faithful and not touched by
this packet.

## Cross-Packet Dependencies

- **Depends on:** packet 150 (`status: implemented`) for the G4 Flow-spacing wiring the D-105
  beading fix relies on; the active arachne topology work (packet 153 / ADR-0034) that left
  `connectJunctions`/`getNextUnconnected` current.
- **Does not modify:** the D-105 beading fix, D-105B/C/E sentinel fixes — confirmed out of scope.
- **Unblocks:** G4 observability on thin strips (the D-105 beading fix is correct but masked by
  the collapse) and a clean thin-strip parity story for the Arachne roadmap.

## OrcaSlicer Reference Paths (per `requirements.md` §OrcaSlicer Reference Obligations)

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` — `discretize()` /
  `discretize_edge()` case analysis (Candidate C)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` — thin-strip special cases
  (Candidate D: does OrcaSlicer drop or emit?)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp` —
  `connectJunctions()` / `getNextUnconnected()` (Candidate A)

All reads delegated; never load `OrcaSlicerDocumented/` directly.

## Step Dependency Graph

```
Step 1 (diagnose root cause) — M  [GATE: names A/B/C/D]
    │
    ├─(verdict A/B/C)─► Step 2 (faithful fix) — S/M
    │                       └─► Step 3 (validate 4 thin-strip + G4) — S
    │                               └─► Step 4 (workspace gate) — S
    │                                       └─► Step 5 (re-bless + close D-105D) — S
    │
    └─(verdict D: OrcaSlicer identical)─► Step 5 directly (golden re-bless, no code change)
```

Step 1 is the single point of failure and the gate for everything else. If Step 1 concludes
Candidate D, Steps 2-4 are skipped and Step 5 is the entire fix (golden re-blessing).

## Forbidden Mechanism (carried from D-105D + AC-N1)

The reverted fabricated `from_polygons_with_beading` spine subdivision — subdividing all
`!is_curved` edges longer than `2 * optimal_width` — is explicitly forbidden. It is NOT a faithful
port (OrcaSlicer `discretize` returns `{start, end}` for the seg-seg edges of a thin strip's
spine) and violates ADR-0034. Any packet-step proposal to reintroduce it must be rejected at
review.

## Grilling-Session Decisions (traceability)

This packet's shape was set by the authoring request (documentation-only spec packet). Key
decisions recorded here for a future reader:

1. Investigation-first: no solution prescribed; `design.md` is TBD pending Step 1.
2. `status: active` so the active-packet search finds it (per the authoring request).
3. The D-105 beading fix and D-105B/C/E are explicitly out of scope (confirmed faithful).
4. The G4 test is in `slicer-runtime --test arachne_parity_gaps`, not a like-named binary.
5. Faithfulness gate (AC-N1/AC-N2) is mandatory regardless of which candidate wins — a green
   test suite produced by a fabricated subdivision is NOT acceptable.
