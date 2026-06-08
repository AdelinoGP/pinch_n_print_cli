# Requirements: 93_region-mapping-cross-product

## Packet Metadata

- Grouped task IDs:
  - `TASK-243` — RegionMapping cross-product expansion (variant_chain populated; polygons empty).
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1c — RegionMapping cross-product expansion"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

After P91 (schema) and P92 (manifest + dispatch), the IR has a `variant_chain` slot on `RegionKey` and the host knows how to dispatch by it — but no code populates it. `execute_region_mapping_inner` at `crates/slicer-core/src/algos/region_mapping.rs:384` produces one `RegionPlan` per `(layer, ActiveRegion)` with empty `variant_chain` for every entry. The chain dimension simply does not exist in the kernel today.

Today's `overlapping_semantics_for_region` (lines 286-319) computes config overlays at layer-wide granularity — it returns the set of paint semantics present anywhere on that layer, and the caller at line 494 applies those overlays to every `ActiveRegion` of every object on that layer. This is fit-for-purpose under the current single-entry-per-region model, but the cross-product model (D2) demands chain-keyed overlays: a `RegionPlan` with `variant_chain = [(material, ToolIndex(2))]` needs the `material:2` overlay specifically, NOT every `material:*` overlay present on the layer. The existing function's layer-wide return shape cannot express that.

OrcaSlicer's `PrintApply.cpp:1138-1156` solves the symmetric problem by emitting one `PaintedRegion` per `(volume_region × extruder_index)` combination present on the object. The shape we adopt is analogous: one `RegionPlan` per `(ActiveRegion × variant_chain)` element, where `variant_chain` ranges over the canonical cross-product of paint values present on the object across all semantics in `aggregated_region_split`.

This packet extends the kernel to:
- Accept `aggregated_region_split: &BTreeMap<String, AggregatedRegionSplitEntry>` from the scheduler (P92's output).
- Scan each `mesh.objects[*].paint_data` once, collecting distinct `PaintValue`s per opted-in semantic per object.
- For each `(layer, ActiveRegion)`, enumerate the canonical cross-product (including the empty subset = base region) and emit one `RegionPlan` per chain. Per-variant polygons remain empty in this packet — P95 fills them later.
- Intern `ResolvedConfig`s via the P91 helper so the 16-color × 16-variant case does not replicate full configs in `RegionMapIR.configs`.
- Derive each chain's overlay by folding the matching paint-semantic `ResolvedConfig`s onto the modifier-stamped base via `overlay_resolved` (line 110). This is the chain-derived overlay path that replaces `overlapping_semantics_for_region`'s layer-wide derivation.
- Delete `overlapping_semantics_for_region` and its call site at line 494. The chain-derived path with `chain = []` (the empty chain emitted for every region when `aggregated_region_split.is_empty()`) reproduces the deleted path's output exactly — there is no need for a separate fallback.
- Preserve `stamp_modifier_config_deltas` (line 217); modifier-volume stamping composes with the chain dimension as the base on which paint-semantic overlays fold.

Because no production module declares `[[region_split]]` in P93's scope (P95's task), `aggregated_region_split` is empty by default and the cross-product collapses to the empty chain only — production behavior is preserved. The byte-identical g-code check (AC-10) is the integration-level verification of that equivalence.

## In Scope

- Extend `execute_region_mapping_inner` at `crates/slicer-core/src/algos/region_mapping.rs:384` to accept `aggregated_region_split` and execute the cross-product loop.
- Add the per-object paint scan: `scan_paint_data(&mesh.objects, &aggregated_region_split) -> HashMap<ObjectId, HashMap<String, Vec<PaintValue>>>`.
- Add (or extract to `crates/slicer-ir/src/region_split_registry.rs`) the `enumerate_canonical_chains` helper that produces every subset of (semantic, value) pairs in canonical order.
- Delete `overlapping_semantics_for_region` (line 286) AND its call site at line 494. Replace with the chain-derived overlay path that folds matching paint-semantic `ResolvedConfig`s onto the modifier-stamped base via `overlay_resolved`. The empty-chain case for `aggregated_region_split.is_empty()` is the new "fallback" — there is no separate fallback code path.
- Preserve `stamp_modifier_config_deltas` at line 217 — the cross-product loop composes with modifier-volume stamping rather than replacing it.
- Update the producer wrapper `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` to thread `aggregated_region_split` from the scheduler into the kernel.
- Raise `DEFAULT_REGION_MAP_CAP` from its current value `1_000` (at `crates/slicer-ir/src/slice_ir.rs:1196`) to `750_000`; update the doc-comment with the 750× headroom rationale.
- Update the cap-overflow diagnostic to include the worst-contributing `ObjectId`.
- Add six net-new kernel unit tests in `crates/slicer-core/tests/algo_region_mapping_tdd.rs` (enumerated in `packet.spec.md` AC-9).
- Add a unit test for `enumerate_canonical_chains` with the 2×1 case (6 chains) and the 0-semantic case (1 chain = empty), housed in `slicer-ir`.
- Add a unit test for `scan_paint_data` (4-distinct-tool-index case).
- Add an integration test for the cap-overflow diagnostic.

## Out of Scope

