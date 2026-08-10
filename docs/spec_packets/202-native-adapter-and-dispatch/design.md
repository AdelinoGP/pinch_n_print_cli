# Design: 202-native-adapter-and-dispatch

## Controlling Code Paths

- Primary code path: `#[slicer_module]` expansion (`slicer_module` → `generate_slicer_module_impl` and the per-world `wit_bindgen::generate!` glue emitters in `crates/slicer-macros/src/lib.rs`) on the guest side; `WasmRuntimeDispatcher`'s four `impl *StageRunner` blocks (`crates/slicer-wasm-host/src/dispatch.rs`) with inputs from `crates/slicer-wasm-host/src/binding.rs` (`CompiledModuleLive`, `LayerStageInput` and siblings) on the host side; binding assembly in `load_live_modules_for_plan_with_integrated` (`crates/slicer-wasm-host/src/execution_plan_live.rs`, created by packet 201).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/contract/macro_all_worlds_roundtrip_tdd.rs` (drives `sdk-layer-infill-guest.component.wasm` through real dispatch — the working end-to-end driver the parity test reuses; aggregator `crates/slicer-runtime/tests/contract/main.rs`); `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`; test doubles for the runner traits exist only under tests (`NoopLayerRunner` in `crates/pnp-cli/tests/e2e_integration_tdd.rs`, `ScriptedRunner`/`SeedingRunner` in `crates/slicer-runtime/tests/`).
- OrcaSlicer comparison: none — no OrcaSlicer behavior in this packet; §OrcaSlicer Reference Obligations deliberately absent from `packet.spec.md`/`requirements.md`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- **Marshalling boundary — ONE answer.** CONTEXT.md §Marshalling boundary states the one-answer rule but does *not* name a directory; the path pin below is this packet's, read off the tree: the boundary stays `crates/slicer-wasm-host/src/marshal/`. The native path is a second *transport* through the same boundary, not a second boundary: response commit re-enters at the `*OutputCollected` accumulator layer (`marshal/accumulators.rs`) so the `out.rs` converters (`convert_infill_output`, `convert_perimeter_output`, …) and `origin.rs` `OriginBucket` re-attribution run **unchanged and shared** between transports. The only dual leg is view construction (wasm: resource methods + guest glue; native: `marshal/native.rs` builds `slicer_sdk::views::*` directly from IR) — that leg is exactly what packet 204's parity gate audits. Parity implication stated explicitly: divergence between paths can originate only in (a) the input-view leg field mapping and (b) module-body libm/codegen ULP drift; origin re-attribution and output conversion cannot diverge because they are the same code.
- ADR-0005 seam invariants hold: the native branch lives *inside* the runner impls; no `HostExecutionContext` (never constructed on the native path), no wasm-host-internal type crosses the trait boundary; outputs remain IR-typed (`LayerStageCommit`, `PrepassStageOutput`, `PostpassOutput`, `FinalizationOutput`). Note the asymmetry, verified at authoring: `PrepassStageOutput` is defined in **`crates/slicer-core/src/stage_io.rs`**, not `slicer-ir` like its three peers — import it from the right crate in Step 4.
- ADR-0056 Decision item 5: single-threaded module logic on both paths — the native adapter and `marshal/native.rs` must not introduce rayon or thread spawns; host-side layer fan-out and batched host services stay the only parallelism.
- ADR-0033 layer 3 already gives SDK host-service wrappers native arms (`crates/slicer-sdk/src/host.rs` cfg-split; `slicer-sdk/Cargo.toml` enables `host-algos` only under `cfg(not(target_arch = "wasm32"))` — verified at authoring). Module code calling `slicer_sdk::host::*` therefore works natively today; this packet adds no wrapper changes (wasm32 bridge arms are DEV-094 / packet 200).
- Config crosses the native seam as the literal shared struct: `CompiledModuleLive.config_view: Arc<ConfigView>` is handed to `from_config` directly (`ConfigView` is the same `slicer_ir` type on both sides). Config keys stay snake_case.

## Code Change Surface

