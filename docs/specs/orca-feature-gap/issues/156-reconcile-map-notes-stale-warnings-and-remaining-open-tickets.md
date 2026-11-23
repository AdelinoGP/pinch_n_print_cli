# 156 — Reconcile Map Notes, stale `⚠` badges, and remaining open tickets against current `master`

Type: task
Status: open
Assignee: —
Blocked by: 154, 155
Map: ../map.md

## Question

Apply the verified re-grounding findings from [154 — Re-ground config-scope assumptions (`ADRs 0067–0070`) across Map Notes and open tickets](./154-reground-config-scope-assumptions-on-map-and-open-tickets.md) and [155 — Re-ground layer-preparation (`ADRs 0066/0071/0073/0074`) and feedrate (`ADR-0072`) across Map Notes and open tickets](./155-reground-layer-preparation-and-feedrate-on-map-and-open-tickets.md) to clean up [map.md](../map.md) and reconcile the remaining open tickets that were not directly re-grounded by `154` / `155`.

### Work items

1. **Rewrite or retire drifted standing Notes in `map.md`:**
   - Apply `154`'s verdicts to the four config Notes (dead manifest defaults, `ConfigView::from_declared`, per-tool config axis / `resolve_per_tool_configs`, percent-key units) and link `154`'s config-registration checklist for future packets.
   - Apply `155`'s verdicts to the cross-layer architecture Note (retiring the deleted `LightningTreeIR` example and citing `LayerPreparation::prepare_print` / ADR-0066 / ADR-0074) and feedrate/emitter Notes.
   - Check the `order_lock` Note (`remap_infill_order_locks_from`, `crates/slicer-runtime/src/layer_executor.rs`) and `ObjectMesh.transform` / `internal_solid_fill` Note against the merged tree (`71a4c832` already turned internal-solid fill into a five-way fill partition on this branch — verify whether that Note is now stale!).
   - Clean up the obsolete *"Live ledger note: 200–205 untracked"* paragraph.
2. **Fix stale `⚠ Correction required` badges and pre-split paths in `map.md` `## Decisions so far`:**
   - Git history shows packets `254a/b`, `255`, `256`, `257a/b`, `258`, `259a/b`, `260a/b`, `262a/b`, `263`–`266` were already re-authored/split in early Sept 2026 (and `261` removed), yet `## Decisions so far` still displays stale `⚠ Correction required` warnings and pre-split directory names (`254-prime-tower-keys-wipe-tower/`, `257-brim-type-and-brim-keys/`, `259-fuzzy-skin-keys/`, `260-support-interface-keys/`, `261-raft-keys/`, `262-infill-pattern-keys/`). Reconcile each entry accurately against disk and commit history (preserving the historical fact that those drafts still need Phase 2 SOTA re-alignment against current `master`, without misreporting their Sept re-authoring status).
3. **Reconcile the remaining non-config-scope open tickets against the current tree:**
   - [124 — Author packet — sequential printing (print-by-object) and toolhead clearance validation](./124-author-packet-sequential-printing-and-toolhead-clearance.md) — confirm premise still holds, update packet-standard obligations to the 2-file `orca-feature-gaps_<NN>_<slug>/` standard.
   - [127 — Rule on gyroid-infill's density for solid, top/bottom and bridge surfaces](./127-gyroid-bridge-and-solid-fill-density.md) — verify `modules/core-modules/gyroid-infill/src/lib.rs` still reads only `sparse_infill_density` after master merges.
   - [135 — Author packet — support base-pattern holder](./135-author-packet-support-base-pattern-holder.md) — explicitly reconcile the conflict with `support_base_pattern` references merged from `origin/master` (check `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` and `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`) and note any interaction with [151](./151-audit-merge-residue-and-bring-branch-gates-green.md).
   - [150 — Phase 5 width-limiting deletes slice area instead of returning it to BASE](./150-phase5-width-limit-drops-area-instead-of-returning-it-to-base.md) — verify `run_phase5_width_limit` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) still exhibits the defect after `proptest-openrscad` / `perf-vs-orca` merges.
   - Check the 12 `125`-blocked refiled tickets (`119`, `136`–`147`): verify their `Blocked by:` headers are accurate (including `119` on `125`) and note the updated 2-file packet standard so nobody authors a legacy 5-file packet once `125` resolves.
4. **Prune/update `## Not yet specified` fog patches** that were solved, changed, or invalidated on `master` (e.g., seam-plan painted variants vs ADR-0073, `STAGE_ORDER` vs `155`, `internal_solid_fill` / `ObjectMesh.transform`).

### Obligations

- Do **not** resolve the substantive HITL rulings in `125`, `127`, `128`, or `148` — only update their stale premises, blocking edges, and Map Notes so each ticket starts from truth when claimed.
- Re-derive every path and symbol from disk at edit time.
- Resolved when `map.md` and the affected open tickets' headers/premise notes are updated and verified cleanly, with the staleness banner narrowed to reflect that Map Notes are now re-grounded while Phase 2 packet re-alignment and open rulings remain in flight.

## Answer
