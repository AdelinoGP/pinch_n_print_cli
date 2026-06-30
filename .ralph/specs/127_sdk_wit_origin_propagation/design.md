# Design: 127_sdk_wit_origin_propagation

## Controlling Code Paths

- Primary code path: WIT `perimeter-output-builder.set-current-origin` → `HostExecutionContext.explicit_perimeter_origin` → `effective_perimeter_origin()` (highest-precedence fallback) → `HostPerimeterOutputBuilder::set_infill_areas` / `push_wall_loop` / `push_seam_candidate` / `push_reordered_wall_loop` (all read `effective_perimeter_origin()`) → `PerimeterOutputCollected.infill_areas_origins` / `wall_loop_origins` / etc. → `convert_perimeter_output` drains through `OriginBucket` → `PerimeterIR.regions` (one `PerimeterRegion` per distinct origin).
- SDK side: `PerimeterOutputBuilder.begin_region(object_id, region_id)` sets `self.current_origin` → each push method appends `self.current_origin.clone()` to its parallel `*_origins` Vec → macro `__slicer_drain_perimeter` reads per-item origins and calls `wit.set_current_origin(...)` before each WIT push.
- Neighboring tests or fixtures: `crates/slicer-wasm-host/tests/contract/effective_perimeter_origin_integration_tdd.rs` (fallback path — must still pass), `crates/slicer-wasm-host/tests/contract/perimeter_infill_per_origin_route_tdd.rs` (marshal contract — must still pass), `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` (wall colour — must still pass), `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` (gcode structural — must still pass).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load). Do not restate the delegation rules here.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see the project instructions §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **Builder backing structs are stateless tags.** `PerimeterOutputBuilderData`, `InfillOutputBuilderData`, `SupportOutputBuilderData` are empty structs (host.rs:195-238). Every push ignores the resource handle and writes to one per-stage collector on `HostExecutionContext` keyed by origin tag. This packet adds a new field (`explicit_perimeter_origin`) to `HostExecutionContext`, not to the backing struct — the single-builder WIT contract is preserved. A `list<builder>` approach (Shape 1, rejected) would have forced these structs to become stateful and the collector to become a `Vec` — a marshal/dispatch architecture change this packet avoids.
- **Origin machinery is shared by perimeter, infill, and support.** `effective_perimeter_origin()` and `OriginBucket` are used by `HostPerimeterOutputBuilder` (6 sites), `HostInfillOutputBuilder` (3 sites via `current_slice_region.clone()`), and `HostSupportOutputBuilder` (3 sites via `current_slice_region.clone()`). This packet's additive `.or_else()` in `effective_perimeter_origin` affects only the perimeter path (infill/support read `current_slice_region` directly, not via `effective_perimeter_origin`). A perimeter-only migration leaves the origin machinery standing for infill/support — no parallel mechanisms.
- **Support IR is flat.** `SupportIR` has no per-region identity; support prints as T0 (layer_executor.rs:816-839). Per-region builders buy support nothing until its IR gains tool semantics (a schema change, not a builder change). This packet does not touch support.

## Code Change Surface

- **Selected approach:** Shape 2 + Sub-shape 2A — single builder + new `set-current-origin` WIT method + `begin_region` SDK context method. Additive origin chain (explicit origin takes precedence, `touch_*` fallback stays as defence-in-depth).
- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  - `crates/slicer-schema/wit/deps/ir-types.wit:87-93` — add `set-current-origin` method to `resource perimeter-output-builder`.
  - `crates/slicer-sdk/src/builders.rs:117-292` — add `current_origin: Option<OriginId>` field + `begin_region` method; each push method appends `self.current_origin.clone()` to its origins Vec.
  - `crates/slicer-wasm-host/src/host.rs:641-646` (field decl), `:811-812` (builder init), `:937-941` (`effective_perimeter_origin`), `:2341-2420` (`HostPerimeterOutputBuilder` impl) — add `explicit_perimeter_origin` field, implement `set_current_origin` WIT method, prepend `.or_else()` to `effective_perimeter_origin`.
  - `crates/slicer-macros/src/lib.rs:2384-2428` (`__slicer_drain_perimeter`) — call `wit.set_current_origin(...)` before each WIT push, forwarding SDK item origins.
  - `modules/core-modules/classic-perimeters/src/lib.rs:193` — add `begin_region` call.
  - `modules/core-modules/arachne-perimeters/src/lib.rs:199` — add `begin_region` call.
  - `modules/core-modules/seam-placer/src/lib.rs:219` — add `begin_region` call.
  - `modules/core-modules/fuzzy-skin/src/lib.rs:80` — add `begin_region` call.
  - `crates/slicer-wasm-host/tests/contract/set_current_origin_routes_to_correct_bucket_tdd.rs` — NEW test.
  - `crates/slicer-wasm-host/tests/contract/main.rs` — register new test module.
  - `crates/slicer-runtime/tests/executor/cube_4color_sparse_infill_per_painted_region_tdd.rs` — NEW test.
  - `crates/slicer-runtime/tests/executor/main.rs` — register new test module.
