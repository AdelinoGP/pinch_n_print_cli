# Requirements: 177-arachne-baselines-to-structural-invariants

## Packet Metadata

- Grouped task IDs: none. No `TASK-###` in `docs/07_implementation_status.md` covers this audit-driven slice; do not invent one.
- Backlog source: `docs/DEVIATION_LOG.md` row `D-112-SELFCAPTURED-BASELINES` (Status: Open). Audit-driven precedent — packets 150-156 were all authored against `backlog_source` with no task ID.
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Every Arachne baseline in this tree is captured from Pinch 'n Print's own output. There is no OrcaSlicer oracle in-repo, so the entire corpus proves only self-regression: the pipeline still does what it did last commit. `D-112-SELFCAPTURED-BASELINES` records this as an accepted limitation. It is no longer acceptable, because the limitation has now demonstrably cost correctness.

`docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` documents the bill. D5 let roughly 16 mm of Benchy bow drift across roughly 40 layers while every fixture stayed green. Worse, `propagation_fills_gap_from_central_neighbor` (`crates/slicer-core/tests/propagation.rs`) asserted the defective behaviour and **passed** — a self-captured baseline does not merely fail to catch a regression, it actively ratifies it and then defends it against repair.

**This packet's thesis, and the only standard it should be judged against: "would this have caught D5?" — not test count, not fixture count, not coverage of the corpus.** That standard has a sharp consequence which must not be softened during execution: of the ADR-0042 invariant classes, **coverage-ratio-vs-a-known-correct-reference is the only one that would have caught D5**. Closure within tolerance, loop count and nesting, and bead-count sequence were **all green throughout D5**. A packet that lands four elegant invariants and omits the coverage ratio has failed its thesis while looking complete. The coverage invariant is the deliverable; the rest are supporting structure.

The corpus is additionally entangled by a live trap: `max_bead_count` is odd where canonical always derives an even `2 * inset_count` (canonical `WallToolPaths.cpp::generate`), and `LimitedBeadingStrategy.cpp::compute`'s odd-`max_bead_count` branch parks a giant centre bead. `docs/specs/arachne-parity-recovery.md` states this "entangles the tapered-wedge self-captured baselines (Track B)". It must be untangled before any conversion, or the converted invariants inherit the trap.

**Scope note — the odd-`max_bead_count` inventory is a ledger fact. Re-derive it; never quote one.** The trap survives at **two production sites** — `ArachneParams::default()` (`crates/slicer-core/src/arachne/pipeline.rs`) and `BeadingFactoryParams::default()` (`crates/slicer-core/src/beading/factory.rs`) — plus several test helpers. The `factory_params()` helpers in `crates/slicer-core/tests/generate_toolpaths.rs` and `crates/slicer-core/tests/bead_count.rs` **already carry `10`** and document the correction inline; do not "fix" them again.

Do **not** trust the preceding sentence for the full list. Derive it at the point of use:

```
rg -n 'max_bead_count:\s*9' crates/slicer-core/
```

That command is the sole authority for Step 1's edit surface. An earlier draft of this packet asserted "exactly two places" from a grep narrowed to four hand-picked files, and thereby missed the `beading/factory.rs` **production** site entirely — the exact failure mode this re-derivation rule exists to prevent, and a reminder that a confident inventory is worth less than the command that produces one.

This is one coherent slice because the blocker, the conversion, and the demotion share a single failure mode: a self-captured number standing where a structural property belongs.

## In Scope

