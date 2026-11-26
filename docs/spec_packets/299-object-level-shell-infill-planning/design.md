# Design: 299-object-level-shell-infill-planning

## Controlling Code Paths

- Primary code path: `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) gains the vertical-shell and extra-solid stages inside its build-then-commit flow, plus a combination stage running after `convert_small_sparse_islands`; `resolve_shell_counts`' `region_map.config_for` read pattern is the per-region config entry for the three new stages.
- Emission path: `RectilinearInfill::run_infill` (`modules/core-modules/rectilinear-infill/src/lib.rs`) reads the grouped height through the `SliceRegionView` accessor and applies it to the sparse role only (walls keep `effective_layer_height`).
- Neighboring tests/fixtures: `#[cfg(test)] mod tests` in `slice_postprocess_prepass.rs` (the `rect`/`slice_with` fixture helpers, lines 1137–1160); slicer-runtime `--test integration` fixtures; `dispatch_infill_output_tdd` contract arms.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Schema/version constants: the `SlicedRegion.combined_infill_height` addition bumps `CURRENT_SLICE_IR_SCHEMA_VERSION` minor at activation, computed from the live constant at edit time — never hardcoded (S3). The serde `#[serde(default)]` keeps deserialization of old fixtures working; the whole-struct `PartialEq` in `resolved_config.rs`'s test module covers the new fields automatically (ticket-126 drift guard).
- Prepass ordering: shell stages run before `convert_small_sparse_islands` (islands must count as solid support for the layer above); the combination stage runs after it and before the bridge gates (bridge areas are excluded from grouping). Both orderings already hold in `commit_shell_classification_builtin`'s sequence; the new stages slot into it, never around it.
- ADR-0062/0063 locked-path conformance: combination and shell growth subtract their footprint from untagged fill only; locked paths are neither clipped nor merged (AC-N2). The five-way partition's pairwise-disjoint invariant (`SlicedRegion.sparse_infill_area` doc, `crates/slicer-ir/src/slice_ir.rs`) must survive — combined regions move sparse area into the grouped layer and void the lower ones inside the existing partition hook (`sync_perimeter_infill_areas_into_slice`, `crates/slicer-runtime/src/region_partition.rs`), not by a second subtraction.
- Rule 4 does not fire: `ensure_vertical_shell_thickness`'s four modes branch inside one prepass stage; `infill_combination` is a single grouping stage plus one emitter arm — no cross-module algorithm selection, no claim holders (`seam_position` precedent, map Notes Q8).
- `ConfigView::from_declared` whitelist is not engaged: these four keys are host-prepass inputs read through `region_map.config_for` (the `resolve_shell_counts` pattern), so no module manifest must declare them for the host stages to read; the rectilinear manifest declares nothing new.
- Bounds: no numeric range rejection anywhere (ticket-113 rule — canonical min/max are GUI hints); the only rejection is strict-parse of the mode enum string (AC-N1), which is `TypeMismatch`, not a range error.

## Code Change Surface

