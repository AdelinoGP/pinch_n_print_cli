# Implementation Plan: 154-arachne-thin-strip-collapse

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (none — tracked by D-105D).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`,
  and `spec-review`. The fields below are the budget contract for this step.
- **No solution is prescribed.** Steps 2-4 are TBD pending Step 1's diagnosis; the plan states
  the gating rule, not the code.

## Steps

### Step 1: Diagnose the responsible root-cause location

- Task IDs: none (tracked by `docs/DEVIATION_LOG.md` D-105D)
- Objective: Determine which candidate is responsible for the thin-strip medial-axis collapse.
  Reproduce the failure across all 4 thin-strip tests + the G4 test; delegate OrcaSlicer reads for
  (a) `discretize` case analysis — seg-seg vs point-point for a thin rectangle spine; (b)
  `WallToolPaths.cpp` thin-strip behavior — does OrcaSlicer itself drop/zero-length-loop or emit a
  real wall; (c) `connectJunctions`/`getNextUnconnected` single-edge-domain handling; and trace
  `BeadingPropagation` locally for a single-edge-domain degenerate bead count. Write the verdict
  into `design.md` §Step 1 Findings, naming exactly one responsible mechanism (A/B/C/D).
- Precondition: none (first step).
- Postcondition: AC-1 green — `design.md` §Step 1 Findings names the responsible mechanism with
  evidence.
- Files allowed to read: `crates/slicer-core/src/arachne/generate_toolpaths.rs`
  (`connectJunctions`/`getNextUnconnected`); `crates/slicer-core/src/skeletal_trapezoidation/
  propagation.rs` (`BeadingPropagation`); `crates/slicer-core/src/skeletal_trapezoidation/
  graph.rs` (`discretize_edge`); the 4 thin-strip test files + G4 test file; `docs/DEVIATION_LOG.md`
  D-105D; `docs/adr/0034-*.md`; `docs/18_arachne_parity_audit.md` §G4.
- Files allowed to edit (≤ 3): `.ralph/specs/154-arachne-thin-strip-collapse/design.md` (append
  findings). No source edit in this step.
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` — delegate; classic-perimeters; the
  D-105/B/C/E fixes.
- Expected sub-agent dispatches:
  - "Run the 4 thin-strip tests + G4; return SNIPPETS (≤ 20 lines each) of the failing assertion
    and the wall-loop state (length, junction count, `is_closed`)." — purpose: establish failure
    shape
  - (delegated) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp`
    `discretize`/`discretize_edge` case analysis: for a thin rectangle strip is the medial-axis
    spine seg-seg (case 1, `{start,end}`) or point-point (case 3, subdivided)? Return SUMMARY (≤
    200 words) + one 30-line excerpt. No other code." — purpose: resolve Candidate C
  - (delegated) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` thin-
    strip special cases: does OrcaSlicer emit a zero-length loop / drop a thin strip, or emit a
    real wall? Return SUMMARY (≤ 200 words). No code." — purpose: resolve Candidate D
  - (delegated) "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp`
    `connectJunctions`/`getNextUnconnected` for a single-edge (two-node) spine domain: how is a
    one-edge domain traversed? Return SUMMARY (≤ 200 words). No code." — purpose: resolve
    Candidate A
  - "Trace `BeadingPropagation` in `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`
    for a single-edge domain; return LOCATIONS (file:line + 1-line note) for the bead-count
    assignment that could collapse all junctions to one vertex." — purpose: resolve Candidate B
- Context cost: M
- Authoritative docs: `docs/DEVIATION_LOG.md` (D-105D), `docs/adr/0034-*.md`, `docs/18_arachne_parity_audit.md` §G4.
- OrcaSlicer refs: `SkeletalTrapezoidation.cpp` (`discretize`), `WallToolPaths.cpp`,
  `SkeletalTrapezoidationGraph.cpp` (`connectJunctions`) — all delegated, default SUMMARY-only.
- Verification: `rg -q 'Step 1 Findings' .ralph/specs/154-arachne-thin-strip-collapse/design.md` — dispatch as FACT pass/fail (AC-1).
- Exit condition: AC-1 green; `design.md` §Step 1 Findings names exactly one responsible
  mechanism with evidence. If Candidate D, skip to Step 5. Otherwise proceed to Steps 2-4
  (mechanism-specific).

### Step 2: Implement the faithful fix (mechanism-specific, TBD pending Step 1)

- Task IDs: none
- Objective: Implement the faithful fix for the mechanism Step 1 named. The exact surface is
  determined by the verdict: A → `generate_toolpaths.rs` `connectJunctions`/`getNextUnconnected`
  single-edge-domain traversal; B → `propagation.rs` `BeadingPropagation` degenerate bead count; C
  → `graph.rs` `discretize_edge` case-3 port (only if Step 1 proved case 3 genuinely required for
  the thin strip). The mechanism MUST be traceable to a specific OrcaSlicer
  `discretize`/`WallToolPaths`/`connectJunctions` case (AC-N2) and MUST NOT subdivide `!is_curved`
  edges > `2 * optimal_width` (AC-N1).
