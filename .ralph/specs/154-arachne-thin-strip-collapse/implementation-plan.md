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
- Objective: Determine which of the **revised** candidates (A′ / B′ / C′ / D′ — see `design.md`
  §Controlling Code Paths) is responsible for the thin-strip collapse. Reproduce the failure across
  the 4 thin-strip tests + G4, then test A′ and B′ locally in that order. **Read `design.md`
  §Canonical Facts first** — C-1…C-8 already answer the OrcaSlicer questions the draft planned to
  dispatch (spine is seg-seg branch 1; canonical emits zero junctions on a flat spine; PnP already
  matches; `collapseSmallEdges` snap_dist is correctly converted). Re-dispatching them is wasted
  budget. Write the verdict into `design.md` §Step 1 Findings in the exact `verdict: <X>` form.
- Precondition: none (first step).
- Postcondition: AC-1 green — §Step 1 Findings carries a `verdict:` line naming one mechanism with
  `file:line` evidence.
- Files allowed to read: `crates/slicer-core/src/arachne/generate_toolpaths.rs` (Candidates A′/C′);
  `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` (Candidate B′); the 4 thin-strip test
  files + G4 test file; `docs/DEVIATION_LOG.md` D-105D; `docs/adr/0034-*.md`;
  `docs/18_arachne_parity_audit.md` §G4.
