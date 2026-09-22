# Perimeter/dispatch split — measured attribution (2026-09-06)

Follow-up to `tmp/perf-refresh/FINDINGS.md` (user-selected direction:
perimeter and dispatch attribution). All production probe code has been
removed; the tree is back to HEAD. Evidence and harnesses retained here.

## Method

Temporary, opt-in (`PNP_PERF_SPLIT=1`) host-side timing brackets, validated on
the real classic-perimeters WASM artifact
(`integrated_parity_classic_perimeters_native_matches_wasm`) before capture:

- **executor** scope (`execute_single_layer_inner`, `layer_executor.rs`):
  `prepare` → `runner` → `instrumentation` → `apply` → `tail`.
- **runner** scope (`WasmRuntimeDispatcher::run_stage`, `dispatch.rs`):
  `prepare` (claims/config/envelope resolution) → `dispatch` →
  `output_conversion` (`deconstruct_layer_ctx`).
- **dispatch** scope (`dispatch_layer_call`): `pool_wait` →
  `linker_context_config` → `instantiate` → `input_marshalling`
  (`push_slice_regions` + paint/output builders) → `typed_call` →
  `harvest`.
- **service timers** in host.rs (`host.offset_polygons`, `host.clip_polygons`,
  `host.medial_axis`) and marshal sub-intervals in
  `sliced_region_to_data` (`marshal.derive_needs_support`,
  `marshal.overhang_bands`, `marshal.remaining_fields…`, `marshal.view_annotations`).

Reconciliation enforced per call before totals were trusted: child scopes fit
inside parents; the executor runner interval covers the module event. All
captures passed. Captures: 2 × benchy + 2 × base external (runs 1–2), plus one
all-integrated comparison run each. Probe patch saved: `probe-2.patch`.

## Finding 1 — the clustered modules: input marshalling, not guest work

Base run 2, per module (495 calls each), accumulated worker elapsed:

| Bucket | lightning-infill | top-surface-ironing | support-surface-ironing |
| --- | ---: | ---: | ---: |
| dispatch total | 34.90 s | 34.98 s | 34.93 s |
| **input_marshalling** | **33.94 s** | **33.97 s** | **34.00 s** |
| typed_call | 0.46 s | 0.52 s | 0.42 s |
| instantiate | 0.05 s | 0.05 s | 0.05 s |

~97% of each clustered module's cost is **input marshalling on the host**, and
lightning-infill's guest body does no geometry at all (it only replays
precomputed lightning-tree segments) — proof the cost is shared host-side work,
not five coincidentally equal algorithms. The prior "shared fixed per-layer
cost" observation is confirmed and now located.

Inside `sliced_region_to_data` (`crates/slicer-wasm-host/src/marshal/in_.rs`),
per region and per module dispatch:

- `view.derive_needs_support` — **30.38 s** per module on base (the SDK method
  calls `slicer_core::polygon_ops::intersection_ex` against *every* overhang
  footprint of the object, per region, per dispatch).
- remaining region-field conversion (incl. `populate`-style surface work) —
  **30.40 s** per module on base.
- `overhang_bands` clipping — 3.51 s; view construction itself — ~0.01 s.

So the whole cluster ≈ 6 modules × ~34 s ≈ **206 s of accumulated worker
elapsed on base**, and the same `derive_needs_support` + band clipping is
re-executed by **every** module that receives slice regions. `grep` confirms no
production guest module consumes the `needs_support` flag (only a comment in
tree-support notes the render-time gate was *removed*), so this per-dispatch
intersection work is paid for a signal that is unread.

## Finding 2 — classic perimeters: genuinely expensive guest/host geometry

Base classic-perimeters (495 calls): input_marshalling 33.9–70.7 s (same shared
cost as the cluster) but the **typed_call bucket dominates**: 1,669.7 s
(run 2) to 3,229.4 s (run 1) accumulated worker elapsed. Inside the call,
native host services account for a minority: `host.offset_polygons` 80.9 s,
`host.medial_axis` 32.7 s, `host.clip_polygons` 6.5 s (run 2). The rest is
guest-side clipper2 execution (`offset2_ex`, `opening_ex`,
`split_top_surfaces`) plus module logic — consistent with ADR-0055's older
measurement that clipper2 ≈ 39% of the module. The run-1 vs run-2 typed_call
spread (~2×) is a load effect (machine was busy during run 1); neither figure
is a wall-clock claim.

