---
status: implemented
packet: "236-support-stabilization"
task_ids:
  - TASK-344
  - TASK-345
  - TASK-346
  - TASK-347
  - TASK-348
  - TASK-349
  - TASK-350
  - TASK-351
  - TASK-352
---

# 236-support-stabilization

## Goal

Make the support-families branch contract-stable and fully green: per-region AC-8 family assignment lands, the startup validator stops flagging expected post-221 multi-holder support claims, the only tree-geometry golden exercises real collision/avoidance inputs, `support_threshold_angle` gains declared bounds with regenerated config docs, the integrated-parity harnesses assert guest freshness before comparing, the native/wasm view seam and paint BASE fallback get pinned, drafts 215–218 are deleted, ADR-0059 is accepted, and post-fix parity ratios are measured.

## Problem Statement

The support-families branch (`parity/support-planners-clean`) carries a deliberately red AC-8 test, a validator that flags expected post-221 claim topology as permanent noise (G-21), a golden that proves nothing about collision or avoidance because its fixture feeds empty occupancy (G-23), an unbounded host config key whose doc reference was left stale by a past manifest deletion (G-22), and parity harnesses that can report spurious geometry divergences from guest staleness (G-24). Two latent correctness hazards remain unpinned: the native/wasm layer-view construction seam that silently rendered inputs three times during packet 224 (T9), and `execute_paint_segmentation`'s BASE fallback built from whole-layer all-object contours. The queue also still lists four never-implemented draft packets and a `proposed` ADR that this queue's Ruling 1 amends. These are one coherent slice: each item is small, independently verifiable, and every later packet in the completion queue (237, 238a/b/c, 240) builds on a branch that must be green and honestly instrumented first.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The AC-8 fix changes which entries exist in `SupportAnalysisIR.family_assignments` (host IR, no WIT/schema bump): the SDK view type `SupportAnalysisView.family_assignments: Vec<SupportFamilyAssignment>` (`crates/slicer-sdk/src/prepass_types.rs:445–468`) already carries per-region rows keyed by `(object_id, region_id)` strings, so NO WIT edit is needed — but the guest-side minting parity (the tree planner's own `family_assignments` consumption) must be re-checked after the host change with fresh guests (first constraint bullet).
- Config keys are snake_case everywhere (E9). New `[config.schema]` entries + regenerated `docs/15_config_keys_reference.md` land in one commit (T8).
- Invariant 15 (plan §6): every RegionMap region gets exactly one attributed plan entry; regions requiring no support carry a structured no-work/declined record — never silence. Invariant 16: no acceptance command may match zero tests; every filter asserts a non-zero matched count.

## Data and Contract Notes

- IR/manifest contracts: `SupportAnalysisIR.family_assignments` gains rows for candidate-less regions — additive only; consumers (executor routing, `backfill_active_region_configs` pairing) must tolerate a region whose assignment exists without candidates; verify `structured_support_identity` integration test stays green.
- WIT boundary: none touched (no `.wit` edits this packet). The SDK view already carries the needed shape (`SupportFamilyAssignment { object_id, region_id, family_id }`).
- Determinism/scheduler constraints: the validator exemption is order-independent (set membership, not ordering); the per-region minting iterates RegionMap entries deterministically (BTreeMap-style keys as today) so serial/parallel runs stay identical (invariant 12).

## Locked Assumptions and Invariants

- Frozen golden tolerances: branch-count drift ≤ 10%, Hausdorff ≤ 0.5 mm — never widened (E3).
- The AC-8 test's count assertion is NOT weakened (plan Ruling 1).
- `support_threshold_angle` canonical semantics locked: min 0, max 90, default 30.0 (canonical `PrintConfig.cpp` coInt; recorded in-tree at `resolved_config.rs:968` macro doc).
- Family-scoped claim exemption applies to the GLOBAL conflict pass only; per-region conflict detection is unchanged. The `SupportPlanIR` / `SupportIR` multi-writer advisories are resolved by ORDERABILITY recognition (ADR-0059's host-aggregator aggregation edge), preserving the ADR's sole-writer model — no amendment deviation, no superseding ADR.
- check-literals violation count unchanged from inherited baseline (61 across 34 files, T10).

## Risks and Tradeoffs

- Per-region minting could double-count assignments if a region HAS candidates and the region-walk re-adds them — mitigate by keying the map insert (entry API) so each `(object, region)` lands exactly once; the executor-side consumer must see identical assignments for candidate-bearing regions as before.
- The G-21 write-conflict handling risks masking a future genuine dual-writer on `SupportPlanIR` — mitigated by implementing it as orderability recognition (aggregation edge = ordering, per ADR-0059's own clause) scoped to family-scoped support claim holders, plus the AC-N1 negative guard `genuine_write_conflict_still_rejected_after_aggregation_recognition`; a non-support dual-writer pair still conflicts.
- Tripwire rebless bakes current planner output as baseline; if the inputs change planner output materially, drift classification MUST precede regeneration (E3) or the golden loses meaning.
- Freshness assertion inside shared harness affects all integrated-parity suites (not just support): acceptable — it fails loudly only when artifacts are genuinely stale, which is the desired T4 posture; document in `docs/04_host_scheduler.md`? No — harness is test-common, note it in the harness doc comment instead.
- Deleting draft dirs removes their task-map crosswalks; git history is provenance (plan §10) — remediation rows record the absorption mapping.
