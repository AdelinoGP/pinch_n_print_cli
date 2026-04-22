# Design: 26_live-support-module-evidence

## Controlling Code Paths

1. **`crates/slicer-host/tests/live_support_generation_tdd.rs`** — current support evidence; needs tier split.
2. **`crates/slicer-host/tests/dispatch_tdd.rs`** — production-dispatch support/paint fixtures to reuse.
3. **`crates/slicer-host/tests/live_seam_path_tdd.rs`** — pattern for loading real `.wasm` on live host path (to be reused).
4. **`crates/slicer-host/tests/benchy_end_to_end_tdd.rs`** — current Benchy harness; needs `--config` extension and filtered module-dir builder.
5. **`crates/slicer-host/src/main.rs`** — real binary that already parses `--config JSON`; constrains fixture shape.
6. **`resources/test_config/`** — existing config fixture directory; new JSON fixture goes here.

## Architecture Constraints

- Real support module loading requires `WasmInstancePool::get` + `WasmRuntimeDispatcher::dispatch_layer_call` on the production path (not a synthetic fixture).
- The filtered module-dir builder must produce a directory that passes the claim resolution system (tree-support wins over traditional-support).
- JSON config fixture must use keys that match the live `tree-support.toml` manifest.
- Benchy acceptance assertions must prove actual support on the live path, not just printable output.

## Implementation Approach

### Tier Split for `live_support_generation_tdd.rs`

**Keep as-is** (commit-path tests):
- `tree_support_dispatch_commits_support_material_paths` — uses `commit_layer_outputs_for_test` with synthetic data
- `traditional_support_dispatch_commits_support_material_paths` — same synthetic pattern

**Add new** (real live-dispatch tests):
- `tree_support_live_dispatch_produces_non_empty_support_ir` — loads real `tree-support.wasm` via pool, dispatches `Layer::Support`, asserts `SupportIR.support_paths` non-empty with `SupportMaterial` roles
- `traditional_support_live_dispatch_produces_non_empty_support_ir` — same for traditional-support
- `support_deterministic_across_repeated_runs` — runs same dispatch twice, asserts byte-identical output
- Optionally: `support_enforcer_blocker_paint_precedence` — uses `PaintRegionIR` helpers from `dispatch_tdd.rs`

### Reusing `live_seam_path_tdd.rs` Pattern

The `live_seam_path_tdd.rs` file already has:
- `load_wasm_module` helper that reads a `.wasm` from `modules/core-modules/`
- `WasmInstancePool` setup
- `WasmRuntimeDispatcher` dispatch call
- Result inspection

Adapt this pattern for support modules. The key difference is that support uses `Layer::Support` stage instead of `Layer::PerimetersPostProcess`.

### Extended `run_slicer_host` Helper

The existing `run_slicer_host` in `benchy_end_to_end_tdd.rs` has signature:
```rust
fn run_slicer_host(model: &Path, module_dir: &Path, output: &Path) -> std::process::Output
```

Extend to:
```rust
fn run_slicer_host(model: &Path, module_dir: &Path, output: &Path, config: Option<&Path>) -> std::process::Output
```

When `config` is `Some(path)`, append `--config path` to the CLI call.

### JSON Config Fixture

Create `resources/test_config/benchy-tree-support.json`:
```json
{
  "support_enabled": true,
  "support_module": "tree-support",
  "support_type": "tree",
  "support_threshold": 45.0,
  "support_density": 0.1
}
```

The exact keys must match those accepted by `tree-support.toml`. Confirm from the manifest before writing.

### Filtered Module-Dir Builder

Add a helper function in the Benchy test:
```rust
fn filtered_module_dir_for_tree_support(tmp: &tempfile::TempDir) -> PathBuf {
    let src = core_modules_dir();
    let dst = tmp.path().join("tree-support-modules");
    // Copy all modules except traditional-support.wasm
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "wasm") {
            if path.file_name().map_or(false, |n| n == "traditional-support.wasm") {
                continue; // skip to make tree-support the active holder
            }
            std::fs::copy(&path, dst.join(path.file_name().unwrap())).unwrap();
        }
        if path.extension().map_or(false, |e| e == "toml") {
            if path.file_name().map_or(false, |n| n == "traditional-support.toml") {
                continue;
            }
            std::fs::copy(&path, dst.join(path.file_name().unwrap())).unwrap();
        }
    }
    dst
}
```

Note: The TOML must also be excluded because it declares the competing `support-generator` claim.

### Support-Specific Benchy Assertions

Add new tests:
- `benchy_with_support_enabled` — runs Benchy with tree-support filtered dir and JSON config, asserts binary exits 0 and `.gcode` is non-empty
- `benchy_support_marker_present` — asserts `.gcode` contains `;TYPE:Support` or `;TYPE:Support interface`
- `benchy_support_deterministic` — runs identical command twice, asserts byte-identical output

## Data and Contract Notes

- `SupportIR.support_paths` is the canonical output surface for `Layer::Support`.
- `ExtrusionRole::SupportMaterial` is the correct role for support paths.
- Claim resolution: `tree-support.wasm` holds `support-generator`; `traditional-support.wasm` also holds `support-generator`; with `traditional-support` excluded, `tree-support` wins.
- `;TYPE:Support` and `;TYPE:Support interface` are the OrcaSlicer-compatible comment markers.

## Risks and Tradeoffs

- **Risk**: Stale checked-in `.wasm` binaries cause the live dispatch tests to fail or behave non-deterministically.
  - Mitigation: Packet 27 runs `build-core-modules.sh` before the focused test matrix; rebuild as part of Packet 26 if tests reveal stale binaries.
- **Risk**: The JSON config keys don't match the live `tree-support.toml` schema.
  - Mitigation: Read `tree-support.toml` to confirm accepted keys before writing the fixture.

## Open Questions

- Q1: What exact keys does `tree-support.toml` accept for `support_enabled`, `support_type`, etc.?
  - **Resolution**: Read `modules/core-modules/tree-support/tree-support.toml` before writing the JSON fixture.
- Q2: Does the existing `dispatch_tdd.rs` have `PaintRegionIR` helpers for SupportEnforcer/SupportBlocker paint precedence?
  - **Resolution**: Search `dispatch_tdd.rs` for `SupportEnforcer`/`SupportBlocker` patterns before adding that test.

## Locked Assumptions

1. Real `.wasm` modules are checked in under `modules/core-modules/`.
2. `WasmInstancePool` can load modules by ID.
3. `WasmRuntimeDispatcher::dispatch_layer_call` accepts a `&StageId` and a module ID.
4. `;TYPE:Support` and `;TYPE:Support interface` are the OrcaSlicer-compatible markers used by the emitter.
