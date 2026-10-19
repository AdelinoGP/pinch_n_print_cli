# Ticket 47 — Native postprocess-view enrichment repair: FINDINGS

Route ticket: [Native postprocess-view enrichment repair](../../issues/47-native-postprocess-view-enrichment.md).
Enabling gate: [Integrated/external matched-output oracle gate](../../issues/33-integrated-external-matched-output-oracle-gate.md)
(gate FAILED 2026-10-02 with the fix scoped but unwritten).

**Verdict: the t33 oracle gate now PASSES from proof on the examined cell
(benchy classic supports-off), and the frozen t44 job corroborates
byte-identical arms.** Three native-dispatch projection gaps were found and
fixed in this take — the scoped postprocess-view enrichment, plus two the
gate's residual deltas exposed. The committed tree is the acceptance subject;
no adoption/production change, no default-mode switch.

## Fix 1 (the scoped repair): `Layer::InfillPostProcess` perimeter-view enrichment

`build_native_layer_request_impl`
(`crates/slicer-wasm-host/src/marshal/native.rs`) built `perimeter_regions`
from `PerimeterIR` via `PerimeterRegionView::from_ir`
(`crates/slicer-sdk/src/views.rs`), whose four role-partition fields default
empty — the linker's `RoleBoundaries::is_partitioned`
(`modules/core-modules/infill-linker/src/orchestrate.rs`) was unreachable and
`for_role` handed every role the union boundary. The native leg now routes
`Layer::InfillPostProcess` through `native_infill_postprocess_regions`: one
view per `SliceIR` region, wall geometry from the region's own `PerimeterIR`
entry (else the `wall_source_region_id` base donor), the four partitions +
`raft_fill` mirrored verbatim, `tool_index` via `resolve_region_tool_index`
(`crates/slicer-wasm-host/src/dispatch.rs`), `wall_source_region_id` for
virtual regions, `Custom` paint values degraded to `ToolIndex(0)` exactly as
the WIT seam shows them to WASM modules, and per-region config via the same
`config_by_region` table. Both WASM fallbacks mirrored (no `PerimeterIR` →
empty list; missing `SliceIR` → legacy views); other stages untouched.
Contract pin: `infill_postprocess_view_identity_tdd`
(`crates/slicer-wasm-host/tests/contract/`).

Measured effect (proof attempt 1 vs t33's FAILED gate, benchy classic-off
integrated arm): bytes 4,550,163 → 4,329,951 (external 4,330,081); per-role
printed length totals went from +2,322 mm excess to 0.0 mm delta over the
101,091 mm job (`TYPE:` counts still differed: Sparse 204→140, Bottom 24→7,
Bridge 22→7, Top 70→31). Evidence preserved in
`proof-attempt1-postprocess-only/`.

## Fix 2: the ordered-entities snapshot at native path-optimization

The remaining gate delta was a path-ordering divergence localized to
individual layers (t44 job: 6 of 240 layers carry real move-multiset deltas;
classic-off: layer-0 re-grouping). Root cause: the native
`run_path_optimization` arm built an empty `LayerCollectionBuilder`, so
`get_ordered_entities()` returned nothing and the module skipped travel
reordering — while the WASM arm pushes the staged snapshot via
`project_ordered_entities_from` (`crates/slicer-wasm-host/src/dispatch.rs`).
`NativeLayerRequest` gained `ordered_entities`
(`crates/slicer-sdk/src/native.rs`); `build_native_layer_request_impl`
populates it for `Layer::PathOptimization` and `Layer::AnchoredEvents`
(the two builder-consuming stages) from `input.layer_collection`; the macro
arms feed it to the SDK builder (`crates/slicer-macros/src/lib.rs`). Pin:
`native_ordered_entities_snapshot_matches_the_wasm_seam` (same contract file).

Measured effect (proof attempt 2): integrated bytes 4,332,881 == external
4,332,881 on TYPE counts, but sha differed only by annotation presence —
the external leg emitted ZERO fan commands (integrated 281).

## Fix 3: the finalization annotation channel (WAS missing on the WASM seam)

The integrated leg emitted 281 fan commands (`M106 S255` ×279, `M107` ×2);
the external leg, zero. Direction check: `part-cooling` authors
`push_fan_speed`/`push_annotation` onto `FinalizationOutputBuilder`; the
native commit merges them via `apply_to`; the WASM drain-back
(`build_finalization_world_glue`, `crates/slicer-macros/src/lib.rs`) had **no
WIT channel to replay them** — the finalization-output-builder resource
exposed no annotation method, so every guest annotation was silently dropped
on the external transport. This predates ticket 47 (the t33 findings even
recorded "an inserted `M106 S255`" as integrated-side noise without
attributing its absence on the external leg). This is the opposite direction
of fix 1: the external leg was output-degraded, not the native one.

