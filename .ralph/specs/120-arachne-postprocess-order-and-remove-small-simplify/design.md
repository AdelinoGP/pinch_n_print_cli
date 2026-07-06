# Design: 120-arachne-postprocess-order-and-remove-small-simplify

## Controlling Code Paths

- Primary code path (N11): `crates/slicer-core/src/arachne/pipeline.rs:360-375` — the post-processing pipeline order (`stitch → simplify → remove_small`) E reorders to canonical (`stitch → remove_small → separate_out_inner_contour → simplify → remove_empty`).
- Primary code path (N12): `crates/slicer-core/src/arachne/remove_small.rs:40-50` — the caller-supplied constant `min_width` E replaces with per-line minimum junction width + layer-type divisor.
- Primary code path (N13): `crates/slicer-core/src/arachne/simplify.rs:43-121` — the iterative multi-pass area-only sweep E replaces with the canonical distance-gated single pass.
- Neighboring tests/fixtures: `arachne_postprocess_order.rs` (NEW — AC-1), `arachne_remove_small_per_line_min_width.rs` (NEW — AC-2), `arachne_simplify_distance_gates.rs` (NEW — AC-3), `crates/slicer-core/tests/fixtures/arachne/stitch_*.json`/`simplify_*.json`/`remove_small_*.json` (re-baseline if they exist).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **E's simplify distance gates may need new config keys** (`meshfix_maximum_resolution`/`_deviation`). The implementer confirms via `docs/15_config_keys_reference.md` whether they are already registered. If not, E adds them to `ArachneParams` + the `arachne-params` WIT record — a WIT record change E must surface in its commit message, not silently absorb. Check `crates/slicer-schema/wit/` for the `arachne-params` record.
- Packet-specific constraint: **E's `separate_out_inner_contour` is a NEW function** (no PNP equivalent); inner-surface bookkeeping for infill boundary. The implementer confirms its exact responsibility via a delegated SUMMARY of `WallToolPaths.cpp:685`'s `separateOutInnerContour`.
- Packet-specific constraint: **E supersedes `D-112-SIMPLIFY-DP`** (113a's DP→VW port) for the simplify layer; the iterative area-only sweep is replaced with the canonical distance-gated single pass. `calculateExtrusionAreaDeviationError` becomes an *extra* guard on the near-colinear fast path only, not the primary gate.
- Packet-specific constraint: **WASM staleness MAY apply** if E adds fields to the `arachne-params` WIT record (which feeds guest WASM). The implementer MUST run `cargo xtask build-guests --check` after any WIT change. If E does NOT touch WIT (distance gates sourced from existing config keys), WASM staleness does not apply. Include the `wasm-staleness` snippet conditionally.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see the project instructions §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

- Selected approach: reorder the post-processing pipeline to canonical, port per-line `min_width` with layer-type divisor, and replace the iterative area-only simplify sweep with the canonical distance-gated single pass. The three changes are bundled because they are all post-processing-pipeline concerns and share the `pipeline.rs`/`remove_small.rs`/`simplify.rs` context.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-core/src/arachne/pipeline.rs:360-375` — reorder the pipeline stages; insert `separate_out_inner_contour` + `remove_empty_toolpaths`.
  - `crates/slicer-core/src/arachne/remove_small.rs:40-50` — per-line `min_width` (minimum junction width over the line) + layer-type divisor (`min_width/2` top/bottom, `min_width * min_length_factor` otherwise); needs `is_initial_layer`.
  - `crates/slicer-core/src/arachne/simplify.rs:43-121` — replace iterative multi-pass area-only sweep with single linear pass gated by `smallest_line_segment_squared` / `allowed_error_distance_squared`; `calculateExtrusionAreaDeviationError` as extra guard on near-colinear fast path only.
  - `crates/slicer-core/src/arachne/separate_inner_contour.rs` (NEW, or inline in `pipeline.rs`) — `separate_out_inner_contour` (inner-surface bookkeeping for infill boundary).
  - `crates/slicer-core/src/arachne/pipeline.rs` `ArachneParams` — possibly new fields for `meshfix_maximum_resolution`/`_deviation` if not already registered (WIT record change — surface, don't silently absorb).
  - `crates/slicer-core/tests/arachne_postprocess_order.rs` (NEW) — AC-1.
  - `crates/slicer-core/tests/arachne_remove_small_per_line_min_width.rs` (NEW) — AC-2.
  - `crates/slicer-core/tests/arachne_simplify_distance_gates.rs` (NEW) — AC-3.
  - `crates/slicer-core/tests/fixtures/arachne/stitch_*.json`/`simplify_*.json`/`remove_small_*.json` (if they exist) — re-baselined via self-capture.
- Rejected alternatives:
  - **Keep the iterative area-only simplify sweep** — rejected (canonical uses distance gates; PNP's sweep consumes long low-curvature arcs canonical would keep).
  - **Make `separate_out_inner_contour` a no-op stub** — rejected (the audit flags its absence; E ports it, even if the inner-surface bookkeeping is initially minimal).
  - **Split N11/N12/N13 into three packets** — rejected (all three are post-processing-pipeline concerns sharing the same context; bundling as one S packet).

## Files in Scope (read + edit)

- `crates/slicer-core/src/arachne/pipeline.rs` — role: N11 order swap + `separate_out_inner_contour`/`remove_empty_toolpaths` insertion + possibly new `ArachneParams` fields for the distance gates; expected change: reorder `:360-375`, insert two new stages, possibly add config fields.
- `crates/slicer-core/src/arachne/remove_small.rs` — role: N12 per-line `min_width`; expected change: rewrite `:40-50` to compute per-line min junction width + layer-type divisor.
- `crates/slicer-core/src/arachne/simplify.rs` — role: N13 distance gates; expected change: replace `:43-121` iterative sweep with distance-gated single pass.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-core/src/arachne/stitch.rs` — read-only; the stitch stage is unchanged (E only reorders it).
- `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — full (202 lines); AC-N1 oracle pattern.
- `docs/15_config_keys_reference.md` — `min_length_factor`, `meshfix_maximum_resolution`/`_deviation` (confirm whether already registered).
- `docs/DEVIATION_LOG.md` `D-112-SIMPLIFY-DP` entry — addendum target.
- `crates/slicer-schema/wit/` — the `arachne-params` WIT record (confirm whether new fields are needed for the distance gates; if yes, E adds them + threads through `slicer-sdk`/`slicer-wasm-host`).

## Out-of-Bounds Files

Files the implementer must NOT load directly. Delegate any fact-checks.

- `OrcaSlicerDocumented/...` — delegate parity checks via the `orca-delegation` contract; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — A1/A2/D's scope.
- `crates/slicer-core/src/skeletal_trapezoidation/*` — A1/B/C/D's scope.
- `crates/slicer-core/src/beading/*` — B's scope.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` — Packet F.

## Expected Sub-Agent Dispatches

List the dispatches the implementer is expected to make.

- "SUMMARY of `WallToolPaths.cpp:679-699` canonical post-process order — ask for the exact stage sequence + the `separateOutInnerContour` responsibility; return ≤ 200 words" — purpose: confirm Step 1's order swap.
- "SUMMARY of `WallToolPaths.cpp:838-856` `removeSmallLines` — ask for the per-line `min_width` computation + the layer-type divisor (`min_width/2` top/bottom, `min_width * min_length_factor` otherwise); return ≤ 200 words" — purpose: confirm Step 2's per-line `min_width`.
- "SUMMARY of `ExtrusionLine.cpp:56-243` `simplifyToolpaths` — ask for the distance-gate thresholds (`smallest_line_segment_squared` / `allowed_error_distance_squared`) + the near-colinear fast-path guard (`calculateExtrusionAreaDeviationError`); return ≤ 200 words" — purpose: confirm Step 3's distance gates.
- "SUMMARY of `WallToolPaths.cpp:868-872` — ask for the `meshfix_maximum_resolution`/`_deviation` sourcing (config keys or hardcoded); return ≤ 200 words" — purpose: confirm the distance-gate config keys.
- "Run `rg -q 'meshfix_maximum_resolution' docs/15_config_keys_reference.md`; return FACT pass/fail" — purpose: confirm whether the distance-gate config keys are already registered.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_postprocess_order --nocapture`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-1.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_remove_small_per_line_min_width --nocapture`; return FACT pass/fail" — purpose: validate AC-2.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_simplify_distance_gates --nocapture`; return FACT pass/fail" — purpose: validate AC-3.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT pass (expected — AC-N1, N1 stays green)" — purpose: gate E didn't regress A1.
- "Run `cargo test -p slicer-core --features host-algos --test stitch --test simplify --test remove_small 2>&1`; return FACT pass/fail (fixtures re-baselined)" — purpose: regression gate.

## Data and Contract Notes

- IR or manifest contracts touched: **possibly** the `arachne-params` WIT record (if `meshfix_maximum_resolution`/`_deviation` are not already registered and E must add them). The implementer confirms via `docs/15_config_keys_reference.md` + `crates/slicer-schema/wit/`. If a WIT record change is needed, E surfaces it in the commit message (not silently absorbed) and threads through `slicer-sdk`/`slicer-wasm-host`.
- WIT boundary considerations: **conditional**. If E adds fields to the `arachne-params` WIT record, the host boundary (`slicer-sdk/src/host.rs`, `slicer-wasm-host/src/host.rs`) must thread them. This is a WIT record change, not a schema change — but it must be surfaced.
- Determinism: E's changes preserve determinism (the canonical single-pass simplify is deterministic; the per-line `min_width` is a deterministic per-line computation).

## Locked Assumptions and Invariants

- E's post-process order is canonical: `stitch → remove_small → separate_out_inner_contour → simplify → remove_empty`.
- E's per-line `min_width` = minimum junction width over the line; divisor `min_width/2` on top/bottom layers, `min_width * min_length_factor` otherwise; needs `is_initial_layer` (already on `ArachneParams`).
- E's simplify is a single linear pass gated by `smallest_line_segment_squared` / `allowed_error_distance_squared`; `calculateExtrusionAreaDeviationError` is an extra guard on the near-colinear fast path only.
- E keeps N1, N2, N3, N4 red tests GREEN (gated).
- E supersedes `D-112-SIMPLIFY-DP` for the simplify layer.
- E's `separate_out_inner_contour` is a NEW function (no PNP equivalent).
- Fixture re-baseline uses the self-capture pattern; never read the JSONs directly.
- If E adds WIT record fields, it surfaces the change (not silently absorbed) + runs `cargo xtask build-guests --check`.

## Risks and Tradeoffs

- **The order swap changes when `remove_small` runs relative to `simplify`.** Canonical runs `remove_small` BEFORE `simplify`; PNP runs it AFTER. This means lines that `simplify` would have shortened below the removal threshold are now removed before `simplify` touches them. The regression suite gates this; the `remove_small`/`simplify` fixtures re-baseline.
- **Per-line `min_width` changes the removal threshold for every line.** Lines with a thin junction (slivers) get a smaller threshold (more likely removed); lines with uniform wide junctions get a larger threshold. The N4 red tests (A2's `is_odd` fix) gate that real walls aren't mis-removed.
- **The simplify distance gates need config keys.** If `meshfix_maximum_resolution`/`_deviation` are not registered, E must add them — a WIT record change. Risk is contained by the `rg` check (dispatch listed).
- **`separate_out_inner_contour`'s exact responsibility is unclear without a delegated SUMMARY.** The audit flags its absence but doesn't detail its bookkeeping. E's implementer confirms via the SUMMARY; if the bookkeeping is non-trivial, E may stub it minimally and flag a follow-up.

## Context Cost Estimate

- Aggregate (sum across all steps): `S`
- Largest single step: `S` (Step 1 — the order swap + `separate_out_inner_contour` + `remove_empty_toolpaths`, the most complex of the three).
- Highest-risk dispatch: the `simplifyToolpaths` SUMMARY (`ExtrusionLine.cpp:56-243`) — its return could blow budget if it returns code instead of prose. Required return format: `SUMMARY ≤ 200 words, no code unless asked`.

## Open Questions

- [FWD] Are `meshfix_maximum_resolution`/`_deviation` already registered config keys, or does E need to add them? The `rg` check (dispatch listed) answers this. If yes, E threads them; if no, E adds them to `ArachneParams` + the `arachne-params` WIT record (surface the WIT change).
- [FWD] Does `separate_out_inner_contour` need to produce an output (e.g., a separate inner-contour `ExtrusionLine` set) or is it in-place bookkeeping on the existing lines? The delegated SUMMARY of `WallToolPaths.cpp:685` clarifies.
- [FWD] Does E's per-line `min_width` need the `is_initial_layer` flag from `ArachneParams`, or is the layer-type determined differently (e.g., a `layer_type` enum)? `ArachneParams::is_initial_layer` exists (per A2's `is_odd` fix); E should use it.

None activation-blocking.