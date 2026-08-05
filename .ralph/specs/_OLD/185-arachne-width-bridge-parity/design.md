# Design: 185-arachne-width-bridge-parity

## Controlling Code Paths

- Primary code path: new shared resolver `resolve_role_width` in `crates/slicer-core/src/flow.rs` (alongside `line_width_to_spacing`, `flow_to_width`, `bridging_flow`), consumed by `modules/core-modules/{classic-perimeters,arachne-perimeters,rectilinear-infill,gyroid-infill,lightning-infill}/src/lib.rs`; percent transport through `parse_config_field_entry` / `read_config_schema` (`crates/slicer-scheduler/src/manifest.rs`, `read_config_schema` at :1036) and `crates/slicer-scheduler/src/config_resolution.rs` into `ResolvedConfig.extensions` (`crates/slicer-ir/src/resolved_config.rs`, `extensions` field at :650); part-level width metadata preserved by `object_metadata_to_config_data` (`crates/slicer-model-io/src/loader.rs`).
- Neighboring tests/fixtures: `crates/slicer-core/tests/flow_tdd.rs`; `modules/core-modules/arachne-perimeters/tests/only_one_wall_top_tdd.rs`; scheduler config-resolution tests; perimeter-parity golden fixtures under `crates/slicer-runtime/tests/fixtures/perimeter_parity/`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Canonical precedence order (locked): configured `bridge_line_width`; else positive `initial_layer_line_width` on the first layer; else the role width; a zero role width falls back to `line_width`, then auto — `0` is the auto sentinel (canonical `Flow.cpp::auto_extrusion_width`, `1.125 × nozzle_diameter`). Geometric widths/spacing only; flow-ratio controls are deferred to `DEV-102`.
- Config keys belong to modules: each module manifest declares the flow keys it consumes; there is NO central schema mechanism. All keys snake_case. Canonical key set: `line_width`, `initial_layer_line_width`, `sparse_infill_line_width`, `internal_solid_infill_line_width`, `top_surface_line_width`, `bridge_line_width`.
- Defaults move to auto (`0`), including global `line_width`; an explicit `0.4` stays explicit. The scheduler accepts the schema-aware legacy alias `first_layer_line_width` and REJECTS any profile specifying both alias and canonical key.
- `BottomSolidInfill` uses `internal_solid_infill_line_width` except first-layer/bridge overrides; no bottom-specific width key is introduced.
- Classic owns the canonical overlap keys in its manifest: `infill_wall_overlap` is percent default `15`, `top_bottom_infill_wall_overlap` is percent default `25`, and both use `inner_wall_line_width` as `ratio_over`; the module selects the absolute value through `ConfigView::get_abs_value`.
- `D-152-TOP-AREA-SOURCE` stays OPEN: do not design `upper_slices` IR/WIT access in this packet.
- ADR-0043 amendment is explicit: its `Decision` item 2 says Arachne wall-width keys are “plain mm floats, default 0.4, range [0.1, 2.0]”. This packet changes that contract for canonical parity, adds `Arachne wall-width ADR amendment` to `docs/DEVIATION_LOG.md`, and appends an amendment record to the ADR file.
- ADR-0014 amendment is explicit: its freshness list says “`slicer-core` and `slicer-helpers` are explicitly NOT tracked”. The current guest freshness contract tracks `slicer-core` as a universal guest dependency, so this packet adds `guest-freshness ADR amendment` to `docs/DEVIATION_LOG.md` and appends an amendment record to the ADR file.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach:
  1. Scheduler retains parsed percent values: `parse_config_field_entry`'s call sites (`crates/slicer-scheduler/src/manifest.rs`) currently invoke `parse_percent_default` as a bare validation statement and discard the returned `ConfigValue`; thread the retained `ConfigValue::Percent` / `ConfigValue::FloatOrPercent` (`crates/slicer-ir/src/slice_ir.rs`, `ConfigValue` at :691) through `config_resolution.rs` into `ResolvedConfig.extensions` (`resolved_config.rs:650`). `ResolvedConfig::to_config_map`'s extensions pass-through is already transparent — do not touch it (per the wall-width/percent-transport residual).
  2. Rename the `ResolvedConfig` field declared at `resolved_config.rs:829` (`cli "first_layer_line_width" first_layer_line_width: f32 = 0.4`) to `initial_layer_line_width` inside the `declare_resolved_config!` invocation; update `ResolvedConfig::to_config_map`, `PartialEq`, `Hash`, and the region-mapping accessor/assignment sites; scheduler accepts the legacy CLI key as a schema-aware alias and rejects profiles specifying both keys.
  3. New `resolve_role_width` in `crates/slicer-core/src/flow.rs`, keyed by explicit canonical role + first-layer/bridge context, implementing the precedence chain above. Existing `line_width_to_spacing` (:86), `flow_to_width`, `bridging_flow` are unchanged.
  4. Classic: port canonical `PerimeterGenerator.cpp::process_classic`'s final-infill-boundary formula, replacing the raw `-inner_wall_line_width` offset in `ClassicPerimeters::emit_walls` (`classic-perimeters/src/lib.rs:1104`; closes the `flow-spacing integration gap` surviving residual). Add module-local `only_one_wall_top` behavior: topmost (`top_shell_index == Some(0)`, `crates/slicer-sdk/src/views.rs:41`) unconditional one wall; threshold on non-topmost top sub-areas, modeled on `emit_only_one_wall_top_second_pass` (`arachne-perimeters/src/lib.rs:923`). Overlap keys: `top_bottom_infill_wall_overlap` for layer 0 and `top_shell_index == Some(0)`; `infill_wall_overlap` otherwise.
  5. Arachne + the three infill modules swap their raw `line_width` reads (`rectilinear-infill/src/lib.rs:80`, `gyroid-infill/src/lib.rs:125`, `lightning-infill/src/lib.rs:66`) for `resolve_role_width`; arachne wall-width keys align to the same resolver path (shared wall-width/percent-transport residual).
  6. Each of `classic-perimeters.toml`, `arachne-perimeters.toml`, `rectilinear-infill.toml`, `gyroid-infill.toml`, and `lightning-infill.toml` declares only the flow keys its corresponding module consumes.
  7. `object_metadata_to_config_data` (`crates/slicer-model-io/src/loader.rs`) adds the three width keys required by TASK-212b to the part-level allowlist; the existing `parses_cube_cilindrical_modifier_sidecar` regression asserts the keys survive in `config_delta.fields`.