- Selected approach: four `declare_resolved_config!` rows (canonical defaults) → three new prepass stages in `slice_postprocess_prepass.rs` (vertical-shell projection behind the mode gate; `check_layer_id_pattern` port for extra-solid insertion; sparse grouping behind the bool + percent cap) → one IR field (`SlicedRegion.combined_infill_height: Option<f32>`) + `SliceRegionView::combined_infill_height()` accessor (+ its WIT `combined-infill-height: func() -> option<f32>` line on `slice-region-view`) → one emitter arm in `run_infill` consuming the grouped height on the sparse role. The mode gate is `ensure_all`-active-by-default (one intended default output change, AC-2) with `none` pinning the pre-packet baseline.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs` — 4 macro rows (`ensure_vertical_shell_thickness` String via `extract_string`, `extra_solid_infills` String via `extract_string`, `infill_combination` bool via `extract_bool`, `infill_combination_max_layer_height` `ResolvedFloatOrPercent` via `extract_float_or_percent`).
  - `crates/slicer-ir/src/slice_ir.rs` — `SlicedRegion.combined_infill_height` field (serde-defaulted).
  - `crates/slicer-schema/wit/deps/ir-types.wit` — `combined-infill-height` accessor on `slice-region-view`.
  - `crates/slicer-sdk/src/views.rs` — `combined_infill_height` on `SliceRegionView` + getter.
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — `resolve_vertical_shell_mode` + `discover_vertical_solid_shells` + `insert_extra_solid_layers` (incl. the `check_layer_id_pattern` port) + `combine_sparse_infill` + tests.
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — grouped-height arm in `run_infill`'s sparse role (+ test).
  - `docs/DEVIATION_LOG.md` — DEV-190; `docs/15_config_keys_reference.md` — regen via `cargo xtask gen-config-docs`.
- Rejected alternatives and reasons: (a) declare the keys in infill-module manifests and read them module-side — rejected because the decision points are cross-layer (a layer's shell state depends on neighbours), which the per-layer module dispatch cannot see; the prepass is the architecture's cross-layer seam (rule 4: new decision points go where the architecture puts them). (b) Fold into draft packet 264 (top/bottom surfaces) — rejected: 264's keys are per-surface density/pattern; these four are cross-layer planning, and 264's preflight is closed (map rule 7, 264 not in the re-author list). (c) Emit a `combined_infill` marker rather than a height field — rejected: the emitters need the summed height to scale extrusion; a marker would still need it, and the IR doc for `internal_solid_fill` already warns against marker-vs-domain confusion (map Notes).

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - role: prepass host; the three new stages + tests land here; expected change: three stage functions + call sites in `commit_shell_classification_builtin` + unit tests.
- `crates/slicer-ir/src/resolved_config.rs` - role: single source of truth for the four defaults; expected change: four macro rows (+ doc comments; the macro emits `Default`, `apply_cli_key`, `to_config_map`, and overlay arms per ticket 126).
- `crates/slicer-ir/src/slice_ir.rs` - role: IR field; expected change: `combined_infill_height` on `SlicedRegion`.
- `crates/slicer-sdk/src/views.rs` - role: view accessor; expected change: field + getter mirroring `top_shell_index`.
- `crates/slicer-schema/wit/deps/ir-types.wit` - role: WIT boundary; expected change: one accessor line on `slice-region-view`.
- `modules/core-modules/rectilinear-infill/src/lib.rs` - role: sparse emitter arm; expected change: grouped-height consumption in `run_infill`.
- `docs/DEVIATION_LOG.md`, `docs/15_config_keys_reference.md` - role: DEV-190 + regen.
- Justification for >3 primary files: the four-way split (config → IR/WIT/view → prepass → emitter) is the architecture's own seam order; no two of these can merge without a cross-seam dependency.

## Read-Only Context

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `116-230` and `1028-1106` only - purpose: `commit_shell_classification_builtin` staging order, island conversion, `resolve_shell_counts`' `config_for` pattern.
- `crates/slicer-ir/src/resolved_config.rs` - lines `1231-1330` (macro shape) and `1975-2060` (neighbouring declaration rows) only - purpose: exact DSL row shape and the `@ {}` attribute-arms example (`flat_bridge_closing_join`).
- `modules/core-modules/rectilinear-infill/src/lib.rs` - lines `198-360` and `600-680` only - purpose: `run_infill` role emission + `solid_fill_role` boundary.
- `crates/slicer-sdk/src/views.rs` - lines `21-145` and `370-420` only - purpose: `SliceRegionView` field + from-region mapping + getter shapes.
- `crates/slicer-runtime/src/region_partition.rs` - lines `263-330` only - purpose: five-way partition precondition the combination stage must respect.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Unrelated crates - delegate symbol lookups; do not browse
- `crates/slicer-wasm-host/test-guests/**` - no test-guest edit needed; the accessor addition is additive and the guest regen rides `build-guests --check`
- Draft packets `262*/`, `264/`, `275/` - read Goal lines only via dispatch; never open design files

## Expected Sub-Agent Dispatches

- Question: exact `discover_vertical_shells` projection + regularization constants and `combine_infill` cap arithmetic; scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SUMMARY`; purpose: Step 3/4 formula borrow.
- Question: `check_layer_id_pattern` edge semantics (1-based, `N#K`, comma lists, negative/zero rejection); scope: `OrcaSlicerDocumented/src/libslic3r/utils.cpp`; return: `SUMMARY`; purpose: Step 3 parser port.
- Question: did `cargo check --workspace --all-targets` pass; scope: workspace; return: `FACT`; purpose: Step gates.
- Question: struct-literal blast radius for the new `SlicedRegion` field; scope: `crates/ modules/` non-`serde` struct-literal sites of `SlicedRegion`; return: `LOCATIONS`; purpose: Step 2 budget (the field is serde-defaulted and additive; expectation is zero non-fixture sites, verified not assumed).

