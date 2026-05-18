# Requirements: 03-rev1_wit-canonical-source-and-validation

## Packet Metadata

- Grouped task IDs:
  - `TASK-144` — Host WIT consolidation with `include_str!` (incomplete — Step 4 not executed)
  - `TASK-145` — Restore missing `push-z-hop` in `wit/world-postpass.wit` (incomplete — Step 5 partially done)
  - `TASK-146` — Clippy gate fix (new — blocking completion)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Supersedes: `03_wit-canonical-source-and-validation` (implemented but incomplete per audit)

## Problem Statement

The original `03_wit-canonical-source-and-validation` packet was marked `[x]` in docs/07 but a spec review audit (2026-04-18) revealed three incomplete steps:

1. **CRIT-2 — Host WIT consolidation (Steps 3-4):** `crates/slicer-host/src/wit_host.rs` still contains four `inline: r#"..."#` blocks (at lines 178, 378, 572, 681). The implementation plan required replacing these with `include_str!` references to canonical `wit/world-*.wit` files. This was not done.

2. **CRIT-1 — Missing `push-z-hop` in canonical disk (Step 5):** `wit/world-postpass.wit` `gcode-output-builder` has only 7 methods. The macro (`lib.rs:571`) and host inline (`wit_host.rs:737-745`) both have `push-z-hop`, but the canonical disk source is missing it.

3. **HIGH-2 — Clippy gate failure:** `cargo clippy --workspace -- -D warnings` fails with errors in `slicer-core`: `find_unused_line` dead code, `clone_on_copy` on `PaintValue`, and a redundant closure. This blocks the completion gate.

## In Scope

- Add `push-z-hop: func(after-entity-index: u32, hop-height: f32) -> result<_, string>;` to `gcode-output-builder` in `wit/world-postpass.wit`
- Replace all four `inline: r#"..."#` blocks in `wit_host.rs` with `include_str!` references to canonical `wit/world-*.wit` files
- Fix three clippy errors in `crates/slicer-core/src/triangle_mesh_slicer.rs` and `crates/slicer-core/src/paint_region.rs`
- Re-verify all acceptance criteria from the original packet
- Mark docs/07 TASK-144/145/146 `[x]` again once this rev is complete

## Out of Scope

- Custom payload widening (TASK-149/150) — separate packet `04_custom-payload-widening`
- WIT type shape changes (extrusion-role, paint-semantic, wall-feature-flag)
- IR schema version changes
- New task ID creation

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — canonical WIT structure, `wit/world-postpass.wit` section
- `docs/04_host_scheduler.md` — module load validation, `validate_wit_world`
- `crates/slicer-host/src/wit_host.rs` — host bindgen blocks (lines 176-767)
- `crates/slicer-host/src/manifest.rs` — `WIT_WORLD_ALLOWLIST` (line 653), `validate_wit_world` (line 664)
- `wit/world-postpass.wit` — canonical postpass world (missing `push-z-hop`)
- `crates/slicer-core/src/triangle_mesh_slicer.rs` — clippy errors at line 344 and line 56
- `crates/slicer-core/src/paint_region.rs` — clippy error at line 54

## OrcaSlicer Reference Obligations

None. This is internal WIT infrastructure consolidation.

## Acceptance Summary

- Positive cases:
  - `push-z-hop` present in canonical `wit/world-postpass.wit` `gcode-output-builder`
  - `wit_host.rs` has zero `inline: r#"..."#` patterns (all four worlds use `include_str!`)
  - `wit_drift_detection_tdd` passes with zero drift across all four worlds and three dep interfaces
  - `cargo clippy --workspace -- -D warnings` exits with code 0
  - `validate_wit_world` correctly accepts canonical names and rejects pre-consolidation names
- Negative cases:
  - Pre-consolidation package name `slicer:layer-world@1.0.0` rejected with diagnostic
  - Future major version `slicer:world-layer@2.0.0` rejected with diagnostic
  - Post-consolidation disk modification caught by drift detection test
- Measurable outcomes:
  - `grep -c 'inline: r#"' crates/slicer-host/src/wit_host.rs` → `4`
  - `grep "push-z-hop" wit/world-postpass.wit` → returns the method signature
  - `cargo clippy --workspace -- -D warnings` → zero errors, zero warnings

## Verification Commands

- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture`
- `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture`
- `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

- Step 1 (postpass WIT fix): `grep "push-z-hop" wit/world-postpass.wit` returns the method
- Step 2 (host consolidation): `grep -c 'inline: r#"' crates/slicer-host/src/wit_host.rs` returns `0`
- Step 3 (clippy gate): `cargo clippy --workspace -- -D warnings` exits with code 0
- Step 4 (re-verification): All packet acceptance criteria green; docs/07 marked `[x]` again