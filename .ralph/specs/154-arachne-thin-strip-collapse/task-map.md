# Task Map: 154-arachne-thin-strip-collapse

This packet is a focused root-cause investigation of the thin-strip medial-axis collapse, tracked
by `docs/DEVIATION_LOG.md` `D-105D` (line 27). No `TASK-###` entries exist in
`docs/07_implementation_status.md` for this work; the provenance is a `/diagnose` session
(2026-07-13) that reverted a fabricated spine subdivision, plus the D-105D deviation entry.

## Task Crosswalk

| Packet Step | Candidate Investigated | Resolution Path | OrcaSlicer Ref |
|---|---|---|---|
| Step 1: Diagnose root cause | A′ (quad chain walk: `resolve_to_vertex`/`quad_peak_position`/`chain_junctions_for_bead`/`emit_chain_lines`), B′ (`build_quad_rib_topology` in `rib.rs`), C′ (`generate_local_maxima_single_beads`), D′ (OrcaSlicer identical) | Writes a `verdict: <X>` line in `design.md` §Step 1 Findings | none by default — canonical facts C-1…C-8 pre-answered in `design.md`; none — the `connectJunctions` chaining rule is pinned in C-5 |
| Step 2: Faithful fix (mechanism-specific) | the mechanism Step 1 named | Code fix traceable to an OrcaSlicer `file:line` (AC-N3) | per Step 1 verdict |
| Step 3: Validate 4 thin-strip + G4 | (validation) | AC-2..AC-5 + AC-N1/AC-N2/AC-N3 green | none |
| Step 4: Workspace gate | (verification) | check / clippy / build-guests --check clean | none |
| Step 5: Re-bless goldens + close D-105D, file D-154-DISCRETIZE-POINT-POINT-CASE | (closure) | 6 goldens re-blessed; D-105D Closed with symbol list corrected; D-154-DISCRETIZE-POINT-POINT-CASE opened; G4 note | none |

> **Refined 2026-07-14.** The original candidate set (A/B/C/D) was retired: it named PnP symbols
> that do not exist (`connect_junctions`, `get_next_unconnected`, `BeadingPropagation`) and put
> `connectJunctions` in the wrong OrcaSlicer file. See `design.md` §Falsified Premises.

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
| `D-105D` | Open (root cause unknown; tests RED; symbol list wrong) | **Closed** (Step 5, against verified OrcaSlicer-parity evidence: root cause named + faithful fix OR golden re-bless per OrcaSlicer-identical behavior), with its symbol list corrected per `design.md` F-1/F-2/F-3 | Steps 1-5 |
| `D-154-DISCRETIZE-POINT-POINT-CASE` (new) | did not exist | **Open** (Step 5) — `discretize_edge` conflates canonical `discretize` branch 1 (seg-seg / secondary) and branch 3 (point-point), so point-point VD edges are never subdivided. Real parity gap; provably not the thin-strip root cause (C-2) | Step 5 (file only) |

`D-105`, `D-105B`, `D-105C`, `D-105E` are out of scope — confirmed faithful and not touched by
this packet.

## Cross-Packet Dependencies

- **Depends on:** packet 150 (`status: implemented`) for the G4 Flow-spacing wiring the D-105
  beading fix relies on; the active arachne topology work (packet 153 / ADR-0034) that left the
  quad chain walk in `generate_toolpaths.rs` current.
- **Does not modify:** the D-105 beading fix, D-105B/C/E sentinel fixes — confirmed out of scope.
- **Unblocks:** G4 observability on thin strips (the D-105 beading fix is correct but masked by
  the collapse) and a clean thin-strip parity story for the Arachne roadmap.

## OrcaSlicer Reference Paths (per `requirements.md` §OrcaSlicer Reference Obligations)

The canonical reads are **already done** — `design.md` §Canonical Facts (C-1…C-8) carries verified
`file:line` answers, including the `connectJunctions` chaining rule (C-5). Do not re-dispatch them.
Step 1 needs **zero** OrcaSlicer dispatches. One conditional read remains:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp` — `makeRib()` (only
  if the verdict is B′ and the faithful rib construction must be ported)

All reads delegated; never load `OrcaSlicerDocumented/` directly.

## Step Dependency Graph

```
Step 1 (diagnose root cause) — M  [GATE: writes `verdict: <X>`, X in A′/B′/C′/D′]
    │
    ├─(verdict A′/B′/C′)─► Step 2 (faithful fix) — S/M
    │                          └─► Step 3 (validate 4 thin-strip + G4) — S
    │                                  └─► Step 4 (workspace gate) — S
    │                                          └─► Step 5 (re-bless + close D-105D, file D-154-DISCRETIZE-POINT-POINT-CASE) — S
    │
    └─(verdict D′: OrcaSlicer identical)─► Step 5 directly (golden re-bless, no code change)
```

Step 1 is the single point of failure and the gate for everything else. If Step 1 concludes D′,
Steps 2-4 are skipped and Step 5 is the entire fix (golden re-blessing) — but D′ requires positive
OrcaSlicer `file:line` evidence that canonical also degenerates (C-5 indicates it does not).

## Forbidden Mechanism (carried from D-105D + AC-N1)

The reverted fabricated `from_polygons_with_beading` spine subdivision — subdividing all
`!is_curved` edges longer than `2 * optimal_width` — is explicitly forbidden. It is NOT a faithful
port and violates ADR-0034. Canonical evidence (`design.md` C-2/C-3): OrcaSlicer's `discretize`
returns `{start, end}` for a thin strip's seg-seg spine, **and** `generateJunctions`
(`SkeletalTrapezoidation.cpp:1740`) emits *zero* junctions on it because its R is constant. So
subdividing the spine to create junctions is exactly backwards. Any packet-step proposal to
reintroduce it must be rejected at review.

## Grilling-Session Decisions (traceability)

This packet's shape was set by the authoring request (documentation-only spec packet). Key
decisions recorded here for a future reader:

1. Investigation-first: no solution prescribed; `design.md` is TBD pending Step 1.
3. The D-105 beading fix and D-105B/C/E are explicitly out of scope (confirmed faithful).
4. The G4 test is in `slicer-runtime --test arachne_parity_gaps`, not a like-named binary.
5. Faithfulness gate (AC-N1/AC-N2/AC-N3) is mandatory regardless of which candidate wins — a green
   test suite produced by a fabricated subdivision is NOT acceptable.
6. (Refinement, 2026-07-14) The candidate set was rebuilt against verified canonical evidence after
   the original A/B/C/D set was found to name three non-existent PnP symbols and the wrong
   OrcaSlicer file for `connectJunctions`. The `discretize_edge` gap it did surface is real but
   off-target, and is now filed separately as D-154-DISCRETIZE-POINT-POINT-CASE rather than fixed here.
