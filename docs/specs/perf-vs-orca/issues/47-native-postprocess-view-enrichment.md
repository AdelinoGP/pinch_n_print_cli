# Native postprocess-view enrichment repair (ticket 33's named fix)

Type: task
Status: resolved (2026-10-03; three projection gaps fixed, t33 gate re-run from proof PASSES — see ## Answer)
Assignee: this session (wayfinder; human-authorized take 2026-10-02)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Can the native (integrated-edition) dispatch be made output-equivalent to the
external/WASM dispatch for `Layer::InfillPostProcess`, by enriching the native
request's `perimeter_regions` from the arena's partitioned `SliceIR` exactly
the way `push_infill_postprocess_regions`
(`crates/slicer-wasm-host/src/dispatch.rs`) does — including `tool_index` and
`wall_source_region_id` — so `RoleBoundaries::is_partitioned`
(`modules/core-modules/infill-linker/src/orchestrate.rs`) is reachable through
the real native request builder, ticket 37's `Some(empty)` verdict applies on
both legs, and the
[Integrated/external matched-output oracle gate](33-integrated-external-matched-output-oracle-gate.md)
re-runs from proof and PASSES on the examined cell?

## Localized root cause (from ticket 33's gate, static + two-job reproduction)

Claim record (2026-10-02): human authorized this take via the wayfinder
session's take-selection question; the claim is this Status line plus the
map update below. No other session may take it concurrently.

`build_native_layer_request_impl`
(`crates/slicer-wasm-host/src/marshal/native.rs`) builds `perimeter_regions`
from `input.perimeter` via `PerimeterRegionView::from_ir`
(`crates/slicer-sdk/src/views.rs`), which carries only what `PerimeterIR`
holds: walls, `infill_areas`, seam data, variant chain. The four partitioned
fill polygons (`sparse_infill_area`, `top_solid_fill`, `bottom_solid_fill`,
`bridge_areas`), `raft_fill`, `tool_index`, and `wall_source_region_id` all
default empty/0/`None`. The WASM leg's
`push_infill_postprocess_regions` instead iterates the `SliceIR` regions
(whose fill polygons are the partitioned, pairwise-disjoint polygons written
by `sync_perimeter_infill_areas_into_slice`
(`crates/slicer-runtime/src/region_partition.rs`)), copies donor wall geometry
from the region's own `PerimeterIR` entry (else the wall-source base region's
entry), mirrors the five fill fields verbatim, resolves the tool index via
`resolve_region_tool_index` (`crates/slicer-wasm-host/src/dispatch.rs`), and
reports `wall-source-region-id` for virtual regions. With the native leg's
all-empty partitions the linker's `is_partitioned()` is false, `for_role`
hands every role the union boundary, and the integrated leg prints paths the
external leg clips away (benchy classic-off: Sparse 204→143, Bridge 22→7,
Bottom 24→7, Top 70→31 `;TYPE:` transitions external→integrated).

## Answer

**Resolved (2026-10-03): the t33 oracle gate re-runs from proof and PASSES on
the examined cell (benchy classic supports-off), byte-identical arms
(`e1088b3d…`, 4,332,881 bytes, 3 measured ABBA pairs + proof), and the frozen
t44 job corroborates byte-identical (`11e7d38a…`).** Three projection gaps
were fixed, not one:

1. **The scoped repair** — native `Layer::InfillPostProcess` perimeter views
   now enriched from the partitioned `SliceIR` exactly as
   `push_infill_postprocess_regions` does (`native_infill_postprocess_regions`,
   `crates/slicer-wasm-host/src/marshal/native.rs`): per-`SliceIR`-region
   views, `PerimeterIR` wall donor (own entry → wall-source base → empty),
   four role partitions + `raft_fill` verbatim, pinned
   `resolve_region_tool_index`, `wall_source_region_id`, `Custom`→
   `ToolIndex(0)` degrade, `config_by_region` config; both WASM fallbacks
   mirrored. `RoleBoundaries::is_partitioned` is reachable natively and
   ticket 37's `Some(empty)` verdict applies on both legs.
2. **Ordered-entities snapshot** (surfaced by the re-run gate's residual
   ordering deltas): the native `run_path_optimization`/`run_anchored_events`
   arms handed modules an EMPTY `LayerCollectionBuilder`, so travel
   reordering silently skipped natively. `NativeLayerRequest.ordered_entities`
   now carries the staged snapshot (`project_ordered_entities_from`) for the
   two builder-consuming stages, and the macro arms populate the SDK builder
   — the same content the WASM seam's `get-ordered-entities` returns.
