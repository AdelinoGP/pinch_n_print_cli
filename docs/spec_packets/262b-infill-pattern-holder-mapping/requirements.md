# Requirements: infill-pattern-holder-mapping

## Packet Metadata

- Packet directory: `docs/spec_packets/262b-infill-pattern-holder-mapping/`
- Slug: `infill-pattern-holder-mapping`
- Status: `draft`
- Tier: **C** (re-derived — the packet ships three new modules, a new claim, and a new config-resolution decision point; see `design.md` §Tier Derivation)
- Backlog source: `docs/specs/orca-feature-gap/issues/15-author-packet-p08-strength-infill-infill-modules.md`
- Re-authored under the map's **Authoring rules 1–6**; the 262a/262b split was approved this session (210a/210b, 238a/238b precedent).

## Problem Statement

The pre-split packet declared `sparse_infill_pattern` and `internal_solid_infill_pattern` with-gap, reasoning that "the port's pattern IS module identity". That reasoning is right and the conclusion was wrong: module identity *is* the claim-holder mechanism, so the correct port of a pattern enum is a value→holder mapping plus one module per shipped value (map Authoring rule 4). Declaring the enum on one module and marking it with-gap covers nothing (rule 1).

`gap_fill_target` was likewise declared with-gap on the reasoning that the port's gap fill is the perimeter-side `process_classic` mechanism, which canonical's key does not gate. That is accurate — and it means the key needs a **fill-side** pass to gate. Canonical's `Fill::_create_gap_fill` is a self-contained medial-axis pass over the area the fill lines did not cover, and this port already has every primitive it needs (`slicer_sdk::host::medial_axis`, `slicer_ir::variable_width`, `ExtrusionRole::GapFill`, and the `Layer::InfillPostProcess` stage). So the packet builds it rather than shedding the key.

## Key Disposition Table

Classification: **(a)** live decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue; **(d)** dead-in-canonical.

| Key | Class | Owner | Decision point this packet builds | Non-default AC |
| --- | --- | --- | --- | --- |
| `sparse_infill_pattern` | **(b)** | host — `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`) | value → `claim:sparse-fill` holder (`sparse_fill_holder`), with `crosshatch-infill` shipped as a new module; unshipped values rejected by name | AC-2, AC-4, AC-N3 |
| `internal_solid_infill_pattern` | **(b)** | host — `resolve_global_config` | value → `claim:top-fill` holder (`top_fill_holder`), with `monotonic-infill` shipped as a new module; unshipped values rejected by name | AC-3, AC-5, AC-N3 |
| `gap_fill_target` | **(b)** | `infill-gap-fill` (new module, `Layer::InfillPostProcess`) | region scope of a new medial-axis fill-side gap-fill pass | AC-6 |

Counts: **(a) 0 · (b) 3 · (c) 0 · (d) 0.** Zero declaration-only keys (map gate (a)); every key has at least one AC asserting a behaviour change at a non-default value (map gate (b)).

## Returned to Queue — unimplemented

**None.** Together with packet 262a, every key on the P08 list is implemented.

The *unshipped enum values* are a different thing from an unimplemented key and are not "returned": the key is live, and a value the port does not implement is **rejected by name** at config resolution (AC-N3) rather than silently accepted. Shipped values this packet supports:

| Key | Shipped values → module |
| --- | --- |
| `sparse_infill_pattern` | `rectilinear` → `rectilinear-infill`; `gyroid` → `gyroid-infill`; `lightning` → `lightning-infill`; `crosshatch` → `crosshatch-infill` (new) |
| `internal_solid_infill_pattern` | `rectilinear` → `rectilinear-infill`; `monotonic` → `monotonic-infill` (new) |

A future packet adds values by adding modules and one table row each. Packet 263's three pattern modules are the natural next entries (`lockedzag`, `lateral_lattice`, `lateral_honeycomb`) once both packets land.

## Ruled Dead-in-Canonical

**None.** All three keys have read sites inside OrcaSlicer's slicing pipeline under `src/libslic3r/`: the two pattern enums through `Fill::new_from_type` and `Layer::make_fills`, and `gap_fill_target` in `Fill::_create_gap_fill` (`Fill/FillBase.cpp`), called from `Fill::fill_surface_extrusion`.

## In Scope

