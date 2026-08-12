# Design: 205b-native-transport-completion

## Controlling Code Paths

- Primary transport path: `crates/slicer-wasm-host/src/marshal/native.rs`, specifically `commit_native_layer_response` for `Layer::PathOptimization` and `commit_native_postpass_response` for emitted gcode commands.
- Registry path: `crates/slicer-integrated-modules/src/lib.rs` adds the two optional registrations and native entries, following 205a's feature-gated pattern.
- Parity seam: the existing `WasmRuntimeDispatcher`, `CompiledModuleLive`, and stage runners execute native and wasm values against byte-identical inputs.
- Coverage consumer: packet 205's integrated feature coverage gate and `dist/editions.toml`; edition membership is unchanged.

## Architecture Constraints

- Native output must be committed into the existing IR types; do not bypass the stage runner or weaken a parity comparator.
- Parity compares structural invariants and measured coordinate tolerance, never floating-point byte equality.
- The two modules remain behind off-by-default per-module features. `integrated_registrations()` and `native_entries()` must be enabled by the same feature set.
- Preserve external-module precedence: an external module selected by the normal binding plan must not acquire an integrated native entry.
- No hardcoded module count may be introduced. Registry tests derive expected coverage from the registered set and existing contract conventions.
- No geometry call sites, WIT schema, dispatch routing, macro emission, CLI surface, `dist/editions.toml`, `docs/07_implementation_status.md`, or `docs/07` content is edited.
- Native module logic remains single-threaded; neither module may add `rayon` or parallel iterator usage.

## Code Change Surface

