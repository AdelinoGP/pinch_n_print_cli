# Design: infill-pattern-specific-keys

## Tier Derivation

**Tier C.** The map's rubric puts a packet that *builds* a decision point at B or C; this packet builds three new geometry modules (new crates, new fill algorithms, new guest artifacts) rather than wiring existing decision points, so it sits at the top of that range. The previous revision's Tier A rating was a consequence of the prohibited declaration-only disposition and is void.

## Approach

OrcaSlicer expresses "which sparse infill pattern" as an enum on one region config, and every pattern-specific key is a field on the same `PrintObjectConfig`, gated at fill time by `Layer::make_fills` checking `params.pattern`. This port expresses the same choice as **module identity**: a fill pattern is a module holding `claim:sparse-fill`, selected per print by `ResolvedConfig.sparse_fill_holder` and per region by `module_overrides` (`docs/01_system_architecture.md` §Claim System, `docs/04_host_scheduler.md` §Claim Resolution).

The packet therefore ships one module per canonical pattern class:

| New module | Claim | Canonical class | Keys owned |
| --- | --- | --- | --- |
| `lateral-lattice-infill` | `claim:sparse-fill` | `FillLateralLattice` | `lateral_lattice_angle_1`, `lateral_lattice_angle_2` |
| `lateral-honeycomb-infill` | `claim:sparse-fill` | `FillLateralHoneycomb` | `infill_overhang_angle` |
| `locked-zag-infill` | `claim:sparse-fill` | `FillLockedZag` | `infill_lock_depth`, `skin_infill_depth`, `skin_infill_density`, `skeleton_infill_density`, `skin_infill_line_width`, `skeleton_infill_line_width`, `symmetric_infill_y_axis` |

Each module declares **only** the keys its own algorithm reads. Ownership *is* the gate: a key that applies to one pattern is unreachable from any other fill module, so canonical's runtime `params.pattern` check has no port equivalent to write.

## Controlling Code Paths

