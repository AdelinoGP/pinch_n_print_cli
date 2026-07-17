# Implementation Plan: 177-arachne-baselines-to-structural-invariants

## Execution Rules

- Work one atomic step at a time; this packet has no `TASK-###` mapping (audit-driven, `backlog_source: docs/DEVIATION_LOG.md D-112-SELFCAPTURED-BASELINES`).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Steps 1a → 1b → 2 → 3 are strictly ordered and are the packet's thesis.** Steps 4-9 follow Step 3. If execution must be staged (see `design.md` §Context Cost Estimate), cut between Step 3 and Step 4, carrying the measured threshold table forward.

## Steps

### Step 1: Untangle the odd `max_bead_count` blocker

**Step 1 is split into 1a (production defaults) and 1b (test helpers) because the real inventory spans more files than the ≤3-file edit cap allows.** Run the inventory command first; it, not this packet, defines the surface.

#### Step 1a: production defaults

- Task IDs: none (backlog_source packet).
- Objective: make both **production** `max_bead_count` defaults even, so no measurement or invariant inherits `LimitedBeadingStrategy.cpp::compute`'s giant-centre-bead artifact.
- Precondition: `rg -n 'max_bead_count:\s*9' crates/slicer-core/src/` returns a non-empty inventory. **Re-derive it; do not trust any list in this packet.** At authoring time it was two production sites: `ArachneParams::default()` (`crates/slicer-core/src/arachne/pipeline.rs`) and `BeadingFactoryParams::default()` (`crates/slicer-core/src/beading/factory.rs`). If the inventory differs, the inventory wins.
- Postcondition: `rg -n 'max_bead_count:\s*9' crates/slicer-core/src/` returns empty; both structs' doc comments no longer document `max_bead_count` = 9 (`ArachneParams`'s doc comment states it explicitly and will otherwise go stale).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/arachne/pipeline.rs` - locate `ArachneParams::default()`, ±40 lines
  - `crates/slicer-core/src/beading/factory.rs` - locate `BeadingFactoryParams::default()`, ±40 lines
  - `crates/slicer-core/tests/bead_count.rs` - locate `factory_params()`, ±20 lines, read-only: documents the already-landed correction and the canonical rationale
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/arachne/pipeline.rs`
  - `crates/slicer-core/src/beading/factory.rs`
- Files explicitly out of bounds:
  - all `crates/slicer-core/tests/**` - Step 1b's surface
  - `modules/core-modules/*/module.toml` - the manifest sentinel already landed
  - `OrcaSlicerDocumented/` - delegate only
- Expected sub-agent dispatches:
  - Question: does canonical `WallToolPaths.cpp::generate` derive `max_bead_count = 2 * inset_count` unconditionally, and what is `LimitedBeadingStrategy.cpp::compute`'s odd-value behaviour?; scope: `OrcaSlicerDocumented/`; return: `FACT` (5 lines max)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/arachne-parity-recovery.md` - delegated SUMMARY of the odd-`max_bead_count` entry only
- OrcaSlicer refs:
  - `WallToolPaths.cpp::generate`, `LimitedBeadingStrategy.cpp::compute` - delegate; never load; cite by function, never by line
- Verification:
  - `rg -c 'max_bead_count:\s*9' crates/slicer-core/src/ ; test $? -eq 1 && echo CLEAN || echo ODD-REMAINS` - FACT
  - `cargo check -p slicer-core --all-targets` - FACT pass/fail
- Exit condition: zero odd production sites remain. **`BeadingFactoryParams::default()` is the site an earlier draft of this packet missed entirely** — confirm it is covered before claiming this step done.

#### Step 1b: test helpers

- Task IDs: none.
- Objective: make every surviving odd `max_bead_count` test helper even, so no converted fixture is measured against the trap.
- Precondition: Step 1a complete; `rg -n 'max_bead_count:\s*9' crates/slicer-core/tests/` returns the surface. At authoring time: `propagation.rs`, `arachne_annulus_split.rs`, `arachne_beding_propagation_side_table.rs`, `arachne_junction_upward_half_edge_only.rs`. **Re-derive; four files exceeds the edit cap, so iterate in sub-passes of ≤3.**
- Postcondition: `rg -n 'max_bead_count:\s*9' crates/slicer-core/` (whole crate, both src and tests) returns empty.
- Files allowed to read, with ranges when over 300 lines:
  - each file the inventory names - locate its `factory_params()`/params literal, ±40 lines