- Files allowed to edit (≤ 3): `.ralph/specs/154-arachne-thin-strip-collapse/design.md` (append
  findings). No source edit in this step.
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` — delegate; `propagation.rs` and
  `graph.rs::discretize_edge` (draft candidates B and C, retired by F-3/F-4); classic-perimeters;
  the D-105/B/C/E fixes.
- Expected sub-agent dispatches:
  - "Run `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd`, `cargo
    test -p slicer-runtime --test arachne_parity`, and `cargo test -p slicer-runtime --test
    arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width --exact`;
    return SNIPPETS (≤ 20 lines each) of the failing assertion and the wall-loop state (length,
    junction count, `is_closed`)." — purpose: establish failure shape
  - "In `crates/slicer-core/src/arachne/generate_toolpaths.rs`, using the single-edge-domain
    fixture at `:1112`: does `resolve_to_vertex` (`:149`) / `quad_peak_position` (`:491`) resolve
    EVERY quad's peak to the same vertex for a thin strip? Return FACT: distinct peak-vertex count
    + the per-bead junction positions emitted." — purpose: resolve Candidate A′ (prime suspect)
  - "In `crates/slicer-core/src/skeletal_trapezoidation/rib.rs` `build_quad_rib_topology` (`:101`):
    for a thin rectangle, how many rib edges exist and what is their endpoints'
    `distance_to_boundary` range? Return FACT (rib count + R range)." — purpose: resolve Candidate
    B′ — per C-5 these R-varying edges must carry ALL of a thin strip's junctions
  - **(No OrcaSlicer dispatch required.)** The `connectJunctions` chaining rule is pinned in C-5;
    C-7 near-exonerates C′ and C-8 near-kills D′. Delegate only if a finding contradicts C-5.
  - "In `crates/slicer-core/src/arachne/generate_toolpaths.rs` `emit_chain_lines` (`:693`): does it
    append successive rib-quad segments onto the TAIL of the same `ExtrusionLine` using a ~0.01 mm
    proximity join (canonical `addToolpathSegment`, `SkeletalTrapezoidation.cpp:1906-1925`), or does
    it start a new line per quad? Return FACT + the join tolerance if any." — purpose: the
    highest-value single check per C-5
- Context cost: M
- Authoritative docs: `design.md` §Canonical Facts (read first); `docs/DEVIATION_LOG.md` (D-105D),
  `docs/adr/0034-*.md`, `docs/18_arachne_parity_audit.md` §G4.
- OrcaSlicer refs: **none** — C-1…C-8 (including the `connectJunctions` chaining rule) are all
  pre-answered in `design.md`.
- Verification: `rg -q "verdict: (A′|B′|C′|D′)" .ralph/specs/154-arachne-thin-strip-collapse/design.md`
  — dispatch as FACT pass/fail (AC-1).
- Exit condition: AC-1 green; §Step 1 Findings carries one `verdict:` line with `file:line`
  evidence. If D′, skip to Step 5 — but only with positive OrcaSlicer `file:line` evidence that
  canonical also degenerates (C-5 says it should not). Otherwise proceed to Steps 2-4.

### Step 2: Implement the faithful fix (mechanism-specific, TBD pending Step 1)

- Task IDs: none
- Objective: Implement the faithful fix for the mechanism Step 1 named. Surface by verdict:
  **A′** → the quad chain walk in `generate_toolpaths.rs` (`resolve_to_vertex` `:149` /
  `quad_peak_position` `:491` / `chain_junctions_for_bead` `:536` / `emit_chain_lines` `:693`);
  **B′** → `rib.rs` `build_quad_rib_topology` (`:101`) vs canonical `graph.makeRib()`; **C′** →
  `generate_local_maxima_single_beads` (`generate_toolpaths.rs:803`) vs canonical
  `generateLocalMaximaSingleBeads` (`SkeletalTrapezoidation.cpp:1529`). The mechanism MUST be
  traceable to a specific OrcaSlicer `file:line` (AC-N3), MUST NOT reintroduce
  `from_polygons_with_beading` (AC-N1), MUST NOT add junction emission on flat/equal-R edges
  (AC-N2 — canonical skips them, C-3/C-4), and MUST NOT add interior nodes to the two-node
  seg-seg spine (AC-N3 — that topology is canonical, C-2).
- Precondition: Step 1 green (responsible mechanism named via a `verdict:` line).
- Postcondition: the targeted source fix is in place and compiles; AC-2..AC-5 trend toward green.
- Files allowed to read: the implicated candidate file (full) + its existing tests/fixtures; the
  relevant OrcaSlicer reference for the chosen case.
- Files allowed to edit (≤ 3): exactly one of `generate_toolpaths.rs` / `rib.rs` (Step 1's verdict
  decides); its fixture JSON if needed.
- Files explicitly out-of-bounds: `OrcaSlicerDocumented/` — delegate; the non-implicated candidate
  file; `propagation.rs` and `graph.rs::discretize_edge` (retired candidates, F-3/F-4);
  classic-perimeters; the D-105/B/C/E fixes.
- Expected sub-agent dispatches:
  - (delegated, if A′ or C′) "`OrcaSlicerDocumented/src/libslic3r/Arachne/
    SkeletalTrapezoidation.cpp:1934` `connectJunctions()` (and `:1529`
    `generateLocalMaximaSingleBeads` if C′): return a ≤30-line excerpt of the junction-accumulation
    walk across the `prev`/`next` quad chain. No other code." — purpose: ground the faithful port
  - (delegated, if B′) "`OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp`
    `makeRib()`: return a ≤30-line excerpt showing when a rib is inserted and what its endpoints
    are. No other code." — purpose: ground the faithful B′ fix
- Context cost: S (if A′) / M (if B′ or C′)
- Authoritative docs: `docs/adr/0034-*.md` (faithfulness constraint); `design.md` §Canonical Facts.
- OrcaSlicer refs: determined by Step 1's verdict (see the two dispatches above).
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
  - "Run the AC-N1/AC-N2/AC-N3 commands from `packet.spec.md` verbatim; return FACT pass/fail per
    command." — purpose: faithfulness gates (no reintroduced fabrication; the equal-R skip in
    `generate_junctions` survives; the mechanism cites an OrcaSlicer `file:line`)
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
  - AC-N1: `! rg -q 'from_polygons_with_beading' crates/slicer-core/src/skeletal_trapezoidation/graph.rs`
  - AC-N2: `rg -q 'if from_r >= to_r \{' crates/slicer-core/src/arachne/generate_toolpaths.rs`
  - AC-N3: `rg -q 'SkeletalTrapezoidation(Graph)?\.(cpp|hpp):[0-9]+' .ralph/specs/154-arachne-thin-strip-collapse/design.md`
- Exit condition: all seven criteria green (AC-2..AC-5, AC-N1..AC-N3).

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
  mechanism **and correct its symbol list** (the row cites `connectJunctions` /
  `getNextUnconnected` / `BeadingPropagation` as PnP symbols; none exists — `design.md` F-1/F-2/F-3,
  AC-6); **open a new deviation row `D-154-DISCRETIZE-POINT-POINT-CASE`** for the `discretize_edge` branch-1/branch-3
  conflation (F-4, AC-7); record the investigation outcome under `docs/18_arachne_parity_audit.md` §G4. If
  Step 1 concluded D′ (OrcaSlicer identical), this step IS the fix (golden re-blessing, no source
  change) and still satisfies CR-1 — but only with the positive OrcaSlicer `file:line` evidence
  Step 1's exit condition demands.
- Precondition: Steps 3-4 green (or, for D′, Step 1 green).
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
  - AC-6: `rg -q '^\| D-105D \|.*\| *Closed' docs/DEVIATION_LOG.md`
  - AC-7: `rg -q '^\| D-154-DISCRETIZE-POINT-POINT-CASE \|' docs/DEVIATION_LOG.md`
  - AC-2..AC-5 re-confirmed after re-bless: the same commands as Step 3.
  - `rg -q 'thin-strip' docs/18_arachne_parity_audit.md` (G4 note present)
- Exit condition: AC-6 and AC-7 green; all 6 goldens re-blessed; D-105D closed **with its symbol
  list corrected**; the `discretize_edge` gap filed as its own Open row; G4 note recorded;

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1: Diagnose root cause | S/M | 3 local traces (A′ peak-vertex, A′ chain-join, B′ ribs); ZERO OrcaSlicer dispatches — C-1…C-8 pre-answered; gates everything |
| Step 2: Faithful fix (mechanism-specific) | S (A/B) / M (C) | TBD pending Step 1 verdict |
| Step 3: Validate 4 thin-strip + G4 | S | AC-2..AC-5 + AC-N1/AC-N2 |
| Step 4: Workspace gate | S | check / clippy / build-guests --check |
| Step 5: Re-bless goldens + close D-105D | S | 6 goldens + deviation-log + G4 note |

Sum: M aggregate (Step 1 dominates, but is cheaper than the draft's — 3 of 4 OrcaSlicer dispatches
are pre-answered in `design.md` §Canonical Facts). No step is L. If Step 1 concludes D′, Steps 2-4 are
skipped and the cost drops to S.

## Packet Completion Gate

- Step 1 findings carry a `verdict: <X>` line naming the responsible mechanism (AC-1).
- Steps 2-4 complete (or skipped via D′) with AC-2..AC-5 and AC-N1..AC-N3 green.
- Step 5 complete: all 6 goldens re-blessed; D-105D closed with its symbol list corrected (AC-6);
  the `discretize_edge` branch-1/branch-3 gap filed as a new Open row (AC-7); G4 note recorded.
- `cargo xtask build-guests --check` clean (or no guest-relevant edit, with the check run as
  precaution).
- No `from_polygons_with_beading` subdivision re-introduced (AC-N1); the equal-R skip in
  `generate_junctions` survives (AC-N2); the mechanism cites an OrcaSlicer `file:line` (AC-N3).

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green (check / clippy / build-guests --check).
- Record the verified root cause and faithful mechanism explicitly before moving to
  `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%.
- If Step 1 concluded D′, explicitly record that the "fix" was golden re-blessing (no
  code change) with the OrcaSlicer file:line evidence that OrcaSlicer behaves identically.