- `slicer_sdk::traits::LayerModule::run_infill(&self, layer_index, regions, paint, output, config)` — the entry point each new module implements, exactly as `RectilinearInfill` and `LightningInfill` do. `#[slicer_module]` (`slicer_sdk::slicer_module`) emits the component export; the `wit-guest/` cdylib wrapper re-exports the type so the export survives into the `.wasm`.
- `slicer_sdk::views::SliceRegionView` — supplies `sparse_infill_area()` (the host-partitioned sparse polygon), `z()` (needed by both lateral patterns' z-proportional geometry), `object_id()` (for the `symmetric_infill_y_axis` mirror axis), and `effective_layer_height()`.
- `slicer_sdk::builders::InfillOutputBuilder::{begin_region, push_sparse_path}` — the only emission surface these modules need; they push no solid, bridge, or ironing path.
- `slicer_sdk::host::{clip_polygons, offset_polygons}` — the Clipper equivalents of canonical's `intersection_pl` / `offset_ex` / `diff_ex`; `slicer_sdk::host::object_bounds` supplies the mirror axis for `symmetric_infill_y_axis`.
- `crates/slicer-ir/src/resolved_config.rs` — `sparse_fill_holder` (existing `String` field, CLI key `sparse_fill_holder`, default `"rectilinear-infill"`). Selecting a new module is a config value, not a code change.
- `crates/slicer-integrated-modules/src/lib.rs` — `manifest_const!` and `integrated_registry!` entries plus their `#[cfg(not(feature = …))]` arms.
- `xtask/src/build_guests.rs` — discovers core guests by walking `modules/core-modules/`; no per-module registration list to edit. `guest_input_paths` charges the module's `*.toml`, so a manifest edit alone makes the guest stale.
- `xtask/src/dist.rs` — derives `integrated-<name>` feature names per module and errors when a passthrough feature is missing from `crates/pnp-cli/Cargo.toml`.

## What Carries the New Data

Nothing new. The three mechanisms already exist:

1. **Manifest + SDK config** — the 10 keys ride each module's `[config.schema]` and arrive as `ConfigView` in `run_infill`. No host plumbing.
2. **Existing holder resolution** — `sparse_fill_holder` / `module_overrides` select the module. No new `ResolvedConfig` field.
3. **`ExtrusionPath3D` with `Point3WithWidth`** — locked-zag's two extrusion widths ride per-vertex widths on separate paths in one `InfillIR` region. No IR schema bump.

There is no `[BLOCK]` in this packet: no new WIT interface, no IR schema bump, no new host `ResolvedConfig` field.

## Recorded Divergences (port improves on or intentionally differs from canonical)

- **DIV-1 — pattern gating is structural, not runtime.** Canonical sets `symmetric_infill_y_axis` on `FillParams` only when `params.pattern ∈ {ipZigZag, ipCrossZag, ipLockedZag}` inside `Layer::make_fills`, i.e. the key is globally declared and conditionally ignored. Here the key exists only in `locked-zag-infill.toml`, so no other pattern can read it and no conditional exists to get wrong. Rationale: config robustness (`docs/00_project_overview.md`) — a key that silently does nothing under some other setting is a user-facing trap. AC-N1 pins the ownership. Consequence recorded for future readers: shipping zigzag/crosszag later means adding the key to those manifests, not adding a check.
- **DIV-2 — width keys are millimetres with `0.0 = inherit`.** Canonical `skin_infill_line_width` / `skeleton_infill_line_width` are `coFloatOrPercent` defaulting to `100%`, resolved against nozzle diameter by `Flow::new_from_config_width`. This port already expresses fill widths as `line_width` floats where `0.0` means "inherit the region line width" (`gyroid-infill.toml`, `rectilinear-infill.toml`), and the host owns flow. Adopting the percent form here would introduce a second width convention in the same stage. Rationale: one convention per stage.
- **DIV-3 — density keys are percent numbers.** `coPercent` becomes `float` in percent units per the ticket-107 in-tree convention (`sparse_infill_density = 25.0` means 25 %).
- **DIV-4 — dual flow becomes per-vertex width.** Canonical emits two `ExtrusionEntityCollection`s built from two `Flow`s and splits polylines between them in `generate_for_different_flow`. The port's `ExtrusionPath3D` already carries per-vertex width, so skin and skeleton paths are emitted into the same region with their own widths and no collection split. Rationale: the port's IR is strictly more expressive here; adding a flow-partition concept would be reproducing canonical's coupling.
- **DIV-5 — multiline is not ported.** Canonical's `fill_surface_by_multilines` applies `multiline_fill` and `remove_overlapped`. This packet emits the single-line case only; `fill_multiline` is packet 262a's key and is not declared on these modules. Recorded so a reviewer does not read the omission as an oversight.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Every canonical `scale_()`/`unscale()` in the three fill classes is a unit boundary: `dx = tan(angle) · z` is computed in mm and converted once; `skin_infill_depth` and `infill_lock_depth` are mm offsets converted before `offset_polygons`.
- snake_case config key strings only; all 10 keys are canonical snake_case with no aliases.
- Claim discipline: each new module holds `claim:sparse-fill` and nothing else (AC-1, AC-N3). Holding the solid/bridge claims as well would make the module compete with `rectilinear-infill` for roles it does not implement.
- ADR-0027 (`gyroid-multi-role-fill-holder`): unaffected — default `*_fill_holder` values stay `"rectilinear-infill"`, gyroid keeps its four claims, and no default is repointed. No amendment deviation required.
- ADR-0056 (integrated-modules native dispatch): each new module must appear in the integrated registry and carry a `pnp-cli` passthrough feature, or `cargo xtask dist --edition integrated` fails.

## Code Change Surface

**New files (per module `M` in {`lateral-lattice-infill`, `lateral-honeycomb-infill`, `locked-zag-infill`}):**

- `modules/core-modules/M/Cargo.toml` — modelled on `modules/core-modules/gyroid-infill/Cargo.toml` (deps `slicer-sdk`, `slicer-schema`, `slicer-ir`, `slicer-core`; dev-dep `slicer-sdk` with `features = ["test"]`; wasm32-only `wit-bindgen`).
- `modules/core-modules/M/src/lib.rs` — the `LayerModule` impl.
- `modules/core-modules/M/M.toml` — module manifest (`[module]`, `[stage] Layer::Infill`, `[ir-access]`, `[claims] holds = ["claim:sparse-fill"]`, `[compatibility]`, `[config.schema]`).
- `modules/core-modules/M/wit-guest/{Cargo.toml,src/lib.rs}` — cdylib wrapper with its own `[workspace]` sentinel, modelled on `modules/core-modules/gyroid-infill/wit-guest/`.
- `modules/core-modules/M/tests/<m>_tdd.rs` — behaviour tests.
- `modules/core-modules/locked-zag-infill/tests/infill_pattern_specific_config_schema_tdd.rs` — the shared manifest guard (parses all three new manifests plus the three existing fill manifests for AC-N1); needs a `toml` dev-dependency on that crate.

**Edited files:**

- `Cargo.toml` (root) — three `[workspace] members` entries.
- `crates/slicer-integrated-modules/Cargo.toml` — three optional path deps + three features.
- `crates/slicer-integrated-modules/src/lib.rs` — three `manifest_const!` entries, three `integrated_registry!` rows, and the corresponding `#[cfg(not(feature = …))]` arms.
- `crates/pnp-cli/Cargo.toml` — three `integrated-<name>` passthrough features.
- `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` — module-count assertion +3 and its comment.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — AC-14 arms.
- `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs` — AC-2 / AC-N3 arms.
- `docs/15_config_keys_reference.md` — regenerated only (`cargo xtask gen-config-docs`).
- Three new `modules/core-modules/M/M.wasm` guest artifacts (produced by `cargo xtask build-guests`, committed like the existing guests).

**Blast radius owned by this packet:** the module-count assertion above is the only hard-coded count that moves. No new struct field and no public schema/version constant is introduced, so there is no struct-literal or constant-assertion fallout. `crates/slicer-runtime/Cargo.toml` gains a dev-dependency on a new module crate only if an integration test drives it natively — the AC-2/AC-N3 arms are written against the existing manifest/claim machinery and do not require one; if a worker finds otherwise, adding the dev-dependency is in scope for that step.

## Files in Scope (read + edit)

- The three new `modules/core-modules/M/**` trees (created by this packet).
- `Cargo.toml`, `crates/slicer-integrated-modules/Cargo.toml`, `crates/slicer-integrated-modules/src/lib.rs`, `crates/pnp-cli/Cargo.toml`.
- `crates/slicer-scheduler/tests/integration/{manifest_ingestion_tdd.rs,config_bounds_enforcement_tdd.rs}`.
- `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs`.

## Read-Only Context

- `modules/core-modules/gyroid-infill/**` and `modules/core-modules/lightning-infill/**` — the shape to copy (crate layout, manifest, guest wrapper, sparse-only claim set, test harness via `slicer_sdk::test_prelude`).
- `modules/core-modules/rectilinear-infill/src/lib.rs` — the scan-line emission and per-role partition contract.
- `crates/slicer-sdk/src/{views.rs,builders.rs,host.rs}` — the API surface used.
- `crates/slicer-ir/src/slice_ir.rs` — `ExtrusionPath3D`, `Point3WithWidth`, `InfillRegion`.

## Out-of-Bounds Files

- `modules/core-modules/{rectilinear,gyroid,lightning}-infill/**` — read-only; **no edits** (their behaviour and manifests must not change; AC-N1 depends on that).
- `crates/slicer-gcode/src/serialize.rs` — `ORCA_CONFIG_PADDING` must not be touched (AC-N2).
- `crates/slicer-schema/wit/**`, `crates/slicer-ir/src/slice_ir.rs` schema constants, `crates/slicer-ir/src/resolved_config.rs` — no WIT, IR-schema, or `ResolvedConfig` change is permitted; a worker who believes one is needed must stop and raise it as a `[BLOCK]` rather than making it.
- `docs/specs/orca-feature-gap/**` — the map and tickets are not edited by this packet.
- Any other packet directory under `docs/spec_packets/`.

## Expected Sub-Agent Dispatches

- `SUMMARY` — canonical `FillLateralLattice::fill_surface` exact shift/angle arithmetic and odd-layer reversal (once, at Step 2).
- `SUMMARY` — canonical `FillLateralHoneycomb::fill_surface` period split, `horizontal_position` interpolation, density rescale, and stagger (once, at Step 3).
- `SUMMARY` — canonical `FillLockedZag::fill_surface_locked_zag` + `fill_surface_extrusion` region algebra and flow split (once, at Step 4).
- `SUMMARY` — canonical `Layer::make_fills` mirror axis and `MultiPoint::symmetric_y` arithmetic (once, at Step 5).
- `FACT` — each `cargo test` / `cargo check` / `cargo clippy` / `cargo xtask` run: pass/fail plus failing test names only.

## Data and Contract Notes

- The new modules consume the host-partitioned `sparse_infill_area()` only; they must not re-derive infill areas from `polygons()` (the four canonical fill polygons are pairwise disjoint and the host owns the partition).
- `symmetric_infill_y_axis` needs the *object* bounding box, not the region bbox: canonical uses `extended_object_bounding_box().center().x()`. The port reads `slicer_sdk::host::object_bounds(region.object_id())` and takes the x centre. When `object_bounds` returns `HostUnavailable`, the module logs a warn and emits the unmirrored fill — a mirror is a print-quality preference, never a correctness gate.
- Density → spacing conversion follows the existing fill modules' convention so `sparse_infill_density` behaves identically across pattern modules.
- Speed: paths are emitted with `speed_factor = 1.0`; the host resolves the feedrate from the role, as `rectilinear-infill` documents.

## Locked Assumptions and Invariants

1. `ResolvedConfig.sparse_fill_holder` accepts any module id string and is the only selector needed — verified in `crates/slicer-ir/src/resolved_config.rs` and by `lightning-infill`'s sparse-only precedent.
2. A module may hold `claim:sparse-fill` alone — verified: `lightning-infill.toml` declares exactly that.
3. `xtask build-guests` discovers core guests by directory walk under `modules/core-modules/`, so no guest list edit is required — verified in `xtask/src/build_guests.rs`.
4. `ExtrusionPath3D` carries per-vertex width via `Point3WithWidth`, so two extrusion widths need no IR change — verified in `crates/slicer-ir/src/slice_ir.rs`.
5. The only hard-coded module count in the test suite is the assertion in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`. If a worker finds a second, it joins that step's edit list.
6. The three new patterns are sparse-only; solid, bridge, and ironing roles remain with their existing holders.

## Risks and Tradeoffs

- **Locked-zag is the least self-contained of the three** (canonical recurses into its own `FillRectilinear::fill_surface` base for the zig-zag path and depends on `offset_ex`/`diff_ex`/`intersection_ex`). Mitigation: the port implements the zig-zag base inside the module rather than depending on `rectilinear-infill` (modules cannot call each other), and Step 4 is split from Steps 2–3 so a failure there does not block the two lateral modules.
- **Honeycomb's AC-5 is a statistical assertion** (band-count ratio within 10 %). It is falsifiable and stable for a fixed fixture, but a worker who finds the ratio sensitive to the fixture size must widen the z sweep rather than loosen the tolerance.
- **Three guests to rebuild** lengthens the guest build; `cargo xtask build-guests --check` must be run and must return exit 0 before closure.
- **Adding three modules changes the DAG breadth.** They are inert unless selected by `sparse_fill_holder`, and AC-2 pins that default prints are unaffected.

## Context Cost Estimate

Aggregate **L** (roll-up of the per-step S/M costs in `implementation-plan.md`). No single step is rated L; the packet is decomposed so each module is its own step.

## Open Questions

- `[FWD]` Should the three new patterns also be reachable through a future `sparse_infill_pattern` value→holder mapping (packet 262b's scope)? This packet deliberately does not add such a mapping; when 262b lands it, the three module ids become the natural targets for `lockedzag`, `lateral_lattice`, and `lateral_honeycomb`. Recorded so the two packets do not both invent one.
- `[FWD]` Canonical's `LockRegionParam` supports per-painted-region skin/skeleton density and flow. This packet resolves both from the region's own config. A future painting packet can extend it through `SliceRegionView` metadata without changing the manifest keys.

No `[BLOCK]` items.