- Files allowed to edit (at most 3 per sub-pass; iterate until the inventory is empty):
  - the test files the inventory names
- Files explicitly out of bounds:
  - `crates/slicer-core/tests/generate_toolpaths.rs`, `crates/slicer-core/tests/bead_count.rs` - already carry `10`; editing them re-does landed work
  - `crates/slicer-core/src/**` - Step 1a's surface, already done
- Expected sub-agent dispatches:
  - Question: result lines for the affected test binaries after the even-value change?; scope: cargo; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/arachne-parity-recovery.md` - delegated SUMMARY; the entanglement claim
- OrcaSlicer refs:
  - `LimitedBeadingStrategy.cpp::compute` - delegate; never load
- Verification:
  - `rg -c 'max_bead_count:\s*9' crates/slicer-core/ ; test $? -eq 1 && echo CLEAN || echo ODD-REMAINS` - FACT: expect CLEAN
  - `cargo test -p slicer-core --features host-algos --test propagation 2>&1 | rg '^test result'` - FACT pass/fail
- Exit condition: the crate-wide inventory is empty **and** the affected binaries pass with unchanged test counts. If the even value changes results, that is a real finding about the odd default — record it; **do not revert to odd to keep tests green.**

### Step 2: Measure coverage-ratio-vs-classic across the corpus

- Task IDs: none.
- Objective: produce the measured table in `design.md` §Measured Coverage Baseline — one observed coverage ratio per fixture, at an aligned Z plane — and from it derive `observed_min`, a margin, and the threshold, with the margin justified in prose.
- Precondition: Steps 1a and 1b complete. Measuring against an odd default bakes the giant-centre-bead artifact into the threshold permanently and undetectably.
- Postcondition: every row of the table is filled or explicitly marked not-applicable with a reason; `observed_min`, margin, threshold, and the margin justification are written into `design.md`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_d5_taper_coverage.rs` - full (short); the existing coverage-ratio measurement shape to generalize
  - `crates/slicer-core/tests/arachne_invariants.rs` - the fixture helpers (`square_10mm`, `rectangle_20x10mm`, `wedge_trapezoid`, `simple_fixtures`, `build_propagated_graph`, `mm`) to reuse
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/config.json` - 4 lines; shows `"wall_generator": "arachne"`
- Files allowed to edit (at most 3):
  - `.ralph/specs/177-arachne-baselines-to-structural-invariants/design.md` (the measurement table only)
  - a temporary measurement harness under `crates/slicer-core/tests/` (may be folded into `arachne_invariants.rs` in Step 3)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/expected_perimeter_ir.json` - roughly 71 KB; never load
  - `docs/18_arachne_parity_audit.md` - exists and is git-tracked; out of scope, do not load, do not cite as deviation authority
  - `target/`, `Cargo.lock`
- Expected sub-agent dispatches:
  - Question: which config key/value selects the classic wall generator and where is it resolved?; scope: `modules/core-modules/*/module.toml`, `crates/slicer-core/src/algos/region_mapping.rs`; return: `LOCATIONS` <=20
  - Question: measurement-harness run output — per-fixture coverage ratio only; scope: cargo; return: `FACT` (one ratio per line, <=12 lines)
