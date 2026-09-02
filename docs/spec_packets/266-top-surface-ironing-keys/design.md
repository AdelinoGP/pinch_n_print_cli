# Design: 266-top-surface-ironing-keys

## Controlling Code Paths

- Primary module path: `TopSurfaceIroning::from_config` and `TopSurfaceIroning::run_infill` in `modules/core-modules/top-surface-ironing/src/lib.rs`. The existing `generate_zigzag_strokes_for_polygon` function owns row generation and `InfillOutputBuilder::push_ironing_path` owns output insertion.
- Surface data path: `SliceRegionView::top_shell_index`, `top_solid_fill`, `bottom_solid_fill`, and `internal_solid_fill` in `crates/slicer-sdk/src/views.rs`; these are already present and avoid an IR/WIT change.
- Test path: `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` already constructs module configs and region views and inspects `ironing_paths()`.
- Config paths: the top manifest is the pre-filtering input for the per-region `ConfigView` that `SliceRegionView::config()` exposes (filtered by `crates/slicer-scheduler/src/execution_plan.rs`); host-side bounds/enum enforcement is `ConfigBoundsIndex::from_modules` / `check` in `crates/slicer-scheduler/src/config_resolution.rs`.
- OrcaSlicer comparison: see `requirements.md` section `OrcaSlicer Reference Obligations`; do not repeat delegation rules here.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- All runtime config key strings remain snake_case. The four new keys are module-owned and must be declared before `ConfigView::from_declared` can expose them.
- No WIT, IR, or public schema-version change is needed. `SliceRegionView` already carries `top_solid_fill`, `bottom_solid_fill`, and `internal_solid_fill`, and its `config()` accessor already delivers the per-region `ConfigView` the base angle is read from.
- `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) and every CONFIG_BLOCK twin are **out of bounds**: not read, not edited, not asserted. Map Authoring rule 2 — the padding table is not parity evidence and is never a deliverable. Whatever these keys do in the CONFIG_BLOCK follows from their being live and is not this packet's business.
- The top and support `ironing_enabled` entries are independent declarations in separately filtered module views. P14 must not rewrite support-side behavior while P15 is responsible for `support_ironing`.

## Tier, Claims, and Carriers (map rules 1 and 4)

- **Tier: B.** The packet adds new logic inside an existing owner at the correct seam — `top-surface-ironing`, `[stage] id = "Layer::Infill"`, ordered after the fill modules by its `[compatibility].requires` edges. The decision points it builds (mode-driven surface selection, an angled scan frame, an inward inset) do not exist today, so it is not Tier A plumbing; it ships no new module, so it is not Tier C. The ticket's "3 Tier A + 1 Tier B" split is superseded; the ticket/map update is listed in the session report.
- **Claims: the module already holds `claim:ironing` (`[claims] holds = ["claim:ironing"]` in `modules/core-modules/top-surface-ironing/top-surface-ironing.toml`) and this packet neither adds nor removes a claim.** Rule 4's holder-per-value shape fires on *cross-module algorithm selection* — an Orca enum whose values are separate implementations resolved through `*_fill_holder` / `module_overrides`. `ironing_type`'s values are **scopes of one algorithm** (which surfaces to iron), not four algorithms, so the rule's own trigger test puts it in the `seam_position` / `support_style` category: one module branching internally over behaviour it implements itself, behind the single existing `claim:ironing`. The enum that *would* be holder work here is `ironing_pattern`, and it is explicitly out of this packet's scope for exactly that reason.
- **Which existing mechanism carries the new data:**
  - The four keys ride the module manifest `[config.schema]` into the per-region `ConfigView` that `SliceRegionView::config()` already exposes (pre-filtered per module by `crates/slicer-scheduler/src/execution_plan.rs`). No typed `ResolvedConfig` field.
  - The base angle rides the **same channel**: `infill_direction` is co-declared in the top manifest and read from `ConfigView`. It is already a typed host field (`slicer_ir::resolved_config`, CLI key `infill_direction`, default `45.0`) and is already declared in `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` with the same type/bounds, so co-declaration merges bounds with no host change.
  - Geometry rides `slicer_sdk::host::offset_polygons` for the inset and the module's own rotate-scan-rotate-back for the angle.
  - **No WIT change, no IR schema bump, no host `ResolvedConfig` field, no new module.** Verified this session: `SliceRegionView` carries no fill-direction field (its only angle is `bridge_orientation_deg`), and adding one would mean a new field on `slicer_ir::slice_ir::SlicedRegion` — inside `SliceIR`, hence a `CURRENT_SLICE_IR_SCHEMA_VERSION` bump — plus a WIT accessor on `resource slice-region-view`. The packet deliberately does **not** take that route.

## Recorded Divergences

`DIV-266-A` and `DIV-266-B` are design-local labels for this packet only. They are **not** `docs/DEVIATION_LOG.md` IDs and must not be greped for there; per ticket 02 a log row is filed only after the human has been asked and signed off.

- **DIV-266-A — withdrawn: the layer-index base turn.** The previous draft substituted a deterministic 90-degree turn on odd `layer_index` for canonical's base direction, and recorded exact parity as "map fog". That was an invention with no canonical counterpart: canonical `Layer::make_ironing` computes `base + ironing_angle`, and nothing in it alternates by layer. This revision reads the real base from `infill_direction` and drops the parity turn entirely. Recorded here so a reader of the older draft does not restore it.
- **DIV-266-B — the base is the shared config *input*, not the fill module's *computed* angle.** Ironing reads `infill_direction`; the fill module that actually filled the region reads the same key. Verified this session: `RectilinearInfill::from_config` stores it unmodified as `base_angle` and `run_infill` uses that one angle for sparse, top-solid, bottom-solid, and internal-solid fill — so the two agree exactly for every rectilinear-filled region, which is the case canonical's rule is about. `GyroidInfill::from_config` adds a module-private `CORRECTION_ANGLE_DEG`, so ironing over a gyroid-filled region is off by that correction. Note the module's `[compatibility].requires` already orders it *after* the fill modules, so the fill has happened — what is missing is a channel to observe the angle it used, not the ordering. **Rationale for accepting it:** closing the gap requires the fill module's computed angle to reach `SliceRegionView`, which costs an IR schema bump plus a WIT accessor — a disproportionate contract change for a case (ironing a gyroid sparse region) canonical itself scopes to solid surfaces. The divergence is bounded, named, and cheap to close later; the alternative in the previous draft was to invent a base angle that matched nothing at all.

## Code Change Surface

- Selected approach: make canonical `ironing_type` the top module's sole enable/mode input; parse the three scalar controls; offset each selected polygon before scanning; rotate the scan frame around the polygon's local bounding-box center; and rotate points back before emitting. Fixed mode uses the configured absolute angle. Non-fixed mode uses `infill_direction + ironing_angle` — canonical `Layer::make_ironing`'s base-plus-offset shape — with `infill_direction` read from the module's own `ConfigView` after being co-declared in the manifest.
- Mode mapping: `no ironing` selects no regions; `topmost` preserves the current `top_shell_index() == Some(0)` behavior; `top` selects all regions with `top_shell_index().is_some()`; `solid` selects top, bottom, and internal solid-fill collections, while never selecting bridge-only regions.
- Inset mapping: `ironing_inset == 0.0` becomes `IRONING_LINE_WIDTH / 2.0`; a positive value is an inward millimetre offset through `slicer_sdk::host::offset_polygons` and an existing `OffsetJoinType`. Empty offsets emit no path. The generator must keep the current clipping and alternating row order after transformation.
- Exact functions, traits, manifests, tests, and fixtures:
  - `TopSurfaceIroning::from_config`, `TopSurfaceIroning::run_infill`, and `generate_zigzag_strokes_for_polygon` in `modules/core-modules/top-surface-ironing/src/lib.rs`.
  - `[config.schema]` in `modules/core-modules/top-surface-ironing/top-surface-ironing.toml`.
  - `top_surface_ironing_config_schema_tdd.rs` and the existing `top_surface_ironing_emission_tdd.rs` test binary.
  - Top-owned runtime contract/executor/e2e fixtures named in `requirements.md`.
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` bounds arms.
  - Generated `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`.