- Selected approach — **macro-emitted SDK-typed adapter + data-driven branch inside `WasmRuntimeDispatcher`**:
  1. `crates/slicer-sdk/src/native.rs` (new; `#[cfg(not(target_arch = "wasm32"))]`; `pub mod native` in `lib.rs`): per-family request/response envelopes mirroring the runner-trait families — `NativeLayerRequest { layer_index, regions: Vec<views::SliceRegionView>, paint/prior-stage fields as Options mirroring LayerStageInput's optionality, stage_export: &'static str }`, `NativeLayerResponse` carrying the drained SDK-builder state (paths + origins, wall loops + origins, seam data — the accessors already exposed by `crates/slicer-sdk/src/builders.rs`: `sparse_paths()`, `sparse_path_origins()`, `wall_loops()`, `wall_loop_origins()`, …), plus prepass/postpass/finalization counterparts built on `prepass_builders.rs`/`postpass_builders.rs` state; `pub enum NativeStageEntry { Layer(fn(&NativeLayerRequest) -> Result<NativeLayerResponse, ModuleError>), Prepass(..), Postpass(..), Finalization(..) }`. Envelope fields are enumerated per family by the implementer from the wasm32 glue emitters (dispatched SNIPPETS), not invented.
  2. `crates/slicer-macros/src/lib.rs`: `generate_slicer_module_impl` additionally emits, per detected `StageSpec`, a `#[cfg(not(target_arch = "wasm32"))]` inherent `pub fn __slicer_native_entry() -> ::slicer_sdk::native::NativeStageEntry` — body: match its own stage; `from_config(&req.config)`; call the trait stage method with SDK views from the request and a locally constructed SDK builder; drain the builder into the response. This mirrors, at SDK level, exactly what the wasm32 glue does after WIT→SDK adaptation (e.g. `__slicer_adapt_slice_regions`), so no WIT types appear anywhere in the native path.
  3. `crates/slicer-wasm-host/src/marshal/native.rs` (new) + `marshal/mod.rs` registration: `build_native_layer_request(stage_export, layer, &LayerStageInput, module claims/config context from CompiledModuleLive) -> NativeLayerRequest` — module context is required, not optional: e.g. `SliceRegionView`'s held claims derive from `CompiledModuleLive.claims`, not from `LayerStageInput`. Verified at authoring: `held_claims` is a **private** field of `slicer_sdk::views::SliceRegionView` with `held_claims()` / `set_held_claims()` accessors, so `marshal/native.rs` must go through the setter — it cannot construct the view by struct literal (constructs `slicer_sdk::views::SliceRegionView` etc. from the same IR sources `in_.rs` uses to back the WIT resources — SDK views hold `slicer_ir` types directly, so this leg needs no unit or type conversion) — and `commit_native_layer_response(resp, ..) -> Option<LayerStageCommit>` (fills `InfillOutputCollected`/`PerimeterOutputCollected`/… then calls the existing `out.rs` converters + `OriginBucket`); prepass/postpass/finalization counterparts.
  4. `crates/slicer-wasm-host/src/binding.rs`: `CompiledModuleLive` gains `pub native_entry: Option<NativeStageEntry>`; `CompiledModuleLive::new` keeps its 5-argument signature and sets `native_entry: None`; new `pub fn with_native_entry(..)`. Measured at authoring: all constructions go through `::new` (6 production call sites — `layer_executor.rs` ×2, `postpass.rs` ×2, `prepass.rs` ×1, `layer_finalization.rs` ×1 in `crates/slicer-runtime/src/` — plus test sites in 13 files); zero struct-literal constructions outside `binding.rs`, so adding the field churns no caller.
  5. `crates/slicer-wasm-host/src/dispatch.rs`: each of the four `impl *StageRunner for WasmRuntimeDispatcher` blocks starts with the native branch (route on `module.native_entry` before export resolution / pool lease / `HostExecutionContext` construction). Path selection is therefore data-driven per call — which IS the parity seam: one dispatcher, two `CompiledModuleLive` values.
  6. `crates/slicer-wasm-host/src/execution_plan_live.rs`: `load_live_modules_for_plan_with_integrated` gains `native_entries: &[(ModuleId, NativeStageEntry)]`; attachment rule per AC-4/AC-N1 (Integrated provenance AND id in table → `Some`, else `None`); orchestrator projection sites (`crates/slicer-runtime/src/layer_executor.rs` and siblings) thread `binding.native_entry` into `CompiledModuleLive` via `with_native_entry`.
  7. `crates/slicer-integrated-modules/src/lib.rs`: `pub fn native_entries() -> Vec<(ModuleId, NativeStageEntry)>` — feature arms empty in this packet (204 populates); `crates/slicer-runtime/src/run.rs` passes it.
  8. Parity/contract tests: `crates/slicer-runtime/tests/contract/native_adapter_tdd.rs` (AC-1) and `native_dispatch_parity_seam_tdd.rs` (AC-2, AC-3), registered in `tests/contract/main.rs`; `sdk-layer-infill-guest` added as a `slicer-runtime` dev-dependency by path (its `[workspace]` sentinel keeps it a non-member; a path dep is legal and compiles it natively); AC-4/N1/N2 tests in `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`.
