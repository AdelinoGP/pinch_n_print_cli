# Design: top-bottom-surface-keys

## Tier Derivation

**Tier C.** Map Authoring rule 1 requires a packet that builds a decision point to be re-tiered B or C. This packet builds two: a host-side selection derivation (two keys onto two existing `ResolvedConfig` holder fields) and six new `Layer::Infill` filler modules, plus a density wire inside an existing module. The Tier B shape in this queue is a single-module diff; six new crates with independent geometry ports is above that ceiling, so the tier is C. The prior revision was Tier A on a "declare + wire two cheap keys" reading that Authoring rule 1 no longer permits. Ticket 17's tier table needs the same correction (listed in the session handoff; this packet does not edit the map).

## Approach

Two independent mechanisms, one per key pair.

**Pattern keys → claim holders.** OrcaSlicer resolves `top_surface_pattern` inside `Fill::new_from_type`, a switch from an enum value to a C++ filler class. This port already has the equivalent mechanism and it is better: a claim held by a module, selected per region through a config key. `claim:top-fill` and `claim:bottom-fill` exist; `top_fill_holder` and `bottom_fill_holder` exist as `ResolvedConfig` fields with the default `"rectilinear-infill"`; `FillHolders::holder_for` already resolves a claim to a module at runtime and `SliceRegionView::should_emit` already gates emission on the module holding the claim for that region. So the whole of canonical's switch reduces to a **string→string derivation** in config resolution: read `top_surface_pattern`, write `top_fill_holder`. No new field, no new claim, no new WIT, no IR change.

The derivation is the same shape packet 262b introduces for `sparse_infill_pattern` / `internal_solid_infill_pattern`. This packet extends 262b's helper with two more key→field pairs rather than adding a parallel one, and additionally applies it on the per-object overlay path so a per-object pattern is not dropped.

What the derivation cannot do is invent fillers. Canonical offers eight values per key; the tree has one (`rectilinear-infill`) plus 262b's `monotonic-infill`. Under Authoring rule 1 the remaining six must be built or the values must be rejected; the user ruling for this packet is to build all eight, so six new module crates ship here.

**Density keys → spacing divisors.** `rectilinear-infill` computes `solid_spacing = mm_to_units(solid_line_width / SOLID_DENSITY)` twice inside `RectilinearInfill::run_infill`, once for the top block and once for the bottom, with `SOLID_DENSITY` a hardcoded `1.0`. That constant *is* the decision point canonical parameterises: `group_fills` sets `params.density` from the surface's density key and `Layer::make_fills` normalizes it with `0.01 * density`. Replacing the constant with two resolved per-role fractions makes both keys live with a diff confined to `LayerModule::from_config` and `run_infill`. Canonical's `density <= 0` top-surface skip becomes a `> 0` gate on the exposed-top block only; the bottom block gets no gate because canonical's min of 10 makes zero unreachable there, and internal solid keeps a fixed 1.0 because canonical hardcodes `100.f` for `stInternalSolid`.

## Controlling Code Paths

- `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`) — iterates the raw source, enforces manifest bounds via `ConfigBoundsIndex::check`, dispatches into `ResolvedConfig::apply_cli_key`, then threads schema defaults. The pattern→holder derivation runs after that loop, so an explicit `top_fill_holder` already applied by `apply_cli_key` can be detected and left alone.
- `apply_overlay` (same file) — builds a per-object `ResolvedConfig` from the global base plus `object_config:<id>:<key>` entries, called by `resolve_per_object_configs`. The derivation runs here too.
- `ResolvedConfig::apply_cli_key` and the `declare_resolved_config!` block (`crates/slicer-ir/src/resolved_config.rs`) — where `top_fill_holder` / `bottom_fill_holder` are declared, `String`, default `"rectilinear-infill"`. Read-only for this packet.
- `FillHolders`, `FillHolders::holder_for`, `module_id_matches_holder`, `resolve_held_claims`, `FILL_CLAIM_IDS` (`crates/slicer-scheduler/src/validation.rs`) — the runtime side that turns the resolved holder string into the `held_claims` a module sees. Read-only for this packet; the derivation feeds it.
- `SliceRegionView::should_emit`, `held_claims`, `top_shell_index`, `bottom_shell_index`, `top_solid_fill`, `bottom_solid_fill`, `internal_solid_fill` (`crates/slicer-sdk/src/views.rs`) — the per-region contract every new filler implements against. Read-only.
- `RectilinearInfill`'s `LayerModule::from_config` impl, `RectilinearInfill::run_infill`, `solid_fill_role`, `adjust_solid_spacing`, `SOLID_DENSITY` (`modules/core-modules/rectilinear-infill/src/lib.rs`) — the density decision points.
- `crates/slicer-core/src/algos/region_mapping.rs` — already copies `top_fill_holder` / `bottom_fill_holder` from a region overlay when they differ from the default, so per-region holder selection works once the derivation writes the fields. Read-only; no edit needed.