## Data and Contract Notes

- IR/manifest contracts: `combined_infill_height` is `None` on ungrouped layers (including every layer of a default slice — AC-4's baseline). The five-way partition's pairwise-disjoint invariant holds by moving area, not re-clipping.
- WIT boundary: one additive accessor on `slice-region-view`; both host (`bindgen! path:`) and guest (`include_str!`) read the canonical file directly — no inline copy (map WIT checklist).
- Determinism/scheduler constraints: the vertical-shell and combination stages are sequential cross-layer passes inside the existing prepass (the timeline loop already runs sequentially per the `commit_shell_classification_builtin` comment); no parallel determinism change.

## Locked Assumptions and Invariants

- Canonical defaults are locked: `ensure_all` / `""` / `false` / `100%`-percent-true. The vertical-shell stage being active at default is an intended output change, pinned by AC-2, and recorded as a DEV-190 clause (canonical parity, not a deviation — the deviation clauses are the simplifications below).
- DEV-190 clauses (minted at authoring, re-derive `max(DEV-*)` before writing): (a) the vertical-shell port uses the port's `top_solid_fill`/`internal_solid_fill` partition vocabulary rather than canonical's `Surface` type zoo (`stInternal`/`stInternalVoid`/`stInternalSolid` tri-typing) — the port has no `stInternalVoid` analogue; void-marking for combined lower layers rides `sparse_infill_area` emptiness plus the new height field, not a new void IR domain; (b) the multi-material `interface_shells` gating of the vertical pass is not borrowed (P76 scope); (c) canonical's `Print.cpp` reslice-invalidation entries ride ticket 124.
- The port's uniform planner means `infill_combination_max_layer_height` groups over uniform layer heights only; canonical's variable-profile grouping arithmetic is not borrowed (ticket-75/144 envelope, named non-borrow).

## Risks and Tradeoffs

- Default-path change (vertical shells active at `ensure_all`) touches every existing slice's `internal_solid_fill` — the AC-2 fixture comparison plus the existing slicer-runtime integration suite pin the blast radius; any baseline churn lands as test-fixture updates with measured justification (map test discipline).
- The combination stage's void-marking depends on `sparse_infill_area` emptiness being a stable contract; the partition doc comment pins it — if implementation finds a partition consumer that re-adds sparse area, the stage must use an explicit marker instead (escalate, do not improvise).
- The `check_layer_id_pattern` port is a string parser — the 1-based/0-based off-by-one is the likeliest silent defect; AC-3 pins both the exact-match and `N#K` interval forms against a 5-layer fixture.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 — three stages in one file)
- Highest-risk dispatch and required return format: vertical-shell formula borrow — `SUMMARY` (≤200 words) from `PrintObject.cpp`; reject any reply pasting the 700-line function.

## Open Questions

None. `[FWD]` none pending; `[BLOCK]` none.