- **BLOCKER, sequenced first.** Correct every odd `max_bead_count` in `crates/slicer-core` to an even value matching canonical's `2 * inset_count` derivation — covering **both** production defaults (`ArachneParams::default()` in `crates/slicer-core/src/arachne/pipeline.rs`, `BeadingFactoryParams::default()` in `crates/slicer-core/src/beading/factory.rs`) and the surviving odd test helpers. Derive the surface from `rg -n 'max_bead_count:\s*9' crates/slicer-core/`, not from any list in this packet.
- **MEASURE, then set the threshold.** Measure coverage-ratio-vs-`wall_generator=classic` across the existing fixture corpus plus the D5 benchy bow cross-section at `crates/slicer-core/tests/fixtures/arachne/d5_benchy_call1.txt`. **Verified: there is no benchy STL in this tree** — it was retired (see `.ralph/specs/89_benchy-3mf-retirement`), and `resources/benchy.stl`, which `.claude/aux-commands.md` still names, does not resolve. The cross-section fixture that `crates/slicer-core/tests/arachne_d5_taper_coverage.rs` already consumes is the only in-tree benchy datapoint. Record every observation. Set the threshold to `observed_min - margin` and justify the margin in writing. The threshold is an **output** of this packet, not an input.
- **CENTREPIECE.** Convert `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/expected_perimeter_ir.json` (consumed by `arachne_perimeter_parity` in `crates/slicer-runtime/tests/integration/perimeter_parity.rs`) into a structural invariant. **Premise correction — verified against the tree, and load-bearing:** this packet was briefed that the fixture is "currently deliberately RED" (actual `9.82146` vs expected `3.7797625`). It is **not**. The test **passes** (`12 passed; 0 failed`), the fixture holds `9.82146` and no `3.7797625`, and it was **already re-baselined** by `9ca62ba0`. The rebless the brief said must be prevented has already occurred; the conversion is more warranted, not less, because a green re-blessed self-captured baseline ratifies post-D5 output silently and permanently. See the `[BLOCK]` in `design.md` §Open Questions — the scope call is a maintainer decision.
- **CONVERT.** The eight `crates/slicer-core/tests/fixtures/arachne/*.json` baselines: `centrality_{square,wedge,multi_feature}`, `propagation_{uniform,varying,multi_feature}`, `bead_count_tapered_wedge`, `toolpaths_tapered_wedge`. Most already store structural data — reframe and add real assertions; do not redesign the fixture format.
- **DEMOTE.** Normalize provenance on all eight so surviving self-captured data is an explicitly-labelled change-detector.
- **EXTEND** `crates/slicer-core/tests/arachne_invariants.rs` (already exists; four tests from packet 113c; its docstring already carries the by-construction thesis).
- **HYGIENE.** Rehome the nine `crates/slicer-core/tests/arachne_parity_red_*.rs` files into stage-grouped homes; correct the stale `fails on purpose` header in `crates/slicer-runtime/tests/arachne_parity.rs`.

## Out of Scope

- Splitting this packet. The user explicitly chose full Track B in one packet over decomposition, accepting that it exceeds reference sizes. Do not split it.
- Re-capturing or re-blessing `expected_perimeter_ir.json` to turn the RED tapered-wedge case green. The recorders (`record_tapered_wedge` and siblings in `crates/slicer-runtime/tests/integration/perimeter_parity.rs`) must not be run against it.
- `crates/slicer-core/tests/{stitch,simplify,remove_small}.rs` — inline consts only, no data files, nothing to convert. Named here only to close the question.
- Any change to Arachne production geometry beyond the `max_bead_count` default correction.
- The D-104f concentric-infill open red test in `crates/slicer-runtime/tests/arachne_parity.rs` — its header is corrected, its status is not.
- `docs/18_arachne_parity_audit.md` — **out of scope; do not cite it anywhere.** `docs/DEVIATION_LOG.md` is the authority for deviation state. (The file **exists and is git-tracked** — verified at authoring time. This packet was briefed that it had been deleted this session; the tree says otherwise, so the exclusion is by scope, not by absence.)

## Authoritative Docs

- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` - large; read only the invariant-class table row and the bead-width section, or delegate a SUMMARY. Source of the invariant list and of the D5 cost narrative.
- `docs/specs/arachne-parity-recovery.md` - large; delegate a SUMMARY of the Track B section and the odd-`max_bead_count` entry. Do not read in full.
- `docs/DEVIATION_LOG.md` - very large, single-line rows of extreme length; **never read in full, never open ranges blindly**. Delegate a FACT for the `D-112-SELFCAPTURED-BASELINES` row.
- `docs/08_coordinate_system.md` - ranged read of the porting checklist.

## OrcaSlicer Reference Obligations

Delegate every OrcaSlicer inspection; never read OrcaSlicer source directly, and never load `OrcaSlicerDocumented/` into this packet's context. Cite canonical code by `File.cpp::function` only — never by line number, because OrcaSlicer is not vendored here and line pins silently rot against whatever revision their author had open.

Canonical references this packet depends on:

- `WallToolPaths.cpp::generate` - derives `max_bead_count = 2 * inset_count`, always even. Authority for AC-1/AC-2.
- `LimitedBeadingStrategy.cpp::compute` - the odd-`max_bead_count` branch that parks a giant centre bead. Authority for why odd is a live trap.
- `BeadingStrategyFactory.cpp::makeStrategy` - the `optimal_width` selection this packet's bead-width invariant caps against.

Dispatch contract: question, exact scope under `OrcaSlicerDocumented/`, return `FACT` (5 lines max) or `SNIPPETS` (3 max, 30 lines each). Reject anything larger and redispatch.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-10`.
  - AC-3 and AC-4 are the thesis pair: measure-then-set, then prove the chosen threshold rejects `0.668`. Reference datapoints for sanity only, not to be hardcoded: D5 measured 66.8% broken → 99.0% fixed; benchy maxX tracks classic at roughly 1.001.
  - AC-5/AC-6 are the centrepiece: the RED tapered-wedge case passes structurally, never by rebless.
  - AC-9/AC-10 are hygiene and carry no behavioural claim.
