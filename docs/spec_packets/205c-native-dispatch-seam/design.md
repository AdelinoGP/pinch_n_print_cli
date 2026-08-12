# Design: 205c-native-dispatch-seam

## Controlling Code Paths

- Primary code path: `CompiledModuleLive` construction, `LayerStageRunner`/sibling runner native branches, `marshal/native.rs` request/response adapters, `marshal/in_.rs` WASM view construction, and the support-origin chain (WIT `support-output-builder` → SDK `SupportOutputBuilder` → host `HostSupportOutputBuilder` → `collect_support` → `convert_support_output` → `SupportIR`).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs`, `native_dispatch_parity_seam_tdd.rs`, `dispatch_support_output_tdd.rs`, `dispatch_identity_tdd.rs`, `integrated_parity_fuzzy_skin_tdd.rs`, `integrated_parity_seam_placer_tdd.rs`, `parity_invariants_selftest_tdd.rs`, executor support tests, and `crates/slicer-scheduler/tests/integration/integrated_tier_tdd.rs`.
- OrcaSlicer comparison: no direct geometry port; parity uses existing structural invariants.

## Architecture Constraints

- ADR-0021 remains the marshalling-boundary authority; do not create a third translation or move origin reconstruction into module code.
- ADR-0056's one module-loading model and external override rule remain unchanged.
- `NativeStageEntry` remains the SDK seam; the dispatch mode becomes explicit through validated provenance plus entry presence, not a replacement field.
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

- Selected approach: establish one host-side view authority — constructors on the plain SDK view types (`SliceRegionView`/`PerimeterRegionView`, `crates/slicer-sdk/src/views.rs:20,554`) that build the view from IR; make both adapters consume it (the WASM leg's `sliced_region_to_data` calls the constructor then adapts the plain view to its WIT resource type, filling WIT-specific fields from IR; the native leg's `build_native_layer_request` consumes the constructors directly and drops its completeness-mirror table). Centralize held claims on the scheduler-owned `resolve_held_claims` (`crates/slicer-scheduler/src/validation.rs:90`; `slicer-wasm-host` already depends on `slicer-scheduler`). Complete or explicitly reject every supported native response variant. Add the support-origin contract: `set-current-origin` on the WIT `support-output-builder` resource, SDK `SupportOutputBuilder` origin tracking, host `set_current_origin`, per-region `SupportIR`, and origin-preserving conversion. Validate the existing optional `native_entry` invariant at load time.
- Exact functions: `build_native_layer_request` (`marshal/native.rs:112-223`), `sliced_region_to_data` (`marshal/in_.rs:290-425`), `commit_native_prepass_response` (`marshal/native.rs:405-593`), `commit_native_layer_response` (`marshal/native.rs:851-980`), `collect_support` (`marshal/native.rs:827-848`), `collect_infill` (`marshal/native.rs:742-763`), `convert_support_output` (`marshal/out.rs:172-269`), `resolve_layer_held_claims_map` (`dispatch.rs:2490`), `CompiledModuleLive` construction, and integrated live-loader construction.
- Rejected alternative: keep parallel conversions and add more parity tests; the last fixes prove tests detect drift after the seam has already leaked.

## Files in Scope (read + edit)

- `crates/slicer-wasm-host/src/marshal/native.rs` - native request/response adapter, lossless commit ownership, mirror-table removal.
- `crates/slicer-wasm-host/src/marshal/in_.rs` - WASM view construction consumes the shared constructors (plain view → WIT resource adapter).
- `crates/slicer-wasm-host/src/marshal/out.rs` - `convert_support_output` per-region emission.
- `crates/slicer-wasm-host/src/dispatch.rs` - native branch, held-claim authority, shared adapter call sites.
- `crates/slicer-wasm-host/src/binding.rs` and `execution_plan_live.rs` - explicit validated dispatch mode; justified fourth file because the invariant spans construction and storage.
- `crates/slicer-wasm-host/src/host.rs` - `HostSupportOutputBuilder::set_current_origin` (mirrors infill/perimeter impls).
- `crates/slicer-schema/wit/deps/ir-types.wit` - additive `set-current-origin` on `support-output-builder` (mirrors infill:156 / perimeter:165); no version bump.
- `crates/slicer-sdk/src/views.rs` - shared view-construction authority (constructors on the plain view types).
- `crates/slicer-sdk/src/builders.rs` - `SupportOutputBuilder` origin tracking (mirrors `InfillOutputBuilder`).
- `crates/slicer-sdk/src/prepass_types.rs` - additive reason field only if the native commit's candidate type lacks it (`ScoredSeamCandidate.reason` exists at `prepass_types.rs:238`; verify in Step 1).
- `crates/slicer-macros/src/lib.rs` - drain forwards `SupportOutputBuilder::current_origin` to the WIT method.
- `crates/slicer-ir/src/slice_ir.rs` - `SupportIR` per-region shape (mirrors `InfillIR`/`InfillRegion`).
- `crates/slicer-runtime/src/layer_executor.rs` and `visual_debug_render.rs` - `SupportIR` consumers (gcode emission, debug render).
- Focused regression tests under `crates/slicer-runtime/tests/`, `crates/slicer-wasm-host/tests/`, and `crates/slicer-scheduler/tests/integration/`.

## Read-Only Context

- `crates/slicer-scheduler/src/validation.rs` lines 60-105 - canonical holder matching/resolution.
- `crates/slicer-wasm-host/src/host.rs` lines 1280-1300, 1500-1700, 3500-3635, 3867-3923 - per-region claim storage, origin trackers, infill/perimeter `set_current_origin` impls, support builder impl.
- `crates/slicer-sdk/src/native.rs` lines 17-148 - native envelopes and entry families.
- `crates/slicer-sdk/src/prepass_types.rs` lines 225-274 - `SeamReason`, `ScoredSeamCandidate`, `SeamPlanEntry`, `SupportPlanEntry`.
- `docs/adr/0005...` amendment section, `docs/adr/0021...` decision, `docs/adr/0056...` decisions 1/3/4.

## Out-of-Bounds Files

- `modules/core-modules/**` algorithm implementations.
- WIT package/version changes (the additive method is in scope; version bumps are not).
- `dist/editions.toml`, `xtask/**` (except running `cargo xtask build-guests --check`), and CLI flag plumbing.
- `target/`, `Cargo.lock`, generated code, vendored dependencies.

## Expected Sub-Agent Dispatches

- Question: enumerate every constructor and struct literal affected by the dispatch-mode/view shape; scope: `crates/slicer-wasm-host/**`, `crates/slicer-sdk/**`, tests; return: `LOCATIONS`; purpose: blast-radius planning.
- Question: enumerate every `SupportIR` construction site and consumer; scope: `crates/slicer-ir/**`, `crates/slicer-runtime/**`, tests; return: `LOCATIONS`; purpose: per-region shape blast radius (7 construction sites, 7 consumers — no module-crate edits).
- Question: identify supported native commit variants with no WASM-equivalent preservation; scope: `marshal/native.rs`, `host.rs`, `marshal/out.rs`; return: `FACT`; purpose: lossless-commit design.

## Data and Contract Notes

- IR/manifest contracts: region origin and held-claim identity must survive native dispatch exactly as they do through WASM. `SupportIR` gains `regions: Vec<SupportRegion>` with `object_id`/`region_id` (mirroring `InfillIR`/`InfillRegion` at `slice_ir.rs:2149/2133`); the flat `support_paths`/`interface_paths`/`raft_paths`/`ironing_paths` fields move into `SupportRegion`.
- WIT boundary: adapters preserve existing WIT-facing semantics; the `support-output-builder` resource gains one additive method; no package/version change.
- Determinism/scheduler constraints: claim resolution remains per `(layer, object, region)` and dispatch mode does not affect scheduling.

## Locked Assumptions and Invariants

- Integrated modules use native dispatch; external overrides use WASM dispatch.
- A missing integrated native entry is a load-time error.
- Empty perimeter input is a valid no-output postprocess case.
- The WASM leg's support output shape changes with `SupportIR` (both legs emit per-region); parity comparators compare the new shape on both sides.

## Risks and Tradeoffs

- A shared view authority may require changing bindgen resource storage; keep resource accessors as adapters and preserve ownership/lifetime rules.
- The per-region `SupportIR` change ripples into gcode emission, debug render, and test fixtures; the blast radius is bounded (7 construction sites, 7 consumers, no module crates) and lands in one step.
- The WIT/SDK/macros change feeds guest WASM; stale guests must be rebuilt via `cargo xtask build-guests`.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: cross-crate struct-literal inventory, `LOCATIONS`.

## Open Questions

- `[FWD]` Whether the native seam-plan commit reads the candidate reason from the envelope's `ScoredSeamCandidate` (`prepass_types.rs:238`) or needs an additive envelope field; the commit must preserve the reason either way.
- `[FWD]` Whether the canonical per-region held-claim map builder lives in `slicer-scheduler/src/validation.rs` or is composed there from `resolve_held_claims`; the inlined `resolve_layer_held_claims_map` (`dispatch.rs:2490`) is deleted either way.
- `[FWD]` The exact WASM-leg harvest for `PrePass::PaintSegmentation` and `Layer::SlicePostProcess` that the new native commits must mirror.
- `None.` — no activation blockers remain.