- Exact functions, traits, manifests, tests, and fixtures: as enumerated per step in `implementation-plan.md`; parameterized precedence-matrix tests (every role × first-layer × bridge override/fallback × percent transport × module parity × both top-overlap contexts) land BEFORE any golden re-bless.
- Rejected alternatives and reasons:
  - Central config-schema registry — rejected; keys are module-owned (decision 9).
  - Flow-ratio controls (`bridge_flow_ratio` plumbing through the resolver) — deferred, recorded as `DEV-102`.
  - Modifying `top_surface_split.rs::split_top_surfaces` — rejected; the classic one-wall-top behavior is module-local, the generic splitter stays untouched.
  - `upper_slices`/`lower_slices` IR or WIT access — rejected; `D-152-TOP-AREA-SOURCE` stays open.
  - Editing `ResolvedConfig::to_config_map` — rejected; the wall-width/percent-transport residual verified it is not the barrier.

## Files in Scope (read + edit)

More than 3 primary files by construction — this packet spans scheduler + IR + slicer-core + five modules; each implementation-plan step caps edits at 3 files.

- `crates/slicer-scheduler/src/manifest.rs` - role: percent parse retention; expected change: `parse_config_field_entry` call sites retain and return the parsed `ConfigValue`
- `crates/slicer-scheduler/src/config_resolution.rs` - role: percent transport + legacy alias; expected change: thread retained values into `extensions`; alias `first_layer_line_width` with both-keys rejection
- `crates/slicer-ir/src/resolved_config.rs` - role: field rename; expected change: one `declare_resolved_config!` line (:829) plus blast-radius struct-literal sites
- `crates/slicer-core/src/algos/region_mapping.rs` - role: field rename blast radius; expected change: region overlay comparison and assignment use `initial_layer_line_width`
- `crates/slicer-core/src/flow.rs` - role: resolver home; expected change: add `resolve_role_width`
- `modules/core-modules/classic-perimeters/src/lib.rs` + `classic-perimeters.toml` - role: resolver consumer, flow-spacing integration gap residual formula, overlap + one-wall-top; expected change: largest module diff
- `modules/core-modules/arachne-perimeters/src/lib.rs` + `arachne-perimeters.toml` - role: resolver consumer; expected change: width resolution via shared fn
- `modules/core-modules/{rectilinear,gyroid,lightning}-infill/src/lib.rs` + manifests - role: resolver consumers; expected change: replace raw `line_width` reads, declare consumed keys
- `crates/slicer-model-io/src/loader.rs` - role: TASK-212b part-level metadata allowlist; expected change: preserve `inner_wall_line_width`, `outer_wall_line_width`, and `sparse_infill_line_width`
- `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` - role: allowlist regression; expected change: extend `parses_cube_cilindrical_modifier_sidecar`
- `docs/DEVIATION_LOG.md` - role: deviation ledger; expected change: add the next free DEV row for deferred flow-ratio controls and retain current width residual evidence; the wall-width and freshness amendments are recorded in their ADR files
- `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` - role: append-only amendment record for `Arachne wall-width ADR amendment`; expected change: preserve the original Decision item 2 and append the approved type/default amendment
- `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` - role: append-only amendment record for `guest-freshness ADR amendment`; expected change: preserve the original freshness rule and append the current `slicer-core` tracking decision

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` - lines `[686-720]` only - purpose: `ConfigValue::Percent` / `FloatOrPercent{value,is_percent}` shape (:691)
- `crates/slicer-ir/src/resolved_config.rs` - lines `[630-670]` only - purpose: `extensions` BTreeMap declaration (:650) and macro emit site
- `crates/slicer-sdk/src/views.rs` - lines `[30-55]` only - purpose: `top_shell_index` / `bottom_shell_index` (:41/:43) and `is_bridge` semantics
- `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` - lines `[235-305]` only - purpose: existing modifier-part fixture and `parses_cube_cilindrical_modifier_sidecar` assertions
- `modules/core-modules/arachne-perimeters/src/lib.rs` - lines `[900-960]` only - purpose: `emit_only_one_wall_top_second_pass` (:923) as the classic one-wall-top model
- `modules/core-modules/classic-perimeters/src/lib.rs` - lines `[570-620]` and `[1095-1115]` only - purpose: `line_width_for` (:591) and the raw `-inner_wall_line_width` infill inset (:1104)
- `docs/DEVIATION_LOG.md` - current width residuals, `D-152-TOP-AREA-SOURCE`, and `DEV-102` only - purpose: residual wording to close/amend
- `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` - lines `[29-44]` only - purpose: quote the normative wall-width clause for the required deviation amendment
- `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` - lines `[17-35]` only - purpose: quote the stale freshness exclusion for the required deviation amendment
- `docs/07_implementation_status.md` - lines `[150-175]` only - purpose: `TASK-303` (:173), `TASK-212b` (:154) rows

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-core/src/top_surface_split.rs` - read-only context only; `split_top_surfaces` must not be edited
- Any WIT file under `crates/slicer-schema/wit/` - no WIT change in this packet; `upper_slices` access is explicitly out of scope (`D-152-TOP-AREA-SOURCE` open)
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: enumerate every struct-literal and test site compiling against `ResolvedConfig.first_layer_line_width` (pattern `first_layer_line_width`); scope: `crates/** modules/**`; return: `LOCATIONS`; purpose: Step 2 blast radius (pre-bake into 'Files allowed to edit')
- Question: quote canonical `PerimeterGenerator.cpp::process_classic`'s final infill-boundary inset formula (spacing vs raw width) and the `only_one_wall_top` topmost/non-topmost branches; scope: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`; return: `SNIPPETS`; purpose: Steps 4 and 9
- Question: does any WIT record or host dispatch path carry the key string `first_layer_line_width`?; scope: `crates/slicer-schema/wit/** crates/slicer-runtime/src/**`; return: `FACT`; purpose: Step 2 contract safety
- Question: which golden fixtures assert classic/arachne wall or infill geometry sensitive to width resolution?; scope: `crates/slicer-runtime/tests/fixtures/perimeter_parity/** modules/core-modules/**/tests/**`; return: `SUMMARY`; purpose: Step 11/12 re-bless scope
- Question: which part-level width keys does `object_metadata_to_config_data` currently drop and which existing fixture asserts their preservation?; scope: `crates/slicer-model-io/src/loader.rs crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`; return: `LOCATIONS`; purpose: TASK-212b allowlist step

