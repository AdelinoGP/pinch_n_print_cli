# Design: 03_wit-canonical-source-and-validation

## Controlling Code Paths

- **Primary code path — Macro WIT glue generation**: `crates/slicer-macros/src/lib.rs` — `WIT_WORLD_MAP` (lines 96-97) and `build_*_world_glue` functions (one per world). These functions produce the inline WIT string passed to `wit_bindgen::generate!`. All four worlds are covered: layer, prepass, postpass, finalization.
- **Primary code path — Host WIT bindings**: `crates/slicer-host/src/wit_host.rs` — inline `wasmtime::component::bindgen!({ inline: r#"..."# })` blocks in `pub mod layer` (line 176+), `pub mod prepass` (line 376+), and analogous blocks for postpass and finalization. These are the host's WIT type bindings used at runtime.
- **Primary code path — `wit_world` validation**: `crates/slicer-host/src/manifest.rs` — `Manifest::validate` or a new `validate_wit_world` function called during module load in `dag.rs` or a new `module_load.rs`. Startup path: `main.rs` → module load → manifest parse → `validate_wit_world`.
- **Drift detection test**: New file `crates/slicer-host/tests/wit_drift_detection_tdd.rs` — reads on-disk WIT files and compares them against embedded strings extracted from macro lib.rs and host wit_host.rs using `include_str!` macro invocations.

## Neighboring Tests or Fixtures

- `crates/slicer-host/tests/manifest_ingestion_tdd.rs` — existing manifest parsing tests; add `wit_world` mismatch cases here
- `crates/slicer-host/tests/live_module_loading_tdd.rs` — existing live loading tests
- `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` — existing macro WIT roundtrip tests; may need updating after consolidation
- `crates/slicer-host/src/dag.rs:158` and `crates/slicer-host/src/execution_plan.rs:858` — hardcoded `wit_world: "slicer:world-layer@1.0.0"` in test helpers; these must be updated to use the canonical identifier from the now-single source

## Architecture Constraints

- `slicer-macros` is a proc-macro crate — it compiles in its own compilation context before the main crate. `include_str!` is resolved at compile time in that context. The relative path from `crates/slicer-macros/src/lib.rs` to `wit/` must be validated at the start of the packet.
- `wit_bindgen::generate!` accepts inline WIT as a `&str`. The `include_str!` result (`&'static str`) satisfies this.
- The four canonical world identifiers are defined by the on-disk `wit/` files. The allowlist in the host must be derived from those same files (hardcoded constants matching the canonical names, updated together with any WIT changes).
- Version (`@1.0.0` vs `@1.1.0`) is part of the identifier for allowlist purposes — `slicer:ir-types@1.1.0` ≠ `slicer:ir-types@1.0.0`.
- The drift detection test must not require runtime I/O that could make it non-deterministic. It should compare `include_str!` results at compile/test time.

## Code Change Surface

### Selected approach

Consolidate by replacing inline WIT string literals with `include_str!` references to the canonical on-disk files. This is the minimal-change approach that preserves existing code structure while eliminating the drift root cause.

### Exact functions, traits, manifests, tests, or fixtures expected to change:

1. **`crates/slicer-macros/src/lib.rs`**: Replace hardcoded WIT string literals in `build_layer_world_glue` (near line 537+), `build_prepass_world_glue`, `build_postpass_world_glue`, and `build_finalization_world_glue` with `include_str!("../../wit/deps/types.wit")`, etc. The `WIT_WORLD_MAP` entries (`"slicer:world-layer@1.0.0"`, `"slicer:world-prepass@1.0.0"`) stay as-is — those are correct canonical identifiers.

2. **`crates/slicer-host/src/wit_host.rs`**: Replace inline WIT strings in all four `wasmtime::component::bindgen!` blocks with `include_str!` references (`../../wit/deps/types.wit`, etc.). Fix package name `slicer:layer-world@1.0.0` → `slicer:world-layer@1.0.0` (line 179) and `slicer:prepass-world@1.0.0` → `slicer:world-prepass@1.0.0` (line 379).

3. **`wit/deps/ir-types.wit`**: Add `needs-support` interface (matching what macro and host have inline). Check `deps/ir-types.wit` against the DEV-014 note: "needs-support is missing from inline WIT".

4. **`wit/world-postpass.wit`**: Add `push-z-hop` to `gcode-output-builder` (matching what layer world has). DEV-014 note: "postpass inline gcode-output-builder omits push-z-hop".

5. **`crates/slicer-host/src/manifest.rs`**: Add `validate_wit_world(manifest: &Manifest) -> Result<(), ManifestError>` function that checks `manifest.wit_world` against a hardcoded `&'static [&'static str]` allowlist: `["slicer:world-layer@1.0.0", "slicer:world-prepass@1.0.0", "slicer:world-postpass@1.0.0", "slicer:world-finalization@1.0.0"]`. Call this from the module-load path (likely in `dag.rs` or a new `module_load.rs`).

