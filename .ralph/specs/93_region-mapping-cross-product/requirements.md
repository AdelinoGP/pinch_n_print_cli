# Requirements: 93_region-mapping-cross-product

## Packet Metadata

- Grouped task IDs:
  - `TASK-243` — RegionMapping cross-product expansion (variant_chain populated; polygons empty).
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1c — RegionMapping cross-product expansion"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

After P1a (schema) and P1b (manifest + dispatch), the IR has slots for `variant_chain` and the host knows how to filter by it — but no code populates `variant_chain`. The RegionMapping kernel at `crates/slicer-core/src/algos/region_mapping.rs` currently produces one `RegionPlan` per `(layer, ActiveRegion)` with empty `variant_chain` for every entry. Worse, the existing `overlapping_semantics_for_region` at lines 286-319 hardcodes `return true`, stamping every paint semantic onto every region regardless of object identity or geometric overlap. The current code is structurally broken in two ways:

1. **No cross-product expansion.** OrcaSlicer's `PrintApply.cpp:1138-1156` produces one `PaintedRegion` per `(volume_region × extruder_index)` combination actually present on the object. Our equivalent must produce one `RegionPlan` per `(ActiveRegion × variant_chain)` cross-product element. Without this expansion, paint-segmentation in P3 has no per-variant slot to write polygons into.
2. **`return true` paint overlay.** The current "stamp every paint semantic" logic isn't structurally cross-product — it's a hack predating the painted-variant model. It would, today, attribute material-1 paint from object A onto a region of object B if both objects share a layer. P3's downstream consumers would not survive this, so the cleanup happens here even though P3 is the consumer.