- **Rejected alternatives that were considered and why they were not chosen:**
  - **Shape 1 (`list<perimeter-output-builder>`):** rejected because builder backing structs are stateless tags (host.rs:193-238); pushes write to one per-stage collector on `HostExecutionContext`. Shape 1 forces builders to become stateful (carry region identity), the collector to become a `Vec`, and commit to go per-builder — a marshal/dispatch architecture change. The "kills `OriginBucket` complexity" pro was false for perimeter-only: `OriginBucket`/`OriginId`/`effective_perimeter_origin` are shared by infill and support, so a perimeter-only Shape 1 leaves all of it standing, creating two parallel output mechanisms.
  - **Sub-shape 2B (explicit origin parameter on every push method):** rejected because it requires editing every push call site (classic-perimeters has ~7 push sites, seam-placer ~3, fuzzy-skin ~1, arachne ~N) vs. one `begin_region` call per loop. Sub-shape 2A's "forget to call `begin_region`" risk is the same class as 2B's "pass wrong origin argument" risk, but 2A has fewer call sites to get wrong.
  - **Option A (forward-through SDK, from the prior spec):** rejected because it does not fix the bug. `effective_perimeter_origin()` reads `current_slice_region` from the host, which is set only by `touch_slice_region()` (WIT accessor calls). The guest's `run_perimeters` iterates SDK `SliceRegionView`s (plain data, no host callback), so `current_slice_region` is stale. Forwarding at SDK push time still captures the stale origin.
  - **Option C (bindgen `self`):** rejected because it breaks every existing WASM guest at once, and even with `self` the drain still has no per-item origin info.

## Files in Scope (read + edit)

- `crates/slicer-schema/wit/deps/ir-types.wit` — role: WIT contract source; expected change: add one method to `perimeter-output-builder` resource (line 87-93).
- `crates/slicer-sdk/src/builders.rs` — role: SDK `PerimeterOutputBuilder` struct; expected change: add `current_origin` field + `begin_region` method + per-push origin append (lines 117-292).
- `crates/slicer-wasm-host/src/host.rs` — role: host WIT impl + origin chain; expected change: add `explicit_perimeter_origin` field + `set_current_origin` impl + `.or_else()` in `effective_perimeter_origin` (lines 641-646, 811-812, 937-941, 2341-2420).

Additional files (migration + tests, mechanical):
- `crates/slicer-macros/src/lib.rs` (lines 2384-2428 only) — drain forwarding.
- 4 guest module `src/lib.rs` files — one `begin_region` call each.
- 2 new test files + 2 `main.rs` registrations.
- `docs/07_implementation_status.md`, `CONTEXT.md`, `docs/adr/0022-*.md` — doc impact.

## Read-Only Context

Files the implementer is allowed to read but not edit. Include line-range hints whenever the file is > 300 lines.

