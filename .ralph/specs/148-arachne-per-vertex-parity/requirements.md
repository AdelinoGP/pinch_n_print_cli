# Requirements: 148-arachne-per-vertex-parity

## Packet Metadata

- Grouped task IDs: none (audit-only; not yet on `docs/07_implementation_status.md`).
- Backlog source: `tmp/arachne_parity_audit_20260709.md` (working artifact; not canonical — `docs/DEVIATION_LOG.md` is canonical per the grilling-session decision).
- Packet status: `draft`
- Aggregate context cost: `M` (sum of per-step S/M costs in `implementation-plan.md`).

## Problem Statement

The audit (`tmp/arachne_parity_audit_20260709.md`) found that `arachne-perimeters` produces real walls (P112 + P141–P147) but emits them as a degenerate `WallLoop`: `boundary_type` is hardcoded `Interior` for every wall, `LoopType::ThinWall` is never returned by `classify_line`, `is_thin_wall`/`is_bridge` per-vertex flags are never set, `overhang_quartile` is never populated, no seam candidates are emitted, and `precise_outer_wall` is not even registered in the manifest. The pipeline reaches OrcaSlicer parity via the classic path (which already does all of these), but the arachne path diverges. Seven red tests in `crates/slicer-runtime/tests/arachne_parity.rs` lock this gap; closing them is the runnable acceptance criterion for this packet.

The classic path's `crates/slicer-core/src/perimeter_utils.rs` already exports the helpers this packet needs (`point_in_any_polygon`, `generate_sharp_corner_seam_candidates`, `WallSequence`); the work is wiring them into the arachne module's `run_perimeters` loop and `classify_line` function, plus the manifest entries for two new config keys.

## In Scope

- Edit `modules/core-modules/arachne-perimeters/Cargo.toml`:
  - Add `slicer-core = { path = "../../../crates/slicer-core", default-features = false }` (the helpers in `slicer_core::perimeter_utils` live there; classic already has the dep, and the `default-features = false` pin prevents `host-algos` (voronoi/rayon) from being enabled on the wasm guest). `slicer-core` itself compiles to `wasm32-unknown-unknown` with default features (verified: `cargo build -p slicer-core --target wasm32-unknown-unknown` finishes in 15s).
- Edit `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`:
  - Add `[config.schema.precise_outer_wall]` (bool, default `false`, matches classic).
  - Add `[config.schema.seam_candidate_angle_threshold_deg]` (float, default `30.0`, range `0.0..=180.0`, matches classic).