- Context cost: `M` (largest step: ten fixtures × two generators)
- Authoritative docs:
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` - the D3 Z-alignment corollary; ranged read or delegated SUMMARY
  - `docs/08_coordinate_system.md` - ranged read; mm↔unit boundaries in the ratio computation
- OrcaSlicer refs: none this step (the reference is in-tree classic, not Orca).
- Verification:
  - `rg -q 'Chosen threshold' .ralph/specs/177-arachne-baselines-to-structural-invariants/design.md` - FACT: present and numeric
  - measurement harness run - FACT: <=12 lines, one ratio per fixture
- Exit condition: the table is filled and the threshold is written **with its margin justified in prose**. A number without a justification does not satisfy this step. **Falsifying stop:** if `observed_min - margin` would admit `0.668`, do not lower the threshold to fit — record the `[BLOCK]` already anticipated in `design.md` §Open Questions and escalate.

### Step 3: Encode the coverage threshold and prove it discriminates D5

- Task IDs: none.
- Objective: encode Step 2's threshold as a named `const` in `crates/slicer-core/tests/arachne_invariants.rs`, land the coverage-ratio-vs-classic invariant, and prove the threshold rejects `0.668` and admits `0.990` without needing a live geometry run.
- Precondition: Step 2's table and threshold are committed in `design.md`. **If the table is empty, stop.** Do not re-measure here; do not invent a number.
- Postcondition: AC-3, AC-4, AC-5 and AC-N1 pass. The threshold in code equals the threshold in the table.
- Files allowed to read, with ranges when over 300 lines:
  - `.ralph/specs/177-arachne-baselines-to-structural-invariants/design.md` - §Measured Coverage Baseline only
  - `crates/slicer-core/tests/arachne_d5_taper_coverage.rs` - full; the assertion-message convention AC-5 requires
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/arachne_invariants.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/tests/arachne_d5_taper_coverage.rs` - read-only prior art this step; its own `>= 0.90` input-bbox assertion is not this packet's gate
  - `OrcaSlicerDocumented/`, `target/`
- Expected sub-agent dispatches:
  - Question: `cargo test -p slicer-core --features host-algos --test arachne_invariants` pass/fail and failing assertion text?; scope: cargo; return: `FACT` pass/fail, <=20 lines on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` - the invariant-class table row; ranged read
- OrcaSlicer refs: none this step.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_invariants -- coverage_threshold_rejects_d5_broken_ratio --nocapture` - FACT pass/fail
  - `cargo test -p slicer-core --features host-algos --test arachne_invariants -- tapered_wedge_coverage_vs_classic --nocapture` - FACT pass/fail
  - `cargo test -p slicer-core --features host-algos --test arachne_invariants -- coverage_invariant_rejects_synthetic_d5_regression --nocapture` - FACT pass/fail
- Exit condition: the threshold const rejects `0.668`, admits `0.990`, and the synthetic-regression negative **fails when fed 0.668** — an invariant that cannot fail is not an invariant. **This step is the packet's thesis. If the packet ships nothing else, it must ship this.**

### Step 4: Land the bead-width cap invariant

- Task IDs: none.
- Objective: assert no bead wider than roughly `2 * optimal_width` (D4 saw 19.6 mm beads on a 0.45 mm nozzle), covering the ADR-0042 width class and giving `toolpaths_tapered_wedge.json`'s absolute `junction_widths_mm` a structural successor.
- Precondition: Step 3 complete; `optimal_width`'s in-tree derivation located.
- Postcondition: AC-N2 passes; the invariant runs over the `simple_fixtures` corpus.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_invariants.rs` - the helper block
  - `crates/slicer-core/src/arachne/pipeline.rs` - locate `ArachneParams` width fields, ±40 lines
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/arachne_invariants.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/`, `target/`, other packet dirs
- Expected sub-agent dispatches:
  - Question: how does `BeadingStrategyFactory.cpp::makeStrategy` select `optimal_width`, and is it `max_bead_count <= 2 ? outer : inner`?; scope: `OrcaSlicerDocumented/`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` - the "no bead wider than ~2× optimal width" section; ranged read
- OrcaSlicer refs:
  - `BeadingStrategyFactory.cpp::makeStrategy` - delegate; never load; cite by function
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_invariants -- bead_width_invariant_rejects_oversized_bead --nocapture` - FACT pass/fail
- Exit condition: a synthetic 19.6 mm bead on a 0.45 mm nozzle fails the invariant with a message naming the offending width and the cap.

### Step 5: Convert the centrepiece — the tapered-wedge parity case (green, already re-blessed)