- `crates/slicer-wasm-host/src/host.rs` — read lines `641-646, 811-812, 901-941, 2035-2052, 2207-2223, 2341-2420` only — purpose: understand the origin chain, `touch_*` mechanism, and `HostPerimeterOutputBuilder` impl. Never load the full ~3700-line file.
- `crates/slicer-macros/src/lib.rs` — read lines `1726-1765, 2097-2193, 2384-2428` only — purpose: understand the macro arms that create the SDK builder + call the drain, and the adaptation functions that trigger `touch_*`. Never load the full ~2726-line file.
- `crates/slicer-runtime/src/layer_executor.rs` — read lines `626-879, 1012-1037, 1062-1135` only — purpose: understand the spatial fallback (defence-in-depth, unchanged) and `backfill_resolved_seam` (out of scope, must not break). Never load the full ~1509-line file.
- `crates/slicer-wasm-host/src/marshal/out.rs` — read lines `277-460` only — purpose: confirm `convert_perimeter_output` and `OriginBucket` are unchanged by this packet.
- `crates/slicer-wasm-host/src/marshal/accumulators.rs` — read lines `42-78` only — purpose: confirm `PerimeterOutputCollected` field shape (the `*_origins` Vecs the SDK appends to).
- `crates/slicer-runtime/src/region_partition.rs` — read lines `64-160` only — purpose: understand `sync_perimeter_infill_areas_into_slice` (the downstream consumer that benefits from the fix; unchanged).
- `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` — read directly (single ADR, < 200 lines) — purpose: the `OriginBucket` all-or-none attribution rule this packet preserves.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate parity checks; never load.
- `target/`, `Cargo.lock`, generated bindgen code — never load.
- `crates/slicer-wasm-host/src/dispatch.rs` — NOT edited by this packet (the dispatch path is unchanged — single builder, single handle; `set_current_origin` is called from the drain, not dispatch). Delegate any dispatch fact-check.
- `crates/slicer-runtime/src/run.rs` — NOT edited (the `run_slice` API is unchanged). Delegate any run-path fact-check.
- `crates/slicer-wasm-host/src/marshal/origin.rs` — NOT edited (`OriginBucket` is unchanged). Read only to understand the bucketing rule.

## Expected Sub-Agent Dispatches

- "Run `cargo check --workspace --all-targets 2>&1 | tail -3`; return FACT pass/fail" — purpose: validate WIT change compiles after Step 2.
- "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3`; return FACT pass/fail" — purpose: acceptance gate (AC-6).
- "Run `cargo test -p slicer-wasm-host --test contract -- set_current_origin_routes_to_correct_bucket 2>&1 | tail -3`; return FACT pass/fail; on failure SNIPPETS with test name + assertion + ≤ 20 lines" — purpose: validate AC-4.
- "Run `cargo test -p slicer-wasm-host --test contract -- layer_perimeters_origin_falls_back_to_slice_region_through_host_trait 2>&1 | tail -3`; return FACT pass/fail" — purpose: validate AC-5 (fallback preserved).
- "Run `cargo test -p slicer-runtime --test executor -- cube_4color 2>&1 | tail -3`; return FACT pass/fail; on failure SNIPPETS with test name + assertion" — purpose: validate AC-2, AC-3.
- "Run `cargo xtask build-guests --check 2>&1; echo EXIT=$?`; return FACT: EXIT=0 or STALE: lines" — purpose: guest freshness gate before running tests.
- "Find all callers of `effective_perimeter_origin` in `crates/slicer-wasm-host/src/host.rs`; return LOCATIONS (file:line, ≤ 20 entries)" — purpose: confirm the additive `.or_else()` does not miss any origin-reading site.

## Data and Contract Notes

- **IR contracts touched:** none. `PerimeterIR`, `PerimeterRegion`, `PerimeterOutputCollected` are unchanged. The `*_origins` Vecs already exist (from the marshal precondition).
- **WIT boundary considerations:** one new method on `perimeter-output-builder` resource. This regenerates every guest's bindgen output — `cargo xtask build-guests` is mandatory. The method is `set-current-origin: func(object-id: string, region-id: string) -> result<_, string>;` — takes string-typed identity (matching the existing `region-key` pattern in the WIT), returns `result` for consistency with other builder methods.
- **SDK contract:** `begin_region(&mut self, object_id: &str, region_id: u64)` — takes `&str` + `u64` (matching `ObjectId = String` + `RegionId = u64` in slicer-ir). Sets `self.current_origin = Some(OriginId { object_id: object_id.to_string(), region_id })`. Does NOT return `Result` — it's a pure setter with no capacity check.
- **Determinism or scheduler constraints:** none. The origin is set synchronously in the guest's loop; the drain is synchronous after the guest returns. No reordering.

