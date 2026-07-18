# Refinement Audit: 93_region-mapping-cross-product

Generated during the `spec-packet-generator refine` pass on the draft packet. All three audits have been **RESOLVED** in the subsequent refinement pass — see "Decision applied" on each section. The cross-cutting silent fixes are listed in §4.

---

## Audit 1 — The "broken `return true` bug" framing — **RESOLVED (additive framing locked)**

**Decision applied**: Goal, Problem Statement, and AC-7 rewritten as additive ("introduce the chain dimension; migrate the existing layer-wide overlay path to chain-derived"). `overlapping_semantics_for_region` AND its call site at line 494 are fully DELETED in this packet; the chain-derived overlay path's empty-chain case (`chain = []`) reproduces the deleted path's output exactly. AC-7 now asserts deletion (`! rg -q 'overlapping_semantics_for_region' ...`); AC-7b asserts overlay equivalence with a recorded baseline fixture; AC-10 (byte-identical g-code) is the integration-level verification.

### Claim under audit
The packet's Goal, requirements.md §Problem Statement, AC-7, and design.md all assert that `overlapping_semantics_for_region` at `region_mapping.rs:286-319` contains a hardcoded `return true` "stamps every paint semantic onto every region regardless of object" bug, and that P93 fixes it as part of the cross-product expansion.

### Source-of-truth findings (from sub-agent inspection of the actual code)

1. The function body (lines 286-319) returns `Vec<PaintSemantic>`, not `bool`. It builds an iterator over `paint_regions.per_layer[global_layer_index].semantic_regions.keys()`, filters by an `srs.iter().any(|sr| { ... })` predicate, sorts, and returns.
2. The inner predicate has two arms:
   - Arm A (`if sr.polygons.is_empty() { return true; }`) — documented in the function's comment as "whole-layer coverage". This is a deliberate convention, not a bug.
   - Arm B (the final unconditional `true`) — accompanied by an inline comment explaining: `ActiveRegion` carries no polygon data at this stage, so the kernel cannot do a real geometric intersection check; treating any non-empty SemanticRegion as overlapping is the documented current behavior.
3. **Call sites in the whole repo: 1.** That call site is at `region_mapping.rs:494`, inside the per-region loop that derives `effective_config` and `paint_overrides`. The returned `Vec<PaintSemantic>` is used to apply paint-semantic config overlays in sort order. It does **not** gate inclusion in `RegionMapIR.entries` — the region is already being emitted by the time `overlapping_semantics_for_region` is consulted; only the overlay set is affected.
4. The function never returns `false` in any realistic path; the inner predicate's two arms together amount to "always yes if any SemanticRegion exists for this semantic on this layer". This is structurally close to the packet's accusation, but the consequence is config-overlay scope, not variant-chain stamping.

### What's actually true vs the packet's claim

| Packet claim | Reality |
| --- | --- |
| "Hardcoded `return true` bug" | Two arms, both yielding `true` via documented conventions. Not a hardcoded constant. |
| "Stamps every paint semantic onto every region regardless of object" | The stamping is real but affects `effective_config` overlays, not `RegionMapIR.entries` membership. The cross-product/variant_chain dimension does not exist in the current code at all. |
| "P93 replaces it with per-object cross-product expansion" | P93 should ADD per-object cross-product expansion (a new concern). Whether `overlapping_semantics_for_region` becomes dead code depends on whether the new flow subsumes the existing config-overlay pathway. That is not yet decided in the packet. |
| AC-7's verification: `! rg -nE 'fn overlapping_semantics_for_region[^}]*\n[^}]*return true'` | Would pass today (the regex doesn't match the current multi-line filter body), so AC-7 has no signal even pre-implementation. |

### Recommendation (not applied)

Rewrite the framing as **additive**:

- P93 introduces cross-product expansion as a new dimension of `RegionMapIR.entries` keying — this dimension does not currently exist.
- `overlapping_semantics_for_region` and its lone caller at line 494 become **either** (a) dead code subsumed by the new chain-derived overlay logic, **or** (b) retained but explicitly relegated to the "fallback when `aggregated_region_split.is_empty()`" path. The packet must pick one and assert it.
- AC-7 is replaced by an AC that asserts the new chain-derived overlay path is reachable (positive coverage), plus a follow-up AC that asserts behavior when no region-split semantic is declared (negative coverage; ties into the existing AC-N1 on empty-aggregation behavior).

Until this is decided, the packet's "bug fix" narrative misleads the implementer about what's being changed.

---

## Audit 2 — Empty-polygon filter `[BLOCK]` in design.md — **RESOLVED (Option B locked)**

**Decision applied**: Option B — defer to P95. P93 keeps D15 unconditional emission; polygon-emptiness is P95's responsibility (polygons live on `SlicedRegion`, populated by P95 via `replace_slice_ir`). The `[BLOCK]` section in `design.md` has been removed; the decision is recorded as a Locked Assumption in `design.md` §Locked Assumptions ("Empty-polygon entries persist (D15); the filter lives at P95"). Cross-packet ripples applied: P92's design.md clarification + P95's design.md ownership clause (§Architecture Constraints).