- Negative: `AC-N1` through `AC-N3`. AC-N1 is the anti-vacuity gate — an invariant that cannot fail is not an invariant. AC-N3 is the no-rebless gate.
- Cross-packet impact: closes `D-112-SELFCAPTURED-BASELINES`; provides ADR-0042 its first measured instantiation. No other active packet touches `crates/slicer-core/tests/fixtures/arachne/`.

## Verification Commands

Authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -n 'max_bead_count:\s*9' crates/slicer-core/` | Blocker inventory re-derived at point of use; empty output after Step 1 | FACT: match count |
| `cargo test -p slicer-core --features host-algos --test arachne_invariants` | Primary contract: all invariants incl. coverage and bead-width | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-core --features host-algos --test propagation` | AC-2; propagation tests survive the even-`max_bead_count` change | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test centrality --test bead_count` | AC-8; reframed structural assertions | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test generate_toolpaths` | Regression guard on the borderline `toolpaths_tapered_wedge` fixture | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration perimeter_parity` | AC-6; centrepiece no longer RED | FACT pass/fail |
| `rg -L -c 'CHANGE-DETECTOR, NOT a correctness oracle -- ADR-0042' crates/slicer-core/tests/fixtures/arachne/*.json` | AC-7; provenance normalized across all eight | FACT: list of files with count 0 |
| `git diff --stat HEAD -- crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/expected_perimeter_ir.json` | AC-N3; no-rebless gate | FACT: empty or deletion only |
| `cargo check --workspace --all-targets` | Compile gate incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |

Never use `cargo test --workspace` as an AC command. It is permitted once, at the closure acceptance ceremony only, via `cargo xtask test --workspace --summary` dispatched to a subagent returning `FACT pass/fail`.

## Step Completion Expectations

- **Ordering is load-bearing.** Step 1 (the `max_bead_count` blocker) must land before any measurement. Measuring against an odd-`max_bead_count` tree bakes `LimitedBeadingStrategy.cpp::compute`'s giant-centre-bead artifact into the threshold, and the resulting number would be permanently wrong in a way no later step can detect.
- **The threshold flows forward as shared state.** Step 2 measures and records; Step 3 encodes. Step 3 must read the threshold from Step 2's committed table in `design.md`, not re-measure and not re-derive. If Step 2's table is absent, Step 3 stops — it does not invent a number.
- **Z alignment is a precondition of every measurement**, per ADR-0042's D3 corollary: arachne-vs-classic comparisons are valid only at aligned Z sampling planes (there is a roughly 0.25 mm label offset against Orca). A ratio measured across misaligned planes is noise wearing a number's clothes.
- Steps 5-7 (convert, demote, hygiene) are independent of each other and may be reordered, but all follow Step 3.

## Context Discipline Notes

- `docs/DEVIATION_LOG.md` rows are single lines of many thousands of characters. A naive `Read` of any range can blow the budget on one row. Always `rg` for the row ID with a bounded context, or delegate a FACT.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/expected_perimeter_ir.json` is roughly 71 KB of serialized IR. **Never load it.** Everything this packet needs about it is already recorded here and in `design.md`: it is **green today and already re-blessed** to the post-D5 value by `9ca62ba0` (it holds `9.82146`; the `3.7797625` "expected" from this packet's brief is **not in the file** and the RED state is a stale ledger fact — see the `[BLOCK]` in `design.md` §Open Questions). The file's fate is deletion or untouched preservation; neither requires reading it.
- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` is a very large harness. Locate `arachne_perimeter_parity` and open a bounded window; do not browse the ten `record_*` recorder functions.
- `crates/slicer-core/tests/fixtures/arachne/*.json` — read the `provenance` string and the structural field names via `rg`/`jq`-style bounded extraction. Do not load whole fixtures to learn their shape.
- Ledger facts (the packet number, deviation IDs, line counts, SHAs) are **not** frozen in this packet by design. Re-derive each at its point of use with the command given beside it.
