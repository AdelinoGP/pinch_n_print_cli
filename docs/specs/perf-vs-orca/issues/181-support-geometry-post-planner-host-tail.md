# `SupportGeometry` post-planner host tail attribution and reduction screen

Type: task / attribution + candidate screen (no production commit in this take;
probe reverted if used)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t175 full-matrix attribution and 0.50x headroom map](177-post-t175-full-matrix-attribution.md)
([`evidence/t177-post-t175-attribution/FINDINGS.md`](../evidence/t177-post-t175-attribution/FINDINGS.md)),
which discovered that **16.1–20.0 %** of `PrePass::SupportGeometry` — **`2,293 ms`
(7.8 % of cell wall)** on `base-classic-on`, **`2,017 ms` (7.0 %)** on
`base-arachne-on`, **`215 ms` (6.2 %)** on `benchy-classic-on`, and **`216 ms`
(6.1 %)** on `benchy-arachne-on` (median of 3 passes; `0 ms` on all four
supports-off cells) — elapses *after*
`com.core.tree-support-planner::module_complete` and *before*
`PrePass::SupportGeometry::stage_complete`. On `benchy` supports-on this host
tail is larger than Phase-3 union itself, and on `base` supports-on it closes
nearly half of the remaining `16.7–18.1 %` cut to `< 1.0x`.

## Question

**What executes on the host during the `2,017–2,293 ms` (`base` supports-on,
7.0–7.8 % of cell) and `215–216 ms` (`benchy` supports-on, 6.1–6.2 % of cell)
between `com.core.tree-support-planner::module_complete` and
`PrePass::SupportGeometry::stage_complete`, and what exact, output-preserving
candidate reduces it cleanly under the standing `t137` 4-cell planner gate
without weakening the zero-tolerance exact-Z occupancy tripwire?**

Concretely:

1. **Sub-bracket the post-planner tail** in `crates/slicer-runtime/src/prepass.rs`
   (after `com.core.tree-support-planner` returns) through
   `crates/slicer-runtime/src/blackboard.rs::Blackboard::commit_support_geometry`
   and `crates/slicer-runtime/src/builtins/support_analysis_producer.rs::validate_entry`
   on `base-classic-on` and `benchy-classic-on`:
   - WIT result decode / conversion into `SupportGeometryIR`,
   - `SupportGeometry` merge / validation / exact-Z occupancy gate
     (`validate_entry`, including whether it runs serially across bodies/layers
     or rebuilds spatial/polygon indexes per entry),
   - `blackboard.commit_support_geometry` storage/cloning/indexing,
   - temporary buffer / WASM store drop before `stage_complete`.
2. **Verify concurrency and operand structure:** measure whether the tail is
   single-threaded (`1.0x`) on 12 cores, how many support bodies / polygons /
   occupancy queries it processes, and whether any duplicate work (e.g., cloning
   large polygon sets, rebuilding per-layer occupancy indexes, or serial
   per-body validation) dominates.
3. **Screen and price concrete exact candidates:** for each candidate (e.g.,
   parallelizing independent per-layer/per-body occupancy validation with Rayon,
   spatial indexing / bbox culling inside exact validation without relaxing its
   zero-tolerance check, eliminating redundant clones or WIT-to-IR conversions,
   or deferring/avoiding unnecessary drops), state its ceiling in ms and % of
   cell wall across all four supports-on cells (`base` and `benchy`), confirm
   strict zero-tolerance preservation of `validate_entry` (0 code-1200,
   byte-identical 8/8 output), and check whether it clears the standing `t137`
   4-cell gate.
4. **Decide graduation:** graduate an implementation + standing-gate ticket if
   a verified exact candidate captures a material fraction of the `6.1–7.8 %`
   tail across all four supports-on cells (and note its contribution alongside
   [Planner Phase-3 batched union attribution and mechanism screen](178-planner-phase3-union-attribution.md)
   toward closing `base` supports-on to `4/8` and cutting binding
   `benchy-classic-on`).

## Scope

- Attribution and candidate pricing only; no production commit in this take.
  Revert any diagnostic probe cleanly and preserve it as `probe.patch` in the
  evidence directory.
- **Hard invariant (`map.md` occupancy-gate rule):** never weaken
  `validate_entry` to a tolerance; exact-Z occupancy validation must remain
  zero-tolerance and fire on any real plane mismatch.

## Pre-declared gate

**Attribution + screen only (no landing gate).** Correctness bar:

- `cargo xtask build-guests --check` exits 0 before and after probe revert.
- Unmodified baseline runs reproduce `t175`'s active `reference.json`
  byte-for-byte (`exit 0`, `degraded = false`, `0` code-1200).
- The sub-brackets sum to and reconcile with `t177`'s measured `2,293 ms`
  (`base-classic-on`) and `215 ms` (`benchy-classic-on`) post-planner tail.

## Acceptance

- Evidence directory
  `docs/specs/perf-vs-orca/evidence/t181-support-geometry-host-tail/` with
  `FINDINGS.md`, `REPRODUCE.md`, reduced JSONs, `probe.patch` (if used), and
  checkout-only `verify.py` (`verify.py` + `verify.py --self-test` exit 0).
- Updated `## Answer` in this ticket, `map.md` update, and graduated
  implementation ticket if a candidate clears the `t137` bar.

## Answer

_(open)_