### The `[BLOCK]`
design.md §"Empty-Polygon Filter Decision (from P92 audit) — OPEN QUESTION" (lines 19-26) asks: should the kernel filter empty-polygon `RegionPlan` entries (Option A, override D15 partially), or defer the filter to P95 paint-segmentation (Option B)?

### Architectural facts

- `RegionPlan` lives in `RegionMapIR.entries`. Polygons live on `SlicedRegion` in `SliceIR.regions`. The two IRs are filled by different stages: RegionMapping (this packet) emits `RegionPlan`; paint-segmentation (P95) emits `SlicedRegion`s via `replace_slice_ir`.
- At the time `RegionMapIR.entries` is being built, **the kernel does not have per-variant polygons** — it has `ActiveRegion`s (the result of the active-region stage), which carry no polygon data per `overlapping_semantics_for_region`'s own inline comment.
- Therefore "filter empty-polygon entries" at the kernel would require the kernel to predict what P95 will produce. That is a wrong-direction dependency.

### Why this looks tempting anyway

The audit context P92 left behind hints at concern that `RegionMapIR.entries` cardinality explodes under cross-product if variants with no actual paint coverage are emitted unconditionally. But:

- For an object that has paint values `{ToolIndex(1), ToolIndex(3)}` for `material`, the cross-product produces 3 chains: `[]`, `[material:1]`, `[material:3]`. None of these are "polygon-empty" at this stage because polygons don't exist yet; what the cap controls is **chain cardinality**, not polygon emptiness.
- The genuine post-P95 question — "should a `RegionPlan` whose `SlicedRegion` siblings ended up polygon-empty be removed" — is naturally P95's concern: P95 has the polygons in hand and can decide whether to also drop the matched `RegionPlan` (or leave it as a no-op).

### Recommendation (not applied)

Lock **Option B** in a future pass:
- Remove the `[BLOCK]` from design.md.
- Add one line to P95's design (when that packet is generated) noting that empty-polygon entries are P95's responsibility.
- P93's D15 (unconditional emission) stays.

This unblocks P93 for activation once P92 closes, without changing the kernel's data ownership.

---

## Audit 3 — Cube_4color AC-9 / AC-10 has no backing in the current test file — **RESOLVED (AC-9 rescoped, AC-10 dropped)**

**Decision applied**: AC-9 rescoped to six net-new kernel unit tests in `crates/slicer-core/tests/algo_region_mapping_tdd.rs` that drive `execute_region_mapping_inner` with synthetic mesh + synthetic `BTreeMap<String, AggregatedRegionSplitEntry>` and assert exact `variant_chain` keysets. AC-10 (the "7 RED stay RED" line) dropped; cube_4color tests remain P95's acceptance concern. Subsequent ACs renumbered: AC-11 → AC-10 (byte-identical g-code), AC-12 → AC-11 (guest WASM). Cherry-pick `5c272ef` references removed from P93; a single lineage note added to `requirements.md` §Authoritative Docs ("Cherry-pick 5c272ef provides paint-segmentation acceptance fixtures consumed by P95, not P93").

### Facts from the test file
`crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` is 1115 lines and contains 12 `#[test]` functions:

| # | Line | Test | What it asserts |
| --- | --- | --- | --- |
| 1 | 169 | `cube_4color_full_pipeline_no_panic` | Pipeline smoke test. |
| 2 | 225 | `cube_4color_paint_segmentation_4_tool_indices_across_layers` | ≥4 distinct Material ToolIndex values across 50 layers. |
| 3 | 264 | `cube_4color_all_50_layers_have_layer_map_entries` | Every layer has a `LayerPaintMap` entry. |
| 4 | 292 | `cube_4color_mid_layer_has_material_paint` | Z=12.5mm has Material paint on contour points. |
| 5 | 369 | `cube_4color_fuzzy_without_data_is_error` | FuzzySkin request without FuzzySkin paint → error. |
| 6 | 438 | `cube_4color_top_face_two_tool_indices_requires_projection_coverage` | RED — top-face ToolIndex coverage. |
| 7 | 513 | `cube_4color_bottom_face_painted_and_unpainted_requires_projection_coverage` | RED — bottom-face projection. |
| 8 | 596 | `cube_4color_top_face_per_point_variation` | RED — per-point variation. |
| 9 | 708 | `cube_4color_front_face_banded_by_z_requires_subfacet_strokes` | RED — banded strokes. |
| 10 | 817 | `cube_4color_left_face_circles_produce_per_point_variation` | RED — circles. |
| 11 | 928 | `cube_4color_right_face_uniform_requires_vertical_face_projection` | RED — vertical projection. |
| 12 | 1024 | `cube_4color_back_face_uniform_requires_vertical_face_projection` | RED — vertical projection. |

