---
status: implemented
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
  beading fix relies on; packet 153 (`status: implemented`) — the arachne topology work that left
  the quad chain walk in `generate_toolpaths.rs` current; D-105D in `docs/DEVIATION_LOG.md` (the
  open entry this packet closes).
- No activation blocker. The investigation is self-contained.

## Acceptance Criteria

- **AC-1.** A written root-cause diagnosis exists in `design.md` (§Step 1 Findings) naming exactly
  one candidate location as the responsible mechanism, citing the failing-test evidence and the
  `file:line` that proves it. The findings MUST NOT contradict a locked canonical fact (C-1…C-8 in
  `design.md`) without citing a contradicting OrcaSlicer `file:line`. |
  `rg -q "verdict: (A′|B′|C′|D′|E′)" .ralph/specs/154-arachne-thin-strip-collapse/design.md`

  > **Amended 2026-07-14 (Step 1 outcome).** The original criterion admitted only **A′** (quad chain
  > walk), **B′** (rib topology), **C′** (`generate_local_maxima_single_beads`), or **D′**
  > ("OrcaSlicer behaves identically"). The diagnosis falsified that set's *completeness*: all four
  > presuppose the skeletal graph is built from the actual rectangle, and it is not. The real cause
  > sits upstream of every one of them, in outline preprocessing — recorded as candidate **E′**, and
  > the pattern is widened accordingly. No locked canonical fact (C-1…C-8) is contradicted; all
  > survive. A′, B′ and C′ are affirmatively exonerated and D′ is killed — see `design.md` §Step 1
  > Findings. Writing an `A′` verdict to satisfy the original grep would have been verification
  > gaming, not a pass.
- **AC-2.** `arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd` is GREEN. |
  `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd 2>&1 | tee target/test-output-thin-flag.log | tail -5; grep -q '^test result: ok' target/test-output-thin-flag.log`
- **AC-3.** `arachne-perimeters --test arachne_parity_thin_wall_loop_type_tdd` is GREEN. |
  `cargo test -p arachne-perimeters --test arachne_parity_thin_wall_loop_type_tdd 2>&1 | tee target/test-output-thin-loop.log | tail -5; grep -q '^test result: ok' target/test-output-thin-loop.log`
- **AC-4.** `slicer-runtime --test arachne_parity` — the 2 thin-strip tests are GREEN. |
  `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_arachne_path_thin_wall_loop_type_emitted arachne_parity_arachne_path_is_thin_wall_flag_set_on_thin_wall_loops 2>&1 | tee target/test-output-runtime-thin.log | tail -5; grep -q '^test result: ok' target/test-output-runtime-thin.log`

  > **Amended 2026-07-14 (Step 1 outcome).** The original command ran the whole `arachne_parity`
  > binary and grepped for a clean `test result: ok`. That binary also contains
  > `arachne_parity_pipeline_concentric_infill_uses_arachne` — a pre-existing, out-of-scope parity
  > gap (concentric infill is never dispatched through `run_arachne_pipeline` in `slicer-runtime`'s
  > `run.rs`; it proposes deviation `D-104f-CONCENTRIC-INFILL-NO-ARACHNE`). It was already red before
  > this packet and no thin-strip fix can turn it green, so the original AC-4 was unsatisfiable by
  > construction. The command now names the 2 thin-strip tests explicitly, which is the scope AC-4's
  > own prose always described. The concentric-infill gap is filed as its own row in Step 5 rather
  > than absorbed here. (Confirmed with the user, 2026-07-14.)