## What Carries the New Data

Nothing new carries anything. This is the point of the design:

- The pattern *choice* travels as the existing `top_fill_holder` / `bottom_fill_holder` strings on `ResolvedConfig`, through the existing region-overlay path in `region_mapping.rs`, into the existing `held_claims` list on `SliceRegionView`.
- The density *values* travel as ordinary module config keys declared on `rectilinear-infill.toml` and read in `LayerModule::from_config`, exactly like every other key that module owns.
- The surface *kind* a density applies to is already distinguishable: `top_shell_index() == Some(0)` is exposed top, `Some(n >= 1)` is internal solid, and `bottom_shell_index()` mirrors it.

No prepass IR change, no new `SliceRegionView` metadata, no new `PostPass` claim, no manifest-schema extension, no SDK change.

## Recorded Divergences (port improves on or intentionally differs from canonical)

- **DIV-1 — pattern selection is extensible; canonical's is closed.** Canonical's `Fill::new_from_type` is a `switch` over a fixed enum: a new filler requires editing libslic3r. Here the eight values map onto module ids, so a community module that holds `claim:top-fill` can be selected by setting `top_fill_holder` directly, with no host change. The eight canonical *names* are a convenience mapping layered on top of a more general mechanism. Rationale: `docs/00_project_overview.md`'s community-extensibility goal, and map Authoring rule 4.
- **DIV-2 — bridge and void-extension fills are decoupled from `top_surface_pattern`.** Canonical `group_fills` reads `top_surface_pattern` when picking a filler for bridges above layer 0 (choosing `ipMonotonic` if the top pattern is either monotonic variant, else `ipRectilinear`) and again for the synthesized `stInternalSolid` void extension. That is coupling, not intent: a user changing their top-surface look silently changes bridge geometry. The port keeps bridges on `bridge_fill_holder` and internal solid on 262b's `internal_solid_infill_pattern` mapping, so the three are independently selectable. Recorded as an intentional divergence, not a gap.
- **DIV-3 — per-object pattern override.** Canonical's pattern keys are `PrintRegionConfig` members and are per-region already, but the port additionally resolves them on the per-object overlay path (AC-3), so `object_config:<id>:top_surface_pattern` works without a region split. Strictly additive.
- **DIV-4 — `adjust_solid_spacing` already diverges.** The existing D-209-ADJUST-SOLID-SPACING-DIVERGENCE recorded in `modules/core-modules/rectilinear-infill/src/lib.rs` (bare `width` instead of `width - EPSILON`, rounding instead of truncation, unmodified `distance` on the over-cap branch) is pre-existing and unchanged by this packet. Making the density a variable widens the range of inputs that reach it; the divergence stays recorded, and the packet does not "fix" it, because doing so would change default output and break AC-N1.
- **DIV-5 — no emission-time pattern coupling.** Canonical's `GCode::_needSAFC` and `GCode::retract` change flow compensation and retraction based on which top/bottom pattern is selected. The port's emitter does not know the fill holder and this packet does not give it that knowledge. Recorded; a future packet that wants SAFC would introduce the seam deliberately rather than inheriting canonical's reach-through.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **Claim uniqueness.** `docs/04_host_scheduler.md` § Claim Resolution: validation fails if a claim has more than one effective holder. Eight modules will declare `claim:top-fill`, so all but one must be disabled for any given region. This is exactly what the `*_fill_holder` mechanism does (`resolve_held_claims` filters a manifest's `holds` by the configured holder) — the same arrangement `rectilinear-infill` and `gyroid-infill` already live under. The six new modules must not be `incompatible-with` one another; that would turn a normal selection into a load-time conflict.
- **Struct-literal churn gate.** Adding two fields to `RectilinearInfill` puts it over the five-named-field watchlist threshold if it is not already. Every test-code literal of that type needs a `..` rest or an `// exhaustive: <reason>` waiver; `cargo xtask check-literals` enforces it. The step that adds the fields owns that blast radius (`docs/21_data_defaults_and_fixtures.md`).
- **Module-count ledger fact.** `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` asserts a module count. Re-derive it from disk at the moment of editing; packets 262b and 263 also move it, so any number written here would be stale before it is read.
- **ADR-0056** governs registration of a new core module: workspace member, integrated-modules optional dep + feature + `manifest_const!` + `integrated_registry!` entry and its `#[cfg(not(feature = …))]` arm, and a `pnp-cli` passthrough feature.