None of these are `#[ignore]`d.

### Critical observation
**None of the 12 tests assert on `RegionMapIR.entries` or `variant_chain` shape.** Every assertion targets paint-segmentation outcomes: Material ToolIndex per point, projection coverage, banded strokes, per-point variation, etc. These are P95 territory.

### Implication for AC-9 / AC-10

- AC-9 promises that "5 cube_4color tests turn GREEN: variant_chain assertions" after P93. With the current test file, **zero tests assert on variant_chain**. AC-9 cannot be verified as written.
- AC-10's verification regex `grep -qE 'test result: (FAILED|ok)\.'` passes whether the test bucket fully fails or fully passes. It does not gate the 5/7 distribution and never has.
- The packet references "cherry-pick 5c272ef's RED suite" as the source of the 5/7 split. That cherry-pick either has not been applied to this branch, or applies a different file than `cube_4color_paint_tdd.rs`. The implementer would need to verify this before Step 6 of the implementation-plan can begin.

### Recommendation (not applied)

Rescope along these lines:
- **AC-9 → net-new kernel unit tests.** Add N tests under `crates/slicer-core/tests/algo_region_mapping_tdd.rs` that drive `execute_region_mapping_inner` with a synthetic mesh + synthetic `BTreeMap<String, AggregatedRegionSplitEntry>` and assert variant_chain shape on the resulting `RegionMapIR.entries`. These are net-new GREEN, not retargeted RED.
- **AC-10 → drop.** The existing 12 cube_4color tests stay RED until P95 lands and remain P95's acceptance concern. P93 does not owe a cube_4color outcome.
- **Cherry-pick 5c272ef references → remove from P93** (or move to a P95 lineage note if/when that packet is generated).

If the cherry-pick truly exists and was meant to land before P93, the recommended action instead is: add a hard prerequisite to P93 ("cherry-pick 5c272ef before implementation begins") and verify the resulting tests' assertion shape before locking the 5/7 split.

---

## §4 — Cross-cutting silent fixes applied during this refinement

These were the silent fixes the user authorized. They have been applied to `packet.spec.md`, `requirements.md`, `design.md`, and `implementation-plan.md`:

| Fix | Before (in current draft) | After (in refined packet) |
| --- | --- | --- |
| Kernel entry function name | `commit_region_mapping_builtin` (does not exist) | `execute_region_mapping_inner` at `region_mapping.rs:384` |
| Config-overlay helper | `derive_resolved_config` (does not exist) | `overlay_resolved` at `region_mapping.rs:110` (existing) |
| `DEFAULT_REGION_MAP_CAP` baseline | "presumably 250_000 or similar" (speculation) | `1_000` (actual, at `crates/slicer-ir/src/slice_ir.rs:1196`) |
| Cap-raise rationale | "raise to 750_000" with 250_000-baseline framing | Raise to 750_000 from 1_000 baseline; rationale block notes the 750× jump and the 16-color × 1000-layer × 16-region headroom assumption |
| Modifier-volume routing scope | "Out of scope — P3" | Out of scope, AND `stamp_modifier_config_deltas` at `region_mapping.rs:217` is explicitly preserved; cross-product loop must compose with modifier stamping rather than replace it |
| Goal / Problem Statement overlap | Goal in `packet.spec.md` restated motivation | Goal trimmed to one solution-shaped sentence; motivation-shape lives only in `requirements.md` §Problem Statement |

---

## What remains for a future refinement pass

All three audits are now resolved (decisions applied above). The remaining outstanding items are routine:

1. The two `[FWD]` open questions in `design.md` (where `aggregated_region_split` enters the producer wrapper; whether `paint_semantic_configs` already exists in kernel scope post-P91). Both are resolved by Step 1 / Step 3 sub-agent dispatches in `implementation-plan.md`; neither blocks activation.
2. Activation gates: P91 `implemented` (already met), P92 `implemented` (still `draft` at time of refinement — packet remains `draft` until P92 closes).

## Cap target re-evaluation (Audit §4 follow-up) — **RESOLVED (750_000 kept)**

**Decision applied**: keep `750_000`. Reasoning baked into `design.md` §Locked Assumptions:

- Cross-product cardinality bound: `∏(1 + K_i)` over opted-in semantics, summed over (objects × layers × base_regions).
- Worst-case envelope from the roadmap: 16-color × 1000-layer × 16-region × 3-modifier ≈ 1.6M pre-modifier-stamp entries.
- The 1_000 baseline (`crates/slicer-ir/src/slice_ir.rs:1196`) is the legacy default for single-entry-per-region scenes. The 750× jump to 750_000 admits realistic multi-color scenes (4-color × 200-layer × 4-region ≈ 3200 entries — comfortably under cap) while still failing on pathological inputs.
- The `RegionMapIR.entries` HashMap allocates per-entry; 1.6M entries costs ~160 MB minimum — exceeding the cap and surfacing the diagnostic is the intended outcome for genuinely-pathological inputs.