3. **Finalization annotation channel** (surfaced as the last byte delta; the
   direction is the reverse of the scoped gap): the external/WASM leg had
   ALWAYS silently dropped every `FinalizationOutputBuilder` annotation —
   `part-cooling`'s `M106`/`M107` fan commands never reached external output
   — because the WIT builder had no annotation method and the macro drain-back
   discarded `sdk_output.annotations()`. The human authorized the WIT repair
   in-take: `finalization-output-builder.push-annotation`
   (`annotation-view` 1:1 with `LayerAnnotation`), package
   `slicer:finalization-layer-finalization` **1.0.0 → 1.1.0** (additive
   method); host records + drains the stream in emission order;
   `apply_finalization_pushes` merges it like the native `apply_to` already
   did; the macro drain-back replays it. **Disclosure: both legs' output
   changes** (external gains fan commands) — pre-repair external baselines
   (e.g. t44's accepted `1b71f83d…`) are no longer current for jobs that run
   part-cooling; post-repair references: classic-off `e1088b3d…`,
   t44-job `11e7d38a…`.

Contract pins (all in `crates/slicer-wasm-host/tests/contract/`):
`infill_postprocess_view_identity_tdd` (native↔WASM postprocess-view
identity + fallbacks + snapshot), `finalization_annotation_relay_tdd`
(comment/raw/M107 round-trips, cross-kind emission order). Evidence:
[t47 FINDINGS](../evidence/t47-native-postprocess-enrichment/FINDINGS.md)
(three fix dossiers, attempt-by-attempt measured deltas, gate tables, rerun
commands). Integrated timings are no longer disqualified **on the examined
cell, bounded to it**; timing use and any adoption/default change still need
their own authorization. Committed as `895aab98` (fix 1), `cd28edd1`
(fix 2), `953fa0b1` (fix 3); keep/drop of the repair itself: human decision
follows, recommended KEEP (the gate result depends on it).

## Work

- Mirroring, not reusing: the WASM leg's enrichment operates on WIT data
  inside a `wasmtime::Store`; the native leg operates on SDK views. Share no
  code where the representations differ; mirror the donor/enrichment logic
  field-for-field and pin the two legs equal in a contract test
  (precedent: the ticket-37 empty-commit mirror, and ticket 23's
  `view_seam_identity_tdd.rs` — this take extends the view-seam identity
  contract to postprocess views).
- Native leg changes only `build_native_layer_request_impl` (or a helper it
  calls) for `stage_export == "Layer::InfillPostProcess"`; every other
  stage's `perimeter_regions` projection is untouched. Preserve the two
  WASM-leg fallbacks: no `PerimeterIR` → empty region list; `SliceIR`
  missing → legacy `PerimeterIR`-driven views. Config resolution stays the
  existing `config_by_region` map (same eligibility gate, same declared-keys
  filter as the WASM leg's `config_fields_per_region`).
- View-seam lossiness: replicate the WASM leg's observable view content —
  including `PaintValue::Custom` degrading to `ToolIndex(0)` in the view's
  variant chain (`crates/slicer-wasm-host/src/marshal/leaf.rs` note) — so a
  native module cannot observe anything a WASM module cannot.
- Prove equivalence at the field level first, then re-run the t33 oracle gate
  from proof (snapshots rebuilt via `cargo xtask dist --edition
  developer`/`--edition integrated`, guests fresh) on the same first cell,
  plus the frozen t44 job corroboration. Byte equality external==integrated
  is the gate; timings disclosed for load context only.
- Gate commands: `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo xtask check-literals`, `check-test-quality --report` on touched
  code, `cargo xtask build-guests --check`, narrow contract tests.

## Not authorized

Production/default-mode switches, adoption retries, auto-commit of perf
candidates beyond this correctness repair, or any weakening of the standing
gates. Keep/drop of the repair itself returns to the human with the gate
verdict.

## Evidence

[Ticket 33 findings](../evidence/t33-integrated-oracle/FINDINGS.md) (root
cause + rerun commands; the FAILED proof evidence stays preserved — the gate
re-run captures into its own evidence directory so the one-shot guard and the
historical record both stay intact).