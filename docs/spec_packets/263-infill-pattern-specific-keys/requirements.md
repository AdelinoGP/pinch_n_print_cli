# Requirements: infill-pattern-specific-keys

## Packet Metadata

- Packet directory: `docs/spec_packets/263-infill-pattern-specific-keys/`
- Slug: `infill-pattern-specific-keys`
- Status: `draft`
- Tier: **C** (re-derived — the packet builds three new geometry modules; see `design.md` §Tier Derivation)
- Backlog source: `docs/specs/orca-feature-gap/issues/16-author-packet-p09-strength-infill-pattern-specific-infill-modules.md`
- Re-authored: under the map's **Authoring rules 1–6** (`docs/specs/orca-feature-gap/map.md` §Notes), which prohibit the previous revision's "declared-with-gap" disposition.

## Problem Statement

The previous revision of this packet declared all 10 P09 keys in `rectilinear-infill.toml` with zero module-source reads and zero behaviour change at any value — a pure-declaration packet. Authoring rule 1 prohibits that disposition: a key is covered only when the behaviour OrcaSlicer attaches to it exists in this tree and the key drives it.

Canonical grounding (re-derived this session) shows every one of the 10 keys is consumed by exactly one unshipped sparse-infill pattern:

| Canonical consumer | Keys |
| --- | --- |
| `FillLateralLattice::fill_surface` | `lateral_lattice_angle_1`, `lateral_lattice_angle_2` |
| `FillLateralHoneycomb::fill_surface` | `infill_overhang_angle` |
| `FillLockedZag::fill_surface_locked_zag` / `fill_surface_extrusion` | `infill_lock_depth`, `skin_infill_depth`, `skin_infill_density`, `skin_infill_line_width`, `skeleton_infill_density`, `skeleton_infill_line_width` |
| `Layer::make_fills` gate + `FillRectilinear::fill_surface_by_lines` mirror branch | `symmetric_infill_y_axis` (activated only when the sparse pattern is `ipZigZag` / `ipCrossZag` / `ipLockedZag`) |

Under Authoring rule 4, an Orca enum whose values are different algorithms is not an enum to declare — it is a set of `claim:*` holders, one module per shipped value, resolved through the existing `sparse_fill_holder` / `module_overrides`. This packet therefore **builds the three pattern modules** so all 10 keys become live decision points. `symmetric_infill_y_axis` follows the pattern module that ships it: `locked-zag-infill` is the only shipped pattern in canonical's activation set, so the key lives in that manifest and nowhere else, which makes canonical's runtime pattern check structurally unnecessary in this port.

## Key Disposition Table

Classification per the map's Authoring rules: **(a)** live behaviour-changing decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue (no decision point, not built here); **(d)** dead-in-canonical.

| Key | Class | Owning module (new) | Decision point this packet builds | Non-default AC |
| --- | --- | --- | --- | --- |
| `lateral_lattice_angle_1` | **(b)** | `lateral-lattice-infill` | per-layer horizontal shift `dx1 = tan(angle_1) · z` on line family 1 | AC-3 |
| `lateral_lattice_angle_2` | **(b)** | `lateral-lattice-infill` | per-layer horizontal shift `dx2 = tan(angle_2) · z` on line family 2 | AC-4 |
| `infill_overhang_angle` | **(b)** | `lateral-honeycomb-infill` | honeycomb vertical period `3 · half_horizontal_period / tan(angle)` — sets the double-line/single-line band heights | AC-5 |
| `infill_lock_depth` | **(b)** | `locked-zag-infill` | dilation of the skeleton core into the skin band before re-clipping (the interlock) | AC-6 |
| `skin_infill_depth` | **(b)** | `locked-zag-infill` | inset distance splitting surface into skin band and skeleton core | AC-7 |
| `skin_infill_density` | **(b)** | `locked-zag-infill` | line spacing inside the skin band | AC-8 |
| `skeleton_infill_density` | **(b)** | `locked-zag-infill` | line spacing inside the skeleton core | AC-9 |
| `skin_infill_line_width` | **(b)** | `locked-zag-infill` | per-vertex extrusion width on skin-band paths | AC-10 |
| `skeleton_infill_line_width` | **(b)** | `locked-zag-infill` | per-vertex extrusion width on skeleton-core paths | AC-11 |
| `symmetric_infill_y_axis` | **(b)** | `locked-zag-infill` | mirror the region about the object bbox centre x, fill, mirror back | AC-12 |

