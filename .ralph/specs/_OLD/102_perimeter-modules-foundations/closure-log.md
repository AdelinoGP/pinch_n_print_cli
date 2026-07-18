# Closure Log: 102_perimeter-modules-foundations

Closed: 2026-06-18

## Acceptance Ceremony Results

All acceptance criteria verified green:

| Criterion | Command | Result |
|-----------|---------|--------|
| AC-1 | `cargo check -p slicer-core --all-targets` + export grep | PASS |
| AC-2 | No duplicate `fn` defs in either module | PASS |
| AC-3 | `material_boundary_widening_tdd` + schema 4.2.0 | PASS |
| AC-4 | No `let _ = output\.` remaining | PASS |
| AC-5 | `per_layer_config_override_tdd` | PASS |
| AC-6 | `manifest_default_reconcile_tdd` | PASS |
| AC-N1 | `perimeter_builder_capacity_error_tdd` | PASS |
| AC-N2 | `perimeter_utils_three_tool_boundary_tdd` | PASS |

Gate commands:
- `cargo check --workspace --all-targets`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- `cargo xtask build-guests --check`: PASS (no stale entries after rebuild)

Regression:
- `classic-perimeters boundary_paint_tdd`: 8/8 PASS
- `arachne-perimeters boundary_paint_tdd`: 6/6 PASS

## Doc Impact Verification

- `rg -q 'MaterialBoundarySegment' docs/02_ir_schemas.md`: PASS
- `rg -q '4\.2\.0.*MaterialBoundary' docs/02_ir_schemas.md`: PASS
- `rg -q 'PerimeterOutputBuilder failure modes' docs/05_module_sdk.md`: PASS
- Config keys reference defaults (3 / 30.0 / 45.0): PASS

## Files Changed

- `crates/slicer-core/src/perimeter_utils.rs` (NEW)
- `crates/slicer-core/src/lib.rs`
- `crates/slicer-ir/src/slice_ir.rs`
- `crates/slicer-ir/src/lib.rs`
- `crates/slicer-schema/wit/deps/ir-types.wit`
- `crates/slicer-sdk/src/builders.rs`
- `crates/slicer-sdk/src/error.rs`
- `modules/core-modules/classic-perimeters/src/lib.rs`
- `modules/core-modules/arachne-perimeters/src/lib.rs`
- `modules/core-modules/classic-perimeters/classic-perimeters.toml`
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
- `modules/core-modules/fuzzy-skin/src/lib.rs`
- `docs/02_ir_schemas.md`
- `docs/05_module_sdk.md`
- `docs/15_config_keys_reference.md`
- `docs/07_implementation_status.md`
- `crates/slicer-ir/tests/material_boundary_widening_tdd.rs` (NEW)
- `crates/slicer-core/tests/perimeter_utils_three_tool_boundary_tdd.rs` (NEW)
- `crates/slicer-runtime/tests/contract/per_layer_config_override_tdd.rs` (NEW)
- `crates/slicer-runtime/tests/contract/perimeter_builder_capacity_error_tdd.rs` (NEW)
- `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` (NEW)
- `crates/slicer-runtime/tests/contract/main.rs`
- `crates/slicer-runtime/Cargo.toml`
- `crates/slicer-sdk/tests/test_support_wall_loop_with_flags_tdd.rs`
- `crates/slicer-ir/tests/ir_tests.rs`

## Notes

- Manifest-vs-code reconcile direction followed `[FWD]` default: manifest wins (3 / 30.0 / 45.0). Code fallbacks aligned.
- `_paint` doc-comment uses the prescribed wording: "intentionally unread in this module — consumed by Phase 2 follow-up packet 102."
- Clippy dead-code fix: `outer_speed_factor`/`inner_speed_factor` fields in `ClassicPerimeters` now used as fallback in `run_perimeters` (`unwrap_or(self.outer_speed_factor * BASE_SPEED)`).
- `arachne-perimeters` struct doesn't store speed factors (only `wall_count`, `line_width`, `perimeter_arc_tolerance`), so no equivalent fix needed.
- All 31 guest WASMs rebuilt after WIT change.

## Post-Review Remediation (2026-06-18)

Spec-review found the production code complete and correct, but flagged verification-layer gaps. Fixed:

- **AC-6 test was vacuous** — `manifest_default_reconcile_tdd` asserted a hardcoded `3` on both the "manifest" and "code" sides (never parsed the TOML; never tested the two speed keys). Rewritten to parse each manifest via `include_str!` + `toml`, observe the code fallback by driving `run_perimeters` with an empty config, and reconcile all three keys (`wall_count`, `outer_wall_speed`, `inner_wall_speed`) for both modules. Passes 2/2.
- **AC-3 verification grep was non-runnable** — the single-line `rg` pattern for `CURRENT_SLICE_IR_SCHEMA_VERSION` never matched the multi-line constant. Switched to `rg -U`. Also added `schema_version_is_4_2_0` and `three_transition_polygon_carries_three_segments` to `material_boundary_widening_tdd` so AC-3's named test is self-contained (6/6).
- **AC-1 grep was vacuous** (alternation passed on any one symbol, masking that `has_adjacent_material_change` / `find_adjacent_tool` don't exist). Reconciled the AC text/grep with deviation D-102-AC1 (names `find_all_transitions`) and made the grep check every symbol individually. AC-2 negative grep updated to match.
- **Stale doc comment** in `ir_tests.rs` still read `SemVer { 4, 1, 0 }` in the header of the version-assert test; corrected to `4, 2, 0`.
- **Migration-adapter precedence** (non-empty `segments` + legacy `adjacent_tool` → segments win) now documented inline in `slice_ir.rs`.
