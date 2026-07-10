---
status: superseded
packet: 03-rev1_wit-canonical-source-and-validation
task_ids:
  - TASK-144
  - TASK-145
  - TASK-146
supersedes: 03_wit-canonical-source-and-validation
---

# 03-rev1_wit-canonical-source-and-validation

## Goal

Complete the three incomplete steps from `03_wit-canonical-source-and-validation` (which was marked `[x]` in docs/07 but audit revealed partial execution): add missing `push-z-hop` to the canonical `wit/world-postpass.wit`, replace remaining `inline: r#"..."#` blocks in `wit_host.rs` with `include_str!` references to canonical `wit/` files, and fix `slicer-core` clippy errors blocking the completion gate.

## Problem Statement

The original `03_wit-canonical-source-and-validation` packet was marked `[x]` in docs/07 but a spec review audit (2026-04-18) revealed three incomplete steps:

1. **CRIT-2 — Host WIT consolidation (Steps 3-4):** `crates/slicer-host/src/wit_host.rs` still contains four `inline: r#"..."#` blocks (at lines 178, 378, 572, 681). The implementation plan required replacing these with `include_str!` references to canonical `wit/world-*.wit` files. This was not done.

2. **CRIT-1 — Missing `push-z-hop` in canonical disk (Step 5):** `wit/world-postpass.wit` `gcode-output-builder` has only 7 methods. The macro (`lib.rs:571`) and host inline (`wit_host.rs:737-745`) both have `push-z-hop`, but the canonical disk source is missing it.

3. **HIGH-2 — Clippy gate failure:** `cargo clippy --workspace -- -D warnings` fails with errors in `slicer-core`: `find_unused_line` dead code, `clone_on_copy` on `PaintValue`, and a redundant closure. This blocks the completion gate.

## Architecture Constraints

- The `include_str!` relative path from `crates/slicer-host/src/wit_host.rs` to `wit/` is `../../wit/`. This path is shorter than the macro's path since `wit_host.rs` is at `crates/slicer-host/src/` (two levels deep from workspace root) vs `crates/slicer-macros/src/` (three levels deep).
- `wit_bindgen::generate!` accepts `&str` — the `include_str!` result (`&'static str`) satisfies this.
- The `WIT_WORLD_ALLOWLIST` in `manifest.rs` must remain in sync with the actual canonical world identifiers. After consolidating host WIT to use `include_str!`, the allowlist stays as-is (it was already correct).
- Version (`@1.0.0` vs `@1.1.0`) is part of the allowlist identifier.

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
