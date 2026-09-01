# Design: infill-pattern-holder-mapping

## Tier Derivation

**Tier C.** The packet builds three new modules (two fill algorithms and a post-pass), a new claim ID, and a new decision point in host config resolution. That is the top of the B/C range. The pre-split packet's Tier A rating was a consequence of the prohibited declaration-only disposition and is void.

## Approach

Three separable mechanisms:

### 1. Pattern → claim-holder derivation (host, no new field)

`ResolvedConfig` already carries `sparse_fill_holder`, `top_fill_holder`, `bottom_fill_holder`, `bridge_fill_holder` (`crates/slicer-ir/src/resolved_config.rs`, all `String`, all defaulting to `"rectilinear-infill"`). The pattern keys therefore need **no new field**: they are an alternative *spelling* of the holder, resolved once at `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`), immediately after the existing key loop and the `bounds.schema_defaults()` pass and before `Ok(cfg)`:

```text
if source has "sparse_infill_pattern" and not "sparse_fill_holder":
    cfg.sparse_fill_holder = map(value)?          // else ConfigResolutionError
if source has "internal_solid_infill_pattern" and not "top_fill_holder":
    cfg.top_fill_holder = map(value)?
```

Precedence is "explicit holder wins", so every existing config, test, and profile keeps its current behaviour. The keys are not declared in any module manifest, because no module reads them — they are host selection keys, exactly like the holder keys themselves. Their discoverability home is `docs/04_host_scheduler.md` §Claim Resolution (AC-11), not the generated module-key table.

`map()` is a total function over the **shipped** values only; anything else returns an error naming the key, the value, and the shipped list. Silent fallback to rectilinear would reintroduce exactly the "the key does nothing" failure the Authoring rules exist to stop.

### 2. Two shipped pattern modules

- `crosshatch-infill` holds `claim:sparse-fill` and ports canonical `FillCrossHatch`. Its defining property, and the thing AC-4 pins, is that the pattern is a function of **absolute z**, not `layer_index`: `generate_infill_layers` derives a period from the grid and `repeat_ratio`, flips the line direction 90° each period, emits straight parallel lines inside the repeat band, and morphs through four-point zig-zag cycles inside the transition band.
- `monotonic-infill` holds `claim:top-fill` and emits solid fill in monotonic sweep order.

### 3. Fill-side gap fill as a post-pass module

`infill-gap-fill` sits at `Layer::InfillPostProcess`, holds the new `claim:infill-gap-fill`, reads `InfillIR` + `PerimeterIR` + `RegionMapIR`, writes `InfillIR`. Per layer region, when `gap_fill_target` selects the region's surface scope:

