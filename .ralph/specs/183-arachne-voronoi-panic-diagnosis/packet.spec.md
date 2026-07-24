---
status: draft
packet: 183-arachne-voronoi-panic-diagnosis
task_ids:
  - TASK-296
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 183-arachne-voronoi-panic-diagnosis

## Goal

Close the defensive asymmetry that makes D-167 invisible: wrap the one unprotected boostvoronoi `Builder::build()` call — in `voronoi_from_segments` (`crates/slicer-core/src/voronoi.rs`) — in `catch_unwind`, mapping a caught `robust_fpt` assertion panic to a distinct `VoronoiError` variant exactly as `medial_axis.rs` and `algos/paint_segmentation/voronoi_graph.rs` already do, then use the now-observable failures to answer D-167's open question: do these panics drop live wall geometry, or are they inert? Record the verdict and either close D-167 or narrow it to a named successor.

## Scope Boundaries

Diagnosis-first packet. Covers the `catch_unwind` guard and error variant in `voronoi_from_segments`, the evidence capture identifying the degenerate inputs, and the written verdict. Does **not** implement the `preprocess_input_outline` pre-snapping hardening that a "geometry is lost" verdict would call for — that becomes a named successor packet. Does not touch `discretize_edge` (D-154, queued separately in T3).

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: the T3 D-154 discretize packet (D-167's verdict gates its design, since both live in the same graph-construction path and a degenerate-input fix may change what `discretize_edge` observes).
- Activation blockers: none. Packet `140_lightning-module-rewrite` is currently `active`; this packet stays `draft` until that clears.

## Acceptance Criteria

- **AC-1. Given** `voronoi_from_segments` in `crates/slicer-core/src/voronoi.rs`, **when** the source is inspected, **then** its boostvoronoi `Builder::build()` call is wrapped in `std::panic::catch_unwind(AssertUnwindSafe(...))` and a caught panic is mapped to a distinct `VoronoiError` variant (not `map_bv_error`'s existing `Result` mapping, which cannot observe a panic). | `bash -c 'rg -q "catch_unwind" crates/slicer-core/src/voronoi.rs && rg -q "AssertUnwindSafe" crates/slicer-core/src/voronoi.rs && echo PASS || echo FAIL'`
- **AC-2. Given** the `perimeter_parity` workload that produced 13 swallowed panics during the D-160 session, **when** it is run after the guard lands, **then** zero raw `is_finite()` assertion panic lines reach stderr and the suite's pass/fail status is unchanged from its pre-change baseline. | `bash -c 'mkdir -p target && cargo xtask build-guests --check && cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | tee target/183-parity.log | rg "^test result"; rg -c "fpv_?\.is_finite|assertion failed.*is_finite" target/183-parity.log || echo "0 raw panics"'`

  The `build-guests --check` prefix is mandatory: `--test integration` loads core-module WASMs, so a stale guest would fail this workload and be misattributed to the new guard. `mkdir -p target` matches the repo's tee convention in `CLAUDE.md`.
- **AC-3. Given** the diagnosis run, **when** it completes, **then** `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md` exists and records, under explicit headings, (a) the count of caught builder panics, (b) a characterization of the offending segment sets (count, coordinate bounds, duplicate/near-collinear/zero-length classification), (c) the owning layer/region ids, and (d) an explicit verdict sentence answering "does the panicking computation feed live geometry or is it discarded". | `bash -c 'rg -q "## Caught panic count" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && rg -q "## Input characterization" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && rg -q "## Verdict" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && echo PASS || echo FAIL'`
- **AC-4. Given** the recorded verdict, **when** `docs/DEVIATION_LOG.md` is updated, **then** the D-167 row's **Status cell** (the row's final column) either begins `Closed` with the evidence summary, or begins `Open — narrowed` and names the successor deviation id owning the `preprocess_input_outline` hardening. | `rg -q '^\|\s*D-167-BOOSTVORONOI-ROBUST-FPT-PANICS\b.*\|\s*\*{0,2}(Closed|Open — narrowed)[^|]*\|?\s*$' docs/DEVIATION_LOG.md && echo PASS || echo FAIL`

  Two escaping hazards, both of which bit an earlier draft — copy this command verbatim:
  - **The alternation pipe must be bare `|`, never `\|`.** ripgrep uses Rust regex, where `\|` is an *escaped literal pipe*; `(Closed\|Open — narrowed)` therefore matches the literal text `Closed|Open — narrowed` and the criterion becomes unpassable by any correct verdict. The pattern is single-quoted, so a bare `|` reaches rg untouched.
  - **The trailing `[^|]*\|?\s*$` is load-bearing.** Without it the match is not confined to the final cell: `.*\|` backtracks, so a Rationale cell merely *containing* (or beginning with) "Closed" would report PASS while the Status column still read `Open`. The `[^|]*` forbids any further cell delimiter after the verdict, which is what pins the match to the Status column and matches the log's own "a deviation is open unless its `Status` begins with `Closed`" rule.

  This exact pattern was verified empirically at authoring time against six inputs — live row with `Status = Open` → no match; `Status = Closed …` → match; `Status = Open — narrowed to …` → match; **Rationale cell containing "Closed" while Status is Open → no match**; `Status = **Closed** …` (bold) → match; and the real `docs/DEVIATION_LOG.md` → no match (D-167 is still open). Re-run those cases if you change a single character.

## Negative Test Cases

- **AC-N1. Given** a deliberately degenerate segment set (duplicate, zero-length, and near-collinear segments, modeled on the existing `crates/slicer-core/tests/medial_axis_degenerate_input_tdd.rs`), **when** it is passed to `voronoi_from_segments`, **then** the call returns a `Result` — `Ok` or `Err(VoronoiError::…)` — and does **not** unwind the calling thread. | `cargo test -p slicer-core --features host-algos --test voronoi_stress -- voronoi_from_segments_degenerate_input_returns_result_not_panic --exact 2>&1 | tail -20`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos --test voronoi_stress 2>&1 | tail -15`

## Authoritative Docs

- `docs/adr/0023-arachne-port-strategy.md` — direct read; establishes the boostvoronoi selection and the "callers pre-snap degenerate input" contract. This packet does not amend it.
- `docs/DEVIATION_LOG.md` — the D-167 row only (large file; ranged read).

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` D-167 row — replaced with the verdict (Closed with evidence, or `Open — narrowed` naming the successor deviation that owns the `preprocess_input_outline` hardening). Verification grep (identical to AC-4's — see its escaping notes): `rg -q '^\|\s*D-167-BOOSTVORONOI-ROBUST-FPT-PANICS\b.*\|\s*\*{0,2}(Closed|Open — narrowed)[^|]*\|?\s*$' docs/DEVIATION_LOG.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