## Data and Contract Notes

- IR/manifest contracts: the rename is a public-struct field rename on `ResolvedConfig` (serde-derived). `to_config_map`, `PartialEq`, `Hash`, and region mapping must use the canonical field; the literal legacy key is retained only in scheduler alias handling and its negative test. Per `TASK-302`'s finding, `ResolvedConfig`'s serde shape is constructed in-process and marshalled over WIT, never persisted — the rename is not a wire break and requires no `CURRENT_*_SCHEMA_VERSION` bump. The CLI/config-map key changes (`first_layer_line_width` → `initial_layer_line_width`); the legacy alias keeps old profiles loading, and both-keys profiles are hard errors.
- WIT boundary: unchanged. Percent values ride the existing `float-or-percent` WIT record shape that `ConfigValue::FloatOrPercent` already mirrors (`slice_ir.rs:706-714`) and the extensions map; no `.wit` edit, so guest bindgen is unaffected by the transport change (guest staleness still applies to `slicer-ir`/`slicer-core`/module source edits per the staleness bullet).
- Determinism/scheduler constraints: `resolve_role_width` is a pure function of (role, first-layer flag, bridge flag, resolved config values); no scheduling-order change. `extensions` is a `BTreeMap`, so percent-carrying iteration stays deterministic. Both-keys rejection is a validation error, not a warning, and fires at config resolution before any module dispatch.