1. Build the covered area: for each emitted fill path, the union of per-segment quads of width = line spacing (the port's stand-in for canonical `polygons_covered_by_spacing`), unioned with `slicer_sdk::host::clip_polygons`.
2. `unextruded = region fill area − covered`, then the gap band `diff(opening(unextruded, min/2), offset2(unextruded, −max/2, +max/2))` with `min = 0.2 · spacing · (1 − INSET_OVERLAP_TOLERANCE)` and `max = 2 · spacing`.
3. `slicer_sdk::host::medial_axis(gap, min, max)` → `Vec<ThickPolyline>`; each becomes an `ExtrusionPath3D` via `slicer_ir::variable_width(&thick, ExtrusionRole::GapFill)` with the layer Z applied, appended to the region.

**Complete-replacement contract (ADR-0028, Option 1b).** `LayerModule::run_infill_postprocess(&self, layer_index, regions: &[PerimeterRegionView], prior_infill: &[InfillRegion], output, config)` (`crates/slicer-sdk/src/traits.rs`) obliges the module to emit the **complete** replacement `InfillIR`, re-emitting every bucket it did not transform. "Appends" above is shorthand for "re-emits `prior_infill` verbatim and adds the gap-fill paths". Under `gap_fill_target = "nowhere"` the module must re-emit `prior_infill` unchanged — emitting nothing would delete every infill path on the layer. AC-6's "adds exactly zero paths and re-emits `prior_infill` verbatim" clause is exactly this assertion, and Step 5's exit condition names it.

The partitioned fill polygons the coverage computation subtracts from (`sparse_infill_area`, `top_solid_fill`, `bottom_solid_fill`, `bridge_areas`) reach the module on the `PerimeterRegionView` slice passed to `run_infill_postprocess`; the host populates them by region enrichment sourced from `SliceIR`, per ADR-0028 — they are not carried *by* `PerimeterIR`. The `reads = ["InfillIR", "PerimeterIR", "RegionMapIR"]` list is the `infill-linker` precedent (`modules/core-modules/infill-linker/infill-linker.toml` declares exactly that set for the same stage), not a derivation from where the polygons come from.

**The linker must not re-process gap fill.** `infill-linker` orchestrates every role it finds in the sparse/solid buckets: `RoleBoundaries::for_role` (`modules/core-modules/infill-linker/src/orchestrate.rs`) has a catch-all arm that would route `ExtrusionRole::GapFill` to the union-boundary fallback, and `remove_short_polylines` (`modules/core-modules/infill-linker/src/offset.rs`) would then drop the short medial-axis chains gap fill is made of. Canonical never links gap fill — `variable_width(..., erGapFill, ...)` output goes straight into the extrusion collection. This packet therefore adds a **verbatim passthrough** for `GapFill`-role paths in `infill-linker`, modelled exactly on the existing `InfillLinker::copy_ironing` passthrough (`modules/core-modules/infill-linker/src/lib.rs`). With that in place the two modules are order-independent in **both** directions, which is what makes the intra-stage ordering question moot rather than merely untested. AC-12 pins it.

**Ordering:** the module is deliberately **order-insensitive** with respect to `infill-linker` inside the same stage. It measures covered *area*, and linking concatenates segments without changing the area they cover. AC-7 pins this by running the module on a linked and an unlinked form of the same fixture and asserting identical output. This is why the manifest declares `requires = []` rather than trying to express an intra-stage ordering the scheduler does not provide: `[claims] requires` is a presence check and `requires-modules` is a *cross-stage* legality check (`validate_cross_stage_dependencies`, `crates/slicer-scheduler/src/validation.rs`) — neither orders two modules inside one stage.

## What Carries the New Data

- **Pattern keys** → the existing `sparse_fill_holder` / `top_fill_holder` `ResolvedConfig` fields, written by the derivation. No new field.
- **`gap_fill_target`** → the `infill-gap-fill` manifest `[config.schema]` → `ConfigView`.
- **Gap-fill geometry** → `ExtrusionPath3D` with `ExtrusionRole::GapFill`, which already exists in `crates/slicer-ir/src/slice_ir.rs`. No IR schema bump.
- **New claim ID** `claim:infill-gap-fill` → a manifest string plus one documentation row. Non-fill claims are not enumerated in code (`FILL_CLAIM_IDS` in `crates/slicer-scheduler/src/validation.rs` lists only the four fill claims; `claim:infill-link` is likewise code-free), so no scheduler production change is required.

There is no `[BLOCK]` in this packet: no new WIT interface, no IR schema bump, no new host `ResolvedConfig` field.

## Recorded Divergences

- **DIV-1 — the port's default pattern stays rectilinear.** Canonical defaults `sparse_infill_pattern` to `crosshatch` and `internal_solid_infill_pattern` to `monotonic`; this port keeps `sparse_fill_holder` / `top_fill_holder` at `"rectilinear-infill"` and derives a holder only from an explicitly supplied pattern key. Rationale: adopting canonical's defaults would change every existing print and every parity baseline in one packet, for a reason unrelated to the keys being live. Recorded as a divergence, pinned by AC-N1; changing the default is a separate, deliberate decision.
- **DIV-2 — unshipped enum values are rejected, not approximated.** Canonical accepts 26 sparse and 8 solid values. The port ships 4 and 2 and errors by name on the rest. Rationale: a slicer that silently substitutes a different infill pattern produces a part the user did not ask for; an error naming the value and the shipped list is diagnosable. This is the same reasoning as 262a's DIV-1 on rotate templates.
- **DIV-3 — monotonic ordering without the ant colony.** Canonical's monotonic branch runs `generate_montonous_regions`, `connect_monotonic_regions`, and an ant-colony `chain_monotonic_regions` with `monotonic_3_opt` to minimise travel between monotonic blocks. This port implements the *observable* contract — lines emitted in non-decreasing sweep order, all in one direction — and leaves travel optimisation to the existing `path-optimization-default` module, which owns ordering in this architecture. Rationale: reproducing a ~1000-line travel optimiser inside a fill module would duplicate a responsibility the port has already factored out. Recorded, with the consequence stated plainly: travel time for monotonic solid fill may exceed canonical's until the ordering module is taught about monotonic blocks.
- **DIV-4 — `internal_solid_infill_pattern` maps to `claim:top-fill`.** The port has four fill-role claims and no dedicated internal-solid claim, so the solid-pattern selection rides the top-fill holder. Rationale: it is the mapping the map's ruling names, and adding a fifth fill claim is a larger architectural change than this key warrants. Recorded so a future packet that introduces an internal-solid claim knows to re-point the mapping.
- **DIV-5 — no gap-fill length filter.** Canonical drops `ThickPolyline`s shorter than `filter_out_gap_fill`. That key is not on P08's list and this packet does not declare it, so no length filter is applied. Recorded rather than silently omitted.
- **DIV-6 — gap fill is a post-pass, not part of each fill module.** Canonical calls `_create_gap_fill` from `Fill::fill_surface_extrusion`, so every pattern class inherits it. Porting that shape would mean the same code in every fill module. A `Layer::InfillPostProcess` claim holder gives one implementation for every present and future pattern, and makes gap fill replaceable by a community module. This is the port improving on canonical's coupling, per Authoring rule 4.
- **DIV-7 — `GapFill` is exempt from the linker's per-role containment re-clip (ADR-0025 amendment 2026-07-24).** That amendment makes containment part of the linker's contract and warns specifically against collapsing the two `for_role` outcomes: *"`None` means no boundary could be resolved and the paths pass through untouched (the historical behaviour); `Some(empty)` means the host partitioned the region and gave this role no area, so the role's paths have nowhere legal to go and clip away. Collapsing those two into 'empty ⇒ pass through' would have turned this fix into a new leak for roles with an empty polygon."* Today `ExtrusionRole::GapFill` reaches `for_role`'s `_ => None` arm and then falls to the union-boundary fallback, so it *is* clipped — and `remove_short_polylines` then drops the short medial-axis chains gap fill consists of. This packet exempts the role **before** boundary resolution, as a whole-role verbatim passthrough alongside `InfillLinker::copy_ironing`, not by collapsing `None` into `Some(empty)`; every other role's containment is untouched. The exemption is safe because gap fill is **contained by construction**: `infill-gap-fill` derives it from `unextruded = region fill area − covered`, which is a subset of the region's own fill area, so the union re-clip can only remove geometry that was already legal. Rationale: containment exists to stop a linker-invented connector escaping its region; a medial-axis chain that never left is not that hazard, and clipping it costs the feature. AC-12 pins the passthrough; the pre-existing linker suites pin that no other role moved.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Crosshatch's period arithmetic is in **absolute z (mm)** and crosses the unit boundary once, at grid alignment. Getting this wrong makes the pattern a function of layer height rather than of z, which AC-4's "same z, different `layer_index`" clause is designed to catch.
- The gap band constants (`0.2 · spacing`, `2 · spacing`) are relative to spacing, so they carry no canonical absolute constant to divide by 100. **`INSET_OVERLAP_TOLERANCE` has no Rust definition in this tree** — it exists only as canonical's `static constexpr double INSET_OVERLAP_TOLERANCE = 0.4;` (`libslic3r/libslic3r.h`), recorded in a comment in `modules/core-modules/classic-perimeters/src/lib.rs`. This packet therefore **defines it locally** in `modules/core-modules/infill-gap-fill/src/lib.rs` as `const INSET_OVERLAP_TOLERANCE: f32 = 0.4;` with that canonical citation, and does not import one.
- snake_case config key strings only.
- ADR-0028: the post-pass consumes prior IR and the host-partitioned polygons; `infill-gap-fill` must not re-derive fill areas from raw slices.
- ADR-0025/0026: `infill-linker` remains the sole holder of `claim:infill-link`; this packet adds a distinct claim and does not contend for it.
- ADR-0027: gyroid's four claims and the default holders are unchanged; `sparse_infill_pattern = "gyroid"` is exactly the opt-in path ADR-0027 describes.
- ADR-0056: each new module needs an integrated-registry entry and a `pnp-cli` `integrated-<name>` passthrough feature, or `cargo xtask dist --edition integrated` fails.

## Code Change Surface

**New files** (per module `M` in {`crosshatch-infill`, `monotonic-infill`, `infill-gap-fill`}): `modules/core-modules/M/Cargo.toml`, `src/lib.rs`, `M.toml`, `wit-guest/{Cargo.toml,src/lib.rs}`, `tests/<m>_tdd.rs`; plus `modules/core-modules/infill-gap-fill/tests/infill_gap_fill_config_schema_tdd.rs` and a `toml` dev-dependency for it; plus `crates/slicer-scheduler/tests/integration/config_resolution_pattern_holder.rs`.

**Edited:**

- `crates/slicer-scheduler/src/config_resolution.rs` — the derivation in `resolve_global_config` and the shipped-value tables.
- `crates/slicer-ir/src/resolved_config.rs` — **variant-only, narrowly authorized.** `ConfigResolutionError` is defined here (the scheduler merely re-exports it: `pub use slicer_ir::ConfigResolutionError;`), it is **not** `#[non_exhaustive]`, and none of its three variants (`TypeMismatch`, `OutOfRange`, `SupportLayerHeightTooFine`) carries the "unshipped pattern value" case. The preferred resolution is to **reuse `TypeMismatch`** and keep this file untouched. Adding a variant is permitted only if reuse cannot carry the key + value + shipped-list message AC-2/AC-3/AC-N3 require; in that case the step owns the full blast radius — every exhaustive `match` on the enum in production and test code, plus its `Display` impl — and the edit is strictly a new variant. Adding a `ResolvedConfig` **field** remains prohibited and is a `[BLOCK]`.
- `crates/slicer-scheduler/tests/integration/main.rs` — `mod config_resolution_pattern_holder;` (**required**: the binary is aggregated; an unregistered file compiles to zero tests and reports a false pass).
- `crates/slicer-scheduler/tests/integration/{manifest_ingestion_tdd.rs,config_bounds_enforcement_tdd.rs}`.
- `modules/core-modules/infill-linker/src/lib.rs` and `modules/core-modules/infill-linker/tests/gap_fill_passthrough_tdd.rs` (new) — the `GapFill` verbatim passthrough only; no other linker behaviour may change.
- `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs`.
- Root `Cargo.toml`; `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}`; `crates/pnp-cli/Cargo.toml`.
- `docs/04_host_scheduler.md`, `docs/03_wit_and_manifest.md` (hand-maintained), `docs/15_config_keys_reference.md` (regenerated only).
- Three new guest `.wasm` artifacts.

**Blast radius owned by this packet:** the module-count assertion in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` (+3, value re-derived by running the test); if a new `ConfigResolutionError` variant is added, every exhaustive `match` on that enum in production and test code belongs to the same step. No public schema or version constant changes.

## Files in Scope (read + edit)

- The three new `modules/core-modules/M/**` trees.
- `crates/slicer-scheduler/src/config_resolution.rs` and `crates/slicer-scheduler/tests/integration/**`.
- `crates/slicer-ir/src/resolved_config.rs` — **only** under the narrow variant-only carve-out in §Code Change Surface; no `ResolvedConfig` field, ever. Prefer reusing `ConfigResolutionError::TypeMismatch` and leaving the file untouched.
- `modules/core-modules/infill-linker/src/lib.rs` and `modules/core-modules/infill-linker/tests/gap_fill_passthrough_tdd.rs` (new) — the `GapFill` passthrough only; every other file under that module is read-only.
- `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs`.
- Root `Cargo.toml`, `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}`, `crates/pnp-cli/Cargo.toml`.
- `docs/04_host_scheduler.md`, `docs/03_wit_and_manifest.md`.

## Read-Only Context

- `modules/core-modules/lightning-infill/**` and `modules/core-modules/gyroid-infill/**` — crate/manifest/guest shape, sparse-only claim precedent.
- `modules/core-modules/infill-linker/src/orchestrate.rs` and `src/offset.rs` — the `Layer::InfillPostProcess` module shape, `RoleBoundaries::for_role`, and `remove_short_polylines` (read-only; the only linker edit is the passthrough in `src/lib.rs`).
- `modules/core-modules/rectilinear-infill/src/lib.rs` — scan-line emission (the monotonic module's baseline and AC-5's comparison arm).
- `crates/slicer-ir/src/{slice_ir.rs,resolved_config.rs}` — `ExtrusionRole::GapFill`, `ThickPolyline`, `variable_width`, the holder fields.
- `crates/slicer-sdk/src/host.rs` — `medial_axis`, `clip_polygons`, `offset_polygons`.

## Out-of-Bounds Files

- `crates/slicer-gcode/src/serialize.rs` — `ORCA_CONFIG_PADDING`; AC-N2 asserts a zero-line diff.
- `crates/slicer-ir/src/resolved_config.rs` — **no new `ResolvedConfig` field** may be added; a worker who believes one is needed stops and raises a `[BLOCK]`. The single narrowly-authorized exception is the `ConfigResolutionError` variant described in §Code Change Surface, and only when variant reuse cannot carry the required message.
- `crates/slicer-schema/wit/**` and every IR schema constant.
- `modules/core-modules/{rectilinear-infill,gyroid-infill,lightning-infill}/**` — read-only; their behaviour must not change (262a owns the first two). `modules/core-modules/infill-linker/**` is read-only **except** for the `GapFill` passthrough named in §Code Change Surface; `claim:infill-link` stays solely the linker's (ADR-0025).
- `docs/spec_packets/262a-infill-angle-and-multiline-keys/**` and every other packet directory.
- `docs/specs/orca-feature-gap/**`.

## Expected Sub-Agent Dispatches

- `SUMMARY` — canonical `FillCrossHatch::_fill_surface_single` and its four static helpers: exact period/phase arithmetic, the low-density `repeat_ratio` clamp, grid alignment, the transition morph, the `0.8 · spacing` drop (once, Step 3).
- `SUMMARY` — canonical `FillMonotonic::fill_surface` and the `params.monotonic` branch: the observable ordering contract, and confirmation that `anchor_length_max` is what distinguishes `ipMonotonic` from `ipMonotonicLine` (once, Step 4).
- `SUMMARY` — canonical `Fill::_create_gap_fill`: the exact band formula, the `density >= 1` guard, the surface-type check per `GapFillTarget` value, and the ordering/simplification steps (once, Step 5).
- `FACT` — each cargo/xtask run: pass/fail plus failing test names only.

## Locked Assumptions and Invariants

1. `ResolvedConfig` already has `sparse_fill_holder` and `top_fill_holder` as `String` fields with `"rectilinear-infill"` defaults — verified in `crates/slicer-ir/src/resolved_config.rs`.
2. `resolve_global_config` is the single seam where raw config becomes `ResolvedConfig`, and it already ends with a post-loop pass (`bounds.schema_defaults()`), so a derivation step fits without restructuring — verified in `crates/slicer-scheduler/src/config_resolution.rs`.
3. Non-fill claim IDs are not enumerated in code; `FILL_CLAIM_IDS` covers only the four fill claims and `claim:infill-link` needs no code row — verified in `crates/slicer-scheduler/src/validation.rs`.
4. `ExtrusionRole::GapFill` exists; `ThickPolyline` and `variable_width(&ThickPolyline, ExtrusionRole)` exist in `crates/slicer-ir/src/slice_ir.rs`; `slicer_sdk::host::medial_axis(&ExPolygon, f32, f32) -> Result<Vec<ThickPolyline>, String>` exists.
5. `crates/slicer-scheduler/tests/integration/` is an aggregated binary with a `main.rs` mod list — a new file there **must** be registered.
6. Module `tests/*.rs` files in `modules/core-modules/*/` are standalone auto-discovered binaries needing no registration — verified against `modules/core-modules/gyroid-infill/tests/`.
7. `xtask build-guests` discovers core guests by directory walk; `xtask/src/dist.rs` requires one `integrated-<name>` passthrough feature per module.

## Risks and Tradeoffs

- **Covered-area approximation.** The port has no `polygons_covered_by_spacing`; the per-segment quad union is a stand-in. If it over-covers, gaps vanish; if it under-covers, spurious gap fill appears between adjacent lines. AC-6's width-band assertion and AC-N1's default-print identity are the guards. A worker who cannot make the approximation stable must report it rather than loosen the assertion.
- **Rejecting unshipped values is a user-visible behaviour change** for configs that currently carry an ignored `sparse_infill_pattern`. That is the intended correction (DIV-2), but it will surface in profile ingestion; the error message must name the shipped list so the fix is obvious.
- **Three guests to rebuild**; `cargo xtask build-guests --check` must return exit 0 before closure.
- **Crosshatch's z-driven period interacts with variable layer height.** The module reads `SliceRegionView::z()`, so it is correct by construction, but a test that varies layer height and asserts geometry-by-z is the honest guard — AC-4's "same z, different `layer_index`" clause.

## Context Cost Estimate

Aggregate **L** (roll-up of the per-step S/M costs in `implementation-plan.md`). No single step is rated L; each module is its own step.

## Open Questions

- `[FWD]` When packet 263 lands, `lockedzag` / `lateral_lattice` / `lateral_honeycomb` should join the `sparse_infill_pattern` mapping table. Deliberately not pre-registered here — the table never names a module that does not exist.
- `[FWD]` Should the port eventually adopt canonical's `crosshatch` / `monotonic` defaults (DIV-1)? That is a print-behaviour decision with a baseline-rebaseline cost, not a config-plumbing decision, and belongs in its own packet.
- `[FWD]` `filter_out_gap_fill` (DIV-5) and a fifth internal-solid fill claim (DIV-4) are the two natural follow-ups.

No `[BLOCK]` items.
