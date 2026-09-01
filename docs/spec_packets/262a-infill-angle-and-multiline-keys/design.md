# Design: infill-angle-and-multiline-keys

## Tier Derivation

**Tier B.** The packet builds decision points, so it cannot be Tier A under the map's Authoring rule 1. It builds them inside two existing modules — no new crate, no new claim, no new host mechanism — so it sits at the bottom of the B/C range. (Its sibling 262b is Tier C because it ships modules and a new pass.)

## Approach

Three small mechanisms, all inside the fill modules:

1. **Role-scoped base angle.** Today both modules derive one base angle from `infill_direction`. The change makes the angle a function of the *role being emitted*: sparse roles keep `infill_direction`; solid roles (`TopSolidInfill`, `BottomSolidInfill`, internal solid) read `solid_infill_direction`. This is a per-role read at the point the module already computes `angle_deg`, not a new plumbing path.
2. **Rotate template.** A tiny resolver, `base_angle → (template, layer_index) → effective angle`: an empty template yields the base angle; a comma-separated list of numbers is cycled by `layer_index % len`; anything else logs one warn naming the key and yields the base angle. Applied to the sparse angle from `sparse_infill_rotate_template` and to the solid angle from `solid_infill_rotate_template`.
3. **Multiline.** In `rectilinear-infill`'s sparse scan-line loop, each scan line becomes `fill_multiline` copies offset by one line width along the scan normal, with the group period unchanged (canonical `multiline_fill` builds the offset list, `fill_surface_by_multilines` clips and de-overlaps). Solid roles are untouched, matching canonical's `erInternalInfill` gate.

## Ownership Rule Applied

A key is declared **only** on a module that reads it:

| Key | `rectilinear-infill` | `gyroid-infill` | `lightning-infill` |
| --- | --- | --- | --- |
| `solid_infill_direction` | declared + read | declared + read | not declared |
| `sparse_infill_rotate_template` | declared + read | declared + read | not declared |
| `solid_infill_rotate_template` | declared + read | declared + read | not declared |
| `fill_multiline` | declared + read | not declared | not declared |

Lightning is a tree-based generator with no scan-line angle and no multiline concept; gyroid's TPMS curves would need curve offsetting rather than line offsetting for multiline. Declaring either would be declaration-only (Authoring rule 1). AC-N2 pins both omissions so a future packet that implements them must update the guard.

## Controlling Code Paths

- `modules/core-modules/rectilinear-infill/src/lib.rs` — the `LayerModule::run_infill` body: the `angle_deg` / `cos_a` / `sin_a` computation, the per-role per-polygon emit loop (the Q3 + Q5 partition contract), and the `infill_shift_step` per-layer shift already keyed off `layer_index`. The role-scoped angle and the multiline copies both land here; the config struct built in the module's `configure` path gains the four fields.
- `modules/core-modules/gyroid-infill/src/lib.rs` — the equivalent angle read before wave generation (the module rotates the ExPolygon around the world bbox centre before generating waves), plus its multi-role emission per ADR-0027.
- `crates/slicer-scheduler/src/config_resolution.rs` — no production change; bounds come from the manifests through `ConfigBoundsIndex` and are enforced by `check_value` (defined there) feeding `ResolvedConfig::apply_cli_key` (defined in `crates/slicer-ir/src/resolved_config.rs`, only called from `config_resolution.rs`). Only the test file changes.
- `crates/slicer-gcode/src/serialize.rs` — **read-only**. Explicit values reach the CONFIG_BLOCK through the raw-config sorted dump; no padding twin is added or corrected.

## What Carries the New Data

Manifest `[config.schema]` → `ConfigView` in `run_infill`. Nothing else. No new WIT type, no IR schema bump, no new `ResolvedConfig` field, no host special case. There is no `[BLOCK]` in this packet.

## Recorded Divergences

- **DIV-1 — the rotate-template metalanguage is rejected, not approximated.** Canonical `calculate_infill_rotation_angle` accepts more than a comma-separated list. This port implements the list form and, for anything else, logs one warn naming the key and the unsupported template and falls back to the base angle (AC-7). Rationale: config robustness. A silently-approximated angle is a print defect the user cannot diagnose; a warn plus the documented base angle is diagnosable. Recorded as a divergence rather than a gap because the key *is* live — it changes geometry for every supported template.
- **DIV-2 — port default sparse pattern is rectilinear, canonical's is crosshatch.** Unchanged by this packet and stated here only so the AC-N4 default-identity baseline is not misread as canonical parity. The pattern default is 262b's subject.
- **DIV-3 — multiline is sparse-only and rectilinear-only in this port.** Canonical gates multiline to `erInternalInfill` (same role scope), but implements it for every `FillRectilinear` subclass. This port ships it for `rectilinear-infill` only; see §Ownership Rule Applied.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The multiline offset is one *line width* in mm converted once to units; the group period must be computed from the base spacing, not from the multiplied line count, or density silently changes.
- snake_case config key strings only; all four keys are canonical snake_case.
- ADR-0027 conformance: gyroid keeps its four fill claims and the default `*_fill_holder` values stay `"rectilinear-infill"`. Giving gyroid a solid-role angle is exactly the multi-role behaviour ADR-0027 contemplates; no amendment deviation is required.
- ADR-0028 (`infill-postprocess-contract-prior-ir-and-partitioned-polygons`): the modules continue to emit from the host-partitioned polygons and must not re-derive fill areas. Multiline changes the number of paths inside `sparse_infill_area`, never the area itself.

## Code Change Surface

**Edited:**