## Code Change Surface

**New (six crates, identical shape):**

- `modules/core-modules/monotonicline-infill/{Cargo.toml,src/lib.rs,monotonicline-infill.toml,wit-guest/**,tests/monotonicline_infill_tdd.rs}`
- `modules/core-modules/alignedrectilinear-infill/{…,tests/alignedrectilinear_infill_tdd.rs}`
- `modules/core-modules/concentric-infill/{…,tests/concentric_infill_tdd.rs}`
- `modules/core-modules/hilbert-curve-infill/{…,tests/hilbert_curve_infill_tdd.rs}`
- `modules/core-modules/archimedean-chords-infill/{…,tests/archimedean_chords_infill_tdd.rs}`
- `modules/core-modules/octagram-spiral-infill/{…,tests/octagram_spiral_infill_tdd.rs}`

Each `src/lib.rs` implements `LayerModule` at `Layer::Infill`, reads `top_solid_fill()` / `bottom_solid_fill()` from `SliceRegionView`, gates on `should_emit(TopSolidInfill)` / `should_emit(BottomSolidInfill)`, and emits its curve. Each manifest declares `holds = ["claim:top-fill", "claim:bottom-fill"]` and re-declares only the shared base keys it reads (`line_width`, `infill_direction`, and the two density keys it consumes for spacing).

**Edited:**

- `crates/slicer-scheduler/src/config_resolution.rs` — extend 262b's pattern→holder derivation with the two new key→field pairs; call it from `apply_overlay` as well as `resolve_global_config`.
- `modules/core-modules/rectilinear-infill/src/lib.rs` — two new fields on `RectilinearInfill`, populated in its `LayerModule::from_config` impl; both `SOLID_DENSITY` uses in `run_infill` replaced by the per-role fraction; a `> 0` gate on the exposed-top block. Delete the now-unused `SOLID_DENSITY` const.
- `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` — two `[config.schema]` tables.
- `modules/core-modules/rectilinear-infill/Cargo.toml` — `toml = "0.8"` dev-dependency (add-if-absent; 262a/263 may have landed it first).
- `modules/core-modules/monotonic-infill/monotonic-infill.toml` — append `"claim:bottom-fill"` to `holds`. **Created by packet 262b**; this packet must not create it.
- `modules/core-modules/monotonic-infill/src/lib.rs` — add the `BottomSolidInfill` emission arm.
- `Cargo.toml` (root) — six workspace members.
- `crates/slicer-integrated-modules/Cargo.toml`, `crates/slicer-integrated-modules/src/lib.rs` — six optional deps, features, `manifest_const!` and `integrated_registry!` entries plus their `#[cfg(not(feature = …))]` arms.
- `crates/pnp-cli/Cargo.toml` — six `integrated-<name>` passthrough features.
- `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` — module count moved by six; new-manifest assertions (AC-10).
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — AC-13 arms.
- `crates/slicer-scheduler/tests/integration/` — net-new `top_bottom_pattern_holder_tdd.rs` plus its `mod` line in `main.rs` (AC-1/2/3, AC-N3).
- `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs` — AC-4, AC-5 arms.
- `modules/core-modules/rectilinear-infill/tests/top_bottom_surface_config_schema_tdd.rs` — net-new (AC-12, AC-N5).
- `crates/slicer-runtime/tests/contract/` — `native_infill_claim_resolution` arms for AC-11 / AC-N4.
- `docs/04_host_scheduler.md`, `docs/03_wit_and_manifest.md` — hand-maintained doc edits.
- `docs/15_config_keys_reference.md` — regenerated, never hand-edited.
- Guest `.wasm` artifacts for the eight affected modules.