- Rejected alternatives:
  - *Native path converses in host bindgen WIT types (full reuse of `in_.rs`/`out.rs` WIT records)*: host-side views/builders are wasmtime **resources** tied to a Store; without an instance there are no resource handles, so "reusing" them means faking a store — more machinery than the SDK-level seam and still a distinct leg. Rejected.
  - *Native path converses in raw host IR with no marshal involvement*: skips `origin.rs` re-attribution — precisely the duplication CONTEXT.md's one-answer rule forbids; parity gate would have to audit identity reconstruction too. Rejected.
  - *A separate `NativeDispatcher`/router type implementing the four traits*: forwarding all `last_*`/`take_*` capture accessors route-dependently adds bookkeeping for zero seam benefit; the in-impl branch keeps capture channels in one struct. Rejected.
  - *Trait objects (`Box<dyn LayerModule>`) instead of fn-pointer entries*: SDK traits take `Self: Sized` constructors (`from_config`), so object safety fails; fn pointers over envelopes sidestep it. Rejected.

## Files in Scope (read + edit)

More than 3 primary files because the slice spans macro, SDK, and host layers; every implementation step stays within the ≤3-edit cap.

- `crates/slicer-sdk/src/native.rs` (new) + `crates/slicer-sdk/src/lib.rs` — role: native seam types.
- `crates/slicer-macros/src/lib.rs` — role: adapter emission; expected change: one new emitter fn + wiring in `generate_slicer_module_impl`.
- `crates/slicer-wasm-host/src/marshal/native.rs` (new) + `marshal/mod.rs` — role: native transport through the marshalling boundary.
- `crates/slicer-wasm-host/src/dispatch.rs`, `binding.rs`, `execution_plan_live.rs` — role: routing + plumbing.
- `crates/slicer-runtime/src/run.rs`, `layer_executor.rs`/`prepass.rs`/`postpass.rs`/`layer_finalization.rs` (only the `CompiledModuleLive` projection lines) — role: thread `native_entry`.
- `crates/slicer-integrated-modules/src/lib.rs` — role: native-entry table.
- Tests: `crates/slicer-runtime/tests/contract/{native_adapter_tdd.rs,native_dispatch_parity_seam_tdd.rs,main.rs}`, `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`, `crates/slicer-runtime/Cargo.toml` (dev-dep).
- Docs: `docs/04_host_scheduler.md` §Phase 4, `docs/05_module_sdk.md` `#[slicer_module]` section.

## Read-Only Context

- `crates/slicer-macros/src/lib.rs` — locate by symbol, then ±80-line windows: `generate_slicer_module_impl`, the glue-world chooser, one light-family glue body (`layer_light_helpers`) — purpose: mirror the post-adaptation call shape; long file, never load whole.
- `crates/slicer-wasm-host/src/dispatch.rs` — the four `impl *StageRunner` entry regions only (locate via `impl PrepassStageRunner for WasmRuntimeDispatcher` and siblings); long file, never load whole.
- `crates/slicer-wasm-host/src/marshal/mod.rs` (short; may be read whole) and pub-item skeletons of `accumulators.rs`/`out.rs`/`origin.rs` via grep — purpose: accumulator re-entry points.
- `crates/slicer-sdk/src/builders.rs` — accessor regions only — purpose: drain surface.
- `crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/src/lib.rs` (short, whole) — purpose: the AC-1/AC-2 witness module.
- `crates/slicer-runtime/tests/contract/macro_all_worlds_roundtrip_tdd.rs` — setup region only — purpose: reuse dispatch driver.