- `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` — 4 net-new `[config.schema]` tables.
- `modules/core-modules/rectilinear-infill/src/lib.rs` — role-scoped angle, template resolver, multiline emission.
- `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` — AC-2, AC-4, AC-5, AC-6, AC-7, AC-N4 arms.
- `modules/core-modules/rectilinear-infill/tests/infill_angle_multiline_config_schema_tdd.rs` — **new** guard binary (standalone test target, auto-discovered; no aggregator registration needed — `modules/core-modules/*/tests/*.rs` are standalone binaries).
- `modules/core-modules/rectilinear-infill/Cargo.toml` — `toml` dev-dependency for the guard, add-if-absent.
- `modules/core-modules/gyroid-infill/gyroid-infill.toml` — 3 net-new tables.
- `modules/core-modules/gyroid-infill/src/lib.rs` — role-scoped angle + template resolver.
- `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` — AC-3, AC-5 arms.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — AC-8 arms.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — AC-9 arm.
- `docs/15_config_keys_reference.md` — regenerated by `cargo xtask gen-config-docs` only.
- `modules/core-modules/{rectilinear-infill,gyroid-infill}/*.wasm` — rebuilt guests.

**Blast radius owned by this packet:** the two modules' config structs gain fields, so every struct literal of those structs in their own test files must be updated in the same step (the struct-literal churn gate requires `..` rest or an `// exhaustive:` waiver in test code — `docs/21_data_defaults_and_fixtures.md`, enforced by `cargo xtask check-literals`). No public schema or version constant changes, so there is no constant-assertion fallout.

## Files in Scope (read + edit)

- `modules/core-modules/rectilinear-infill/**`
- `modules/core-modules/gyroid-infill/**`
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`

## Read-Only Context

- `modules/core-modules/lightning-infill/lightning-infill.toml` — read by the guard for AC-N2 only.
- `crates/slicer-sdk/src/{views.rs,builders.rs}` — `SliceRegionView` role polygons and `InfillOutputBuilder`.
- `crates/slicer-scheduler/src/config_resolution.rs` — the bounds/type contract the AC-8 arms assert against.

## Out-of-Bounds Files

- `crates/slicer-gcode/src/serialize.rs` — `ORCA_CONFIG_PADDING`; AC-N3 asserts a zero-line diff.
- `modules/core-modules/lightning-infill/**` — read-only; declares none of the four keys.
- `crates/slicer-schema/wit/**`, IR schema constants, `crates/slicer-ir/src/resolved_config.rs` — no WIT/IR/`ResolvedConfig` change is permitted; a worker who believes one is needed stops and raises a `[BLOCK]`.
- `docs/spec_packets/262b-infill-pattern-holder-mapping/**` and every other packet directory.
- `docs/specs/orca-feature-gap/**`.

## Expected Sub-Agent Dispatches

- `SUMMARY` — canonical `Fill.cpp::calculate_infill_rotation_angle`: exact list semantics (cycle by layer index? by absolute layer? 0-based?) and what the metalanguage adds (once, Step 2).
- `SUMMARY` — canonical `FillBase.cpp::multiline_fill` + `FillRectilinear.cpp::fill_surface_by_multilines`: the offset list, whether the base spacing is multiplied, and the de-overlap step (once, Step 4).
- `SUMMARY` — canonical `Fill.cpp::Layer::make_fills` / `group_fills`: which surface roles receive `solid_infill_direction` (once, Step 2).
- `FACT` — each cargo/xtask run: pass/fail plus failing test names only.

## Locked Assumptions and Invariants

1. Both modules already compute a single base angle from `infill_direction` and already receive `layer_index` in `run_infill` — verified in `modules/core-modules/rectilinear-infill/src/lib.rs` (the `angle_deg` / `layer_index.is_multiple_of(2)` shift logic) and `modules/core-modules/gyroid-infill/src/lib.rs`.
2. The host pre-partitions each region into `sparse_infill_area`, `top_solid_fill`, `bottom_solid_fill`, `bridge_areas`, so "which role am I emitting" is already known at the emit site — no new classification is required.
3. Module `tests/*.rs` files are standalone auto-discovered binaries (no `main.rs` aggregator), so the new guard test needs no registration — verified against `modules/core-modules/gyroid-infill/tests/`.
4. `crates/slicer-scheduler/tests/integration/` and `crates/slicer-runtime/tests/integration/` **are** aggregated (`main.rs` with a `mod` list); this packet edits existing members of both and adds no file there, so no registration is required.
5. Bounds enforcement is manifest-driven; declaring `min`/`max` is sufficient for AC-8 with no scheduler production change.

## Risks and Tradeoffs

- **Gyroid's "solid angle" is a wave orientation, not a scan-line angle.** The assertion in AC-3 must be written against gyroid's own observable (wave orientation), not against a rectilinear-style line direction. If the worker cannot express it, the honest resolution is to drop `solid_infill_direction` from `gyroid-infill.toml` and record gyroid as unimplemented for that key — never to declare it unread.
- **Multiline and density interact.** If the group period is derived from the multiplied line count, density changes silently. AC-6 pins the period explicitly for that reason.
- **Two manifests are guest-fingerprint inputs**, so the guests go stale on the first manifest edit; `cargo xtask build-guests --check` must return exit 0 before any integration-level AC is claimed.

## Context Cost Estimate

Aggregate **M** (roll-up of the per-step costs in `implementation-plan.md`). No step is rated L.

## Open Questions

- `[FWD]` Should `fill_multiline` eventually apply to gyroid via curve offsetting, and to the three pattern modules packet 263 ships? Deliberately deferred; the ownership table above is the record of what is and is not implemented.

No `[BLOCK]` items.
