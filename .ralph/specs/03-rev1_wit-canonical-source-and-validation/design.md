# Design: 03-rev1_wit-canonical-source-and-validation

## Controlling Code Paths

- **Primary code path — Host WIT consolidation:** `crates/slicer-host/src/wit_host.rs` — four `pub mod` blocks (`layer` at line 176, `prepass` at line 376, `finalization` at line 570, `postpass` at line 679), each with a `wasmtime::component::bindgen!` call containing an `inline: r#"..."#` string. These must be replaced with `include_str!` references to the canonical `wit/world-*.wit` files.

- **Primary code path — Canonical disk update:** `wit/world-postpass.wit` — the `gcode-output-builder` resource definition. The `push-z-hop` method must be added to match what the macro (`lib.rs:571`) and host inline (`wit_host.rs:737-745`) already have.

- **Primary code path — Clippy fixes:** `crates/slicer-core/src/triangle_mesh_slicer.rs` (dead code at line 344, redundant closure at line 56) and `crates/slicer-core/src/paint_region.rs` (`clone_on_copy` at line 54).

- **Drift detection test:** `crates/slicer-host/tests/wit_drift_detection_tdd.rs` — already exists and passes; must remain green after consolidation changes.

- **Allowlist validation:** `crates/slicer-host/src/manifest.rs:653-658` (`WIT_WORLD_ALLOWLIST`) and `manifest.rs:664-677` (`validate_wit_world`). These are already correct and do not need changes.

## Neighboring Tests or Fixtures

- `crates/slicer-host/tests/manifest_ingestion_tdd.rs` — `wit_world_mismatch_rejects_invalid_package_name` and `wit_world_major_version_mismatch_rejects_future_major` tests (already passing)
- `crates/slicer-host/tests/live_module_loading_tdd.rs` — 13 tests (already passing)
- `crates/slicer-host/tests/wit_drift_detection_tdd.rs` — 9 tests (already passing)

## Architecture Constraints

- The `include_str!` relative path from `crates/slicer-host/src/wit_host.rs` to `wit/` is `../../wit/`. This path is shorter than the macro's path since `wit_host.rs` is at `crates/slicer-host/src/` (two levels deep from workspace root) vs `crates/slicer-macros/src/` (three levels deep).
- `wit_bindgen::generate!` accepts `&str` — the `include_str!` result (`&'static str`) satisfies this.
- The `WIT_WORLD_ALLOWLIST` in `manifest.rs` must remain in sync with the actual canonical world identifiers. After consolidating host WIT to use `include_str!`, the allowlist stays as-is (it was already correct).
- Version (`@1.0.0` vs `@1.1.0`) is part of the allowlist identifier.

## Code Change Surface

### Selected approach

**Step 1 — Fix canonical disk (TASK-145):** Add `push-z-hop` to `wit/world-postpass.wit`'s `gcode-output-builder`. This is a one-line addition to the canonical source.

**Step 2 — Host consolidation (TASK-144):** Replace each `inline: r#"..."#` block in `wit_host.rs` with `include_str!` referencing the corresponding `wit/world-*.wit` file. The WIT content from each inline string must be extracted and written to the canonical disk files if not already present, then the `bindgen!` call is updated.

**Step 3 — Clippy gate (TASK-146):** Fix three specific clippy errors in `slicer-core`:
1. `triangle_mesh_slicer.rs:344` — `find_unused_line` is never used: either remove it or add `#[allow(dead_code)]`
2. `triangle_mesh_slicer.rs:56` — redundant closure: replace `|lines| chain_lines_to_expolygons(lines)` with `chain_lines_to_expolygons`
3. `paint_region.rs:54` — `clone_on_copy` on `PaintValue`: change `value.clone()` to `*value`

### Exact files expected to change

1. **`wit/world-postpass.wit`** — add `push-z-hop` to `gcode-output-builder`
2. **`crates/slicer-host/src/wit_host.rs`** — replace 4 `inline: r#"..."#` blocks with `include_str!` to `wit/world-*.wit` files. Keep the `world:` parameter and `with:` parameterization intact — only the WIT source changes.
3. **`crates/slicer-core/src/triangle_mesh_slicer.rs`** — fix dead code and redundant closure
4. **`crates/slicer-core/src/paint_region.rs`** — fix `clone_on_copy`

### Rejected alternatives

- **Regenerate host WIT from disk world files:** Would require `wit/world-layer.wit`, `wit/world-prepass.wit`, `wit/world-finalization.wit` to exist with matching content. The postpass world on disk is currently minimal (only exports, no full inline), so the host inline cannot simply `include_str!` the disk file as-is. Instead, the inline string content must be preserved (it contains full interface definitions the disk file doesn't have), and only the dep includes (`types.wit`, `config.wit`) are pulled from disk. This is the minimal correct fix.

## Data and Contract Notes

- WIT boundary: Consolidation does NOT change WIT types, only their source. The `wit_bindgen!` output types remain identical after switching from `inline:` to `include_str!`.
- `push-z-hop` in postpass world: This method exists in layer world's `gcode-output-builder` but was missing from postpass. Adding it to the postpass canonical disk makes the disk canonical complete.
- The drift detection test (`wit_drift_detection_tdd`) will verify that the postpass world in the host matches the disk file after `push-z-hop` is added.

## Locked Assumptions and Invariants

- The four canonical world identifiers in `WIT_WORLD_ALLOWLIST` (`slicer:world-layer@1.0.0`, `slicer:world-prepass@1.0.0`, `slicer:world-postpass@1.0.0`, `slicer:world-finalization@1.0.0`) are stable and do not change in this packet.
- The `wit/` directory remains the single source of truth for WIT content after consolidation.
- `validate_wit_world` behavior does not change in this packet — it is already correctly implemented.

## Risks and Tradeoffs

- **Path resolution:** `include_str!("../../wit/world-postpass.wit")` from `crates/slicer-host/src/wit_host.rs` must resolve correctly. The path `../../wit/` from `src/` leads to `wit/` at workspace root — same pattern that works for the macro, just one level shallower.
- **Postpass world structure:** The disk `wit/world-postpass.wit` is currently a thin world file that imports from `slicer:host-api/host-services` and `slicer:config/config-types`. The host's inline postpass WIT defines a complete inline world with full `geometry` and `config-types` interface definitions. Simply pointing `include_str!` at `wit/world-postpass.wit` would break the host's bindings because the disk file doesn't have those inline interface definitions. The correct fix is to add `push-z-hop` to the disk file AND keep the host's inline structure, but have it `include "../../wit/deps/types.wit"` and `include "../../wit/deps/config.wit"` for the dep interfaces.

## Open Questions

1. **Postpass include strategy:** Does `wit/world-postpass.wit` need to be expanded to include the full `geometry` and `config-types` interface definitions inline so the host can `include_str!` it, or should the host keep its inline interfaces and only `include_str!` the dep files? Decision: the host's postpass block is a self-contained world definition that doesn't use `include` directives for deps (unlike the macro which uses `include "../../wit/deps/types.wit"`). The simplest correct fix is to add `push-z-hop` to the disk canonical and leave the host's inline block as-is (with the dep `include` directives added to the inline string, matching the macro's pattern).

2. **Clippy allow vs fix:** For `find_unused_line` (line 344 of `triangle_mesh_slicer.rs`), should we remove the function or add `#[allow(dead_code)]`? Decision: remove it — it is genuinely unused and its removal improves code clarity.