- Precondition: Step 1 green (responsible mechanism named).
- Postcondition: the targeted source fix is in place and compiles; AC-2..AC-5 trend toward green.
- Files allowed to read: the implicated candidate file (full) + its existing tests/fixtures; the
  relevant OrcaSlicer reference for the chosen case.
- Files allowed to edit (≤ 3): exactly one of `generate_toolpaths.rs` / `propagation.rs` /
  `graph.rs` (Step 1's verdict decides); its fixture JSON if needed.
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` — delegate; the other two candidate
  files (not implicated); classic-perimeters; the D-105/B/C/E fixes.
- Expected sub-agent dispatches:
  - (delegated, if A) "Summarize `OrcaSlicerDocumented/.../SkeletalTrapezoidationGraph.cpp`
    `connectJunctions`/`getNextUnconnected` for a one-edge domain; return SUMMARY (≤ 200 words).
    No code." — purpose: ground the faithful A fix
  - (delegated, if B) "Summarize `OrcaSlicerDocumented/.../SkeletalTrapezoidation.cpp`
    `BeadingPropagation` bead-count assignment for a degenerate single-edge domain; return
    SUMMARY (≤ 200 words). No code." — purpose: ground the faithful B fix
  - (delegated, if C) "Summarize `OrcaSlicerDocumented/.../SkeletalTrapezoidation.cpp`
    `discretize` case 3 (point-point) subdivision including `discretization_step_size` and marking
    vertices; return up to a 30-line excerpt. No other code." — purpose: ground the faithful C fix
- Context cost: S (if A/B) / M (if C)
- Authoritative docs: `docs/adr/0034-*.md` (faithfulness constraint).
- OrcaSlicer refs: determined by Step 1's verdict.
- Verification: per-mechanism narrow compile + the relevant thin-strip test; e.g. `cargo test -p
  arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd 2>&1 | tee target/test-output-
  thin-flag.log | tail -5; grep -q '^test result: ok' target/test-output-thin-flag.log` (AC-2).
- Exit condition: fix compiles; the targeted thin-strip test(s) pass.

### Step 3: Validate the fix across all 4 thin-strip tests + G4

- Task IDs: none
- Objective: Confirm the faithful fix makes all 4 thin-strip tests (AC-2, AC-3, AC-4) and the G4
  test (AC-5) GREEN, and that the faithfulness gates (AC-N1, AC-N2) hold.
- Precondition: Step 2 green.
- Postcondition: AC-2, AC-3, AC-4, AC-5, AC-N1, AC-N2 all green.
- Files allowed to read: all 4 thin-strip test files + G4 test; the edited source file.
- Files allowed to edit (≤ 3): only golden/fixture JSON files if a legitimate re-bless is needed
  due to corrected (now-faithful) output — NOT to mask a still-broken test.
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` (no new delegation needed here);
  classic-perimeters; the D-105/B/C/E fixes.
- Expected sub-agent dispatches:
  - "Run `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd --test
    arachne_parity_thin_wall_loop_type_tdd` and `cargo test -p slicer-runtime --test arachne_parity
    --test arachne_parity_gaps`; return FACT pass/fail per test." — purpose: gate AC-2..AC-5
  - "Grep `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` for any re-introduced
    `from_polygons_with_beading` / `2 * optimal_width` subdivision; return CLEAN or the offending
    lines." — purpose: AC-N1 gate
- Context cost: S
- Authoritative docs: none beyond Step 2.
- OrcaSlicer refs: none (validation only).
- Verification:
  - AC-2: `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd 2>&1 | tee
    target/test-output-thin-flag.log | tail -5; grep -q '^test result: ok' target/test-output-thin-flag.log`
  - AC-3: `cargo test -p arachne-perimeters --test arachne_parity_thin_wall_loop_type_tdd 2>&1 |
    tee target/test-output-thin-loop.log | tail -5; grep -q '^test result: ok' target/test-output-thin-loop.log`
  - AC-4: `cargo test -p slicer-runtime --test arachne_parity 2>&1 | tee target/test-output-runtime-
    thin.log | tail -5; grep -q '^test result: ok' target/test-output-runtime-thin.log`
  - AC-5: `cargo test -p slicer-runtime --test arachne_parity_gaps --
    arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width --exact 2>&1 | tee target/test-output-g4.log
    | tail -5; grep -q '^test result: ok' target/test-output-g4.log`
  - AC-N1: `rg -L 'from_polygons_with_beading|subdivide.*2 \* optimal_width'
    crates/slicer-core/src/skeletal_trapezoidation/graph.rs || echo CLEAN`
  - AC-N2: `rg -q 'OrcaSlicer' .ralph/specs/154-arachne-thin-strip-collapse/design.md`
- Exit condition: all six criteria green.

### Step 4: Workspace gate (compile / clippy / guest coherence)

- Task IDs: none
- Objective: Confirm the fix does not break the broader workspace (no classic-perimeters
  regression, no guest-WASM incoherence).
- Precondition: Step 3 green.
- Postcondition: `cargo check` / `cargo clippy` / `cargo xtask build-guests --check` clean.
- Files allowed to read: none beyond the edited file.
- Files allowed to edit (≤ 3): none (verification only).
- Files explicitly out-of-bounds: all source not already edited.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D
    warnings`, `cargo xtask build-guests --check`; return FACT per command." — purpose: gate
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: the three gate commands above.
- Exit condition: all three green.

### Step 5: Re-bless goldens + close D-105D (and G4 note)

- Task IDs: none
- Objective: Re-record the 6 stale goldens (4 thin-strip + G4) against verified OrcaSlicer-parity
  behavior; close `D-105D` in `docs/DEVIATION_LOG.md` with the verified root cause and faithful
  mechanism; record the investigation outcome under `docs/18_arachne_parity_audit.md` §G4. If
  Step 1 concluded Candidate D (OrcaSlicer identical), this step IS the fix (golden re-blessing,
  no source change) and still satisfies CR-1 via re-blessed goldens.
- Precondition: Steps 3-4 green (or, for Candidate D, Step 1 green).
- Postcondition: AC-6 green; all 6 goldens re-blessed; D-105D closed; G4 note recorded.
- Files allowed to read: `docs/DEVIATION_LOG.md` (D-105D + surrounding rows full);
  `docs/18_arachne_parity_audit.md` §G4; the 6 golden test files.
- Files allowed to edit (≤ 3 per sub-pass; treat each as its own bounded edit set): the 6 golden
  fixture JSON files (re-record); `docs/DEVIATION_LOG.md`; `docs/18_arachne_parity_audit.md`; the
  test files only if a genuine assertion change is warranted (rare — prefer re-blessing fixtures).
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` (no new delegation); classic-perimeters;
  the D-105/B/C/E fixes.
- Expected sub-agent dispatches:
  - "Re-record golden X via its documented `#[ignore]`d `record_*` function; confirm old-vs-new
    values reflect the faithful (or OrcaSlicer-identical) behavior; report the delta." — purpose:
    per-golden re-bless (repeated for each of the 6)
  - "Draft the D-105D closure text: verified root cause + faithful mechanism + OrcaSlicer
    file:line; return the exact markdown to insert (status Closed)." — purpose: D-105D closure
- Context cost: S
- Authoritative docs: `docs/DEVIATION_LOG.md`, `docs/18_arachne_parity_audit.md`.
- OrcaSlicer refs: none (documentation/fixture step).
- Verification:
  - AC-6: `rg -q 'D-105D' docs/DEVIATION_LOG.md && rg -q 'Closed' docs/DEVIATION_LOG.md`
  - AC-2..AC-5 re-confirmed after re-bless: the same commands as Step 3.
  - `rg -q 'thin-strip' docs/18_arachne_parity_audit.md` (G4 note present)
- Exit condition: AC-6 green; all 6 goldens re-blessed and committed; D-105D closed; G4 note
  recorded; `packet.spec.md` ready to move from `status: active` to `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1: Diagnose root cause | M | 4 delegated OrcaSlicer reads + local `BeadingPropagation` trace; gates everything |
| Step 2: Faithful fix (mechanism-specific) | S (A/B) / M (C) | TBD pending Step 1 verdict |
| Step 3: Validate 4 thin-strip + G4 | S | AC-2..AC-5 + AC-N1/AC-N2 |
| Step 4: Workspace gate | S | check / clippy / build-guests --check |
| Step 5: Re-bless goldens + close D-105D | S | 6 goldens + deviation-log + G4 note |

Sum: M aggregate (Step 1 dominates). No step is L. If Step 1 concludes Candidate D, Steps 2-4 are
skipped and the cost drops to S.

## Packet Completion Gate

- Step 1 findings exist and name the responsible mechanism.
- Steps 2-4 complete (or skipped via Candidate D) with AC-2, AC-3, AC-4, AC-5, AC-N1, AC-N2 green.
- Step 5 complete: all 6 goldens re-blessed; D-105D closed; G4 note recorded.
- `cargo xtask build-guests --check` clean (or no guest-relevant edit, with the check run as
  precaution).
- No `from_polygons_with_beading` / `2 * optimal_width` subdivision re-introduced (AC-N1).
- The mechanism is traceable to a specific OrcaSlicer case (AC-N2).
- `packet.spec.md` ready to move from `status: active` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green (check / clippy / build-guests --check).
- Record the verified root cause and faithful mechanism explicitly before moving to
  `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%.
- If Step 1 concluded Candidate D, explicitly record that the "fix" was golden re-blessing (no
  code change) with the OrcaSlicer file:line evidence that OrcaSlicer behaves identically.