1. **Pattern→holder derivation** in `resolve_global_config`: after the existing key loop and schema-defaults pass, an explicitly supplied `sparse_infill_pattern` / `internal_solid_infill_pattern` maps to the corresponding holder field unless that holder was supplied explicitly (explicit wins). An unshipped value is a `ConfigResolutionError` naming the key, the value, and the shipped list.
2. **`crosshatch-infill`** — new `claim:sparse-fill` module porting canonical `FillCrossHatch`: z-driven period with an orthogonal direction flip each period, straight parallel lines inside repeat bands, four-point zig-zag cycles inside transition bands, grid alignment for phase coherence, short-polyline drop.
3. **`monotonic-infill`** — new `claim:top-fill` module emitting solid fill in monotonic sweep order (all lines in one direction, sweep coordinate non-decreasing).
4. **`infill-gap-fill`** — new `Layer::InfillPostProcess` module holding the new `claim:infill-gap-fill`, reading `InfillIR` + `PerimeterIR` + `RegionMapIR` and writing `InfillIR`. Gated by `gap_fill_target`; computes the area the fill lines did not cover, extracts the gap band, runs `slicer_sdk::host::medial_axis`, and appends `ExtrusionRole::GapFill` paths via `slicer_ir::variable_width`.
5. **`GapFill` passthrough in `infill-linker`** — `GapFill`-role paths in the sparse/solid buckets are re-emitted verbatim instead of being clipped by `RoleBoundaries::for_role`'s catch-all arm and dropped by `remove_short_polylines`; modelled on the existing `InfillLinker::copy_ironing`. No other linker behaviour changes.
6. **Registration** of the three modules (workspace members, integrated registry, `pnp-cli` passthrough features) and the module-count assertion.
7. **Docs**: the pattern→holder mapping in `docs/04_host_scheduler.md` §Claim Resolution; the `claim:infill-gap-fill` row in `docs/03_wit_and_manifest.md` §Known claim IDs; regeneration of `docs/15_config_keys_reference.md`.
8. **Tests**: a new `config_resolution_pattern_holder.rs` under `crates/slicer-scheduler/tests/integration/` **plus its `mod` registration in that directory's `main.rs`** (the binary is aggregated — an unregistered file silently compiles to zero tests); per-module behaviour suites; a manifest guard for `gap_fill_target`; bounds arms; claim-resolution arms.

## Out of Scope

