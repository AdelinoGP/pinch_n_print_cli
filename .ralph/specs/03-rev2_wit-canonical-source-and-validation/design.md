# Design: 03-rev2_wit-canonical-source-and-validation

## Controlling Code Paths

- **Primary code path — Test assertion fix**: `crates/slicer-host/tests/wit_drift_detection_tdd.rs` — the function `host_bindgen_with_keys_use_canonical_world_names` (lines 146-151) asserts that `wit_host.rs` contains version-unsuffixed `with:` keys like `"slicer:world-layer/config-types/config-view"`. This is wrong — wasmtime `bindgen!` emits version-suffixed keys like `"slicer:world-layer/config-types@1.0.0.config-view"`. The test must be updated to check the correct format.

- **Primary code path — Large Result fix**: `crates/slicer-host/src/dag.rs:27` returns `Result<Vec<ModuleNode>, SchedulerError>`; `crates/slicer-host/src/execution_plan.rs:298` returns `Result<LiveModuleLoadOutput, LiveModuleLoadError>`. Both have Err variants > 128 bytes. Fix: box the error with `Box<SchedulerError>` and `Box<LiveModuleLoadError>`.

- **Primary code path — Unused import fix**: `crates/slicer-host/src/execution_plan.rs:12` imports `ConfigFieldEntry` but it is only used inside test helpers (`#[cfg(test)]` block at line 851). The import at line 12 is for the lib scope and should be moved inside the test module or the lib-level import removed.

- **Primary code path — sort_by fix**: `crates/slicer-host/src/layer_executor.rs:263` — change `out.sort_by(|a, b| semantic_sort_key(a).cmp(&semantic_sort_key(b)))` to `out.sort_by_key(semantic_sort_key)`.

- **Primary code path — map_or fix**: `crates/slicer-host/src/manifest.rs:359` — change `sub_path.file_name().and_then(|n| n.to_str()).map_or(true, |n| n != "Cargo.toml")` to `sub_path.file_name().and_then(|n| n.to_str()) != Some("Cargo.toml")`.

- **Primary code path — is_multiple_of fix**: `crates/slicer-host/src/mesh_analysis.rs:119` — change `mesh.indices.len() % 3 != 0` to `!mesh.indices.len().is_multiple_of(3)`.

- **Primary code path — unnecessary unwrap fix**: `crates/slicer-host/src/prepass.rs:219` — after checking `ir_path.is_some()`, calling `.unwrap()` is unnecessary. Use `if let Some(path) = ir_path` pattern or `ir_path.expect("checked is_some")`.

- **Primary code path — clone on Copy fix**: `crates/slicer-host/src/region_mapping.rs:119` — `region.region_id.clone()` on a `u64` which is `Copy`. Remove `.clone()`. Also `slice_postprocess.rs:296,310` — `PaintValue` is `Copy`; remove `.clone()`.

- **Primary code path — too many arguments**: `crates/slicer-host/src/dispatch.rs:253` — function has 11 parameters. Refactor by bundling related parameters into a struct (e.g., a config/options struct).

- **Primary code path — missing docs**: `crates/slicer-host/src/wit_host.rs` — add `#[allow(missing_docs)]` or real doc comments to `pub mod layer`, `pub mod prepass`, `pub mod finalization`, `pub mod postpass`, and the struct fields at lines 862+.

## Architecture Constraints

- The boxed error approach (`Box<SchedulerError>`, `Box<LiveModuleLoadError>`) preserves the existing error semantics while reducing the Result size. This is a standard Rust pattern for large error types.
- The `#[allow(missing_docs)]` approach for `wit_host.rs` modules is the pragmatic choice — the bindgen-generated modules have auto-generated docs that would duplicate std library documentation.

## Code Change Surface

### Selected approach

Each fix follows the minimal-change approach recommended by clippy.

### Exact files expected to change:

1. **`crates/slicer-host/tests/wit_drift_detection_tdd.rs`**: Update lines 146-151 to use version-suffixed `with:` keys:
   - `"slicer:world-layer/config-types/config-view"` → `"slicer:world-layer/config-types@1.0.0.config-view"`
   - `"slicer:world-prepass/config-types/config-view"` → `"slicer:world-prepass/config-types@1.0.0.config-view"`
   - `"slicer:world-finalization/config-types/config-view"` → `"slicer:world-finalization/config-types@1.0.0.config-view"`
   - `"slicer:world-postpass/config-types/config-view"` → `"slicer:world-postpass/config-types@1.0.0.config-view"`

2. **`crates/slicer-host/src/execution_plan.rs`**: Move `ConfigFieldEntry` import inside `#[cfg(test)]` block or remove lib-level import.

3. **`crates/slicer-host/src/dag.rs`**: Change return type to `Result<Vec<ModuleNode>, Box<SchedulerError>>` and box the error at all return sites.

4. **`crates/slicer-host/src/execution_plan.rs`**: Change return type at line 298 to use `Box<LiveModuleLoadError>`.

5. **`crates/slicer-host/src/layer_executor.rs:263`**: `sort_by` → `sort_by_key`.

6. **`crates/slicer-host/src/manifest.rs:359`**: `map_or` simplification.

7. **`crates/slicer-host/src/mesh_analysis.rs:119`**: `!mesh.indices.len().is_multiple_of(3)`.

8. **`crates/slicer-host/src/prepass.rs:219`**: Remove unnecessary `.unwrap()`.

9. **`crates/slicer-host/src/region_mapping.rs:119`**: Remove `.clone()` on `u64`.

10. **`crates/slicer-host/src/slice_postprocess.rs:296,310`**: Remove `.clone()` on `PaintValue`.

11. **`crates/slicer-host/src/dispatch.rs:253`**: Reduce argument count. Group related parameters (e.g., all config/view parameters into one `DispatchConfig` struct).

12. **`crates/slicer-host/src/wit_host.rs`**: Add `#[allow(missing_docs)]` before each `pub mod` block (layer, prepass, finalization, postpass) and struct field blocks, or add real doc comments.

## Locked Assumptions and Invariants

- The wasmtime `bindgen!` `with:` key format is `world/package@version.interface-name`. This is stable and not something this project controls.
- `u64` and `PaintValue` are `Copy` — removing `.clone()` does not change semantics.
- The boxed error types preserve the same error values — no error information is lost.

## Risks and Tradeoffs

- **dispatch.rs argument bundling**: Grouping parameters into a struct is a small API change. Any callers of the function must be updated. However, clippy only fires when there are ≥8 arguments (this function has 11), so the change surface is contained.
- **Box<SchedulerError> change**: Any code that matches on `SchedulerError` directly (without `&`) would break. Verify no such match exists before boxing.

## Open Questions

None. All issues are confirmed from the pre-clean clippy run.