## Locked Assumptions and Invariants

- Resolver math is in mm at the module boundary, matching `flow.rs`'s existing convention; all geometry handed to `polygon_ops`/offsetting converts via `mm_to_units()` (1 unit = 100 nm).
- All config keys snake_case; canonical key set fixed as listed in Architecture Constraints.
- Precedence chain and the `0` = auto sentinel (`1.125 × nozzle_diameter`, canonical `Flow.cpp::auto_extrusion_width`) are locked per Architecture Constraints.
- `BottomSolidInfill` role maps to `internal_solid_infill_line_width`; no bottom-specific key.
- Parameterized precedence-matrix tests land and pass BEFORE any golden re-bless; self-captured baselines that drift in the canonical-correct direction are re-blessed, never the reverse (per CLAUDE.md Test Discipline).
- `D-152-TOP-AREA-SOURCE` untouched; `crates/slicer-core/src/top_surface_split.rs::split_top_surfaces` untouched. TASK-212b is not considered complete until the loader allowlist regression passes.
- `Arachne wall-width ADR amendment` quotes the contested ADR clause and records why canonical `float_or_percent`/auto-`0` behavior is intentional; no silent ADR drift is permitted.
- `guest-freshness ADR amendment` quotes the stale `slicer-core` freshness exclusion and records why the current universal-guest dependency check is required; no stale-guest deflection is permitted.

## Risks and Tradeoffs

- The field rename's blast radius may exceed the surveyed sites — mitigated by the mandatory `LOCATIONS` dispatch plus `cargo check --workspace --all-targets` in Step 2's exit.
- Defaults moving to auto (`0`) is a user-visible output change on every profile that relied on implicit `0.4`; accepted per brief decision 4, absorbed by the matrix-tests-then-re-bless order.
- Classic one-wall-top and the flow-spacing integration gap infill-boundary formula will drift classic goldens; re-bless only after matrix tests pin the intended semantics.
- Percent retention grows `ResolvedConfig.extensions` contents on percent-typed manifests; harmless in-process (TASK-302) but asserted by a round-trip test to prevent silent re-coercion to `Float`.
- Editing `slicer-ir` + `slicer-core` + five module sources stales all guest WASMs; every module step gates on `cargo xtask build-guests --check`.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 rename + blast radius)
- Highest-risk dispatch and required return format: the `first_layer_line_width` struct-literal survey — `LOCATIONS`, consumed verbatim into Step 2's allowed-edits list before authoring.

## Open Questions

- [FWD] Exact set of golden fixtures requiring re-bless after classic/arachne width changes — resolved by the fixture-scope dispatch in Step 11; does not block activation (matrix tests gate the re-bless regardless).
- [FWD] Whether `resolve_role_width` takes `nozzle_diameter` as a parameter or reads it from a context struct — implementer-resolvable in Step 3; the auto-sentinel semantics are locked either way.
