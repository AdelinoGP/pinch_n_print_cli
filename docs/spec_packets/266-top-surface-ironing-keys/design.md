# Design: top-surface-ironing-keys

## Controlling Code Paths

- Primary module path: `TopSurfaceIroning::from_config` and `TopSurfaceIroning::run_infill` in `modules/core-modules/top-surface-ironing/src/lib.rs`. The existing `generate_zigzag_strokes_for_polygon` function owns row generation and `InfillOutputBuilder::push_ironing_path` owns output insertion.
- Surface data path: `SliceRegionView::top_shell_index`, `top_solid_fill`, `bottom_solid_fill`, and `internal_solid_fill` in `crates/slicer-sdk/src/views.rs`; these are already present and avoid an IR/WIT change.
- Test path: `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` already constructs module configs and region views and inspects `ironing_paths()`.
- Config paths: the top manifest is pre-filtering input for `ConfigView`; scheduler bounds use `ConfigBoundsIndex` and runtime CONFIG_BLOCK uses `serialize_config_block` with raw-key deduplication.
- OrcaSlicer comparison: see `requirements.md` section `OrcaSlicer Reference Obligations`; do not repeat delegation rules here.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- All runtime config key strings remain snake_case. The four new keys are module-owned and must be declared before `ConfigView::from_declared` can expose them.
- No WIT, IR, or public schema-version change is needed. `SliceRegionView` already carries the three solid-fill collections and `run_infill` already receives `layer_index`.
- CONFIG_BLOCK padding stays unchanged. The existing `("ironing_type", "no ironing")` entry is deduplicated against a user value; no padding twin is added for `ironing_angle`, `ironing_angle_fixed`, or `ironing_inset`.
- The top and support `ironing_enabled` entries are independent declarations in separately filtered module views. P14 must not rewrite support-side behavior while P15 is responsible for `support_ironing`.

## Code Change Surface

- Selected approach: make canonical `ironing_type` the top module's sole enable/mode input; parse the three scalar controls; offset each selected polygon before scanning; rotate the scan frame around the polygon's local bounding-box center; and rotate points back before emitting. Fixed mode uses the configured absolute angle. Non-fixed mode adds a deterministic 90-degree turn for odd `layer_index` values to represent the existing no-direction fallback without inventing an IR field.
- Mode mapping: `no ironing` selects no regions; `topmost` preserves the current `top_shell_index() == Some(0)` behavior; `top` selects all regions with `top_shell_index().is_some()`; `solid` selects top, bottom, and internal solid-fill collections, while never selecting bridge-only regions.
- Inset mapping: `ironing_inset == 0.0` becomes `IRONING_LINE_WIDTH / 2.0`; a positive value is an inward millimetre offset through `slicer_sdk::host::offset_polygons` and an existing `OffsetJoinType`. Empty offsets emit no path. The generator must keep the current clipping and alternating row order after transformation.
- Exact functions, traits, manifests, tests, and fixtures:
  - `TopSurfaceIroning::from_config`, `TopSurfaceIroning::run_infill`, and `generate_zigzag_strokes_for_polygon` in `modules/core-modules/top-surface-ironing/src/lib.rs`.
  - `[config.schema]` in `modules/core-modules/top-surface-ironing/top-surface-ironing.toml`.
  - `top_surface_ironing_config_schema_tdd.rs` and the existing `top_surface_ironing_emission_tdd.rs` test binary.
  - Top-owned runtime contract/executor/e2e fixtures named in `requirements.md`.
  - `config_bounds_enforcement_tdd.rs` and `gcode_header_thumbnail_config_blocks_tdd.rs` integration arms.
  - Generated `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`.
- Rejected alternatives and reasons:
  - Add the four keys to `support-surface-ironing`: rejected by the canonical consumer read. Support ironing uses `support_ironing`; adding P14 keys to that manifest would make unrelated keys visible and would contradict P15's independent gate.
  - Add `ResolvedConfig` or WIT fields for these keys: rejected because manifest-declared values already reach the module through `ConfigView`, and the needed layer index/fill collections already exist at the module boundary.
  - Keep `ironing_enabled` as a top-side fallback: rejected by the map's standardize-to-Orca decision; it would leave the narrowed bool as a second top control and make `ironing_type` non-authoritative.
  - Claim exact canonical relative-angle parity: rejected because PnP has no solid-infill direction in `SliceRegionView`; the deterministic layer-index fallback is explicit and the exact parity question remains map fog.

## Files in Scope (read + edit)

- `modules/core-modules/top-surface-ironing/top-surface-ironing.toml` - owner manifest; replace the top bool gate and add four exact canonical tables.
- `modules/core-modules/top-surface-ironing/Cargo.toml` - add the TOML parser dev-dependency if absent for the schema guard.
- `modules/core-modules/top-surface-ironing/src/lib.rs` - parse and apply mode, angle, fixed-angle, and inset values.
- `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_config_schema_tdd.rs` - new manifest guard; auto-discovered by Cargo.
- `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` - mode/geometry/legacy-key invariants and fixture updates.
- `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs` - top config key migration.
- `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs` - top config key migration and comment.
- `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs` - top config JSON and comments.
- `resources/test_config/benchy_combined_feature_evidence.json` - top-owned fixture key migration.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - real top-manifest enum/bounds arms.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - exact-once P14 raw-key arms.
- `docs/15_config_keys_reference.md` - generated output only; changed through `cargo xtask gen-config-docs`.

## Read-Only Context