Counts: **(a) 0 · (b) 10 · (c) 0 · (d) 0.** Zero declaration-only keys (map preflight gate (a)); every key carries at least one AC asserting a behaviour change at a non-default value (map preflight gate (b)).

## Returned to Queue — unimplemented

**None.** All 10 keys are implemented by this packet.

## Ruled Dead-in-Canonical

**None.** Every one of the 10 keys has a read site inside OrcaSlicer's slicing pipeline under `src/libslic3r/` (the consumer functions tabulated above), not merely in `ConfigManipulation.cpp`, GUI tooltips, preset plumbing, or an `IGNORE`/legacy-alias set.

## In Scope

1. **`lateral-lattice-infill`** — new core module crate (`Cargo.toml`, `src/lib.rs`, `lateral-lattice-infill.toml`, `wit-guest/`, `tests/lateral_lattice_infill_tdd.rs`), `LayerModule` at `Layer::Infill`, holds `claim:sparse-fill` only. Ports canonical `FillLateralLattice::fill_surface`: two vertical (π/2) line families with z-proportional horizontal shifts, fixed layer angle 0, odd-layer polyline reversal, clipped to `sparse_infill_area`.
2. **`lateral-honeycomb-infill`** — new core module crate with the same shape (`tests/lateral_honeycomb_infill_tdd.rs`). Ports canonical `FillLateralHoneycomb::fill_surface`: vertical period derived from `infill_overhang_angle`, one third of each period emitting the two splayed "double line" families at interpolated ±`horizontal_position`, two thirds emitting the single stem line, per-case density rescale so material per layer is constant, half-period stagger on alternate periods, odd-layer reversal.
3. **`locked-zag-infill`** — new core module crate (`tests/locked_zag_infill_tdd.rs`, plus the shared manifest guard `tests/infill_pattern_specific_config_schema_tdd.rs`). Ports canonical `FillLockedZag`: skin/skeleton decomposition by `skin_infill_depth`, skeleton dilation by `infill_lock_depth`, independent densities and independent extrusion widths for the two zones, and the `symmetric_infill_y_axis` mirror.
4. **Manifest ownership of the 10 keys**, each declared only on the module that reads it, with canonical types/defaults/bounds and a `description` naming the canonical consumer function (AC-13); plus the four shared base tables (`sparse_infill_density`, `infill_direction`, `line_width`, `sparse_infill_speed`) each new module reads for spacing, base angle, width, and speed.
5. **Registration**: workspace members in the root `Cargo.toml`; `crates/slicer-integrated-modules/Cargo.toml` optional deps + features; `crates/slicer-integrated-modules/src/lib.rs` `manifest_const!` + `integrated_registry!` entries and their `#[cfg(not(feature = …))]` arms; `crates/pnp-cli/Cargo.toml` `integrated-<name>` passthrough features (required by `xtask/src/dist.rs`'s per-module passthrough check).
6. **Test-suite fallout**: the core-module count assertion in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` (+3, value re-derived from the test at implementation time); claim-resolution arms in `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs`; bounds/type arms in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`.
7. **Generated docs**: `cargo xtask gen-config-docs` regeneration of `docs/15_config_keys_reference.md`.

## Out of Scope

- Any behaviour change to `rectilinear-infill`, `gyroid-infill`, or `lightning-infill` — their manifests and sources are untouched (AC-N1 pins that the 10 keys are absent from all three).
- `fill_multiline` / `multiline` support in the three new modules (packet 262a owns `fill_multiline`; canonical's `fill_surface_by_multilines` multiline path is deliberately not ported here — the new modules emit the single-line case).
- The zigzag and crosszag patterns, which are canonical's other two `symmetric_infill_y_axis` activators. Until one of them ships, the key's structural gate is "declared only on `locked-zag-infill`". A future packet shipping zigzag/crosszag adds the key to that module's manifest and updates AC-N1's guard.
- `ORCA_CONFIG_PADDING` and every CONFIG_BLOCK twin (Authoring rule 2; AC-N2 asserts a zero-line diff).
- Any WIT interface change, IR schema version bump, or new `ResolvedConfig` field — none is required (see `design.md` §Contract Notes).
- Canonical's `LockRegionParam` multi-region plumbing through `Layer::make_fills` for *painted* per-region density/flow overrides; this packet resolves skin/skeleton density and width from the region's own config only.

## Authoritative Docs

- `docs/01_system_architecture.md` § Claim System — claim ownership and first-winner dedup.
- `docs/04_host_scheduler.md` § Claim Resolution — how `sparse_fill_holder` selects the `claim:sparse-fill` producer per region.
- `docs/03_wit_and_manifest.md` § Known claim IDs; `[config.schema]` table shape.
- `docs/adr/0056-integrated-modules-native-dispatch.md` — integrated-registry registration contract for a new core module.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; every canonical constant divided by 100.
- `docs/15_config_keys_reference.md` — generated; `cargo xtask gen-config-docs` / `--check`.
- `CLAUDE.md` § Guest WASM Staleness — `cargo xtask build-guests --check` exit-code contract for the three new guests.

## Parity Evidence Standard

Every canonical claim in this packet is cited by **file + function name**, never by line number. A worker disputing a claim re-dispatches the read per the delegation contract below and records the correction in `design.md` §Locked Assumptions, rather than editing an AC in place.

## Per-Key Canonical Evidence

Canonical declarations, all on `PrintObjectConfig` in `src/libslic3r/PrintConfig.cpp` (dispatched read; types/defaults/bounds carried forward from the previous revision's grounding and unchanged by this re-authoring):

| Key | Canonical type | Default | Bounds | Consumer (file · function) |
| --- | --- | --- | --- | --- |
| `infill_lock_depth` | `coFloat` | 1 | 0 … 100 | `FillRectilinear.cpp` · `FillLockedZag::fill_surface_locked_zag` |
| `infill_overhang_angle` | `coFloat` | 60 | 15 … 75 | `FillRectilinear.cpp` · `FillLateralHoneycomb::fill_surface` |
| `lateral_lattice_angle_1` | `coFloat` | −45 | −75 … 75 | `FillRectilinear.cpp` · `FillLateralLattice::fill_surface` |
| `lateral_lattice_angle_2` | `coFloat` | 45 | −75 … 75 | `FillRectilinear.cpp` · `FillLateralLattice::fill_surface` |
| `skeleton_infill_density` | `coPercent` | 25 % | 0 … 100 | `FillRectilinear.cpp` · `FillLockedZag::fill_surface_locked_zag` |
| `skeleton_infill_line_width` | `coFloatOrPercent` | 100 % | ≥ 0 | `FillRectilinear.cpp` · `FillLockedZag::fill_surface_extrusion` |
| `skin_infill_density` | `coPercent` | 25 % | 0 … 100 | `FillRectilinear.cpp` · `FillLockedZag::fill_surface_locked_zag` |
| `skin_infill_depth` | `coFloat` | 2 | 0 … 100 | `FillRectilinear.cpp` · `FillLockedZag::fill_surface_locked_zag` |
| `skin_infill_line_width` | `coFloatOrPercent` | 100 % | ≥ 0 | `FillRectilinear.cpp` · `FillLockedZag::fill_surface_extrusion` |
| `symmetric_infill_y_axis` | `coBool` | false | — | `Fill.cpp` · `Layer::make_fills` (activation) + `FillRectilinear.cpp` · `fill_surface_by_lines` (mirror-back) |

Port conventions applied to the declarations (each a recorded divergence in `design.md`): the two `coPercent` keys become `float` percent numbers per the ticket-107 convention; the two `coFloatOrPercent` width keys become `float` millimetres with `0.0` meaning "inherit the region line width", matching the existing `line_width` tables in `gyroid-infill.toml` / `rectilinear-infill.toml` rather than canonical's `Flow::new_from_config_width` percent-of-nozzle resolution.

## Acceptance Summary

| AC | Subject | Key(s) proved live at a non-default value |
| --- | --- | --- |
| AC-1 | three modules discovered, stage/IR-access/claims correct | — (registration) |
| AC-2 | `sparse_fill_holder` selects each new module | — (holder resolution) |
| AC-3 | lattice family-1 shift | `lateral_lattice_angle_1` |
| AC-4 | lattice family-2 shift + odd-layer reversal | `lateral_lattice_angle_2` |
| AC-5 | honeycomb vertical period | `infill_overhang_angle` |
| AC-6 | skeleton dilation (interlock) | `infill_lock_depth` |
| AC-7 | skin/skeleton split depth | `skin_infill_depth` |
| AC-8 | skin line spacing | `skin_infill_density` |
| AC-9 | skeleton line spacing | `skeleton_infill_density` |
| AC-10 | skin per-vertex width | `skin_infill_line_width` |
| AC-11 | skeleton per-vertex width | `skeleton_infill_line_width` |
| AC-12 | mirror about object bbox centre x | `symmetric_infill_y_axis` |
| AC-13 | manifest schema guard (types/defaults/bounds/ownership) | all 10 |
| AC-14 | bounds + type rejection | 6 numeric keys + the bool |
| AC-15 | generated docs rows + unchanged deviation-row count | all 10 |
| AC-N1 | keys absent from the three existing fill manifests | all 10 (structural gate) |
| AC-N2 | zero `ORCA_CONFIG_PADDING` diff | all 10 (rule 2) |
| AC-N3 | new modules emit no solid/bridge paths | — (claim scope) |

## Verification Matrix

| Command | Covers |
| --- | --- |
| `cargo test -p lateral-lattice-infill --test lateral_lattice_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-3, AC-4 |
| `cargo test -p lateral-honeycomb-infill --test lateral_honeycomb_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-5 |
| `cargo test -p locked-zag-infill --test locked_zag_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-6 … AC-12 |
| `cargo test -p locked-zag-infill --test infill_pattern_specific_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-13, AC-N1 |
| `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1 |
| `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-14 |
| `cargo test -p slicer-runtime --test contract native_infill_claim_resolution 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2, AC-N3 |
| `cargo xtask gen-config-docs --check` + the AC-15 key loop | AC-15 |
| `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs \| grep -E "^[+-]" \| grep -cE "infill_lock_depth\|infill_overhang_angle\|lateral_lattice_angle\|skeleton_infill\|skin_infill\|symmetric_infill_y_axis"` | AC-N2 |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness for the three new guests |
| `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` | packet gates |

## Step Completion Expectations

- Each new module crate is complete (manifest + source + guest wrapper + tests) before its behaviour ACs are claimed; a module whose guest does not build leaves `cargo xtask build-guests --check` non-zero and blocks closure.
- Registration (workspace member, integrated registry, pnp-cli passthrough feature) lands in the same step as the crate it registers, so `cargo check --workspace --all-targets` is green at every step boundary.
- The module-count assertion is updated in the same step that adds the third module, and its new value is re-derived by running the test — never by arithmetic on a quoted number.
- Docs regeneration is the last step; the deviation-block row count is captured from disk immediately before the first manifest edit and compared after.

## Context Discipline Notes

- Read budget: standard 120k band. `design.md` §Context Cost Estimate carries the per-step roll-up.
- Never open `OrcaSlicerDocumented/` directly — dispatch per the obligations below.
- `modules/core-modules/rectilinear-infill/src/lib.rs` and `modules/core-modules/gyroid-infill/src/lib.rs` are **read-only reference** for module shape (trait impl, `#[slicer_module]`, `InfillOutputBuilder` usage); they must not be edited.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `FillLateralLattice::fill_surface`, `FillLateralHoneycomb::fill_surface`, `FillLockedZag::fill_surface_locked_zag`, `FillLockedZag::fill_surface_extrusion`, `FillLockedZag::set_lock_region_param`, `fill_surface_by_multilines`, `fill_surface_by_lines` (the `symmetric_y` rotate-back branch).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` (`LockRegionParam` population, `symmetric_infill_y_axis` activation and mirror axis).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Flow::new_from_config_width`.
- `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp` — `MultiPoint::symmetric_y`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the 10 key declarations on `PrintObjectConfig`.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.