- Edit `modules/core-modules/arachne-perimeters/src/lib.rs`:
  - `classify_line` (lib.rs:206-214): return `LoopType::ThinWall` for widened thin-wall beads, gated on `print_thin_walls && medial_axis_enabled` (mirroring classic's gate).
  - `run_perimeters` (lib.rs:236-352): in the `WallLoop` construction loop (lib.rs:296-303), populate `feature_flags` (per-vertex `is_bridge` from `region.bridge_areas()` via `point_in_any_polygon`; per-vertex `is_thin_wall` on `ThinWall` loops only) and `boundary_type` (ExteriorSurface for `inset_idx == 0`, Interior otherwise); populate per-vertex `path.points[i].overhang_quartile` from the band lookup against `region.overhang_quartile_polygons()` (mirrors `expolygon_to_path3d:316-331`); emit seam candidates for the outer wall using the **input region polygon contour** (`region.polygons()[0].contour`), NOT `wall.path` (type bridge: helper takes `&slicer_ir::Polygon` in units, `wall.path` is `ExtrusionPath3D` in mm — call shape matches classic's `lib.rs:889-900`).
  - `arachne_params_from_config` (lib.rs:106-197): thread `precise_outer_wall` and `seam_candidate_angle_threshold_deg` through. **AC-8 mechanism rewrite**: gate `ArachneParams.outer_wall_offset` on `precise_outer_wall && wall_sequence==InnerOuter`; the inset is realized at the beading-stack level via the existing `ArachneParams.outer_wall_offset` plumbing (wired at lib.rs:157) — mirrors OrcaSlicer's `OuterWallInsetBeadingStrategy::compute` at `Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp:44-60`. NOT a post-hoc `wall.path` mutation.
- Add new unit-test files in `modules/core-modules/arachne-perimeters/tests/` (one per rewritten arachne-path AC):
  - `arachne_parity_outer_wall_boundary_type_tdd.rs` (AC-1)
  - `arachne_parity_thin_wall_loop_type_tdd.rs` (AC-2)
  - `arachne_parity_is_thin_wall_flag_tdd.rs` (AC-3)
  - `arachne_parity_is_bridge_flag_tdd.rs` (AC-4)
  - `arachne_parity_overhang_quartile_tdd.rs` (AC-5)
  - `arachne_parity_seam_candidate_tdd.rs` (AC-6)
  - `arachne_parity_precise_outer_wall_manifest_tdd.rs` (AC-7)
  - `precise_outer_wall_tdd.rs` (AC-8 + AC-N2)
  - Each file drives `ArachnePerimeters::run_perimeters` natively via `PerimeterOutputBuilder::new()` + `SliceRegionViewBuilder` + `ConfigViewBuilder` + `PaintRegionLayerView::new(0)`, asserts on `output.wall_loops()` / `output.seam_candidates()`. Harness pattern from `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs:1-50`.
- Edit `crates/slicer-runtime/tests/arachne_parity.rs`:
  - REWRITE the 7 arachne-path red tests to drive `ArachnePerimeters::run_perimeters` natively (the new `arachne-perimeters/tests/*_tdd.rs` files are the canonical coverage; the `arachne_parity.rs` file is updated to delegate to them or is trimmed to the 3 stale-doc + 4 pipeline-config + 1 D-104f tests that stay manifest/wiring-presence greps).
- Edit `docs/DEVIATION_LOG.md`:
  - Refine the `D-104-OVERHANG-QUARTILE-NONE` entry's rationale to "arachne-path-only per-vertex overhang/flag/seam/boundary parity" (the classic path is at parity via T-024, T-077, classic classify_line, etc.).
- Edit `docs/14_deviation_audit_history.md`:
  - Append a one-line record of the D-104 scope refinement.
- Edit `docs/15_config_keys_reference.md`:
  - Append `precise_outer_wall` and `seam_candidate_angle_threshold_deg` to the Walls/Seam section (matches classic's entries).

## Out of Scope

- Pipeline-wide config gaps (G8, G9, G14, G16, G23) — packet 149.
- The host service bridge (`slicer_core::arachne::pipeline::run_arachne_pipeline`) — already correct; no edits.
- The beading-strategy stack (`slicer_core::beading::*`) — already correct; no edits.
- Restoring the 10 removed intentional-design passing baselines — design is documented in the audit doc, not test-locked.
- The 4 pipeline-config red tests (`arachne_parity_pipeline_*`) — packet 149.
- Concentric-infill Arachne wiring (G23 / D-104f) — deferred; deviation registered as open in packet 149.

## Authoritative Docs

- `docs/02_ir_schemas.md` — 2221 lines, MUST be ranged or delegated. Relevant sections: §1418-1428 (WallBoundaryType), §1505-1516 (LoopType), §1520-1533 (WallFeatureFlags), §1542-1558 (Point3WithWidth.overhang_quartile).
- `docs/05_module_sdk.md` — 1348 lines, MUST be ranged. Relevant sections: §"SliceRegionView accessors (packet 104)" and §"SliceRegionView accessors (packet 107)".
- `docs/15_config_keys_reference.md` — size unknown, load only the "Walls" and "Seam" sections.
- `docs/DEVIATION_LOG.md` — direct read for the D-104 refinement target.
- `docs/14_deviation_audit_history.md` — direct read for the append format.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — perimeter_index assignment, wall ordering, is_bridge per-vertex (line 675-678), seam candidate emission (lines 2093-2535), precise_outer_wall path (lines 2146-2158).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.hpp` and `WallToolPaths.cpp` — `WallBoundaryType::ExteriorSurface`, `LoopType::ThinWall`, `generate_sharp_corner_seam_candidates`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:1484-1489` (`precise_outer_wall`), `:1491-1511` (`min_width_top_surface`, packet 149).

## Acceptance Summary

Reference Acceptance Criteria by ID. Do not copy them.

- Positive cases: AC-1 through AC-9. Measurable refinements:
- **AC-1/AC-2/AC-3/AC-4/AC-5/AC-6: the rewritten arachne-path tests drive `ArachnePerimeters::run_perimeters` natively and assert on `output.wall_loops()[0].boundary_type` / `.loop_type` / `.feature_flags[j]` / `.path.points[j].overhang_quartile` / `output.seam_candidates()`. The test predicate is the runnable fingerprint. The original substring-matching tests are deleted (or delegate to the new `arachne-perimeters/tests/*_tdd.rs` files).**
- AC-7: the manifest's `[config.schema]` section MUST contain both keys exactly (case-sensitive snake_case); the test grep-asserts the keys, not the descriptions.
- AC-8: a unit test in `arachne-perimeters/tests/precise_outer_wall_tdd.rs` sets up a region with `wall_sequence=InnerOuter` and `precise_outer_wall=true` and asserts the outer wall's distance-to-boundary via `output.wall_loops()[0].path.points` geometry. The inset is realized at the beading-stack level (`ArachneParams.outer_wall_offset`), not a post-hoc path mutation.
- AC-9: full file must show 10 passed (3 packet-1 stale-doc + 7 packet-148 arachne-path rewritten), 5 still red (4 packet-149 + 1 D-104f).
- Negative cases: AC-N1 (no manifest drift into packet 149's scope) and AC-N2 (precise_outer_wall is opt-in).
- Cross-packet impact: this packet closes the arachne-path half of D-104. Packet 149 closes D-104b/c/d/e (pipeline-config gaps) and registers D-104f (deferred).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test arachne_parity 2>&1 \| tee target/test-output.log` | AC-9: full file count | FACT pass/fail; `grep '^test result' target/test-output.log` |
| `cargo test -p arachne-perimeters --test arachne_parity_outer_wall_boundary_type_tdd 2>&1 \| tee target/test-output.log` | AC-1 | FACT pass/fail |
| `cargo test -p arachne-perimeters --test arachne_parity_thin_wall_loop_type_tdd 2>&1 \| tee target/test-output.log` | AC-2 | FACT pass/fail |
| `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd 2>&1 \| tee target/test-output.log` | AC-3 | FACT pass/fail |
| `cargo test -p arachne-perimeters --test arachne_parity_is_bridge_flag_tdd 2>&1 \| tee target/test-output.log` | AC-4 | FACT pass/fail |
| `cargo test -p arachne-perimeters --test arachne_parity_overhang_quartile_tdd 2>&1 \| tee target/test-output.log` | AC-5 | FACT pass/fail |
| `cargo test -p arachne-perimeters --test arachne_parity_seam_candidate_tdd 2>&1 \| tee target/test-output.log` | AC-6 | FACT pass/fail |
| `cargo test -p arachne-perimeters --test arachne_parity_precise_outer_wall_manifest_tdd 2>&1 \| tee target/test-output.log` | AC-7 | FACT pass/fail |
| `cargo test -p arachne-perimeters --test precise_outer_wall_tdd 2>&1 \| tee target/test-output.log` | AC-8 + AC-N2 | FACT pass/fail |
| `cargo build -p arachne-perimeters --target wasm32-unknown-unknown 2>&1 \| tee target/wasm-build.log` | gate (slicer-core dep with default-features=false must compile to wasm) | FACT exit 0 |
| `rg -q 'config\.schema\.(detect_overhang_wall\|overhang_reverse\|overhang_reverse_internal_only\|min_width_top_surface\|alternate_extra_wall\|bridge_flow\|thick_bridges)' modules/core-modules/arachne-perimeters/arachne-perimeters.toml; [ $? -ne 0 ]` | AC-N1 | FACT exit 0 = pass |
| `cargo clippy -p slicer-runtime --test arachne_parity -- -D warnings 2>&1 \| tee target/clippy-output.log` | gate | FACT exit 0 |
| `cargo xtask build-guests --check 2>&1 \| tee target/guest-check.log` | gate (arachne module is a guest; manifest change rebuilds) | FACT STALE/Fresh |

All verification commands are delegation-friendly (each returns a single fact: pass/fail or grep hit/miss).

## Step Completion Expectations

Cross-step invariants that the per-step blocks in `implementation-plan.md` cannot express:

- No step may regress the 3 packet-1 red tests (AC-1/AC-2/AC-3 stale-doc) by editing the manifest description fields.
- No step may introduce a manifest key whose name collides with a packet-149 key (negative case AC-N1).
- The manifest's `[config.schema.precise_outer_wall].default` MUST match classic's (`false`); the manifest's `[config.schema.seam_candidate_angle_threshold_deg].default` MUST match classic's (`30.0`); the seam key's range MUST match classic's (`0.0..=180.0`). These are exact-string invariants.
- The seam-candidate emission code in `run_perimeters` MUST call `generate_sharp_corner_seam_candidates` exactly once per outer region (not per line), and the resulting `Vec<SeamCandidate>` is bridged to `output.push_seam_candidate(pos, score)` via the SDK's `SeamCandidate` (the `position: Point3WithWidth` shape). The call site MUST be `&region.polygons()[0].contour` (units-space `Polygon`, the input region contour), NOT `&wall.path` (mm-space `ExtrusionPath3D`) — the helper's signature requires a `&slicer_ir::Polygon` and the input contour is the canonical seam-candidate source (mirrors classic's `lib.rs:889-900`).
- `region.bridge_areas()` is called per-region, not per-line, and the result is reused across all `ExtrusionLine`s for that region (one allocation per region, not per line).
- The `is_thin_wall` flag is set ONLY on `LoopType::ThinWall` walls (not on `Outer`/`Inner` even when those walls happen to be narrow); the `is_bridge` flag is set ONLY on `Outer`/`Inner` walls whose path points fall inside `region.bridge_areas()` (NOT on `ThinWall` or `GapFill` — the bridge classifier has no thin-wall special case in OrcaSlicer).

## Context Discipline Notes

- `OrcaSlicerDocumented/` is forbidden to load directly; every parity check is a sub-agent dispatch with `LOCATIONS` or `SUMMARY` return format.
- The arachne module's `lib.rs` is 353 lines; the implementer should range-read by section (lines 1-100 for imports/struct, 100-200 for `arachne_params_from_config`, 200-220 for `classify_line`, 230-353 for `run_perimeters`) rather than loading the whole file.
- `docs/02_ir_schemas.md` is 2221 lines; do NOT load it. Use `rg` for field shapes.
- The largest single step is the seam-candidate emission (Step 5) — it imports a new helper, traverses the outer wall's path, and bridges types. Cost M, never L.
