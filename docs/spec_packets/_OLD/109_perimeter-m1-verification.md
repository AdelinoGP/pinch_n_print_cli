---
status: implemented
packet: 109_perimeter-m1-verification
task_ids:
  - T-100
  - T-101
  - T-102
  - T-103
  - T-104
  - T-105
  - T-P96-A
  - T-P96-C3
  - T-P96-D
  - T-P96-F
---

# 109_perimeter-m1-verification

## Goal

Close M1 of the perimeter parity roadmap: build the reference-fixture parity harness, record 6 OrcaSlicer reference outputs (solid square, holed square, multi-tool triangle, overhang ramp, bridge fixture, spiral-vase cone), reshape and re-baseline the P96 4-color cube AC-22b test to assert per-color fragmentation, delete the now-unused `external_contour` IR field, close every M1 deviation registered since P102, run the full `cargo test --workspace` ceremony, and update `docs/07_implementation_status.md` to mark Classic parity complete.

## Problem Statement

M1 of the perimeter parity roadmap is PLANNED but not yet shipped: P104–P108 are all `status= draft` as of 2026-06-19 (verified by grep). This packet is the M1 closure gate — it cannot activate until P102–P108 are implemented. Once the chain ships, this packet builds the end-to-end verification layer: without recorded OrcaSlicer reference outputs and a parity harness, regressions during M2 work (Voronoi + SkeletalTrapezoidation + BeadingStrategy stack) will land undetected. The audit also enumerated 7 edge cases that lack regression coverage (3-tool polygon, inner-wall material boundary, 0/2-vertex polygon, hole-with-thin-wall, gap-fill-in-overhang, top-flagged region, first-layer override). Finally, the P96 inherited reshape obligation (T-P96-A) leaves the 4-color cube TDD in a divergent state; `external_contour` remains live in `SlicedRegion` (populated by `populate_external_contours` in `bisector_ownership.rs`, with tests at lines ~178-247 and accessed via `views.rs:391/399`) and must be removed as part of T-P96-D once P105's `bisector_edge_skip_mask: Vec<bool>` (flat per-edge, ADR-0013 LOCKED) ships.

This packet closes all four concerns. The parity harness + 6 recorded fixtures give M2 a regression bed; the 7 edge-case TDDs lock down propagation correctness; the cube_4color test gets its final renamed-and-rebased state with a new SHA captured under the packet's deviation entry; `external_contour` is removed end-to-end (IR + WIT + host populator + view accessor — ~5 files); and `docs/07_implementation_status.md` records M1 as shipped.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Schema-bump contract for additive removal: bump from live `4.3.0` by one minor (exact value computed at activation — do NOT hardcode a literal; P105 may bump first, making the target `4.5.0` instead of `4.4.0`); old fixtures must still deserialize. Pattern: `#[serde(default, skip_serializing)]` on whatever vestigial sniff still acknowledges the field name during parse, OR clean removal if the implementer confirms no committed fixture relies on the old shape (delegated FACT).
- T-105 workspace test ceremony is the ONE allowed `cargo test --workspace` invocation per CLAUDE.md exception. All other ACs use targeted per-test invocations.
- The harness's tolerance comparator is the only place that decides "fixture passes" — it MUST be self-tested (AC-1 + AC-N1) before recorded references are considered authoritative.
- Recorded reference fixtures (`expected_perimeter_ir.json`) are committed artifacts. Regenerating them in-test is forbidden; if a recorded fixture must change, the implementer halts and documents the regression cause in the closure log.

## Data and Contract Notes

- IR or manifest contracts touched: `SlicedRegion.external_contour` removed; schema bump from live 4.3.0 by one minor, computed at activation (NOT hardcoded — P105 also bumps `SliceIR`, so the target is 4.4.0 or 4.5.0 depending on landing order; additive removal). WIT mirror removed. **CLARIFIED (in-review): shipped as `4.6.0` — a *compatible-removal* minor bump. Field removal is major by default per the IR Versioning Contract, but this is a documented exception (no consumer, serde-tolerant, and every module declares `max_ir_schema = 5.0.0` so a 5.0.0 host would fail the scheduler version gate). "Additive removal" was imprecise phrasing; the accurate term is "compatible removal". See `docs/02_ir_schemas.md`.**
- WIT boundary considerations: per CLAUDE.md WIT/Type Changes Checklist, `cargo build --tests --workspace` must pass after the WIT edit before Step 5 closes.
- Determinism or scheduler constraints: parity harness comparator must be deterministic (sorted iteration over walls + edges); recorded fixtures must be byte-stable (no timestamps in JSON, no random IDs).
- cube_4color test rename: the renamed function MUST be the only `#[test]` in the executor file with the new name; the old function name is deleted, not deprecated.

## Locked Assumptions and Invariants

- Recorded `expected_perimeter_ir.json` fixtures are authoritative until M2 ships a parity-changing update. Edits to recorded fixtures require closure-log justification per fixture.
- `external_contour` is CURRENTLY LIVE in the tree (as of 2026-06-19): populated by `populate_external_contours` in `bisector_ownership.rs:64`, with tests at lines ~178-247, and accessed via `views.rs:391/399`. It is NOT yet obsolete. Its removal requires P105's `bisector_edge_skip_mask: Vec<bool>` (flat per-edge, ADR-0013 LOCKED shape — NOT `Vec<Vec<bool>>`) to ship first. Once P105 ships, `external_contour` deletion is justified and one-way — the field cannot be re-added without a new ADR. The cascade covers: `bisector_ownership.rs` (function + 3 tests + field assignments), `mod.rs:840` (call site), `prepass_slice.rs:356` (initializer), `views.rs:391+399` (getter + setter), `host.rs` (populator), `ir-types.wit` (WIT record + accessor), `slice_ir.rs` (field + schema version bump).
- `bisector_edge_skip_mask` consumption uses `edge_offset_for_polygon(region, poly_idx) -> usize` (FORWARD-DEP on P105, LOCKED signature). No nested `Vec<Vec<bool>>` — only flat `Vec<bool>`.
- T-105 workspace test ceremony is the M1 close gate. If it fails for reasons unrelated to this packet's edits (pre-existing in-flight work), the implementer documents in the closure log and gates Step 7 specifically on M1-related-test passes.
- The parity harness's per-field tolerances (XYZ ±0.005 mm, width ±0.01 mm) are calibrated for the 6 reference fixtures. Tighter tolerances may surface false positives; looser may mask regressions. Document calibration choice in the harness doc-comment.

## Risks and Tradeoffs

- Recorded-fixture drift: if OrcaSlicer updates change the expected outputs for one of the 6 fixtures, the M1 baseline is no longer accurate. Mitigation: closure log records the OrcaSlicer commit-hash (or version) the fixtures were derived from.
- `cargo test --workspace` time at packet close: >11 minutes per CLAUDE.md. Acceptable as a one-time ceremony; not acceptable as a regression-test loop. Step 7 dispatches it once.
- `external_contour` removal cascade may surface a forgotten caller in slicer-core/paint_segmentation that P105 didn't revert. Step 5's LOCATIONS dispatch catches this before the deletion lands.
- 6 fixture SUMMARYs sequentially dispatched in Step 2 may strain context. Mitigation: implementer can dispatch them in parallel (single user turn with multiple sub-agent calls) per the harness skill's parallel-dispatch convention.