6. **`crates/slicer-host/src/dag.rs:158`**: Update hardcoded `"slicer:world-layer@1.0.0"` if needed (should already be correct but verify).

7. **`crates/slicer-host/src/execution_plan.rs:858`**: Same — verify hardcoded value is canonical.

8. **`crates/slicer-host/tests/wit_drift_detection_tdd.rs`** (new file): Test that compares disk WIT files against embedded strings. The test should extract the `include_str!` paths from macro `lib.rs` and host `wit_host.rs` and verify they match disk. Approach: read `include_str!` paths by searching for the pattern in source, then compare file contents.

9. **`crates/slicer-host/tests/manifest_ingestion_tdd.rs`**: Add test cases for `wit_world` allowlist rejection:
   - `wit_world_mismatch_rejects_invalid_package_name`
   - `wit_world_major_version_mismatch_rejects_future_major`

### Rejected alternatives that were considered and why they were not chosen:

- **Generate Rust constants from WIT**: A build script would generate a `wit_str.rs` module from the disk files. This is more robust but adds a build-step dependency. `include_str!` is simpler and sufficient for this consolidation phase.
- **Derive allowlist from disk at runtime**: Scanning `wit/` at runtime to build the allowlist dynamically would be fragile and slow. Hardcoded allowlist matching canonical identifiers is explicit and testable.
- **Move WIT files into `slicer-macros` crate**: Would require `include_str!` from within the crate, but then host would need a different path. Keeping `wit/` as workspace-root canonical is correct.

## Data and Contract Notes

- WIT boundary considerations: Consolidation does NOT change WIT types, only their source. The `wit_bindgen!` output types remain identical.
- Package name normalization: `slicer:layer-world@1.0.0` (host inline, wrong) → `slicer:world-layer@1.0.0` (canonical, correct). The macro already uses the canonical name. This fix aligns the host with the macro and disk.
- ir-types version: On-disk canonical is `@1.1.0`. If any inline copy uses `@1.0.0`, it must be updated. Check macro `lib.rs` for the `ir-types` version reference.
- Determinism: The `include_str!` macro produces identical `&str` content across builds. No runtime I/O is introduced in hot paths.
- Scheduler constraints: The allowlist check is at module-load time (startup), not per-invocation. No per-layer overhead.

## Locked Assumptions and Invariants

- The four canonical world identifiers (`slicer:world-layer@1.0.0`, `slicer:world-prepass@1.0.0`, `slicer:world-postpass@1.0.0`, `slicer:world-finalization@1.0.0`) are stable for the lifetime of this consolidation packet. They will not change.
- The `wit/` directory is the single source of truth after this packet. All inline copies are eliminated or made to reference it.
- The allowlist contains exactly four entries — one per WIT world. No wildcard or regex matching.
- `needs-support` and `push-z-hop` are additive missing members — adding them to the disk canonical does not break any existing bindings because they are optional interfaces.

## Risks and Tradeoffs

- **Proc-macro `include_str!` path resolution**: The relative path `../../wit/` from `crates/slicer-macros/src/lib.rs` must be verified to work at macro compile time. If it does not, an alternative is to place a copy of the key WIT files in the `slicer-macros` crate directory.
- **Package name changes in host**: Changing `slicer:layer-world@1.0.0` → `slicer:world-layer@1.0.0` in `wit_host.rs` requires the host's `bindgen!` output types to be regenerated. This is a breaking change for any code that imports from the old package name — but since this is host-only code that doesn't expose these types publicly, it should be a contained refactor.
- **ir-types version bump**: If the canonical version is `@1.1.0`, any manifest that declares `min-ir-schema = "1.1.0"` must now pass. Existing manifests that only declare `1.0.0` range may need updating (but the IR schema version is independent of the WIT version per docs/03 architecture rules).
- **Drift detection test fragility**: Extracting `include_str!` paths by string search is fragile. A more robust approach is to use a const that holds the string directly and compare that. Consider defining a const in each crate: `const CANONICAL_WIT_TYPES: &str = include_str!("../../wit/deps/types.wit");` and comparing those consts across crates.

## Open Questions

1. Does `include_str!("../../wit/deps/types.wit")` resolve correctly from `crates/slicer-macros/src/lib.rs` at proc-macro compile time? If not, what is the working alternative?
2. What is the exact content of the missing `needs-support` interface in `deps/ir-types.wit`? (Search macro and host inline WIT for `needs-support` to determine the expected signature.)
3. Does `push-z-hop` already exist in the layer-world `gcode-output-builder` but not in the postpass one? Or is it missing from both? (Verify against `docs/03_wit_and_manifest.md`.)
4. Are there any schema/CLI constants in `crates/slicer-host/src/config_schema.rs` or `crates/slicer-host/src/cli.rs` that reference the wrong WIT package names?
5. Should the drift detection test be a compile-time const assertion (faster, earlier failure) or a runtime test (can run in CI without recompiling)?

Resolve open questions 1-3 before activating the packet. Open questions 4-5 can be resolved during implementation.