Benchy classic-perimeters typed_call: 152–154 s accumulated over 240 calls —
stable across both probe runs.

Pool wait and instantiation are ruled out everywhere: pool_wait ≈ 0.0003–0.0005 s
total, instantiate ≈ 0.04–0.19 s total per module.

## Finding 3 — all-integrated comparison (parity gap found)

One instrumented run per model with every core module integrated
(`pnp_cli_integrated.exe`, saved here; integrated provenance verified via
`module diagnose --no-default-module-paths`: 23 integrated / 0 external).

Process measurements (single runs, machine had background load — direction
only):

| | external (run 2) | integrated |
| --- | ---: | ---: |
| base wall | 580.9 s | 619.3 s |
| base process CPU | 2958.4 s | **2730.8 s** |
| benchy wall | 39.1 s | 32.4 s |
| benchy process CPU | 178.1 s | **147.5 s** |

Process CPU drops with integrated dispatch (the WIT marshalling and conversion
layers are removed for native calls), but wall did not improve on base under
this load — consistent with per-layer being parallel (CPU-bound savings shrink
wall less) and with `build_native_layer_request` re-doing the same
`derive_needs_support` + surface-classification work natively.

**Parity: the integrated run is NOT output-equivalent.** TYPE section counts
(base, integrated vs two identical external runs):

| TYPE | external (both runs) | integrated |
| --- | ---: | ---: |
| Sparse infill | 683 | **482** |
| Bridge | 30 | **15** |
| Bottom surface | 39 | **25** |
| Top surface | 28 | **26** |

Benchy differs similarly (Sparse infill 166→121, Bridge 124→82, Top surface
103→50, Bottom surface 28→4). External-vs-external section counts are stable
across runs, so this is a real dispatch-parity divergence, not nondeterminism.
Base still reports the same 29,108 non-fatal errors, and Support/Interface
sections match, so the divergence is in infill/ironing/bridge emission —
plausibly the infill-stage glue handing `sparse_fill_holder`/`held_claims` or
surface-classification fields differently on the native leg. **This must be
root-caused before any perf decision uses the integrated path** (ADR-0056's
parity gate would currently fail).

## Ranked proposals (for decision)

1. **Compute per-region host-side data once per layer, not once per module
   dispatch.** `sliced_region_to_data` (and its native twin in
   `build_native_layer_request`) re-derives `needs_support`, overhang bands and
   surface fields for every module. Memoizing this per (layer, region) — or at
   minimum skipping `derive_needs_support` when no consumer reads it — attacks
   ~6×30 s of accumulated base worker elapsed in the cluster alone, benefits
   every edition, and needs no guest rebuilds (pure host change). Largest
   evidence-backed win.
2. **Classic-perimeters guest-side clipper2 work.** The typed_call bucket is
   the single biggest item, but it's already parallel across layers and the
   next steps (route more ops through the batched host services per ADR-0049 /
   the ADR-0055 host-bridge evidence) are a larger, separate workstream.
3. **Do not ship the all-integrated comparison as-is.** Root-cause the
   integrated-vs-external G-code divergence first (candidates: infill-stage
   glue's `held_claims`/`sparse_fill_holder` handling or surface-classification
   field population on the native leg, in `build_native_layer_request` and the
   macro glue in `crates/slicer-macros`).

## Caveats

- All totals except the process table are accumulated worker elapsed (summed
  `Instant` durations across rayon workers), not CPU and not wall.
- Run 1 of the probe captures (external) ran under heavy background load
  (other-busy CPU/wall ≈ 6–7); run 2 was the quieter reference
  (≈ 5.5). The integrated runs sat in between. No speedup percentages are
  claimed.
- Integrated comparison is n=1 per model; treat CPU deltas as directional.
- `needs_support` consumption was checked by grep across
  `modules/core-modules/*/src`; the SDK accessor exists and tests use it, but
  no production guest body calls it today.