## Files in Scope (read + edit)

The change surface above is the authoritative list. No file outside it may be edited.

## Read-Only Context

- `crates/slicer-ir/src/resolved_config.rs` — the four `*_fill_holder` rows of `declare_resolved_config!` only. Do not load the file.
- `crates/slicer-scheduler/src/validation.rs` — `FILL_CLAIM_IDS`, `FillHolders::holder_for`, `resolve_held_claims`.
- `crates/slicer-sdk/src/views.rs` — `SliceRegionView` accessors named in § Controlling Code Paths.
- `crates/slicer-core/src/algos/region_mapping.rs` — the holder-overlay block, to confirm no edit is needed.
- `docs/04_host_scheduler.md` § Claim Resolution (the `wall_generator` subsection is the shape to imitate), `docs/03_wit_and_manifest.md` § Known claim IDs.
- `docs/spec_packets/262b-infill-pattern-holder-mapping/design.md` — via a SUMMARY dispatch only, to learn the derivation helper's final name. Never edit anything under that directory.

## Out-of-Bounds Files

- `crates/slicer-gcode/src/serialize.rs` — `ORCA_CONFIG_PADDING`. Zero diff lines (AC-N2, map Authoring rule 2).
- `crates/slicer-schema/wit/**` — no WIT change.
- `crates/slicer-ir/src/slice_ir.rs` — no IR schema change.
- `crates/slicer-ir/src/resolved_config.rs` — no new field; read-only.
- `modules/core-modules/gyroid-infill/**`, `modules/core-modules/lightning-infill/**` — untouched (AC-N5 pins the omission).
- Every other packet directory under `docs/spec_packets/`.
- `docs/specs/orca-feature-gap/map.md` and `docs/specs/orca-feature-gap/issues/**` — read-only; required updates are reported, not applied.
- `docs/15_config_keys_reference.md` — generated; regenerate, never hand-edit.

## Expected Sub-Agent Dispatches