- The four angle/multiline keys — packet 262a.
- The remaining 22 sparse and 6 solid canonical enum values: each needs its own module. Rejected by name, not silently mapped (AC-N3).
- Canonical's ant-colony `chain_monotonic_regions` optimisation and `pinch_contours_insert_phony_outer_intersections` — see `design.md` DIV-3.
- `filter_out_gap_fill` (canonical's minimum gap-fill length) — not on P08's key list; this port applies no length filter, recorded as `design.md` DIV-5.
- Changing any default: `sparse_fill_holder` / `top_fill_holder` keep `"rectilinear-infill"` even though canonical's pattern defaults are `crosshatch` / `monotonic` (`design.md` DIV-1). AC-N1 pins default-print identity.
- `ORCA_CONFIG_PADDING` and every CONFIG_BLOCK twin, including the previous revision's `"grid"` → `"crosshatch"` correction (Authoring rule 2; AC-N2 asserts a zero-line diff).
- Any WIT interface change, IR schema bump, or new `ResolvedConfig` field.

## Authoritative Docs

- `docs/01_system_architecture.md` § Claim System; `docs/04_host_scheduler.md` § Claim Resolution (edited by this packet).
- `docs/03_wit_and_manifest.md` § Known claim IDs (edited by this packet), `[config.schema]` shape.
- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md`, `docs/adr/0028-infill-postprocess-contract-prior-ir-and-partitioned-polygons.md` — the `Layer::InfillPostProcess` contract.
- `docs/adr/0056-integrated-modules-native-dispatch.md` — new-module registration.
- `docs/08_coordinate_system.md`; `docs/15_config_keys_reference.md` (generated).

## Parity Evidence Standard

Canonical is cited by file + function name, never line number. A worker disputing a claim re-dispatches the read and records the correction in `design.md` §Locked Assumptions rather than editing an AC in place.

## Per-Key Canonical Evidence

| Key | Canonical type | Default | Consumer (file · function) |
| --- | --- | --- | --- |
| `sparse_infill_pattern` | `coEnum InfillPattern` | `crosshatch` | `Fill/FillBase.cpp` · `Fill::new_from_type`; `Fill/Fill.cpp` · `Layer::make_fills` (pattern-gated params) |
| `internal_solid_infill_pattern` | `coEnum InfillPattern` (solid subset) | `monotonic` | `Fill/FillBase.cpp` · `Fill::new_from_type` → `FillMonotonic::fill_surface` / `FillMonotonicLines::fill_surface` |
| `gap_fill_target` | `coEnum GapFillTarget` | `nowhere` (`gftNowhere`) | `Fill/FillBase.cpp` · `Fill::_create_gap_fill`, called from `Fill::fill_surface_extrusion` |

Canonical semantics carried into the ACs: `gftEverywhere` = gap fill on top, bottom and internal solid; `gftTopBottom` = skipped when `surface_type == stInternalSolid`; `gftNowhere` = disabled. The gap band is `[0.2 · scaled_spacing · (1 − INSET_OVERLAP_TOLERANCE), 2 · scaled_spacing]` (`INSET_OVERLAP_TOLERANCE = 0.4`, canonical `libslic3r/libslic3r.h`; defined locally by this packet — see `design.md` §Architecture Constraints), and gap fill runs only when `params.density >= 1`.

## Acceptance Summary

| AC | Subject | Key proved live at a non-default value |
| --- | --- | --- |
| AC-1 | three modules discovered with correct stage/claims | — (registration) |
| AC-2 | sparse pattern → sparse holder, precedence, rejection | `sparse_infill_pattern` |
| AC-3 | solid pattern → top holder, precedence, rejection | `internal_solid_infill_pattern` |
| AC-4 | crosshatch z-driven period, flip, transition morph | `sparse_infill_pattern` (value `crosshatch`) |
| AC-5 | monotonic sweep order vs rectilinear alternation | `internal_solid_infill_pattern` (value `monotonic`) |
| AC-6 | gap fill emitted / scoped / suppressed per value | `gap_fill_target` |
| AC-7 | gap fill is order-insensitive w.r.t. `infill-linker` | `gap_fill_target` |
| AC-8 | `gap_fill_target` schema and sole ownership | `gap_fill_target` |
| AC-9 | enum/type rejection in the bounds index | `gap_fill_target` |
| AC-10 | generated docs row + unchanged deviation-row count | `gap_fill_target` |
| AC-11 | hand-maintained docs carry the mapping and the claim row | both pattern keys |
| AC-12 | linker re-emits `GapFill` verbatim (both-direction order independence) | `gap_fill_target` |
| AC-N1 | default print byte-identical (additional evidence only) | all three |
| AC-N2 | zero `ORCA_CONFIG_PADDING` diff | rule 2 |
| AC-N3 | unshipped values rejected by name | both pattern keys |
| AC-N4 | new modules emit only their claimed role | — (claim scope) |

## Verification Matrix

| Command | Covers |
| --- | --- |
| `cargo test -p slicer-scheduler --test scheduler_integration config_resolution_pattern_holder 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2, AC-3, AC-N3 |
| `cargo test -p crosshatch-infill --test crosshatch_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-4 |
| `cargo test -p monotonic-infill --test monotonic_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-5 |
| `cargo test -p infill-gap-fill --test infill_gap_fill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-6, AC-7 |
| `cargo test -p infill-gap-fill --test infill_gap_fill_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-8 |
| `cargo test -p infill-linker --test gap_fill_passthrough_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-12 |
| `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1 |
| `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-9 |
| `cargo test -p slicer-runtime --test contract native_infill_claim_resolution 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N4 |
| `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N1 |
| `cargo xtask gen-config-docs --check` + the AC-10 row probe | AC-10 |
| the AC-11 `rg` chain | AC-11 |
| `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs \| grep -cE "^[+-][^+-]"` | AC-N2 |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness for the three new guests |
| `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals` | packet gates |

## Step Completion Expectations

- The new scheduler test file and its `mod` registration in `crates/slicer-scheduler/tests/integration/main.rs` land in the same step; an unregistered file compiles to zero tests and reports a false pass.
- A pattern value enters the mapping table only in the step that ships its module; the table never names a module that does not exist.
- The gap-fill module's inert path (`gap_fill_target = "nowhere"`) is proved before its active path, so a regression in the default print is caught first.
- The deviation-block row count is captured from disk immediately before the first manifest edit and re-compared in the final step.
- The module-count assertion is updated in the same step that adds the third module, with its new value re-derived by running the test.

## Context Discipline Notes

- Read budget: standard 120k band. `crates/slicer-scheduler/src/config_resolution.rs` is read ranged around `resolve_global_config`, never in full.
- Never open `OrcaSlicerDocumented/` directly — dispatch per the obligations below.
- `modules/core-modules/{rectilinear-infill,gyroid-infill,lightning-infill}/**` are read-only reference for module shape and the post-pass contract. `modules/core-modules/infill-linker/**` is read-only reference **except** for the `GapFill` passthrough in `src/lib.rs` and its new test (In Scope item 5, Step 5b, AC-12); the exemption from the ADR-0025 containment re-clip is recorded as `design.md` DIV-7.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillCrossHatch.cpp` — `FillCrossHatch::_fill_surface_single`, `generate_infill_layers`, `generate_repeat_pattern`, `generate_transform_pattern`, `generate_one_cycle`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `FillMonotonic::fill_surface`, `FillMonotonicLines::fill_surface`, the `params.monotonic` branch of `fill_surface_by_lines`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Fill::_create_gap_fill`, `Fill::fill_surface_extrusion`, `Fill::new_from_type`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` / `PrintConfig.hpp` — the two pattern value lists and `enum GapFillTarget`.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.
