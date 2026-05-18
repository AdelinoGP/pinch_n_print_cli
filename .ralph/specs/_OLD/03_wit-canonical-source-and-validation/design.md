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

### Selected approach (revised after implementation)

**Macro (`slicer-macros/src/lib.rs`)**: The macro uses WIT-level `include` directives inside `const` string literals (e.g., `const LAYER_WORLD_WIT: &str = r#"include "../../wit/deps/types.wit";#` rather than `include_str!`). This is superior to `include_str!` because `wit_bindgen::generate!` processes the WIT string with its own parser, which resolves WIT `include` directives at parse time. The WIT parser resolves `include "../../wit/deps/types.wit"` relative to the macro crate source.

**Host (`wit_host.rs`)**: The inline WIT in `bindgen!` blocks CANNOT be replaced with `include_str!` pointing to disk files. The disk `wit/world-*.wit` files use `import slicer:...` package references (e.g., `import slicer:host-api/host-services`) that require those WIT packages to be resolvable at compile time. The wasmtime `bindgen!` macro's `inline:` parameter expects a WIT string with all interfaces defined inline — external package imports cannot be resolved without the actual package files being present. Therefore the inline WIT blocks in `wit_host.rs` are the correct and necessary approach; they define fully-expanded interfaces for host-side `bindgen!`.

**Drift detection (`wit_drift_detection_tdd.rs`)**: The test uses `std::fs::read_to_string` to read disk WIT files at test runtime, not `include_str!`. It reads the macro `lib.rs` and `wit_host.rs` source files as strings and checks for WIT-level `include` directives (in the macro) and canonical package names (in the host). This approach is correct.

### Exact functions, traits, manifests, tests, or fixtures expected to change:

1. **`crates/slicer-macros/src/lib.rs`**: Update `build_*_world_glue` functions to use WIT-level `include` directives inside const string literals (`const LAYER_WORLD_WIT: &str = r#"include "../../wit/deps/types.wit";#`), pointing to the canonical `wit/deps/` files. The `WIT_WORLD_MAP` entries stay as-is.

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
- The `wit/` directory is the single source of truth for macro WIT glue generation (macro uses WIT-level `include` directives referencing `wit/deps/`). The host `wit_host.rs` inline WIT blocks are retained because wasmtime's `bindgen!` requires fully-expanded inline WIT — external WIT package imports cannot be resolved at `bindgen!` compile time without the packages being present. This is a known deviation from the original "eliminate all inline copies" goal.
- The allowlist contains exactly four entries — one per WIT world. No wildcard or regex matching.
- `needs-support` and `push-z-hop` are additive missing members — adding them to the disk canonical does not break any existing bindings because they are optional interfaces.

## Risks and Tradeoffs

- **Proc-macro `include_str!` path resolution**: The relative path `../../wit/` from `crates/slicer-macros/src/lib.rs` must be verified to work at macro compile time. If it does not, an alternative is to place a copy of the key WIT files in the `slicer-macros` crate directory.
- **Package name changes in host**: Changing `slicer:layer-world@1.0.0` → `slicer:world-layer@1.0.0` in `wit_host.rs` requires the host's `bindgen!` output types to be regenerated. This is a breaking change for any code that imports from the old package name — but since this is host-only code that doesn't expose these types publicly, it should be a contained refactor.
- **ir-types version bump**: If the canonical version is `@1.1.0`, any manifest that declares `min-ir-schema = "1.1.0"` must now pass. Existing manifests that only declare `1.0.0` range may need updating (but the IR schema version is independent of the WIT version per docs/03 architecture rules).
- **Drift detection test fragility**: Extracting `include_str!` paths by string search is fragile. A more robust approach is to use a const that holds the string directly and compare that. Consider defining a const in each crate: `const CANONICAL_WIT_TYPES: &str = include_str!("../../wit/deps/types.wit");` and comparing those consts across crates.

## Open Questions (answered)

1. **Does `include_str!("../../wit/deps/types.wit")` resolve from `crates/slicer-macros/src/lib.rs`?** The `include_str!` approach was not used. WIT-level `include` directives inside const string literals are used instead (e.g., `const LAYER_WORLD_WIT: &str = r#"include "../../wit/deps/types.wit";#`). This works because `wit_bindgen::generate!` processes the WIT string with its own parser that resolves `include` directives at WIT parse time. This is superior to `include_str!` because the WIT parser validates the included content.
2. **What is the exact content of the missing `needs-support` interface?** Added to `wit/deps/ir-types.wit` at line 67 as `needs-support: func() -> bool;`.
3. **Does `push-z-hop` exist in layer-world but not postpass?** Yes. Added to `wit/world-postpass.wit` at line 44. Layer-world already had it.
4. Are there schema/CLI constants referencing wrong WIT package names? No issues found.
5. **Runtime vs compile-time drift detection:** Runtime test (using `std::fs::read_to_string` in the test binary) was chosen. Compile-time assertions would require recompiling the macro on every test run. Runtime is sufficient.

Resolve open questions 1-3 before activating the packet. Open questions 4-5 can be resolved during implementation.
