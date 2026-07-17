# Design: 177-arachne-baselines-to-structural-invariants

## Controlling Code Paths

- Primary code path: `ArachneParams::default()` (`crates/slicer-core/src/arachne/pipeline.rs`) — the sole production edit; the odd `max_bead_count` blocker.
- Primary test home: `crates/slicer-core/tests/arachne_invariants.rs` — **already exists** (four tests from packet 113c: `outer_wall_is_closed_ring_for_simple_polygons`, `quad_chains_span_two_or_three_edges`, `get_next_unconnected_chain_terminates_within_edge_count_bound`, `junction_count_delta_bound_at_domain_chain_stitches`), and its module docstring already states the by-construction thesis. **Extend it. Do not create a parallel home.** It already carries the helpers this packet needs — `square_10mm`, `rectangle_20x10mm`, `wedge_trapezoid`, `simple_fixtures`, `build_propagated_graph`, `mm` — reuse them rather than re-deriving fixtures.
- Prior art for the coverage invariant: `crates/slicer-core/tests/arachne_d5_taper_coverage.rs` — `d5_benchy_bow_cross_section_is_covered_by_arachne_walls` already computes an X-extent coverage ratio, but against the **input bbox** with a hardcoded `>= 0.90`, not against a classic reference. This packet's coverage invariant is the vs-classic generalization of that test. Read it first (it is short); it establishes the measurement shape, the fixture `crates/slicer-core/tests/fixtures/arachne/d5_benchy_call1.txt`, and the assertion-message convention AC-5 requires.
- Neighboring tests/fixtures: `crates/slicer-core/tests/{centrality,propagation,bead_count,generate_toolpaths}.rs` and their eight `crates/slicer-core/tests/fixtures/arachne/*.json` baselines; `arachne_perimeter_parity` in `crates/slicer-runtime/tests/integration/perimeter_parity.rs`; the nine `crates/slicer-core/tests/arachne_parity_red_*.rs` files; `crates/slicer-runtime/tests/arachne_parity.rs`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules here.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **Absolute-coordinate equality is a rejected assertion class** (ADR-0042). This is not stylistic: at 1 unit = 100 nm, an absolute-coordinate baseline encodes the unit convention itself into the fixture, so it fails on a legitimate unit-boundary fix and passes on a real geometric regression that happens to preserve coordinates. Every invariant this packet lands must be expressible as a ratio, a count, a topological property, or a tolerance — never as an equality against a captured coordinate.
- **Z alignment is a validity precondition** (ADR-0042's D3 corollary). Arachne-vs-classic comparisons hold only at aligned Z sampling planes; there is a roughly 0.25 mm label offset against Orca. Any measurement helper must sample both generators at the same plane and assert that alignment, not assume it.
- **Even `max_bead_count` is a canonical invariant, not a preference.** Canonical `WallToolPaths.cpp::generate` derives `2 * inset_count`, always even; `LimitedBeadingStrategy.cpp::compute`'s odd branch parks a giant centre bead. An odd default is a live trap that contaminates any baseline captured against it.

## Code Change Surface

- **Selected approach:** correct the blocker first; then measure the corpus against a `wall_generator=classic` reference and *record* the observations; then set the threshold from the recorded minimum; only then encode invariants. Conversion of the existing fixtures is **reframing plus real assertions**, not a format redesign — the fixtures already store the structural data.
- **Exact surfaces:**
  - `ArachneParams::default()` (`crates/slicer-core/src/arachne/pipeline.rs`) — `max_bead_count: 9` → even `2 * inset_count`; update the adjacent doc comment which currently documents `max_bead_count` = 9.
  - `factory_params()` (`crates/slicer-core/tests/propagation.rs`) — `max_bead_count: 9` → even. **The `factory_params()` helpers in `crates/slicer-core/tests/generate_toolpaths.rs` and `crates/slicer-core/tests/bead_count.rs` already carry `10`** and document the correction inline; they are out of bounds for Step 1. Re-derive with `rg -n 'max_bead_count:\s*9' crates/slicer-core/` before editing — that command, not this bullet, is the authority.
  - `crates/slicer-core/tests/arachne_invariants.rs` — extended with: a named coverage-threshold `const`, a coverage-ratio-vs-classic invariant, a bead-width cap invariant (no bead wider than roughly `2 * optimal_width`; D4 saw 19.6 mm beads on a 0.45 mm nozzle), a threshold-discrimination test (AC-4), and the two anti-vacuity negatives (AC-N1, AC-N2).
  - `crates/slicer-core/tests/fixtures/arachne/*.json` — `provenance` strings only. `bead_count_tapered_wedge.json` and `toolpaths_tapered_wedge.json` already carry the exact ADR-0042 change-detector literal; the six centrality/propagation fixtures carry only older D-109-era wording and must be normalized to the same literal.
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs` — the `tapered_wedge` case within `arachne_perimeter_parity`.
  - `crates/slicer-runtime/tests/arachne_parity.rs` — module header only (the `fails on purpose` sentence).
- **Rejected alternatives:**
  - *Rebless `expected_perimeter_ir.json`.* Rejected by `docs/specs/arachne-parity-recovery.md`, which states the remedy is Track B's conversion, **not** a blind rebless. Commit `1d7eb1de` re-captured two slicer-core fixtures and missed this third; reblessing would repeat exactly the error that produced the current RED state, and would erase the only evidence that the drift is in the correct direction.
  - *Hardcode the 0.95-class threshold from the D5 datapoints.* Rejected: the threshold is an output. D5's 66.8%/99.0% are sanity datapoints for AC-4's discrimination test, not the gate value. A guessed threshold either admits a real regression or blocks a legitimate fixture, and there is no way to tell which without the measurement.
  - *Create a new `arachne_structural_invariants.rs`.* Rejected: `arachne_invariants.rs` exists and already carries the thesis and the fixture helpers.
  - *Convert the fixture JSON format to a structural schema.* Rejected: `edge_count`, per-vertex `central` bools, `has_transition`, and `bead_counts` are already structural. The defect is that tests compare blobs instead of asserting properties — a test-side defect, not a format defect.

## Measured Coverage Baseline

**This table is empty by design. Step 2 fills it; Step 3 reads it. It is the packet's single most important artifact.**

The threshold is an **output** of measurement, not an input. Do not pre-fill this table, do not hardcode `0.95`, and do not proceed to Step 3 against an empty table.

| Fixture | Arachne X-extent (mm) | Classic X-extent (mm) | Coverage ratio | Z plane (mm) | Notes |
| --- | --- | --- | --- | --- | --- |
| `centrality_square` | | | | | |
| `centrality_wedge` | | | | | |
| `centrality_multi_feature` | | | | | |
| `propagation_uniform` | | | | | |
| `propagation_varying` | | | | | |
| `propagation_multi_feature` | | | | | |
| `bead_count_tapered_wedge` | | | | | |
| `toolpaths_tapered_wedge` | | | | | |
| `tapered_wedge` (perimeter_parity STL) | | | | | |
| `d5_benchy_call1` (bow cross-section) | | | | | |

- Observed minimum: _(fill in Step 2)_
- Chosen margin: _(fill in Step 2)_
- **Chosen threshold = observed_min - margin:** _(fill in Step 2)_
- Margin justification (required prose, not a number): _(fill in Step 2 — state what the margin absorbs: fixture-to-fixture geometric variation, Z-sampling granularity, or float noise, and why that magnitude and not another.)_

Sanity datapoints, for orientation only — **never** to be used as the threshold: D5 measured 66.8% broken → 99.0% fixed; benchy maxX tracks classic at roughly 1.001 (arachne slightly exceeds classic, so ratios may legitimately sit marginally above 1.0 and the invariant must be a floor, not an equality band).

## Files in Scope (read + edit)

More than three primary files. Justified: the packet is a corpus-wide conversion and the user explicitly chose full Track B in one packet over decomposition. The edits are shallow and mechanical outside `arachne_invariants.rs`, which carries all the real design.

- `crates/slicer-core/src/arachne/pipeline.rs` - role: `ArachneParams::default()`, the blocker; expected change: one field + its doc comment.
- `crates/slicer-core/tests/arachne_invariants.rs` - role: the extension point and the packet's centre of gravity; expected change: threshold const, coverage-vs-classic invariant, bead-width invariant, discrimination test, two negatives.
- `crates/slicer-core/tests/propagation.rs` - role: last odd `factory_params()`; also home of `propagation_fills_gap_from_central_neighbor`, the test that asserted the D5 defect and passed; expected change: even `max_bead_count`, reframed structural assertions.
- `crates/slicer-core/tests/{centrality,bead_count,generate_toolpaths}.rs` - role: fixture consumers; expected change: blob-equality → property assertions.
- `crates/slicer-core/tests/fixtures/arachne/*.json` (8) - role: demotion; expected change: `provenance` string only.
- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` - role: centrepiece; expected change: `tapered_wedge` case → structural.
- `crates/slicer-runtime/tests/arachne_parity.rs` - role: hygiene; expected change: module header sentence.
- `crates/slicer-core/tests/arachne_parity_red_*.rs` (9) - role: hygiene; expected change: rehomed (moved), content preserved.

## Read-Only Context

- `crates/slicer-core/tests/arachne_d5_taper_coverage.rs` - read in full (short) - purpose: the existing coverage-ratio measurement shape to generalize.
- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` - invariant-class table row + bead-width section only, or delegated SUMMARY - purpose: the authoritative invariant list.
- `docs/specs/arachne-parity-recovery.md` - delegated SUMMARY of Track B + odd-`max_bead_count` entry only - purpose: the no-rebless mandate and the entanglement claim.
- `docs/DEVIATION_LOG.md` - `rg` for `D-112-SELFCAPTURED-BASELINES` only, or delegated FACT - purpose: the row's current Status. Rows are single lines of thousands of characters.
- `docs/08_coordinate_system.md` - ranged read of the porting checklist - purpose: mm↔unit boundaries in the measurement helper.

## Out-of-Bounds Files

- `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/expected_perimeter_ir.json` - roughly 71 KB serialized IR. **Never load.** Its state is already recorded: **green and already re-blessed** to `9.82146` by `9ca62ba0`; it contains no `3.7797625`, and the briefed RED mismatch is a stale ledger fact (see §Open Questions `[BLOCK]`). Its fate is deletion or untouched preservation; neither requires reading it.
- The ten `record_*` functions in `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (`record_tapered_wedge` and siblings) - **never run against the tapered-wedge fixture.** Running one is the failure mode AC-N3 exists to catch.
- `docs/18_arachne_parity_audit.md` - **exists and is git-tracked** (verified at authoring time; clean working-tree status). Out of bounds: do not load it, and **do not cite it as authority for deviation state** — `docs/DEVIATION_LOG.md` is authoritative. Recorded precisely because this packet was briefed that the file had been deleted and it had not been; treat its existence as a fact and its authority as nil.
- `OrcaSlicerDocumented/...` - delegate; never load. Cite as `File.cpp::function`, never `path:line`.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- Other packet directories under `.ralph/specs/` - never modify.
- The four other `perimeter_parity` Arachne fixtures (`narrow_strip_widening`, `max_bead_count_cap`, `complex_multi_feature`, `cube_4color_arachne`) - out of scope for conversion; do not touch their baselines.

## Expected Sub-Agent Dispatches

- Question: what is the current Status and Follow-up text of the `D-112-SELFCAPTURED-BASELINES` row?; scope: `docs/DEVIATION_LOG.md`; return: `FACT`; purpose: Step 8 doc edit. Rows are enormous — the subagent must return the Status verdict, not the row.
- Question: does canonical `WallToolPaths.cpp::generate` derive `max_bead_count` as `2 * inset_count` unconditionally, and what does `LimitedBeadingStrategy.cpp::compute` do on an odd value?; scope: `OrcaSlicerDocumented/`; return: `FACT`; purpose: Step 1 authority.
- Question: what does `docs/specs/arachne-parity-recovery.md` say about Track B's treatment of the tapered-wedge baseline and about the odd-`max_bead_count` entanglement?; scope: that file; return: `SUMMARY` <=200 words; purpose: Steps 1 and 4.
- Question: which config key and value select the classic wall generator, and where is it resolved?; scope: `modules/core-modules/*/module.toml` + `crates/slicer-core/src/algos/region_mapping.rs`; return: `LOCATIONS` <=20; purpose: Step 2's reference run. Known starting point: `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/config.json` carries `"wall_generator": "arachne"`.
- Question: `cargo test -p slicer-core --features host-algos --test arachne_invariants` — pass/fail and failing assertion?; scope: cargo; return: `FACT` pass/fail with <=20 lines on failure; purpose: every step's verification.

## Data and Contract Notes

- IR/manifest contracts: none changed. `max_bead_count`'s module-manifest default was already corrected to a `0` sentinel (recorded in `docs/specs/arachne-parity-recovery.md`); this packet touches only the library default in `ArachneParams::default()` and the test helper. Confirm the manifest state before editing rather than assuming this sentence is still true.
- WIT boundary: untouched. No WIT files change, so no guest-WASM rebuild is triggered by this packet's own edits — the `wasm-staleness` snippet does not apply. If a guest, component, or module-dispatch test fails during execution, run `cargo xtask build-guests --check` before attributing it to anything, per CLAUDE.md.
- Determinism: the coverage measurement must be deterministic across runs at a fixed Z plane. A ratio that varies run-to-run indicates non-determinism upstream and is a stop condition, not a tolerance to widen.

## Locked Assumptions and Invariants

- **Locked: even `max_bead_count`.** After Step 1, no odd `max_bead_count` may be reintroduced anywhere in `crates/slicer-core`. Canonical (`WallToolPaths.cpp::generate`) is the authority; this is not a tunable.
- **Locked: no absolute-coordinate equality.** Per ADR-0042, no invariant landed by this packet may assert equality against a captured coordinate.
- **Locked: no rebless of the tapered-wedge baseline.** AC-N3 enforces it mechanically.
- **Locked: the coverage threshold, once measured, is a floor with a written margin justification.** A later packet may re-measure and move it, but must record why — per ADR-0042's own instruction that an invariant found to be wrong is *replaced with a better invariant and the reason recorded*, never with a fallback to fixture equality.

## Risks and Tradeoffs

- **The thesis risk: landing everything except the coverage invariant.** Closure, loop count, and bead-count sequence are easier to implement, easier to make green, and were **all green throughout D5**. A packet that ships those four and defers coverage will look complete and will have failed. If context forces a cut, cut hygiene (Steps 6-7), never the coverage invariant.
- **Threshold-setting under pressure.** If Step 2's measured minimum comes in awkwardly low, the tempting move is to lower the threshold until the corpus passes. That reproduces the self-captured-baseline failure in a new costume: a threshold fitted to current output ratifies current output. If the observed minimum is low enough that `0.668` would pass, **stop and record a `[BLOCK]`** — a fixture is either genuinely broken or is not a valid coverage subject, and either answer is a finding, not a tuning parameter.
- **Packet size.** Exceeds every reference size. Accepted by explicit user decision (see `requirements.md` §Out of Scope). The length is driven by real breadth — ten ACs across a nine-step corpus conversion — not by repetition. There is no duplication to remove.
- **`toolpaths_tapered_wedge.json` is the borderline case**: it carries absolute `junction_widths_mm`, which is the closest thing in the corpus to a rejected absolute baseline. Widths are a legitimate structural subject via the bead-width cap invariant (`<= 2 * optimal_width`), but the *captured* widths are not an oracle. Convert to the cap assertion; keep the captured values only as a labelled change-detector.
- **Rehoming the nine red files risks silently dropping tests.** AC-9 asserts collection at the new home for exactly this reason; a move that loses a test name passes a naive `ls` check and fails AC-9.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, the measurement sweep — ten fixtures × two generators)
- Highest-risk dispatch and required return format: the `docs/DEVIATION_LOG.md` FACT. Its rows are single lines of many thousands of characters; a subagent that returns the row instead of the verdict blows the budget in one message. Reject and redispatch.
- **Execution note for `/swarm`:** this packet is deliberately not decomposed, on explicit user instruction. It will likely exceed a single worker's comfortable band. `/swarm` should execute it **in stages** — a natural cut is Steps 1-3 (blocker, measure, encode: the thesis) and Steps 4-9 (convert, demote, hygiene, docs) — handing off the measured threshold table in this file as the interface between them. That table is the only state that must survive the handoff. Staging the *execution* is expected; splitting the *packet* is not.

## Open Questions

- **[BLOCK] The packet's briefed centrepiece premise is falsified by the tree, and this changes what the packet is for.** This packet was briefed that `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/expected_perimeter_ir.json` is "currently deliberately RED", failing `regions[0].walls[0].path.points[1].x` with actual `9.82146` vs expected `3.7797625`, and that commit `1d7eb1de` "MISSED this third" fixture. **Verified against the tree at authoring time: all of that is stale.** `arachne_perimeter_parity` **passes** (`12 passed; 0 failed`). The fixture contains `9.82146` and **no** `3.7797625` — it was **already re-baselined**, by `9ca62ba0 test(arachne): re-baseline 2 parity fixtures that pinned the D5 bow-dropout bug`. The RED state now survives only as prose in `docs/specs/arachne-parity-recovery.md`: a classic rotted ledger fact, and an ironic one given this packet's own subject.

  This is not a wording fix. The briefed job was "convert a RED fixture instead of reblessing it". The actual tree state is "the rebless **already happened**" — the exact outcome `docs/specs/arachne-parity-recovery.md` forbade, landed under a commit message that names the D5 bug it was pinning. The conversion is therefore **more** warranted, not less: a green self-captured baseline that was re-recorded to match post-D5 output is the purest form of the failure this packet exists to end — it will now ratify whatever the pipeline emits, silently, forever. But the packet's framing, AC-6, AC-N3's rationale, and `requirements.md`'s "currently deliberately RED" all had to be restated to match reality, and the *scope* question — is silently converting an already-reblessed fixture still the approved job, or does the prior rebless want its own remediation/deviation row first? — is a maintainer decision, not an authoring one. **Also worth deciding:** whether `docs/specs/arachne-parity-recovery.md`'s stale RED prose should be corrected by this packet (currently out of scope) so the next reader is not briefed from it as this packet's author was. Status stays `draft` pending that call.

- **[BLOCK]** Demotion policy for change-detectors: should a surviving self-captured change-detector *fail* CI on drift (a true gate that will need periodic reblessing, with all the ratification risk that carries), or *warn* only (informational, never blocking, and therefore ignorable)? ADR-0042 calls these "useful where a structural invariant does not yet exist" but does not settle their CI authority. The two options have opposite failure modes and the choice determines what AC-7's labelling actually means. Maintainer decision required; the packet stays `draft` until answered.
- **[BLOCK]** If Step 2's measured minimum is low enough that a threshold at `observed_min - margin` would admit `0.668`, AC-4 and AC-3 are in direct conflict and the corpus itself is implicated. Escalate rather than resolving by tuning. Recorded here so the conflict is visible before activation rather than discovered mid-execution.
- **[FWD]** Exact stage-grouped destinations for the nine `crates/slicer-core/tests/arachne_parity_red_*.rs` files. The existing `arachne_*` test files already suggest stage groupings (`arachne_stitch_*`, `arachne_simplify_*`, `propagation`, `bead_count`), but the mapping is mechanical and best decided with the files open. Implementer-resolvable; must preserve every test name (AC-9).
- **[FWD]** The D5 benchy datapoint has only one in-tree source: the bow cross-section at `crates/slicer-core/tests/fixtures/arachne/d5_benchy_call1.txt`, already consumed by `crates/slicer-core/tests/arachne_d5_taper_coverage.rs`. **Verified at authoring time: there is no benchy STL in this tree.** It was retired (`.ralph/specs/89_benchy-3mf-retirement`); `resources/benchy.stl` — which `.claude/aux-commands.md` still names in its `--report` example — does not resolve, and `resources/` contains no benchy model in any format. A full-model benchy coverage sweep is therefore **not available** to this packet without reintroducing a retired asset, which is out of scope. Implementer decision: accept the cross-section as the D5 datapoint (recommended, and sufficient for AC-4's discrimination test, which needs the *ratio* `0.668`, not a live benchy slice), or escalate if a full-model sweep is judged necessary. Do not assume a benchy STL exists because a doc mentions one.
