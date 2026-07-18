# Requirements: 127_sdk_wit_origin_propagation

## Packet Metadata

- Grouped task IDs:
  - `TASK-252` (inferred — no existing `docs/07` entry names this bug; the prior packet's `TASK-251` was an unrelated support-modules task)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Slicing `resources/cube_4color.3mf` produces gcode that is mostly correct but missing infill across internal painted regions. The per-tool `;TYPE:Sparse infill` segment count shows T1 = 30 (just unretract priming moves, no actual extrusion) where OrcaSlicer's golden has T1 = 1243, and T3 = 2425 (absorbing T1/T2 interior infill) where OrcaSlicer has T3 = 992.

**Root cause:** The SDK `PerimeterOutputBuilder` buffers every `set_infill_areas` call. The macro-generated `__slicer_drain_perimeter` forwards the buffered calls to the WIT builder **after** the guest's `run_perimeters` returns. The WIT-level `set_infill_areas` captures `effective_perimeter_origin()` at the moment of the drain call — by which point the host's `current_slice_region` is the **LAST** `(object_id, region_id)` that any WIT `SliceRegionView` accessor touched during `__slicer_adapt_slice_regions`. The guest's `run_perimeters` iterates **SDK** `SliceRegionView`s (plain-data structs with no host callback), so `current_slice_region` is never re-touched during the guest's loop. Every per-region `set_infill_areas` call collapses to one bucket — only the last painted region's `infill_areas` survives, and `sync_perimeter_infill_areas_into_slice` populates `sparse_infill_area` for exactly one region.

The same LIFO-touch bug affects `Layer::PerimetersPostProcess` (seam-placer, fuzzy-skin): after `__slicer_adapt_perimeter_regions`, `current_perimeter_region` is the last region, and all `push_reordered_wall_loop` / `push_wall_loop` calls are tagged with that origin.

**Why this is a coherent slice:** the fix is one WIT method + one SDK method + one `begin_region` call per guest loop + one `.or_else()` line in `effective_perimeter_origin`. The marshal is unchanged. The four guest modules share the same `for region in regions` loop shape. The infill stage has the same bug but is a separate fix surface (different WIT resource, different SDK builder, different modules) — deferring it keeps this packet's blast radius to 4 crates + 4 modules.

This packet supersedes the prior `127_sdk_wit_origin_propagation/packet.spec.md` (authored during the diagnose session), which recommended Option A (forward-through SDK). The grilling session proved Option A does not fix the bug: forwarding at SDK push time still captures the stale `current_slice_region` because the SDK `SliceRegionView` has no host callback to re-touch it. The prior spec's three options (A/B/C) all share this flaw. This packet replaces them with Shape 2 (single builder + explicit `set-current-origin` WIT method + `begin_region` SDK method), which captures origin at SDK push time from the guest's loop context, not from the host's stale touch state.

## In Scope

- Add `set-current-origin: func(object-id: string, region-id: string) -> result<_, string>;` to the WIT `perimeter-output-builder` resource in `crates/slicer-schema/wit/deps/ir-types.wit`.
- Add `explicit_perimeter_origin: Option<OriginId>` field to `HostExecutionContext` in `crates/slicer-wasm-host/src/host.rs`; implement the `set_current_origin` WIT host trait method.
- Modify `effective_perimeter_origin()` in `host.rs` to prepend `self.explicit_perimeter_origin.clone()` as the highest-precedence fallback (additive — existing `current_perimeter_region` / `current_slice_region` chain stays).
- Add `current_origin: Option<OriginId>` field + `begin_region(&mut self, object_id: &str, region_id: u64)` method to the SDK `PerimeterOutputBuilder` in `crates/slicer-sdk/src/builders.rs`. Each push method (`push_wall_loop`, `set_infill_areas`, `push_seam_candidate`, `push_reordered_wall_loop`) appends `self.current_origin.clone()` to its existing parallel `*_origins` Vec.
- Modify the macro's `__slicer_drain_perimeter` in `crates/slicer-macros/src/lib.rs` to call `wit.set_current_origin(...)` before each WIT push, forwarding the SDK item's origin.
- Add `output.begin_region(region.object_id(), *region.region_id());` at the top of the `for region in regions` loop in 4 guest modules: `classic-perimeters`, `arachne-perimeters`, `seam-placer`, `fuzzy-skin`.
- Add new host-level contract test `set_current_origin_routes_to_correct_bucket` in `crates/slicer-wasm-host/tests/contract/`.
- Add new gcode-level parity test `cube_4color_sparse_infill_per_painted_region` in `crates/slicer-runtime/tests/executor/`.
- Fold in the uncommitted marshal precondition (11 files from the diagnose session) as Step 1.
- Update `docs/07_implementation_status.md` with TASK-252, `CONTEXT.md` with the "Per-region output origin" term, and create `docs/adr/0022-explicit-per-region-origin-for-perimeter-output-builders.md`.

## Out of Scope

- The infill stage (`Layer::Infill`) — `HostInfillOutputBuilder` has the same LIFO-touch bug via `current_slice_region.clone()`, but it's a separate WIT resource (`infill-output-builder`), separate SDK builder, and separate modules (rectilinear-infill, gyroid-infill, lightning-infill). Deferred to a sequel packet.
- The support stage (`Layer::Support`) — `SupportIR` is flat (no per-region identity; support prints as T0). Per-region builders buy support nothing until its IR gains tool semantics (a schema change, not a builder change).
- The `resolved_seam` drain gap — the macro's `__slicer_drain_perimeter` never calls `wit.push_resolved_seam(...)`, so seam-placer's `set_resolved_seam` calls have no effect on the output IR. This is masked by `backfill_resolved_seam` in `layer_executor.rs:1020-1037`, which fills `resolved_seam` from `SeamPlanIR`. Fixing the drain gap is a separate bug class (missing drain call, not origin-tagging) and deferred.
- Removing the spatial fallback in `layer_executor.rs:626-879` — kept as defence-in-depth. The explicit origin makes it redundant for walls and infill paths, but it stays.
- Removing the `touch_slice_region` / `touch_perimeter_region` mechanism — stays for infill/support and as the defence-in-depth fallback for any guest that forgets `begin_region`.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT `perimeter-output-builder` resource definition, host-boundary enforcement. **Delegate a SUMMARY** of the `perimeter-output-builder` section (doc is large; only the resource-method table and origin-tagging paragraph needed).
- `docs/02_ir_schemas.md` — `PerimeterIR` / `PerimeterRegion` struct definitions (§IR 2). **Delegate a SUMMARY** of the `PerimeterRegion` field list.
- `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` — `OriginBucket` design ADR. **Read directly** (single ADR file, likely < 200 lines) for the all-or-none origin-attribution rule this packet preserves.
- `CONTEXT.md` §"Marshalling boundary" — glossary term. **Read directly** (small glossary file, ~150 lines).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1501-1506,1644` — per-region `set_infill_areas` call site; OrcaSlicer uses a per-output-builder-instance vector (one `SurfaceCollection*` per `LayerRegion`), not a last-write-wins accumulator.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp:1541-1892` — `prepare_infill` where the host-side partition hook splits infill_areas by region priority.
- `OrcaSlicerDocumented/src/libslic3r/Layer.cpp:296-332` — multi-region path where a temporary `SurfaceCollection` is split back to individual regions via `intersection_ex` against each region's slice geometry.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-6` from `packet.spec.md`. AC-1 is the user-visible gcode parity metric (T1 >= 1000, T3 <= 1500). AC-2 is the no-regression gate on wall colour. AC-3 is the new gcode-level parity test. AC-4 is the new host-level origin-routing test. AC-5 is the fallback-preservation test. AC-6 is the clippy gate.
- Negative cases: `AC-N1` — anonymous-mode fallback preserved when no origin is set at all.
- Cross-packet impact: unblocks a future infill-stage origin-propagation packet (the `begin_region` SDK pattern is reusable). Does not affect support stage (flat IR). Does not fix the `resolved_seam` drain gap (deferred).

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the 2–3 gate commands; this section is the authoritative list with delegation hints.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets 2>&1 \| tail -3` | Type-check all targets (WIT change regenerates bindgen; must compile) | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -3` | No warnings (acceptance gate) | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test contract 2>&1 \| tail -3` | Host-level contract tests including AC-4, AC-5, AC-N1 | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-wasm-host --test contract -- set_current_origin_routes_to_correct_bucket 2>&1 \| tail -3` | AC-4: explicit origin routes to correct bucket | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test contract -- layer_perimeters_origin_falls_back_to_slice_region_through_host_trait 2>&1 \| tail -3` | AC-5: fallback path preserved | FACT pass/fail |
| `cargo test -p slicer-wasm-host --test contract -- effective_perimeter_origin_is_none_when_neither_set 2>&1 \| tail -3` | AC-N1: anonymous mode preserved | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- cube_4color 2>&1 \| tail -3` | Executor tests including AC-2, AC-3 | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test executor -- cube_4color_first_layer_perimeter_colour_matches_bottom_face 2>&1 \| tail -3` | AC-2: no wall-colour regression | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- cube_4color_sparse_infill_per_painted_region 2>&1 \| tail -3` | AC-3: gcode-level parity (T1 >= 1000, T3 <= 1500, all 4 tools in Sparse infill) | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract 2>&1 \| tail -3` | Runtime contract tests (marshal precondition) | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration 2>&1 \| tail -3` | Runtime integration tests (gap_fill_emission shape) | FACT pass/fail |
| `cargo xtask build-guests --check 2>&1; echo EXIT=$?` | Guest freshness — WIT change invalidates every guest's bindgen | FACT: "EXIT=0" (clean) or "STALE:" (rebuild needed) |
| `./target/release/pnp_cli slice --model resources/cube_4color.3mf --no-default-module-paths --module-dir tmp/repro/modules-no-arachne --output tmp/repro/pnp_out.gcode 2>tmp/repro/run.log && awk '...' tmp/repro/pnp_out.gcode \| sort` | AC-1: per-tool sparse-infill metric (T1 >= 1000, T3 <= 1500) | FACT: T1/T3 counts |

## Step Completion Expectations

- **Cross-step invariant:** no step may regress `cube_4color_first_layer_perimeter_colour_matches_bottom_face` (AC-2), even if the test file is not edited by that step. The wall spatial fallback must continue to recover wall tools.
- **Cross-step invariant:** no step may regress the existing `effective_perimeter_origin_integration_tdd.rs` tests (AC-5, AC-N1), even after the `effective_perimeter_origin` signature changes — the additive `.or_else()` must preserve the fallback chain.
- **Step ordering rationale:** Step 1 (fold marshal precondition) must land before Step 2 (WIT change) because the WIT change regenerates bindgen and recompiles everything — the precondition's per-call accumulation must already be in place so the new origins have parallel `*_origins` Vecs to append to.
- **Guest rebuild sequencing:** Step 5 (guest rebuild via `cargo xtask build-guests`) must follow Step 4 (module `begin_region` calls) and must precede Step 6 (tests that slice the cube). Stale guests surface as test failures unrelated to the edit.

## Context Discipline Notes

- `crates/slicer-wasm-host/src/host.rs` is ~3700 lines. **Never load in full.** The implementer reads only: lines 641-646 (field declarations), 811-812 (builder init), 901-941 (origin accessors + `effective_perimeter_origin`), 2035-2052 (`touch_slice_region`), 2207-2223 (`touch_perimeter_region`), 2341-2420 (`HostPerimeterOutputBuilder` impl). Range-read these; delegate any other host.rs fact-check.
- `crates/slicer-macros/src/lib.rs` is ~2726 lines. **Never load in full.** The implementer reads only: lines 1726-1745 (`perimeters_arm`), 1747-1765 (`wall_postprocess_arm`), 2384-2428 (`__slicer_drain_perimeter`), 2097-2154 (`__slicer_adapt_slice_regions`), 2165-2193 (`__slicer_adapt_perimeter_regions`). Range-read these; delegate macro-expansion fact-checks.
- `crates/slicer-runtime/src/layer_executor.rs` is ~1509 lines. **Never load in full.** The implementer reads only: lines 626-879 (`assemble_ordered_entities` spatial fallback), 1012-1037 (`backfill_resolved_seam`), 1062-1135 (`apply` commit arms). Delegate any other layer_executor fact-check.
- **Likely temptation read:** `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — delegate only; never load. The OrcaSlicer parity surface is confirmed in `requirements.md` §OrcaSlicer Reference Obligations.
- **Sub-agent return-format hints:** for `cargo test` dispatches, require "FACT pass/fail; on failure, SNIPPETS with test name + assertion + ≤ 20 lines of relevant code." For `cargo xtask build-guests --check`, require "FACT: EXIT=0 or STALE: lines."