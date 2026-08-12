# Design: 205c-native-dispatch-seam

## Controlling Code Paths

- Primary code path: `CompiledModuleLive` construction, `LayerStageRunner`/sibling runner native branches, and `marshal/native.rs` request/response adapters.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs`, current fuzzy-skin and seam-placer native regression tests, and integrated-tier override tests.
- OrcaSlicer comparison: no direct geometry port; parity uses existing structural invariants.

## Architecture Constraints

- ADR-0021 remains the marshalling-boundary authority; do not create a third translation or move origin reconstruction into module code.
- ADR-0056's one module-loading model and external override rule remain unchanged.
- `NativeStageEntry` remains the SDK seam unless a concrete load-time validation requires an additive type change.

## Code Change Surface

- Selected approach: establish one host-side view authority, make both adapters consume it, centralize held claims, complete or explicitly reject every supported native response variant, and validate the existing optional `native_entry` invariant at load time. ADR-0005's field shape remains unchanged; the mode becomes explicit through validated provenance plus entry presence, not a replacement field.
- Exact functions: `build_native_layer_request`, `build_native_prepass_request`, native commit functions, `resolve_layer_held_claims_map`, `CompiledModuleLive`, and integrated live-loader construction.
- Rejected alternative: keep parallel conversions and add more parity tests; the last fixes prove tests detect drift after the seam has already leaked.

## Files in Scope (read + edit)

- `crates/slicer-wasm-host/src/marshal/native.rs` - native request/response adapter and lossless commit ownership.
- `crates/slicer-wasm-host/src/dispatch.rs` - native branch, held-claim authority, and shared adapter call sites.
- `crates/slicer-wasm-host/src/binding.rs` and `execution_plan_live.rs` - explicit validated dispatch mode; justified fourth file because the invariant spans construction and storage.
- `crates/slicer-sdk/src/native.rs` and `views.rs` - shared view/envelope shape if required.
- Focused regression tests under `crates/slicer-runtime/tests/` and `crates/slicer-wasm-host/tests/`.

## Read-Only Context

- `crates/slicer-scheduler/src/validation.rs` lines 60-105 - canonical holder matching/resolution.
- `crates/slicer-wasm-host/src/host.rs` lines 1280-1300, 1650-1675 - per-region claim storage.
- `crates/slicer-sdk/src/native.rs` lines 17-148 - native envelopes and entry families.
- `docs/adr/0005...` amendment section, `docs/adr/0021...` decision, `docs/adr/0056...` decisions 1/3/4.

## Out-of-Bounds Files

- `modules/core-modules/**` algorithm implementations.
- `crates/slicer-schema/wit/**` and WIT package versions.
- `dist/editions.toml`, `xtask/**`, and CLI flag plumbing.
- `target/`, `Cargo.lock`, generated code, vendored dependencies.

## Expected Sub-Agent Dispatches

- Question: enumerate every constructor and struct literal affected by the dispatch-mode/view shape; scope: `crates/slicer-wasm-host/**`, `crates/slicer-sdk/**`, tests; return: `LOCATIONS`; purpose: blast-radius planning.
- Question: identify supported native commit variants with no WASM-equivalent preservation; scope: `marshal/native.rs`, `host.rs`, `marshal/out.rs`; return: `FACT`; purpose: lossless-commit design.

## Data and Contract Notes

- IR/manifest contracts: region origin and held-claim identity must survive native dispatch exactly as they do through WASM.
- WIT boundary: adapters preserve existing WIT-facing semantics; no package/version change.
- Determinism/scheduler constraints: claim resolution remains per `(layer, object, region)` and dispatch mode does not affect scheduling.

## Locked Assumptions and Invariants

- Integrated modules use native dispatch; external overrides use WASM dispatch.
- A missing integrated native entry is a load-time error.
- Empty perimeter input is a valid no-output postprocess case.

## Risks and Tradeoffs

- A shared view authority may require changing bindgen resource storage; keep resource accessors as adapters and preserve ownership/lifetime rules.
- Completing deferred support origins may require a separate packet if current IR lacks the source identity.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: cross-crate struct-literal inventory, `LOCATIONS`.

## Open Questions

- `[FWD]` Which existing plain SDK view type is the canonical storage type after inspecting all WIT resource constructors?
- `[BLOCK]` If support origins cannot be preserved without an IR/WIT contract change, split that field into a successor packet before activation.