- Paint-segmentation kernel — P95.
- Mesh-segmentation host wiring — P94.
- Any core module's manifest declaring `[[region_split]]` — P95.
- Per-variant polygon population — P95.
- **Empty-polygon `RegionPlan` filtering** — P95 owns this. Polygons live on `SlicedRegion` (populated by P95 via `replace_slice_ir`), not on `RegionPlan`; the kernel cannot predict polygon emptiness from `ActiveRegion` alone. P95 has the polygons in hand and is the rightful owner of the emptiness gate.
- Cube_4color test outcomes — all 12 tests in `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` target paint-segmentation behavior (Material ToolIndex per point, projection coverage, banded strokes, etc.) and remain P95's acceptance concern.
- Modifier-volume stamping behavior changes — `stamp_modifier_config_deltas` is preserved as-is; only its position in the loop (called before the chain fold) is new.
- WIT contract changes — none expected.
- Doc updates to `docs/02` or `docs/04` — P5c (99).
- Performance optimization beyond the cap raise (e.g., HashSet-based interner; can defer to a future packet).

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1c" (~140 lines; read directly).
- `docs/02_ir_schemas.md` — `RegionMapIR`, `RegionPlan`, `RegionKey` sections (range-read).
- `docs/04_host_scheduler.md` §"RegionMapping" stage if present (range-read).
- `crates/slicer-core/src/algos/region_mapping.rs` — primary edit site (535 lines; range-read by symbol).
- `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — small (≤ 60 lines); read in full.
- Cherry-pick `5c272ef` provides paint-segmentation acceptance fixtures consumed by P95, not P93.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:1138-1156` — cross-product expansion; SUMMARY ≤ 150 words.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp:243-289` — `PaintedRegion`/`FuzzySkinPaintedRegion`; SUMMARY ≤ 100 words.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-11` (twelve criteria after the AC-7 → AC-7+AC-7b split and the AC-10 drop). Refinements:
  - `enumerate_canonical_chains` MUST be deterministic — same input ordering → same output ordering. The implementation uses the canonical order (BTreeMap iteration) for the semantic axis and a stable `PaintValue` comparator for tied semantics. Document the chosen ordering for `PaintValue` (e.g., `Flag < ToolIndex(0) < ToolIndex(1) < ... < Custom(s_lex)`) in the helper's doc-comment.
  - The cap-overflow diagnostic (AC-8, AC-N2) reuses the existing structured-event channel — no new event type.
  - AC-7b's overlay-equivalence assertion is exact `ResolvedConfig` equality, not a heuristic. The unit test compares the chain-derived `effective_config` to a fixture that captures the pre-packet layer-wide path's output for the same input.
- Negative cases: `AC-N1` (empty aggregation → only base variants), `AC-N2` (cap overflow), `AC-N3` (no Scalar in variant_chain).
- Cross-packet impact: unblocks P95 (paint-segmentation port can now write `replace_slice_ir` into per-variant `SlicedRegion` slots whose `RegionPlan` entries exist). P95 owns the empty-polygon ownership clause clarified in its design.md §Architecture Constraints.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Workspace compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `cargo test -p slicer-core region_mapping 2>&1 \| tee target/test-output.log` | AC-2, AC-3, AC-4, AC-5, AC-7b, AC-9 (six net-new tests), AC-N1, AC-N3 — kernel tests | FACT pass/fail with per-test breakdown |
| `! rg -q 'overlapping_semantics_for_region' crates/slicer-core/src/algos/region_mapping.rs` | AC-7 — function deleted | FACT pass/fail |
| `cargo test -p slicer-runtime region_map_cap 2>&1 \| tee target/test-output.log` | AC-N2 — cap overflow | FACT pass/fail |
| `rg -q 'DEFAULT_REGION_MAP_CAP\s*[:=]\s*750_000' crates/slicer-ir/src/slice_ir.rs` | AC-8 — cap value | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p93-wedge.gcode && sha256sum /tmp/p93-wedge.gcode` | AC-10 — byte-identical g-code | FACT (sha256); compare to post-P92 baseline |
| `cargo xtask build-guests && cargo xtask build-guests --check` | AC-11 — guest WASM clean | FACT pass/fail |

## Step Completion Expectations

- The `enumerate_canonical_chains` helper (Step 2) is implementation-tested in isolation BEFORE the kernel extension (Step 3). This separation isolates the algorithm bug surface from the threading bug surface.
- The chain-derived overlay path (Step 3) lands together with the deletion of `overlapping_semantics_for_region` and its line-494 caller. Splitting these is not safe — partial deletion leaves the kernel calling a function that no longer exists.
- The producer wrapper (Step 4) is updated only AFTER the kernel signature settles in Step 3 — otherwise the producer-wrapper edit chases a moving target.
- The six net-new kernel unit tests (Step 6) MUST assert exact `variant_chain` keysets and exact `ResolvedConfig` equivalence (AC-7b). Range or threshold assertions are not acceptable — drift hides regressions.
- AC-10 byte-identical check is the most important regression guard for this packet: it confirms that production behavior (no module declares `[[region_split]]`) is unchanged AND that the chain-derived overlay path's empty-chain case is equivalent to the deleted layer-wide path. Any g-code diff must be root-caused before close.

## Context Discipline Notes

- `crates/slicer-core/src/algos/region_mapping.rs` is the primary edit site (535 lines). Range-read by symbol (`pub fn execute_region_mapping_inner` line 384, `fn overlay_resolved` line 110, `fn stamp_modifier_config_deltas` line 217, `fn overlapping_semantics_for_region` line 286) rather than full-load.
- `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` is small (≤ 60 lines); read in full.
- The cube_4color test file is **out of scope** for this packet — do not load it. Its 12 tests target paint-segmentation (P95) and are not gated here.
- Do NOT load the binary 3MF fixtures (`resources/cube_4color.3mf` etc.); delegate any structural inspection.
- The OrcaSlicer parity SUMMARY dispatches should NOT lead to opening any OrcaSlicer file — sub-agent return only.