- `modules/core-modules/top-surface-ironing/src/lib.rs` - config parser, `IRONING_LINE_WIDTH`, `generate_zigzag_strokes_for_polygon`, `run_infill`, and public getters only.
- `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` - existing `config_with`, `region_with`, square/L/U fixtures, and path assertions only.
- `crates/slicer-sdk/src/views.rs` - `SliceRegionView` fill accessors and setters only.
- `crates/slicer-sdk/src/host.rs` - `offset_polygons` and `OffsetJoinType` signatures only.
- `crates/slicer-scheduler/src/manifest.rs` and `config_resolution.rs` - enum/float bounds machinery only.
- `crates/slicer-gcode/src/serialize.rs` - `serialize_config_block`, `emit_config_kv`, and ironing padding entries only.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, and `docs/08_coordinate_system.md` - targeted ranges or delegated summaries only.
- `OrcaSlicerDocumented/...` - delegated canonical inspection only.

## Out-of-Bounds Files

- `modules/core-modules/support-surface-ironing/**` - P15's independent support gate.
- `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-schema/wit/**`, and WIT bindings - no host or boundary field is needed.
- `crates/slicer-gcode/src/serialize.rs` - read-only context; no padding edits.
- `docs/config/host-keys.toml` and `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` - module-owned keys, no host rows.
- `docs/ORCA_CONFIG_REFERENCE.md` - hand-maintained source snapshot, untouched.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, and unrelated crates - never load directly.

## Expected Sub-Agent Dispatches

- Question: confirm the top module's public entry shape and the exact `SliceRegionView` accessors used by each `ironing_type` mode; scope: `modules/core-modules/top-surface-ironing/src/lib.rs` and `crates/slicer-sdk/src/views.rs`; return: `LOCATIONS`; purpose: Step 2.
- Question: confirm the offset helper's mm signature and available join enum; scope: `crates/slicer-sdk/src/host.rs`; return: `LOCATIONS`; purpose: Step 2.
- Question: verify the scheduler integration binary loads the real top-surface-ironing manifest and quote the existing `TypeMismatch`/`OutOfRange` assertion shape; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` and `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: Step 4.
- Question: verify raw P14 values reach `serialize_config_block`, how the existing `ironing_type` padding is deduplicated, and which integration test function drives the block; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`, runtime raw-config construction, and `crates/slicer-gcode/src/serialize.rs` limited to named functions; return: `FACT`; purpose: Step 4.
- Question: verify canonical P14 declarations and `Layer::make_ironing` mode/angle/inset behavior; scope: sibling `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented\src\libslic3r\PrintConfig.cpp`, `PrintConfig.hpp`, `Fill\Fill.cpp`, and `tests\fff_print\test_fill.cpp`; return: `LOCATIONS`; purpose: parity evidence.

## Data and Contract Notes

- IR/manifest contracts: manifest `enum`, `float`, and `bool` tables are host-enforced; `ConfigView::from_declared` drops undeclared keys, so the top manifest replacement is required for reachability.
- WIT boundary: none. The module already receives all selected fill collections and the layer index through the existing `Layer::Infill` entry.
- Geometry: `ironing_inset` and spacing are millimetres at the config/helper boundary; `ExPolygon` vertices remain scaled integer geometry. Use `slicer_sdk::host::offset_polygons` rather than manually scaling a linear constant.
- Determinism: rotate around each polygon's bounding-box center, preserve row ordering, use a stable layer-index parity rule, and avoid unordered iteration when appending selected fill collections.
- CONFIG_BLOCK: raw user keys are serialized once through `emit_config_kv`; the existing `ironing_type` padding line remains the only ironing padding entry.

## Locked Assumptions and Invariants

- `ironing_type = "no ironing"` is the canonical default and is the only default-off gate for the top module.
- `topmost` remains byte-equivalent to the current selection on layer 0: only `top_shell_index() == Some(0)` is eligible.
- `top` includes deeper top-shell regions but not bottom-only regions.
- `solid` consumes only the `top_solid_fill`, `bottom_solid_fill`, and `internal_solid_fill` collections exposed by `SliceRegionView`; bridge areas are not selected.
- `ironing_inset = 0.0` resolves to `IRONING_LINE_WIDTH / 2.0`; explicit positive values are inward offsets in mm.
- No top module code path reads `ironing_enabled` after this packet; support-side reads remain untouched.
- No WIT/IR/schema-version change occurs, so there is no struct-literal blast radius.

## Risks and Tradeoffs

- The non-fixed angle fallback is deterministic but cannot consume canonical solid-infill direction/template metadata absent from PnP's region view. The packet exposes and tests this limitation rather than claiming exact canonical parity; future orientation metadata is map fog.
- Inset offsetting can eliminate very small polygons. Empty offset results must be skipped, matching the existing empty/degenerate polygon behavior rather than emitting invalid paths.
- Removing the top `ironing_enabled` declaration makes stale top-side configs default to `no ironing`; the migration is intentional under the standardize-to-Orca ruling. Support-side configs are not migrated here.
- Manifest edits and module source edits feed guest WASM artifacts; stale artifacts can make otherwise correct module tests fail until the required guest check/rebuild runs.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, the geometry and mode path plus invariant tests)
- Highest-risk dispatch and required return format: canonical angle/inset/mode evidence, `LOCATIONS`; the PnP orientation-data availability check, `FACT`.

## Open Questions

None. Exact solid-infill-template angle parity is deliberately recorded as future map fog, not left as an activation blocker.