- Complete the two existing fatal commit arms in `crates/slicer-wasm-host/src/marshal/native.rs`. Path optimization converts its native output to `LayerStageCommit`; postpass gcode applies the collected commands through the existing accumulator and returns the normal `PostpassOutput`.
- Add optional dependencies, features, registration blocks, and native-entry blocks for `path-optimization-default` and `machine-gcode-emit` in `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}`. Registry tests stay in-file under `#[cfg(test)]`.
- Add `integrated-path-optimization-default` and `integrated-machine-gcode-emit` passthrough features to `crates/pnp-cli/Cargo.toml`.
- Add `integrated_parity_path_optimization` and `integrated_parity_machine_gcode_emit` under `crates/slicer-runtime/tests/contract/`, with `mod` lines in `contract/main.rs`; **amend** the external-override integration test under `tests/integration/` (created and registered by packet 205a) to additionally cover the two new modules — do not create or re-register it.
- Reuse the existing parity invariant helpers. Extend them only if the transport output needs a missing structural assertion, and give every new assertion a negative self-test.
- **User-approved scope expansion:** completing the two transports required expanding scope beyond the original files-in-scope list. The `Layer::PathOptimization` response had no field to carry the module's output, so `crates/slicer-sdk/src/native.rs` gained a `path_optimization` field on `NativeLayerResponse` plus a `NativePathOptimizationOutput` struct; `crates/slicer-macros/src/lib.rs`'s `run_path_optimization` native entry now populates that field (previously it discarded the module's output); the two native postpass callers in `crates/slicer-wasm-host/src/dispatch.rs` now pass the gcode command accumulator to `commit_native_postpass_response`; and `crates/slicer-runtime/Cargo.toml` added `path-optimization-default` and `machine-gcode-emit` as dev-dependencies with their features enabled on the `slicer-integrated-modules` dependency. This expansion was explicitly approved by the user.

## Files in Scope

- `crates/slicer-wasm-host/src/marshal/native.rs` — transport commits.
- `crates/slicer-integrated-modules/Cargo.toml` and `src/lib.rs` — two features, registrations, native entries, and in-file tests.
- `crates/slicer-runtime/tests/common/parity_invariants.rs` — only required comparator additions.
- `crates/slicer-runtime/tests/contract/integrated_parity_path_optimization_tdd.rs`.
- `crates/slicer-runtime/tests/contract/integrated_parity_machine_gcode_emit_tdd.rs`.
- `crates/slicer-runtime/tests/contract/main.rs`.
- `crates/slicer-runtime/tests/integration/full_coverage_external_override_tdd.rs` — **amended** (created and registered by packet 205a); 205b extends it to the two new modules without touching `tests/integration/main.rs`.
- `crates/pnp-cli/Cargo.toml` — passthrough features.
- `crates/slicer-sdk/src/native.rs` — `path_optimization` field on `NativeLayerResponse` + `NativePathOptimizationOutput` (user-approved scope expansion).
- `crates/slicer-macros/src/lib.rs` — `run_path_optimization` native entry populates the new field (user-approved scope expansion).
- `crates/slicer-wasm-host/src/dispatch.rs` — the two native postpass callers pass the gcode command accumulator (user-approved scope expansion).
- `crates/slicer-runtime/Cargo.toml` — dev-deps + features for the two modules (user-approved scope expansion).
- `docs/01_system_architecture.md`, `docs/specs/multi-edition-distribution-plan.md` — the doc edits in `packet.spec.md` §Doc Impact Statement.

## Read-Only Context

- `docs/spec_packets/205a-integrated-edition-coverage/packet.spec.md` and `design.md` — whole files; the pattern this packet completes.
- `crates/slicer-wasm-host/src/marshal/native.rs` — 869 lines; read only the two fatal arms and their nearest committed neighbors: the layer-output commit dispatch (~lines 800-869, incl. the line-862 fatal) and the postpass commit (~lines 560-680, incl. the line-655 gcode-command error).
- `crates/slicer-integrated-modules/src/lib.rs` — whole file (under 300 lines); 205a's feature-gated registration/native-entry pattern.
- `crates/slicer-sdk/src/native.rs` — lines 129-138 only; the four `NativeStageEntry` variants.
- `modules/core-modules/path-optimization-default/src/lib.rs` and `modules/core-modules/machine-gcode-emit/src/lib.rs` — the `#[slicer_module]` type name, SDK trait, and native output shape, by `rg` only (never edited).
- `modules/core-modules/path-optimization-default/path-optimization-default.toml` and `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — the `[module] id` and `[stage] id`.
- `crates/slicer-runtime/tests/common/parity_invariants.rs` — whole file (under 300 lines); the comparator helpers to reuse.
- `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — the dual-dispatch construction to replicate.
- `dist/editions.toml`, `xtask/src/dist.rs`, `xtask/src/build_guests.rs` (`discover_guests`) — read-only; the coverage gate and its registry source.

## Out-of-Bounds Files

- `modules/core-modules/path-optimization-default/**` and `modules/core-modules/machine-gcode-emit/**` — inspect symbols only; do not edit.
- dispatch routing, scheduler, `xtask/**`, `dist/editions.toml`, and generated guests.
- `docs/adr/**`, `docs/07_implementation_status.md`, and `CONTEXT.md` — never modified by this packet.
- `target/`, `Cargo.lock`, vendored dependencies, and `OrcaSlicerDocumented/`.

## Expected Sub-Agent Dispatches

- Question: what are the annotated type names, SDK traits, manifest stage ids, and native output shapes for the two modules? scope: the two module `src/lib.rs` and `.toml` files; return `LOCATIONS`/`SNIPPETS` only.
- Question: what existing converter and accumulator apply path should each transport call? scope: bounded `native.rs` and stage IR definitions; return `SNIPPETS` only.
- Question: how are the dual-dispatch fixtures and aggregators mounted? scope: existing contract and integration tests; return `SNIPPETS` only.
- Question: does `cargo xtask build-guests --check` report clean? scope: repo root; return `FACT` clean or `STALE:` list.

## Locked Assumptions and Invariants

- `path-optimization-default` declares `Layer::PathOptimization`; its native commit must return `Ok(Some(..))` or `Ok(None)` consistently with the wasm path.
- `machine-gcode-emit` declares `PostPass::GCodePostProcess`; every supported collected command must be applied in order and unsupported commands must fail explicitly.
- Integrated feature names equal module directory names, and passthrough feature bodies delegate to the matching registry feature.
- Existing external override behavior and edition membership remain unchanged.

## Risks and Tradeoffs

- Path optimization may expose output fields not represented by the existing layer converter. Resolve by reusing the closest committed layer representation and add a structural parity assertion; do not silently drop paths.
- Gcode command variants may not all have accumulator equivalents. Fail with the command kind rather than treating emitted commands as success.
- Guest artifacts can become stale after manifest or feature changes; run the freshness gate before parity tests.

## Context Cost Estimate

- Aggregate: `M`; no step is `L`.

## Open Questions

- `[FWD]` Which existing path-optimization converter best matches the module's output? Resolve by dispatch against the IR and native converter symbols.
- `[FWD]` Which gcode command variants does the module emit on the contract fixture? Resolve from the module manifest and fixture, then cover each emitted kind structurally.