- Task IDs: none.
- Objective: make `arachne_perimeter_parity`'s `tapered_wedge` case assert structural invariants instead of absolute-coordinate equality — **without re-capturing the baseline again**. **Read the `[BLOCK]` in `design.md` §Open Questions before starting:** the brief's "deliberately RED" premise is falsified — the test passes today and the fixture was already re-blessed by `9ca62ba0`. Its greenness is the defect, not the goal; do not treat a green module result line as this step's exit.
- Precondition: Steps 3-4 complete (the coverage and width invariants exist to assert *with*).
- Postcondition: the `perimeter_parity` module reports zero failures; `expected_perimeter_ir.json` is deleted or byte-identical to its pre-packet content.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs` - locate `arachne_perimeter_parity`, ±60 lines. **Do not browse the ten `record_*` recorders.**
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs`
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/expected_perimeter_ir.json` (**deletion only** — never rewrite)
- Files explicitly out of bounds:
  - the content of `expected_perimeter_ir.json` - roughly 71 KB; never load. Its state is already known and needs no read: the file holds `9.82146` at `regions[0].walls[0].path.points[1].x` and **contains no `3.7797625`** — it was already re-blessed to the post-D5 "emit more" value by `9ca62ba0`, so the briefed "actual vs expected" mismatch no longer exists on disk. A further re-record would compound that; convert instead. See the `[BLOCK]` in `design.md` §Open Questions.
  - `record_tapered_wedge` and sibling recorders - never invoke against this fixture
  - the other four `perimeter_parity` Arachne fixtures (`narrow_strip_widening`, `max_bead_count_cap`, `complex_multi_feature`, `cube_4color_arachne`) - do not touch
- Expected sub-agent dispatches:
  - Question: `cargo test -p slicer-runtime --test integration perimeter_parity` result line and any failing assertion?; scope: cargo; return: `FACT` pass/fail, <=20 lines on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/arachne-parity-recovery.md` - delegated SUMMARY: the "remedy is Track B's convert to a structural invariant, **not** a blind rebless" mandate
- OrcaSlicer refs: none this step.
- Verification:
  - `cargo test -p slicer-runtime --test integration perimeter_parity 2>&1 | rg '^test result'` - FACT pass/fail
  - `git diff --stat HEAD -- crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/expected_perimeter_ir.json` - FACT: empty or a pure deletion
- Exit condition: the named structural test passes **and** the no-further-rebless gate is clean. A green test with a modified baseline is a **failure of this step**, not a pass. The fixture was already re-blessed once (`9ca62ba0`) — do not repeat it.

### Step 6: Convert and reframe the eight slicer-core fixtures