## Out-of-Bounds Files

- `docs/spec_packets/194-*`, `195-*`, `196-*`, `docs/07_implementation_status.md`, `CONTEXT.md`, `docs/specs/multi-edition-distribution-plan.md` — never edit. `docs/adr/*` — never edit **except** the ADR-0005 amendment carved out below.
  - **Carve-out — ADR-0005 requires an amendment (this is the one permitted `docs/adr/` edit).** `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` pins `CompiledModuleLive`'s shape **inside its `## Decision` section** (third bullet: "Module access: `&CompiledModuleLive<'a>` defined in `slicer-wasm-host` with 5 fields (`module_id: &'a ModuleId`, `instance_pool: Arc<WasmInstancePool>`, `wasm_component: Option<Arc<WasmComponent>>`, `claims: &'a [String]`, `config_view: Arc<ConfigView>`). No back-edge dep on `slicer-runtime`."). That is **normative**, not descriptive — verified at authoring. Step 4 adds a 6th field (`native_entry`), so the packet contradicts an ADR Decision clause and **must not do so silently**. Obligations, both in Step 4:
    1. Append an `## Amendment — <date> (packet 202)` section to ADR-0005 quoting the contested bullet verbatim and recording the 6th field. Precedent: `docs/adr/0051-gcode-marker-contract-ownership.md`'s amendment section (packet 187).
    2. While the file is open, correct two pre-existing stale counts in it (**not** caused by this packet, but verified wrong at authoring): its §Decision says "The four `bindgen!` invocations remain co-located in `slicer-wasm-host`", and its §Verification says `grep -cE 'wasmtime::component::bindgen!' crates/slicer-wasm-host/src/host.rs` "returns 4 (one per world)". The live count is **15** — the "one per world" parenthetical is right, there are simply 15 worlds (8 layer, 4 prepass, 2 postpass, 1 finalization), not 4 stage families. Fixing a documented verification command that no longer passes is in scope for an amendment pass; note it in the amendment section.
    3. Add a `docs/DEVIATION_LOG.md` row `D-202-ADR-0005-AMENDED` — the `D-<pkt>-<SLUG>` packet-prefixed convention (precedent: `D-285-ADR-0051-AMENDED`), where `202` is this packet's number, not the `D-` counter. The AC-N3 and Doc-Impact greps match on `ADR-0005-AMENDED` only, so the number is grep-robust.
    The rest of ADR-0005 is unaffected and conformed to: the seam invariants (no `HostExecutionContext` across the trait boundary, IR-typed outputs, no wasm-host-internal type in the signature) all hold, and `NativeStageEntry` lives in `slicer-sdk`, so the "no back-edge dep on `slicer-runtime`" clause is untouched. Only the field count and list change.
- `crates/slicer-schema/wit/**` — WIT is untouched by design; any perceived need for a WIT change falsifies the approach — stop and re-derive.
- `modules/core-modules/*/src/**` module bodies and manifests — single-source means zero edits there.
- `crates/slicer-sdk/src/host.rs` wasm32 arms — packet 200 / DEV-094 territory.
- `OrcaSlicerDocumented/` — not consulted; `target/`, `Cargo.lock`, generated code — never load.

## Expected Sub-Agent Dispatches

- Question: for each glue family (layer light, layer heavy, prepass, postpass, finalization), what SDK-typed values does the wasm32 glue hand the trait method and drain from the builder afterward; scope: `crates/slicer-macros/src/lib.rs`; return: SNIPPETS ≤30 lines per family; purpose: envelope field enumeration (Steps 1–3).
- Question: exact construction sites where the four executors project `CompiledModuleLive` (the 6 `::new` calls in `crates/slicer-runtime/src/`); scope: `crates/slicer-runtime/src/`; return: LOCATIONS ≤10; purpose: Step 7 threading.
- Question: how `macro_all_worlds_roundtrip_tdd.rs` builds engine/pool/dispatcher/`LayerStageInput` for `sdk-layer-infill-guest`; scope: that file; return: SNIPPETS ≤30 lines; purpose: Step 6 test authoring.
- Question: run `cargo xtask build-guests --check` (and rebuild if stale), then each verification command; scope: repo root; return: FACT pass/fail; purpose: step exits.