- **AC-5.** `slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width` is GREEN (the G4 test the D-105 architectural fix targets). |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width --exact 2>&1 | tee target/test-output-g4.log | tail -5; grep -q '^test result: ok' target/test-output-g4.log`

  > **Note (Step 1 outcome).** Contrary to the packet's premise, this test was already GREEN at the
  > start of the run — the G4 Flow-spacing wiring from packet 150 plus the D-105 beading fix are
  > sufficient on their own. AC-5 is retained as a regression guard: the thin-strip fix must not
  > break it.
- **AC-6.** The `D-105D` row in `docs/DEVIATION_LOG.md` is marked `Closed` **on its own row** (not
  merely somewhere in the file), with the verified root cause and the faithful mechanism recorded,
  and its symbol list corrected per `design.md` §Falsified Premises (F-1/F-2/F-3). |
  `rg -q '^\| D-105D \|.*\| *Closed' docs/DEVIATION_LOG.md`
- **AC-7.** The `discretize_edge` branch-1/branch-3 conflation found during refinement
  (`design.md` F-4) is filed as its own **new** deviation row `D-154-DISCRETIZE-POINT-POINT-CASE`, naming `discretize_edge` and
  the point-point (branch-3) case, with status `Open` — rather than being silently absorbed into
  D-105D's closure. |
  `rg -q '^\| D-154-DISCRETIZE-POINT-POINT-CASE \|' docs/DEVIATION_LOG.md`

## Negative Test Cases

- **AC-N1.** The fix does NOT reintroduce the reverted fabricated spine subdivision
  (`from_polygons_with_beading`, subdividing all `!is_curved` edges longer than
  `2 * optimal_width`). The command exits non-zero if the marker is present. |
  `! rg -q 'from_polygons_with_beading' crates/slicer-core/src/skeletal_trapezoidation/graph.rs`
- **AC-N2.** The fix does NOT add junction emission on flat (equal-R) edges — canonical
  `generateJunctions` (`SkeletalTrapezoidation.cpp:1740`) `continue`s on `end_R >= start_R`, and
  PnP already matches (`design.md` C-3/C-4). The upward-half-edge guard in `generate_junctions`
  survives the fix verbatim. |
  `rg -q 'if from_r >= to_r \{' crates/slicer-core/src/arachne/generate_toolpaths.rs`
- **AC-N3.** The fix does NOT add interior nodes to a thin strip's two-node seg-seg spine — that
  topology is canonical (`design.md` C-2), not a defect. The implemented mechanism is traceable to
  a specific OrcaSlicer `file:line` recorded in `design.md` §Step 1 Findings. |
  `rg -q 'WallToolPaths\.(cpp|hpp):[0-9]+' .ralph/specs/154-arachne-thin-strip-collapse/design.md && rg -q 'SkeletalTrapezoidation(Graph)?\.(cpp|hpp):[0-9]+' .ralph/specs/154-arachne-thin-strip-collapse/design.md`

  > **Amended 2026-07-14 (Step 1 outcome).** The original grep accepted only a
  > `SkeletalTrapezoidation*.cpp:NNN` citation. That would now pass **vacuously** — §Canonical Facts
  > is full of such references — while the mechanism actually implemented traces to
  > `WallToolPaths.cpp:86-201` (canonical `simplify()`), whose deviation guard at
  > `WallToolPaths.cpp:161-162` is the ported rule. The command now requires the `WallToolPaths`
  > citation too, so it tests the criterion's stated intent instead of passing on unrelated text.
  > The substantive prohibition is unchanged and is satisfied by construction: the fix lives in
  > outline preprocessing, upstream of graph construction, and adds no spine nodes whatsoever.

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (CLEAN — this packet edits `slicer-core` internals and/or
  `arachne-perimeters` host-side; no WIT changes expected)
- `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd --test arachne_parity_thin_wall_loop_type_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_arachne_path_thin_wall_loop_type_emitted arachne_parity_arachne_path_is_thin_wall_flag_set_on_thin_wall_loops 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test arachne_parity_gaps 2>&1 | tee target/test-output-gaps.log`

  > **Amended 2026-07-14 (Step 1 outcome), for the same reason as AC-4.** The original line ran
  > `--test arachne_parity --test arachne_parity_gaps` unfiltered. `arachne_parity` contains
  > `arachne_parity_pipeline_concentric_infill_uses_arachne` (`crates/slicer-runtime/tests/arachne_parity.rs:928`),
  > a **deliberately-failing parity-gap test** that asserts concentric infill is not routed through
  > Arachne and therefore stays RED until deviation `D-104f-CONCENTRIC-INFILL-NO-ARACHNE` (open since
  > packet 149, 2026-07-09) is closed. It is unrelated to the thin-strip collapse, was red before this
  > packet, and is red after it. The unfiltered command was never satisfiable, and its failure also
  > aborted the `arachne_parity_gaps` binary before it could run. The two binaries are now run
  > separately, with `arachne_parity` scoped to this packet's 2 thin-strip tests. **Known-red and
  > explicitly NOT fixed here:** `arachne_parity_pipeline_concentric_infill_uses_arachne`.

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
  mechanism, noting the 6 goldens re-blessed (4 thin-strip + G4), **and correct its symbol list**:
  the row currently cites `connectJunctions` / `getNextUnconnected` / `BeadingPropagation` as if
  they were PnP symbols, and none of the three exists in the tree (`design.md` F-3). |
  `rg -q '^\| D-105D \|.*\| *Closed' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` — open a **new** row for the `discretize_edge` branch-1/branch-3
  conflation (`design.md` F-4): PnP returns `{start, end}` for all `!is_curved` edges, so canonical
  point-point edges are never subdivided. Real parity gap, not this packet's root cause. File as
  `D-154-DISCRETIZE-POINT-POINT-CASE`, status `Open`. |
  `rg -q '^\| D-154-DISCRETIZE-POINT-POINT-CASE \|' docs/DEVIATION_LOG.md`
- `docs/18_arachne_parity_audit.md` — record the thin-strip collapse investigation outcome under
  the G4 section (the collapse was masking G4 observability on thin strips). |
  `rg -q 'thin-strip' docs/18_arachne_parity_audit.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into
the implementer's own context. Default dispatch contract: return `LOCATIONS` (file:line + 1-line
context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns
are capped at 30 lines.

**Most canonical questions for this packet are already answered.** `design.md` §Canonical Facts
(C-1…C-8) records verified `file:line` answers for `discretize()`'s case analysis, a rectangle
spine's branch, `generateJunctions()`'s flat-edge skip, `collapseSmallEdges`'s `snap_dist`, and the
full stage order. **Do not re-dispatch those reads.** §Falsified Premises (F-1…F-4) records the
wrong references the draft carried — in particular `connectJunctions()` is in
`SkeletalTrapezoidation.cpp:1934`, **not** `SkeletalTrapezoidationGraph.cpp`.

The only OrcaSlicer read this packet may still need:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1934` —
  `connectJunctions()`: how junctions from the `edge_to_peak->prev` quad chain are accumulated
  into one `ExtrusionLine`, and whether a flat (zero-junction) spine edge is traversed or skipped.
  Dispatch **only if** Step 1 exonerates Candidates A′ and B′, or when porting the chaining rule.

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