- **Canonical filler algorithms**, one dispatch per crate, `SUMMARY` ≤ 200 words + at most 3 snippets ≤ 30 lines: `FillMonotonicLines::fill_surface` + `connect_segment_intersections_by_contours`; `FillAlignedRectilinear`; `FillConcentric::_fill_surface_single`; `FillHilbertCurve` / `FillArchimedeanChords` / `FillOctagramSpiral` point generators + `FillPlanePath::fill_surface`.
- **262b's derivation helper name** — `FACT` ≤ 5 lines, from `docs/spec_packets/262b-infill-pattern-holder-mapping/design.md`.
- **Module-count ledger fact** — `FACT` ≤ 5 lines: the current asserted count in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`, re-derived at the moment of editing.
- **Cargo runs** — every `cargo check` / `clippy` / `test` / `xtask` invocation is delegated with a `FACT pass/fail` return.

## Data and Contract Notes

- Manifest `[config.schema]` types are `bool` / `int` / `float` / `string` / `enum` (with `values`). Canonical's coPercent has no manifest equivalent, so both density keys are declared `float` with min/max in percent units (0–100 top, 10–100 bottom) and divided by 100 in `from_config` — the same normalization canonical does in `Layer::make_fills`.
- The two pattern keys are declared in **no** manifest. They are host-side selection keys, like `wall_generator`. Their legal-value list therefore lives in the derivation code and in `docs/04_host_scheduler.md`, and their rejection path is the derivation's own error, not `ConfigBoundsIndex`.
- All config key strings are snake_case (`CLAUDE.md` § Config Key Naming Convention). Module ids in holder values are the short crate names (`module_id_matches_holder` accepts short-name or full-id forms).

## Locked Assumptions and Invariants

0. **Forward dependencies (not shipped symbols).** `modules/core-modules/monotonic-infill/**` and 262b's pattern→holder derivation helper do **not** exist in the tree today; packet 262b (`status: draft`) creates them. Every reference to either in this packet is a FORWARD-DEP whose name and shape are reconciled against 262b's own spec (it plans `monotonic-infill` holding `claim:top-fill`, and a derivation writing `sparse_fill_holder` / `top_fill_holder`). If 262b's plan changes, reconcile both specs before either activates.
1. `top_fill_holder` and `bottom_fill_holder` exist on `ResolvedConfig` with default `"rectilinear-infill"` — verified at authoring. If a worker finds otherwise, stop: the packet's no-new-field claim is falsified and it must be re-scoped.
2. `claim:top-fill` and `claim:bottom-fill` exist and `SliceRegionView::should_emit` maps `TopSolidInfill` / `BottomSolidInfill` onto them — verified at authoring.
3. Default resolution must leave both holders at `"rectilinear-infill"` and both densities at fraction 1.0, so AC-N1 stays byte-identical from the first step that touches spacing onward.
4. Exactly one module holds each fill claim for a given region after `resolve_held_claims`; adding seven more candidate holders must not produce a startup claim conflict.
5. `bottom_surface_density` can never be 0 (canonical min 10). The bottom block must not carry the top block's zero-skip.
6. Internal solid never sees either density key; it stays at fraction 1.0.

## Risks and Tradeoffs

- **Size.** Six new geometry crates in one packet is the largest slice in this queue. Mitigation: the crates are independent and the implementation plan gives each its own step with its own exit condition; a stalled filler blocks only its own step. Recorded as accepted at the user's explicit direction (the "all 8 values" ruling); the narrower alternative was shipping `monotonicline` only and returning five values to the queue.
- **Plane-path fillers have no in-tree precedent.** Hilbert, Archimedean, and octagram curves are a family this tree has never carried. Their ACs assert structural properties (lattice membership, monotonic radius, turn-angle set) rather than golden geometry, so they remain falsifiable without a canonical golden file the port cannot produce.
- **262b coupling.** Two edits land in files 262b creates. If 262b's shape shifts, this packet's Steps 1 and 8 adapt; the mitigation is the SUMMARY dispatch for its helper name rather than a frozen assumption.
- **Removing `SOLID_DENSITY`.** A wrong normalization silently changes every solid fill. AC-N1 is the guard and must be run in the same step.

## Context Cost Estimate

**L aggregate.** No single step exceeds M: each filler crate is one step reading one canonical function; the derivation is one step in one host file; the density wire is one step in one module file.

## Open Questions

None blocking. No `[BLOCK]`: the packet needs no new WIT interface, no IR schema bump, and no new host `ResolvedConfig` field — all three were checked against the tree at authoring and the required carriers already exist.

- `[FWD]` Canonical's `link_max_length = 3 * spacing` above 80% density (set in `Layer::make_fills`) shapes how far a monotonic connector may run. The port has no `link_max_length` concept. `monotonic-infill` (262b) will need one eventually for parity; this packet's AC-6 asserts only the presence/absence of connectors, which is the real difference between the two monotonic classes, and leaves the length cap forward.
- `[FWD]` Canonical's top-surface expansion pass and the `top_surface_density > 0` gates that ride it (`top_fill_replaces_inner_walls`, `detect_surfaces_type`) are a separate feature. When a packet builds it, `top_surface_density` gains a second decision point.