## Data and Contract Notes

- IR/manifest contracts: none change. The native response's origin pairs `(object_id, region_id)` feed the same `OriginBucket` drain the WIT path uses — origin-based identity reconstruction has one owner.
- WIT boundary: untouched (no new WIT funcs, no bindgen changes; the `bindgen!` worlds in `crates/slicer-wasm-host/src/host.rs` are not edited — there are 15 of them, one per stage world, not one per stage family).
- Determinism/scheduler constraints: the native branch must preserve per-call ordering semantics — one module invocation per dispatch, same commit shape; instance-pool bookkeeping is skipped (a native call has no slot), which must not affect layer fan-out (pool concurrency limits WASM instances, not native calls; ADR-0056 Decision item 5 keeps module logic single-threaded so a concurrent native fan-out executes the same pure logic — flag any discovered shared-state hazard as a blocker rather than adding a lock silently).
- FORWARD-DEP contract with 201 (both drafts authored together): `ModuleProvenance` (`slicer-scheduler`), `IntegratedModuleRegistration`, `load_live_modules_for_plan_with_integrated(search_roots, host_parallelism, config_source, profile, integrated)` — this packet appends the `native_entries` parameter; 201's design.md §Risks records that extension.

## Locked Assumptions and Invariants

- SDK builders (`crates/slicer-sdk/src/builders.rs` and prepass/postpass counterparts) are target-independent accumulators with public read accessors (verified at authoring) — the native adapter's drain relies on this; builders must stay cfg-free.
- `NativeStageEntry` values are `'static` (fn pointers + static strs), letting `CompiledModuleLive` carry them by value without lifetime plumbing.
- An integrated module with no native entry stays a loud `MissingComponent` failure (AC-N2) — never a silent skip; this preserves 201's seam semantics for misconfigured builds.
- Byte-equality across paths is explicitly NOT asserted anywhere in this packet (ADR-0056 Decision item 4, DEV-093); AC-2's structural assertions are the ceiling until 204's tolerance gate.

## Risks and Tradeoffs

- Envelope drift risk: `marshal/native.rs` view construction vs `in_.rs` resource backing is the audited dual leg; a missed field surfaces as a 204 parity failure, not a compile error. Mitigation: `adapt_slice_regions_completeness_tdd.rs` (existing) pins the wasm leg's field completeness; Step 4 adds the mirrored field checklist against the same list.
- Macro emission compiles natively in all 21 workspace module crates at once — a signature mistake breaks `cargo check --workspace --all-targets` broadly; Step 2 gates on exactly that command before proceeding.
- Instrumentation asymmetry: native calls return empty `last_*` captures this packet; consumers (`ModuleAccessAudit`, profiling) see zeros for native dispatches — acceptable and documented, revisit in 204 if the audit needs native reads.
- Heavy layer stages (`perimeters_postprocess`, `infill_postprocess`, `path_optimization`) have the richest envelopes; they are in scope but sized as their own step to keep failure blast radius contained.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 5 dispatch branches; Step 2 macro emission)
- Highest-risk dispatch and required return format: per-family glue-shape survey of `slicer-macros` — SNIPPETS ≤30 lines per family; reject anything larger and redispatch per family.

## Open Questions

- [FWD] Native log capture: `slicer_sdk::host::log*` native arms currently bypass the dispatcher's capture channel; whether to route native module logs into `last_log_messages` in this packet or defer to 204 — implementer may defer with a code comment, since no AC depends on it.
- [FWD] Whether `NativeLayerRequest` should carry `Arc` clones vs owned `Vec<SliceRegionView>` — owned is assumed (matches the guest glue's eager deep copy and keeps parity of data lifecycle); implementer may switch to borrows if the copy shows up in profiling, without AC changes.
- [FWD] Envelope coverage for the postpass text variant (`run_text_postprocess`) — the `Postpass` entry must serve both gcode and text methods; implementer chooses one fn pointer with an input enum or two pointers, provided AC-N2's loud-failure semantics hold for stages the entry does not serve.