This packet rewrites the kernel to:
- Read `aggregated_region_split: BTreeMap<String, AggregatedRegionSplitEntry>` from the scheduler (P1b's output) — the registry of opted-in semantics.
- Scan each `mesh.objects[*].paint_data` once, collecting distinct `PaintValue`s per opted-in semantic per object.
- For each `(layer, ActiveRegion)`, enumerate the canonical cross-product of paint values present on that object (including the empty subset = base region), produce one `RegionPlan` per chain.
- Intern `ResolvedConfig`s via P1a's `intern_config` helper.
- Emit per-variant `RegionPlan` entries unconditionally — never gate by geometric coverage (D15; emit even when the variant would have empty polygons; P3 fills polygons or leaves them empty as appropriate).

Because no production module yet declares `[[region_split]]` (P3's task), `aggregated_region_split` is empty by default, and the cross-product collapses to the empty chain only — production behavior is preserved. Synthetic test manifests from P1b's fixture directory drive the cross-product tests in this packet.

## In Scope

- Rewrite `commit_region_mapping_builtin` (or equivalent kernel entry point in `crates/slicer-core/src/algos/region_mapping.rs`) to accept `aggregated_region_split` and use it.
- Add the per-object paint scan: `scan_paint_data(&mesh.objects, &aggregated_region_split) -> HashMap<ObjectId, HashMap<String, Vec<PaintValue>>>`.
- Add (or extract to `crates/slicer-ir/src/region_split_registry.rs`) the `enumerate_canonical_chains` helper that produces every subset of (semantic, value) pairs in canonical order.
- Replace the broken `overlapping_semantics_for_region` with per-object lookup OR delete it if the new flow makes it dead code.
- Update the producer wrapper `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` to thread `aggregated_region_split` from the scheduler into the kernel.
- Raise `DEFAULT_REGION_MAP_CAP` to 750_000.
- Update the cap-overflow diagnostic to include the worst-contributing `ObjectId`.
- Update the 5 GREEN-target cube_4color tests' assertions to inspect `RegionMapIR.entries` variant_chains.
- Leave 7 RED cube_4color tests RED (their polygon-coverage assertions belong to P3).
- Add a unit test for `enumerate_canonical_chains` with the 2×1 case (6 chains) and the 0-semantic case (1 chain = empty).
- Add a unit test for `scan_paint_data` (4-distinct-tool-index case).
- Add an integration test for the cap-overflow diagnostic.

## Out of Scope

- Paint-segmentation kernel — P3 (95).
- Mesh-segmentation host wiring — P2 (94).
- Any core module's manifest declaring `[[region_split]]` — P3.
- Per-variant polygon population — P3.
- Modifier-volume support routing — P3.
- WIT contract changes — none expected.
- Doc updates to `docs/02` or `docs/04` — P5c (99).
- Performance optimization beyond the cap raise (e.g., HashSet-based interner; can defer to a future packet).

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1c" (~140 lines; read directly).
- `docs/02_ir_schemas.md` — `RegionMapIR`, `RegionPlan`, `RegionKey` sections (range-read).
- `docs/04_host_scheduler.md` §"RegionMapping" stage if present (range-read).
- `crates/slicer-core/src/algos/region_mapping.rs` — primary edit site (likely > 300 lines; range-read).
- `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — small (≤ 60 lines); read in full.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:1138-1156` — cross-product expansion; SUMMARY ≤ 150 words.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp:243-289` — `PaintedRegion`/`FuzzySkinPaintedRegion`; SUMMARY ≤ 100 words.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-12`. Refinements:
  - `enumerate_canonical_chains` MUST be deterministic — same input ordering → same output ordering. The implementation uses the canonical order (BTreeMap iteration) for the semantic axis and a stable `PaintValue` comparator for tied semantics. Document the chosen ordering for `PaintValue` (e.g., `Flag < ToolIndex(0) < ToolIndex(1) < ... < Custom(s_lex)`) in the helper's doc-comment.
  - The cap-overflow diagnostic (AC-8, AC-N2) reuses the existing structured-event channel — no new event type.
  - AC-10's "7 RED tests stay RED" is an expected partial failure: the test bucket cargo invocation will return non-zero exit. The closure log documents the exact 7 test names so a future regression of those tests' assertions is distinguishable from new breakage.
- Negative cases: `AC-N1` (empty aggregation → only base variants), `AC-N2` (cap overflow), `AC-N3` (no Scalar in variant_chain).
- Cross-packet impact: unblocks P3 (paint-segmentation port can now write `replace_slice_ir` into per-variant `SlicedRegion` slots whose `RegionPlan` entries exist).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Workspace compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `cargo test -p slicer-core region_mapping 2>&1 \| tee target/test-output.log` | AC-2, AC-3, AC-4, AC-5, AC-N1, AC-N3 — kernel tests | FACT pass/fail with per-test breakdown |
| `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 \| tee target/test-output.log` | AC-9 (5 GREEN), AC-10 (7 RED) — cube acceptance | FACT pass-count, with explicit acknowledgement of expected failures |
| `cargo test -p slicer-runtime region_map_cap 2>&1 \| tee target/test-output.log` | AC-N2 — cap overflow | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p93-wedge.gcode && sha256sum /tmp/p93-wedge.gcode` | AC-11 — byte-identical g-code | FACT (sha256); compare to post-P92 baseline |
| `cargo xtask build-guests && cargo xtask build-guests --check` | AC-12 — guest WASM clean | FACT pass/fail |
| `! rg -nE 'fn overlapping_semantics_for_region[^}]*\n[^}]*return true' crates/slicer-core/src/algos/region_mapping.rs` | AC-7 — broken function removed | FACT pass/fail |

## Step Completion Expectations

- The `enumerate_canonical_chains` helper (Step 2) is implementation-tested in isolation BEFORE the kernel rewrite (Step 3). This separation isolates the algorithm bug surface from the threading bug surface.
- The producer wrapper (Step 4) is updated only AFTER the kernel signature settles in Step 3 — otherwise the producer-wrapper edit chases a moving target.
- The cube_4color RED-test review (Step 5) happens AFTER the kernel and producer settle; the GREEN/RED distribution may shift if the kernel's variant_chain shape doesn't match exactly what the cherry-pick's authors expected. The closure log explicitly enumerates the GREEN/RED test names.
- AC-11 byte-identical check is the most important regression guard for this packet: it confirms that production behavior (no module declares `[[region_split]]`) is unchanged. Any g-code diff must be root-caused before close.

## Context Discipline Notes

- `crates/slicer-core/src/algos/region_mapping.rs` is the primary edit site. It is likely > 400 lines. Range-read by symbol (`pub fn commit_region_mapping_builtin`, `fn overlapping_semantics_for_region`, helper fns) rather than full-load.
- `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` is small (≤ 60 lines); read in full.
- The cube_4color RED tests at `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` may be > 300 lines. Range-read by test name; do not load in full.
- Do NOT load the binary 3MF fixtures (`resources/cube_4color.3mf` etc.); delegate any structural inspection.
- The OrcaSlicer parity SUMMARY dispatches should NOT lead to opening any OrcaSlicer file — sub-agent return only.