## Locked Assumptions and Invariants

- **Invariant: additive origin chain.** `effective_perimeter_origin()` must remain `explicit_perimeter_origin.or(current_perimeter_region).or(current_slice_region)`. The `touch_*` fallback must not be removed — it's defence-in-depth for guests that forget `begin_region` and it's the only origin source for infill/support.
- **Invariant: marshal unchanged.** `convert_perimeter_output` and `OriginBucket` must not be modified by this packet. The origins are just correct now; the bucketing logic is the same.
- **Invariant: `begin_region` is convention-based, not structural.** A guest that forgets `begin_region` falls through to the stale `touch_*` fallback (no hard error). The new host test (AC-4) pins the explicit path; the gcode test (AC-1/AC-3) pins the end-to-end behaviour. The fallback test (AC-5) pins the defence-in-depth path.
- **Invariant: single builder per dispatch.** The WIT `run-perimeters` and `run-wall-postprocess` signatures are unchanged. One `perimeter-output-builder` resource per dispatch call.
- **Invariant: `resolved_seam` drain gap stays.** The macro drain does NOT call `wit.push_resolved_seam(...)`. `backfill_resolved_seam` in `layer_executor.rs:1020-1037` fills from `SeamPlanIR`. This packet does not fix the drain gap. Seam-placer's `set_resolved_seam` calls continue to have no effect on the output IR via the drain path.

## Risks and Tradeoffs

- **WIT change regenerates every guest's bindgen.** `cargo xtask build-guests` mandatory. Stale guests surface as test failures that look unrelated to the edit (typed instantiation mismatches). The `--check` gate must pass before attributing any test failure to the packet's changes.
- **`set-current-origin` is convention-based.** A future guest that forgets `begin_region` gets the stale fallback (same bug as today for that guest). No hard error. This is the trade-off of Shape 2 over Shape 1 (which makes mis-assignment structurally impossible). Accepted because Shape 1's cost (stateful builders + Vec collector + per-builder commit) is disproportionate for a perimeter-only fix.
- **`PerimetersPostProcess` fix changes wall tool attribution for seam-placer/fuzzy-skin output.** Today the spatial fallback in `layer_executor` recovers wall tools (walls sit on their region's perimeter). Post-fix, the origin tag is correct from the source, so the fallback is redundant but not removed. No behaviour change expected for walls (fallback already worked for them). The gcode test (AC-1/AC-3) covers this.
- **The uncommitted marshal precondition (11 files) must land with this packet.** It's the foundation: per-call `infill_areas` accumulation + `OriginBucket` per-origin drain. Without it, the explicit origins have nothing to bucket into. Folded into Step 1.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 5: guest rebuild + gcode feedback loop — the `cargo xtask build-guests` dispatch + the slice run + the awk metric is the heaviest single step, but all dispatchable as FACT returns).
- Highest-risk dispatch: the `cargo test -p slicer-runtime --test executor -- cube_4color` dispatch — if it fails, the sub-agent must return SNIPPETS with the failing test name + assertion + ≤ 20 lines of relevant code, not the full ~920-line test file. The implementer must specify this return format explicitly in the dispatch.

## Open Questions

None. All design questions were settled during the grilling session:
- Shape 2 (single builder + `set-current-origin`) over Shape 1 (list of builders) — builder backing structs are stateless tags; Shape 1 is a marshal/dispatch architecture change disproportionate for perimeter-only.
- Sub-shape 2A (`begin_region` context method) over 2B (explicit origin parameter on every push) — fewer call sites, same risk class.
- `PerimetersPostProcess` included (same WIT resource, same mechanism, same LIFO bug).
- `resolved_seam` deferred (separate drain-gap bug, masked by `backfill_resolved_seam`).
- Additive origin chain (Option i) over replacement (Option ii) — defence-in-depth, zero marshal change, backwards-compatible.
- Infill stage deferred (separate WIT resource, separate modules, separate packet).
- Support stage out of scope (flat IR, no per-region tool semantics).