- Task IDs: none.
- Objective: replace blob-equality with property assertions in the fixture consumers, using the structural data the fixtures already store (`edge_count`, per-vertex `central` bools, `has_transition`, `bead_counts`).
- Precondition: Step 3 complete. Fixture format is **not** redesigned — this is reframing plus real assertions.
- Postcondition: AC-8 passes; `centrality`, `propagation`, `bead_count` and `generate_toolpaths` binaries green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/centrality.rs`, `crates/slicer-core/tests/propagation.rs`, `crates/slicer-core/tests/bead_count.rs`, `crates/slicer-core/tests/generate_toolpaths.rs` - locate the fixture-consuming assertions, ±40 lines each
  - `crates/slicer-core/tests/fixtures/arachne/*.json` - **bounded extraction of `provenance` and field names only**; never load a whole fixture
- Files allowed to edit (at most 3 per sub-pass; iterate file by file):
  - `crates/slicer-core/tests/centrality.rs`
  - `crates/slicer-core/tests/propagation.rs`
  - `crates/slicer-core/tests/bead_count.rs`
  - `crates/slicer-core/tests/generate_toolpaths.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/tests/{stitch,simplify,remove_small}.rs` - inline consts only; nothing to convert
  - `OrcaSlicerDocumented/`, `target/`
- Expected sub-agent dispatches:
  - Question: `cargo test -p slicer-core --features host-algos --test centrality --test propagation --test bead_count --test generate_toolpaths` result lines?; scope: cargo; return: `FACT` pass/fail
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` - invariant-class table; ranged read
- OrcaSlicer refs: none this step.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test centrality --test propagation --test bead_count 2>&1 | rg '^test result'` - FACT pass/fail
  - `cargo test -p slicer-core --features host-algos --test generate_toolpaths 2>&1 | rg '^test result'` - FACT pass/fail
- Exit condition: every fixture-consuming assertion states an ADR-0042 invariant class rather than a blob comparison. **Special attention:** `propagation_fills_gap_from_central_neighbor` (`crates/slicer-core/tests/propagation.rs`) is the test that asserted the D5 defect and passed — it must not survive this step in a form that could do so again. `toolpaths_tapered_wedge.json`'s absolute `junction_widths_mm` becomes Step 4's cap assertion; the captured widths survive only as labelled change-detector data.

### Step 7: Demote surviving baselines to labelled change-detectors

- Task IDs: none.
- Objective: normalize `provenance` across all eight fixtures so every surviving self-captured baseline is explicitly a change-detector, not an oracle.
- Precondition: Step 6 complete; the `[BLOCK]` on change-detector CI authority (`design.md` §Open Questions) resolved.
- Postcondition: AC-7 passes — all eight carry the literal `CHANGE-DETECTOR, NOT a correctness oracle -- ADR-0042`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/fixtures/arachne/*.json` - `provenance` string only, via bounded extraction
- Files allowed to edit (at most 3 per sub-pass; iterate):
  - the six centrality/propagation fixtures under `crates/slicer-core/tests/fixtures/arachne/`
- Files explicitly out of bounds:
  - `bead_count_tapered_wedge.json`, `toolpaths_tapered_wedge.json` - **already carry the exact literal**; do not rewrite their provenance
  - every non-`provenance` field of every fixture - this step touches provenance strings and nothing else
- Expected sub-agent dispatches: none — mechanical, bounded string edits.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` - the change-detector framing; ranged read
- OrcaSlicer refs: none this step.
- Verification:
  - `rg -L -c 'CHANGE-DETECTOR, NOT a correctness oracle -- ADR-0042' crates/slicer-core/tests/fixtures/arachne/*.json` - FACT: expect no file reporting 0
- Exit condition: all eight carry the literal; the six older D-109-era wordings are gone; no non-provenance field changed (verify with `git diff --stat`).

### Step 8: Hygiene — rehome the red files, correct the stale header

- Task IDs: none.
- Objective: move the nine `crates/slicer-core/tests/arachne_parity_red_*.rs` files into stage-grouped homes preserving every test name, and correct `crates/slicer-runtime/tests/arachne_parity.rs`'s false `fails on purpose` header.
- Precondition: Steps 3-7 complete. **Capture the pre-move test-name baseline first — AC-9 diffs against it and cannot be satisfied retroactively:** `cargo test -p slicer-core --features host-algos -- --list 2>/dev/null | rg ': test$' | sort > /tmp/pre.txt`. This step carries no behavioural claim and is the first thing to cut under context pressure.
- Postcondition: AC-9 and AC-10 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_parity_red_*.rs` (9) - module headers and test names only
  - `crates/slicer-runtime/tests/arachne_parity.rs` - the module header, first ~30 lines
- Files allowed to edit (at most 3 per sub-pass; iterate):
  - the nine red files (moves)
  - `crates/slicer-runtime/tests/arachne_parity.rs` (header only)
- Files explicitly out of bounds:
  - the body of any test in `crates/slicer-runtime/tests/arachne_parity.rs` - the D-104f concentric-infill case stays open; its header is corrected, its status is not
  - `docs/18_arachne_parity_audit.md` - exists and is git-tracked; out of scope, do not load, do not cite as deviation authority
- Expected sub-agent dispatches:
  - Question: how many of `crates/slicer-runtime/tests/arachne_parity.rs`'s tests currently pass, and which fail?; scope: cargo; return: `FACT` (counts + failing names). Re-derive rather than trusting the "14/15 green, only D-104f open" figure — it is a ledger fact and will rot.
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - delegated FACT on D-104f's status only
- OrcaSlicer refs: none this step.
- Verification:
  - `ls crates/slicer-core/tests/arachne_parity_red_*.rs 2>&1 | rg -q 'No such file' && echo MOVED` - FACT
  - `! rg -q 'fails on purpose' crates/slicer-runtime/tests/arachne_parity.rs && echo HEADER-OK || { echo STALE-FAIL; false; }` - FACT: expect HEADER-OK, exit 0
  - `cargo test -p slicer-core --features host-algos --test arachne_invariants 2>&1 | rg '^test result'` - FACT pass/fail
- Exit condition: no `arachne_parity_red_*.rs` path remains, every moved test name is still collected, and the corrected header names the D-104f case as the sole open red test **using counts re-derived in this step**, not quoted from this packet.

### Step 9: Close the deviation and instantiate the ADR

- Task IDs: none.
- Objective: transition `D-112-SELFCAPTURED-BASELINES` to Closed with the measured threshold, and record the threshold + margin justification in ADR-0042 as its first concrete instantiation.
- Precondition: Steps 1-8 complete; every pipe-suffixed AC green.
- Postcondition: both Doc Impact greps in `packet.spec.md` pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - **`rg` for the row ID only.** Rows are single lines of many thousands of characters; a range read can blow the budget on one row.
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` - the Consequences section only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md` (the `D-112-SELFCAPTURED-BASELINES` row only)
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` (Consequences only)
- Files explicitly out of bounds:
  - every other row of `docs/DEVIATION_LOG.md`
  - every other ADR
  - `docs/18_arachne_parity_audit.md` - exists and is git-tracked; out of scope, do not load, do not cite as deviation authority
- Expected sub-agent dispatches:
  - Question: current Status text of the `D-112-SELFCAPTURED-BASELINES` row?; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (<=5 lines — the verdict, **never** the row)
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - delegated FACT only
- OrcaSlicer refs: none this step.
- Verification:
  - `rg -q 'D-112-SELFCAPTURED-BASELINES.*Closed' docs/DEVIATION_LOG.md` - FACT
  - `rg -q 'measured coverage threshold' docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` - FACT
- Exit condition: the deviation row states Closed with the measured threshold, and ADR-0042 carries the threshold with its margin justification. If any AC is red, the row stays Open — a closed deviation over a red gate is exactly the class of ratification this packet exists to end.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1a | S | Blocker; both production defaults incl. `beading/factory.rs`. Strictly first. |
| Step 1b | S | Blocker; test helpers, ≤3-file sub-passes (re-derive inventory). |
| Step 2 | M | Largest: 10 fixtures × 2 generators. Produces the threshold. |
| Step 3 | M | Thesis. Encodes threshold + D5 discrimination. |
| Step 4 | S | Bead-width cap invariant. |
| Step 5 | M | Centrepiece; no-rebless gate. |
| Step 6 | M | Corpus reframing across 4 consumer files. |
| Step 7 | S | Provenance strings only. |
| Step 8 | S | Hygiene; first to cut under pressure. |
| Step 9 | S | Doc closure. |

Aggregate: `M`. No single step is `L`. The packet is not split, by explicit user decision (`requirements.md` §Out of Scope); `/swarm` should stage **execution** at the Step 3 / Step 4 boundary, carrying `design.md`'s measured threshold table as the handoff interface.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS — including AC-N1's proof that the coverage invariant can fail.
- `docs/07_implementation_status.md`: no task ID applies to this packet; if a status touch is nonetheless warranted, do it through a worker dispatch, never a full backlog read.
- `D-112-SELFCAPTURED-BASELINES` transitioned to Closed; ADR-0042 carries the measured threshold.
- Both `[BLOCK]` questions in `design.md` §Open Questions resolved.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run the full suite **once**, via `cargo xtask test --workspace --summary`, dispatched to a subagent returning `FACT pass/fail` — never absorbed inline. Self-captured baselines are exactly the class of thing a narrow green run misses; D-112's own history records a narrow-scope closure that a later full run falsified.
- Run `cargo xtask build-guests --check` before attributing any guest/component/dispatch failure to this packet's changes. This packet touches no WIT, but a stale guest is your bug until `--check` proves otherwise.
- Confirm the threshold in `crates/slicer-core/tests/arachne_invariants.rs` still equals the one in `design.md`'s table.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