Human-authorized in-take: add
`finalization-output-builder.push-annotation` (`annotation-view`:
layer-index, after-entity-index, kind = comment(string)|raw(string)),
mirroring `LayerAnnotation` 1:1; package
`slicer:finalization-layer-finalization` **1.1.0** (additive method, minor
bump; `STAGES`/`package_for_stage_id` and part-cooling's binding-surface pin
updated). Host: `FinalizationOutputBuilderData.annotations` recorded,
drained onto `finalization_pushes` at resource drop in emission order
(`crates/slicer-wasm-host/src/host.rs`); `apply_finalization_pushes` routes
the variant through `sdk_builder.push_annotation` (`dispatch.rs`); the macro
drain-back replays `sdk_output.annotations()`. Pin:
`finalization_annotation_relay_tdd`. Docs: `docs/03_wit_and_manifest.md`.

**Disclosure — intentional output change on BOTH legs:** external output now
carries the fan commands part-cooling always intended (classic-off
`M106 S255` ×279 + `M107` ×2; t44-job ×275 + ×2). The t44 accepted fresh
baseline sha `1b71f83d…` (captured pre-repair, zero fan commands) is NOT
reproduced by a post-repair external arm (`11e7d38a…`); the frozen-reference
comparison for future takes must use the post-repair sha.

## Gate re-run (from proof, one-shot guards intact)

Provenance proofs: developer snapshot 24/24 `external`; integrated snapshot
24/24 `integrated`; `cargo xtask build-guests --check` exit 0 (37 guests
rebuilt after the WIT bump). Input fingerprints pinned in
`input-fingerprints.json` (model `6a07f34c…`, classic-off config
`e428f149…`, t44 config `2b8314ed…`, binary shas, rustc 1.96.0).

benchy classic supports-off, matched-pair config, 12 threads:

| phase | arm | wall s | cpu s | cpu/wall | bytes | sha256 (first 12) |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| proof | external | 18.273 | 120.125 | 6.574 | 4,332,881 | `e1088b3df78b` |
| proof | integrated | 14.450 | 103.766 | 7.181 | 4,332,881 | `e1088b3df78b` |
| measure ×3 pairs (ABBA) | external | 16.459/18.160/17.116 | 119.3/119.7/120.6 | 6.59–7.25 | 4,332,881 | `e1088b3df78b` |
| measure ×3 pairs (ABBA) | integrated | 15.139/15.319/14.852 | 103.5/102.2/103.1 | 6.67–6.94 | 4,332,881 | `e1088b3df78b` |

`summarize-t47.ps1 -Cells benchy-classic-off`: **GATE: PASS (bounded to
examined cells)** — sha equality, TYPE-count equality, and matching
status/degraded/non-fatal on every row. All 8 measured runs byte-identical.
Completion clean on both legs (`status=ok, degraded=False, non_fatal=0`), zero
unclosed-loop warnings, zero shadow warnings.

Frozen t44-job corroboration (fresh snapshots, both arms):
**byte-identical** `11e7d38a60f5…` external == integrated.

Timing is disclosed for load context only (the gate is an output gate, not a
keep/drop measurement); no wall/CPU claim attached.

## Gates run in-take

`cargo check --workspace --all-targets`; `cargo clippy --workspace
--all-targets -- -D warnings`; `cargo xtask check-literals` (0);
`check-test-quality --report` (8 findings, all pre-existing on the stashed
baseline, none in touched files); `cargo xtask build-guests --check` exit 0;
slicer-wasm-host contract 149/149; slicer-runtime contract 309/309;
slicer-runtime integration `infill_partition` (10/10) +
`infill_postprocess` (14/14) + `gcode_part_cooling` (4/4); slicer-gcode 21 +
5 modules' suites; part-cooling crate suites green; slicer-integrated-modules
full-feature 3/3. The two integration failures on the first full pass were
environmental (documented feature-gate sentinel `perimeter_spatial_capture`;
stale `pnp_cli.exe` binary — green after `cargo build -p pnp-cli`).

## Consequence for the map

1. **The t33 oracle gate is open**: integrated timings may inform module perf
   work on the examined cell, bounded to it, per ticket 33's original terms.
2. Ticket 33's TYPE-delta divergence class is fully repaired (fix 1 + 2), and
   the annotation channel (fix 3) is a NEW correctness repair the gate
   surfaced — external output changes on every job that runs part-cooling
   (both editions gain the fan commands).
3. Fresh-baseline references captured before this take (e.g. t44's
   `1b71f83d…`) are outdated for any stage whose output includes fan
   commands; future takes must re-derive or pin the post-repair sha
   (`e1088b3d…` classic-off / `11e7d38a…` t44-job) or strip fan lines before
   comparing. Ticket 46's drift-boundary question is unaffected (it concerns
   pre-annotation-repair drift, still unattributed).
4. Recommended NEXT: re-run the human's keep/drop review on integrated
   timing; no timing campaign, adoption retry or default switch is authorized
   by this take alone.

## Reproduction

Reduced proof + measure evidence (CSVs + stderr streams, no raw g-code)
tracked in-repo. Rerun commands (one-shot guards require clearing the cell
dirs first):

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t47-native-postprocess-enrichment/run-campaign-t47.ps1 `
  -Cells benchy-classic-off -Runs 3
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t47-native-postprocess-enrichment/summarize-t47.ps1 `
  -Cells benchy-classic-off
```

t44-job corroboration (no script; exact commands in the t33 findings §).