- Rejected alternatives and reasons:
  - Add the four keys to `support-surface-ironing`: rejected by the canonical consumer read. Support ironing uses `support_ironing`; adding P14 keys to that manifest would make unrelated keys visible and would contradict P15's independent gate.
  - Add `ResolvedConfig` or WIT fields for these keys: rejected because manifest-declared values already reach the module through `ConfigView`, and the needed layer index/fill collections already exist at the module boundary.
  - Keep `ironing_enabled` as a top-side fallback: rejected by the map's standardize-to-Orca decision; it would leave the narrowed bool as a second top control and make `ironing_type` non-authoritative.
  - A layer-index parity turn as the angle base (the previous draft's approach): rejected — it matches no canonical behaviour and made `ironing_angle` absolute rather than relative. Withdrawn as `DIV-266-A`.
  - Carrying the fill module's computed angle through `SliceRegionView`: rejected for this packet — it needs a new field on `slicer_ir::slice_ir::SlicedRegion` (an IR schema bump) and a WIT accessor on `resource slice-region-view`, neither of which this packet is authorized to add. The residual difference is bounded and recorded as `DIV-266-B`.

## Files in Scope (read + edit)

- `modules/core-modules/top-surface-ironing/top-surface-ironing.toml` - owner manifest; replace the top bool gate, add four exact canonical tables, and co-declare `infill_direction` byte-identically to `rectilinear-infill`'s table.
- `modules/core-modules/top-surface-ironing/Cargo.toml` - add the TOML parser dev-dependency if absent for the schema guard.
- `modules/core-modules/top-surface-ironing/src/lib.rs` - parse and apply mode, angle, fixed-angle, and inset values.
- `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_config_schema_tdd.rs` - new manifest guard; auto-discovered by Cargo.
- `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` - mode/geometry/legacy-key invariants and fixture updates.
- `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs` - top config key migration.
- `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs` - top config key migration and comment.
- `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs` - top config JSON and comments.
- `resources/test_config/benchy_combined_feature_evidence.json` - top-owned fixture key migration.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - real top-manifest enum/bounds arms.
- `docs/15_config_keys_reference.md` - generated output only; changed through `cargo xtask gen-config-docs`.

## Read-Only Context

- `modules/core-modules/top-surface-ironing/src/lib.rs` - config parser, `IRONING_LINE_WIDTH`, `generate_zigzag_strokes_for_polygon`, `run_infill`, and public getters only.
- `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` - existing `config_with`, `region_with`, square/L/U fixtures, and path assertions only.
- `crates/slicer-sdk/src/views.rs` - `SliceRegionView` fill accessors and setters only.
- `crates/slicer-sdk/src/host.rs` - `offset_polygons` and `OffsetJoinType` signatures only.
- `crates/slicer-scheduler/src/manifest.rs` (`read_config_schema`) and `crates/slicer-scheduler/src/config_resolution.rs` (`ConfigBoundsIndex::from_modules`, `check`) - bounds/enum machinery only. The error enum itself, `ConfigResolutionError` with its `TypeMismatch` / `OutOfRange` variants, lives in `crates/slicer-ir/src/resolved_config.rs`; the scheduler raises it.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, and `docs/08_coordinate_system.md` - targeted ranges or delegated summaries only.
- `OrcaSlicerDocumented/...` - delegated canonical inspection only.

## Out-of-Bounds Files

- `modules/core-modules/support-surface-ironing/**` - P15's independent support gate.
- `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-schema/wit/**`, and WIT bindings - no host or boundary field is needed.
- `crates/slicer-gcode/src/serialize.rs` - fully out of bounds (`ORCA_CONFIG_PADDING` and CONFIG_BLOCK twins; map rule 2).
- `docs/config/host-keys.toml` and `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` - module-owned keys, no host rows.
- `docs/ORCA_CONFIG_REFERENCE.md` - hand-maintained source snapshot, untouched.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, and unrelated crates - never load directly.

## Expected Sub-Agent Dispatches

- Question: confirm the top module's public entry shape and the exact `SliceRegionView` accessors used by each `ironing_type` mode; scope: `modules/core-modules/top-surface-ironing/src/lib.rs` and `crates/slicer-sdk/src/views.rs`; return: `LOCATIONS`; purpose: Step 2.
- Question: confirm the offset helper's mm signature and available join enum; scope: `crates/slicer-sdk/src/host.rs`; return: `LOCATIONS`; purpose: Step 2.
- Question: verify the scheduler integration binary loads the real top-surface-ironing manifest and quote the existing `TypeMismatch`/`OutOfRange` assertion shape; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` and `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: Step 4.
- Question: confirm the exact `[config.schema.infill_direction]` table in `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` (type, default, min, max, group) so the top manifest's co-declaration is byte-identical, and confirm how `RectilinearInfill::from_config` reads it; scope: that manifest and `modules/core-modules/rectilinear-infill/src/lib.rs`; return: `SNIPPETS` (1, <=15 lines); purpose: Step 1 and Step 2.
- Question: verify canonical P14 declarations and `Layer::make_ironing` mode/angle/inset behavior; scope: sibling `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented\src\libslic3r\PrintConfig.cpp`, `PrintConfig.hpp`, `Fill\Fill.cpp`, and `tests\fff_print\test_fill.cpp`; return: `LOCATIONS`; purpose: parity evidence.

## Data and Contract Notes

- IR/manifest contracts: manifest `enum`, `float`, and `bool` tables are host-enforced; `ConfigView::from_declared` drops undeclared keys, so the top manifest replacement is required for reachability.
- WIT boundary: none. The module already receives all selected fill collections and the layer index through the existing `Layer::Infill` entry.
- Geometry: `ironing_inset` and spacing are millimetres at the config/helper boundary; `ExPolygon` vertices remain scaled integer geometry. Use `slicer_sdk::host::offset_polygons` rather than manually scaling a linear constant.
- Determinism: rotate around each polygon's bounding-box center, preserve row ordering, use a stable layer-index parity rule, and avoid unordered iteration when appending selected fill collections.
- Base angle: `infill_direction` reaches the module only because the manifest declares it — `ConfigView::from_declared` drops undeclared keys, and `crates/slicer-scheduler/src/execution_plan.rs` pre-filters the per-region view to the module's declared schema. The co-declared table must match `rectilinear-infill`'s byte for byte so merged bounds cannot disagree.

## Locked Assumptions and Invariants

- `ironing_type = "no ironing"` is the canonical default and is the only default-off gate for the top module.
- `topmost` remains byte-equivalent to the current selection on layer 0: only `top_shell_index() == Some(0)` is eligible.
- `top` includes deeper top-shell regions but not bottom-only regions.
- `solid` consumes only the `top_solid_fill`, `bottom_solid_fill`, and `internal_solid_fill` collections exposed by `SliceRegionView`; bridge areas are not selected.
- `ironing_inset = 0.0` resolves to `IRONING_LINE_WIDTH / 2.0`; explicit positive values are inward offsets in mm.
- No top module code path reads `ironing_enabled` after this packet; support-side reads remain untouched.
- No WIT/IR/schema-version change occurs, so there is no struct-literal blast radius. The module's manifest `min-ir-schema` / `max-ir-schema` window is unchanged for the same reason.
- `ironing_angle_fixed = false` means `infill_direction + ironing_angle`; `true` means `ironing_angle` absolutely. Nothing in the module derives an angle from `layer_index`.

## Risks and Tradeoffs

- The base angle is the shared `infill_direction` input, not the fill module's computed angle (`DIV-266-B`). Exact for rectilinear-filled regions, off by `CORRECTION_ANGLE_DEG` for gyroid-filled ones. Bounded, named, and tested for what it does claim; the packet does not claim byte parity with canonical on a gyroid region.
- Inset offsetting can eliminate very small polygons. Empty offset results must be skipped, matching the existing empty/degenerate polygon behavior rather than emitting invalid paths.
- Removing the top `ironing_enabled` declaration makes stale top-side configs default to `no ironing`; the migration is intentional under the standardize-to-Orca ruling. Support-side configs are not migrated here.
- Manifest edits and module source edits feed guest WASM artifacts; stale artifacts can make otherwise correct module tests fail until the required guest check/rebuild runs.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, the geometry and mode path plus invariant tests)
- Highest-risk dispatch and required return format: canonical angle/inset/mode evidence, `LOCATIONS`; the PnP orientation-data availability check, `FACT`.

## Open Questions

- `[FWD]` When packet `262a-infill-angle-and-multiline-keys` lands `solid_infill_direction` and the rotate-template keys (verified this session: those three names have zero occurrences in `.rs`/`.toml`/`.wit` today and exist only in that draft packet), this module's base angle should follow whichever key that packet makes authoritative for solid fill. No activation blocker — the change is a one-key swap in the same read.
- `[FWD]` Closing `DIV-266-B` properly means exposing the fill module's computed angle on `SliceRegionView`, which is an IR schema bump plus a WIT accessor. Named here so a future packet does not re-derive the cost.

**No `[BLOCK]`.** The packet needs no new WIT interface, no IR schema bump, and no host `ResolvedConfig` field: the base angle rides an existing config key through an existing per-region `ConfigView`.
