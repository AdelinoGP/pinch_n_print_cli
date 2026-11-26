# Requirements: top-bottom-surface-keys

## Packet Metadata

- **Packet directory:** `docs/spec_packets/264-top-bottom-surface-keys/`
- **Slug:** `top-bottom-surface-keys`
- **Status:** `draft`
- **Task IDs:** none (queue packet — `task_ids: []`, precedent packets 234a, 253–263)
- **Backlog source:** wayfinder ticket 17 (`docs/specs/orca-feature-gap/issues/17-author-packet-p10-strength-top-bottom-shells-infill-modules.md`), map `docs/specs/orca-feature-gap/map.md` packet P10
- **Tier:** **C** — re-derived. The prior revision was Tier A on the "declare + wire the cheap keys" reading. Under map Authoring rule 1 a packet that *builds* a decision point is B or C; this packet builds six new fill modules plus a host-side selection derivation, which is above the Tier B ceiling of a single-module diff. See `design.md` § Tier Derivation.
- **Re-authoring note:** this directory is overwritten in place (number and slug retained) with explicit user approval, under map Authoring rules 1–6.

## Problem Statement

OrcaSlicer lets a user choose *how* top and bottom solid surfaces are filled (`top_surface_pattern`, `bottom_surface_pattern`, eight filler algorithms each) and *how densely* (`top_surface_density`, `bottom_surface_density`). Pinch 'n Print today does neither: solid spacing is hardcoded (`const SOLID_DENSITY: f32 = 1.0` in `modules/core-modules/rectilinear-infill/src/lib.rs`, used twice inside `RectilinearInfill::run_infill`), and the two pattern keys are read nowhere — a survey of the tree found `top_surface_density` and `bottom_surface_density` present only in documentation and gap inventories, never in Rust or TOML source.

The prior revision of this packet wired the two density keys and declared the two pattern keys "with-gap", on the reasoning that pattern selection is module identity in this port. That reasoning is now inverted by map Authoring rule 4: module identity *is* the claim-holder mechanism, so an Orca enum whose values are different algorithms is exactly a set of `claim:*` holders selected through `top_fill_holder` / `bottom_fill_holder`. Declaring the keys without building the holders covers nothing.

This packet therefore makes all four keys drive behaviour: two density divisors inside the existing solid-fill decision points, and two host-side selection keys resolving onto eight real filler modules.

## Key Disposition Table

Classification per the map's Authoring rules: **(a)** live behaviour-changing decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue (no decision point, not built here); **(d)** dead-in-canonical.

| Key | Class | Owner | Decision point this packet builds | Non-default AC |
| --- | --- | --- | --- | --- |
| `top_surface_density` | **(b)** | `rectilinear-infill` (manifest key) | exposed-top solid line spacing divisor, replacing the hardcoded `SOLID_DENSITY`, plus canonical's `density <= 0` emit-nothing skip | AC-4, AC-5 |
| `bottom_surface_density` | **(b)** | `rectilinear-infill` (manifest key) | exposed-bottom solid line spacing divisor (no zero skip — canonical min is 10) | AC-4 |
| `top_surface_pattern` | **(b)** | host (`resolve_global_config`) | value→`top_fill_holder` mapping over eight `claim:top-fill` holder modules, six of them built by this packet | AC-1, AC-3, AC-N3 |
| `bottom_surface_pattern` | **(b)** | host (`resolve_global_config`) | value→`bottom_fill_holder` mapping over the same eight modules, all of which this packet gives `claim:bottom-fill` | AC-2, AC-N3 |

Counts: **(a) 0 · (b) 4 · (c) 0 · (d) 0.** Zero declaration-only keys (map preflight gate (a)); every key carries at least one AC asserting a behaviour change at a non-default value (map preflight gate (b)).

## Returned to Queue — unimplemented

**None.** All four ticket-17 keys are implemented by this packet.

Two *behaviours* adjacent to these keys are deliberately not built here and are recorded in § Out of Scope rather than as returned keys, because they are not keys in ticket 17's list: canonical's top-surface **expansion** pass (which reads `top_surface_density > 0` in `PerimeterGenerator.cpp` `top_fill_replaces_inner_walls` and `PrintObject.cpp` `detect_surfaces_type` — the port has no such pass), and canonical's bridge/void-extension **fallback** that reads `top_surface_pattern` to pick a bridge filler (the port routes bridges through `bridge_fill_holder` instead; see `design.md` DIV-2).

## Ruled Dead-in-Canonical

**None.** Every one of the four keys has at least one read site inside OrcaSlicer's slicing pipeline under `src/libslic3r/`, not merely in `ConfigManipulation.cpp`, GUI tooltips, preset plumbing, or an `IGNORE`/legacy-alias set:

- `top_surface_pattern` — `Fill/Fill.cpp` `group_fills` (top branch, bridge fallback, void-extension path); also `GCode.cpp` `_needSAFC` and `GCode::retract`.
- `bottom_surface_pattern` — `Fill/Fill.cpp` `group_fills` (bottom branch); also `GCode.cpp` `_needSAFC` / `retract`.
- `top_surface_density` — `Fill/Fill.cpp` `group_fills` (top branch, including the `continue` when density <= 0); `PerimeterGenerator.cpp` `top_fill_replaces_inner_walls`; `PrintObject.cpp` `detect_surfaces_type`.
- `bottom_surface_density` — `Fill/Fill.cpp` `group_fills` (bottom branch). This is its *only* slicing read site; its other libslic3r mentions are `PrintObject::invalidate_state_by_config_options` (a key-name list, not a value read) and `Preset.cpp`'s key list. One live read site is sufficient — the key is not dead.

## In Scope

1. **`monotonicline-infill`** — new core module crate (`Cargo.toml`, `src/lib.rs`, `monotonicline-infill.toml`, `wit-guest/`, `tests/monotonicline_infill_tdd.rs`), `LayerModule` at `Layer::Infill`, holds `claim:top-fill` and `claim:bottom-fill`. Ports canonical `FillMonotonicLines::fill_surface`: monotonic sweep ordering with `anchor_length_max = 0`, i.e. discrete unconnected lines.
2. **`alignedrectilinear-infill`** — same crate shape (`tests/alignedrectilinear_infill_tdd.rs`). Ports canonical `FillAlignedRectilinear`: the rectilinear scan-line generator with the per-layer angle pinned to 0 so the fill direction is identical on every layer.
3. **`concentric-infill`** — same crate shape (`tests/concentric_infill_tdd.rs`). Ports canonical `FillConcentric::_fill_surface_single`: successive inward offsets emitted as nested closed loops.
4. **`hilbert-curve-infill`**, **`archimedean-chords-infill`**, **`octagram-spiral-infill`** — three `FillPlanePath`-family crates (`tests/hilbert_curve_infill_tdd.rs`, `tests/archimedean_chords_infill_tdd.rs`, `tests/octagram_spiral_infill_tdd.rs`). Each generates its space-filling curve over the region bounding box, clips to the region, and emits one continuous polyline (canonical `FillPlanePath::fill_surface` never contour-connects a solid plane-path fill).
5. **`monotonic-infill` extension** — append `claim:bottom-fill` to the `holds` list in `modules/core-modules/monotonic-infill/monotonic-infill.toml` (created by packet 262b) and add the `BottomSolidInfill` emission arm to its `run_infill`, so canonical's `bottom_surface_pattern` default (`ipMonotonic`) has a holder.
6. **Host-side pattern→holder derivation** — extend the derivation packet 262b adds to `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`) with `top_surface_pattern` → `top_fill_holder` and `bottom_surface_pattern` → `bottom_fill_holder`, and apply the same derivation on the per-object overlay path (`apply_overlay`, consumed by `resolve_per_object_configs`). Explicit holder keys win; absent keys leave the `"rectilinear-infill"` default; unknown values are rejected by name.
7. **Density wire** — two new fields on `RectilinearInfill` populated in `RectilinearInfill`'s `LayerModule::from_config` impl, replacing both uses of `SOLID_DENSITY` in `RectilinearInfill::run_infill` with the per-role resolved fraction, and gating the exposed-top block on `density > 0`. Internal solid (`solid_fill_role`'s `Some(n >= 1)` arm) keeps full density, matching canonical's fixed `100.f` for `stInternalSolid`.
8. **Manifest ownership of the two density keys** on `rectilinear-infill.toml` with canonical types/defaults/bounds and a `description` naming the canonical consumer; the two *pattern* keys are declared in no manifest at all, because they are host-side selection keys (the `wall_generator` precedent in `docs/04_host_scheduler.md`).
9. **Registration**: workspace members in the root `Cargo.toml`; `crates/slicer-integrated-modules/Cargo.toml` optional deps + features; `crates/slicer-integrated-modules/src/lib.rs` `manifest_const!` + `integrated_registry!` entries and their `#[cfg(not(feature = …))]` arms; `crates/pnp-cli/Cargo.toml` `integrated-<name>` passthrough features (required by `xtask/src/dist.rs`'s per-module passthrough check).
10. **Docs**: the pattern→holder subsection in `docs/04_host_scheduler.md` § Claim Resolution; the six new owners noted on the `claim:top-fill` / `claim:bottom-fill` rows in `docs/03_wit_and_manifest.md` § Known claim IDs; regeneration of `docs/15_config_keys_reference.md`.

## Out of Scope

- **Canonical's top-surface expansion pass.** `PerimeterGenerator.cpp` `top_fill_replaces_inner_walls` (called from `process_classic` and `process_arachne`) and `PrintObject.cpp` `detect_surfaces_type` both gate top-surface *expansion* geometry on `top_surface_density > 0`. The port has no top-surface expansion pass at all, so there is no decision point to gate. Recorded in the key's disposition; a future expansion packet re-opens it.
- **The bridge fallback and void-extension coupling.** Canonical `group_fills` reads `top_surface_pattern` when choosing a filler for bridges above layer 0 and for the synthesized `stInternalSolid` void extension. The port keeps bridge selection on `bridge_fill_holder` and internal solid on packet 262b's `internal_solid_infill_pattern` → `top_fill_holder` mapping. Recorded as `design.md` DIV-2 (a deliberate decoupling, not a gap).
- **The `GCode.cpp` pattern reads.** `_needSAFC` (small-area flow compensation enabled only for the rectilinear/monotonic family) and `GCode::retract` (retraction suppressed for `ipHilbertCurve`) read both pattern keys at emission time. The port has neither behaviour; wiring emission-time pattern reads would require the emitter to know the fill holder, which is not a seam this packet opens.
- **The gyroid opt-in solid path.** ADR-0027's multi-role gyroid emission rides the sparse density; extending the two density keys there would change opt-in behaviour at defaults. AC-N5 pins the omission.
- **`ORCA_CONFIG_PADDING`.** Not touched, per map Authoring rule 2 and AC-N2. Both density keys will appear in the CONFIG_BLOCK as a side effect of being live; neither pattern key gains or loses a padding twin in this packet.
- **`internal_solid_infill_pattern` and `sparse_infill_pattern`.** Packet 262b owns those two keys and the first pattern→holder derivation. This packet extends that mechanism; it does not re-specify it.

## Authoritative Docs

- `docs/01_system_architecture.md` § Claim System — the normative claim concept and the Allowed Claim Transition Matrix.
- `docs/04_host_scheduler.md` § Claim Resolution — the authoritative runtime claim-resolution reference; gains this packet's pattern→holder subsection. Its `wall_generator` subsection is the precedent for a host-side selection key that lives in no module manifest.
- `docs/03_wit_and_manifest.md` § Known claim IDs — the `claim:top-fill` / `claim:bottom-fill` rows and the `[config.schema]` type-table contract.
- `docs/adr/0027-gyroid-multi-role-fill-holder.md` — the gyroid solid-emission divergence.
- `docs/adr/0056-integrated-modules-native-dispatch.md` — registration contract for a new core module.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; every filler in this packet is geometry.
- `docs/15_config_keys_reference.md` — generated by `cargo xtask gen-config-docs`; never hand-edited.

## Parity Evidence Standard

A key counts as covered only when a non-default value changes emitted geometry or emitted G-code, proven by a named test. Default-path identity (AC-N1) is recorded as an additional guard and is never the sole evidence for any key. Canonical evidence is cited by file + function name only, never by line number; in-tree evidence is cited by crate-qualified path + symbol name.

## Per-Key Canonical Evidence

Canonical facts below were established by delegated reads of the sibling `OrcaSlicerDocumented` checkout during authoring; workers re-dispatch rather than re-derive from memory if they dispute any of them.

- **`top_surface_pattern`** — `PrintConfig.cpp` `PrintConfigDef::init_fff_params`: coEnum over `InfillPattern`, default `ipMonotonicLine`, with exactly eight values in this order: `monotonic`, `monotonicline`, `rectilinear`, `alignedrectilinear`, `concentric`, `hilbertcurve`, `archimedeanchords`, `octagramspiral`. Read in `Fill/Fill.cpp` `group_fills` (top branch sets `params.pattern` from it), and twice more in the same function for the bridge fallback and the void-extension fill.
- **`bottom_surface_pattern`** — same file/function; the value list is copied verbatim from the top key's (`enum_values` assignment), default `ipMonotonic`. Read in `group_fills`' bottom branch.
- **`top_surface_density`** — coPercent, default 100, min 0, max 100. Read in `group_fills`' top branch, which sets `params.density` from it and `continue`s (emitting nothing for that surface) when the value is <= 0. `Layer::make_fills` then normalizes with `params.density = 0.01 * surface_fill.params.density` before the filler runs.
- **`bottom_surface_density`** — coPercent, default 100, min **10**, max 100. Read in `group_fills`' bottom branch, which has no zero check — the min of 10 makes zero unreachable, so the port must not copy the top branch's skip onto the bottom block.
- **Pattern → filler class** — `Fill/FillBase.cpp` `Fill::new_from_type` switches `InfillPattern` onto `FillMonotonic`, `FillMonotonicLines`, `FillRectilinear`, `FillAlignedRectilinear`, `FillConcentric`, `FillHilbertCurve`, `FillArchimedeanChords`, `FillOctagramSpiral`. This switch is the thing this packet reimplements as a value→module map.
- **`FillMonotonic` vs `FillMonotonicLines`** — both derive from `FillRectilinear`, both set `monotonic = true` and are `no_sort()`, so both run `fill_surface_by_lines`' monotonic branch (`generate_montonous_regions` / `connect_monotonic_regions` / `chain_monotonic_regions`). The sole difference is that `FillMonotonicLines::fill_surface` also sets `anchor_length_max = 0.0f`, which makes `FillParams::dont_connect()` true, so `connect_segment_intersections_by_contours` demotes every otherwise-valid contour link to `TooLong`. Observable: monotonic emits long polylines joined by perimeter-following U-turns; monotonicline emits discrete unconnected lines in the same monotonic order.
- **`FillAlignedRectilinear`** — identical to `FillRectilinear` except `_layer_angle` is pinned to 0, so the fill direction does not alternate per layer.
- **`FillPlanePath` family** — `FillPlanePath::fill_surface` short-circuits contour connection whenever `dont_connect() || density > 0.5`; solid fills are always above that density, so hilbert / archimedean / octagram solid fills are single continuous curves.
- **Internal solid** — `group_fills` gives `stInternalSolid` `internal_solid_infill_pattern` at a fixed `100.f`. Neither of this packet's density keys may reach internal solid.

## In-Tree Grounding (verified at authoring, 2026-09-01)

- `top_fill_holder`, `bottom_fill_holder`, `bridge_fill_holder`, `sparse_fill_holder` are declared in the `declare_resolved_config!` block in `crates/slicer-ir/src/resolved_config.rs` — all `String`, all `cli`-bound, all defaulting to `"rectilinear-infill"`. **No new `ResolvedConfig` field is needed.**
- `resolve_global_config` and `apply_overlay` live in `crates/slicer-scheduler/src/config_resolution.rs`; the former dispatches values through `ResolvedConfig::apply_cli_key`, the latter builds per-object overlays consumed by `resolve_per_object_configs`.
- Runtime holder matching is `FillHolders`, `FillHolders::holder_for`, `module_id_matches_holder`, and `resolve_held_claims` in `crates/slicer-scheduler/src/validation.rs`; the four fill claim strings are `FILL_CLAIM_IDS` in the same file.
- `claim:top-fill` and `claim:bottom-fill` already exist in `docs/03_wit_and_manifest.md` § Known claim IDs. Today only `rectilinear-infill` and `gyroid-infill` declare them. **No new claim ID is needed.**
- `SliceRegionView` (`crates/slicer-sdk/src/views.rs`) distinguishes surfaces by depth, not by an enum: `top_shell_index()` / `bottom_shell_index()` return `Some(0)` for an exposed surface, `Some(n >= 1)` for internal solid, `None` outside the shell; `top_solid_fill()`, `bottom_solid_fill()`, `internal_solid_fill()` give the polygons; `should_emit(role)` and `held_claims()` gate emission. `should_emit` maps `TopSolidInfill` → `claim:top-fill` and `BottomSolidInfill` → `claim:bottom-fill`, and returns `false` for every role when `held_claims` is empty.
- `SOLID_DENSITY: f32 = 1.0` is a private const in `modules/core-modules/rectilinear-infill/src/lib.rs`, used exactly twice in `RectilinearInfill::run_infill` as `solid_spacing = mm_to_units(solid_line_width / SOLID_DENSITY)` for the top and bottom blocks; `solid_fill_role(shell_index, exposed)` performs the exposed-vs-internal role split; `adjust_solid_spacing` normalizes spacing.
- Neither `top_surface_density` nor `bottom_surface_density` is read anywhere in Rust, TOML, or MoonBit source today — their only occurrences are in docs and gap inventories.
- The scheduler's integration test **target name is `scheduler_integration`** (`[[test]] name = "scheduler_integration", path = "tests/integration/main.rs"` in `crates/slicer-scheduler/Cargo.toml`). The prior revision of this packet used `--test integration`, which names no target; every AC command here uses the real name.
- Existing `rectilinear-infill` test binaries: `bridge_infill_emission_tdd`, `rectilinear_infill_edge_cases_tdd`, `rectilinear_infill_tdd`, `rectilinear_raw_emit_tdd`, `slicer_module_binding_tdd`, `top_bottom_fill_tdd`. AC-4/AC-5 extend `top_bottom_fill_tdd`; AC-12/AC-N5 land in the net-new `top_bottom_surface_config_schema_tdd`.

## Acceptance Summary

Authoritative Given/When/Then text lives in `packet.spec.md`. IDs only here.

| AC | Subject | Key(s) covered |
| --- | --- | --- |
| AC-1 | `top_surface_pattern` → `top_fill_holder`, all 8 values + precedence + rejection | `top_surface_pattern` |
| AC-2 | `bottom_surface_pattern` → `bottom_fill_holder`, all 8 values + independence | `bottom_surface_pattern` |
| AC-3 | per-object overlay carries the derivation | both pattern keys |
| AC-4 | density 50 halves the solid line count (top and bottom) | both density keys |
| AC-5 | `top_surface_density = 0` emits nothing on exposed top; internal solid unaffected | `top_surface_density` |
| AC-6 | `monotonicline-infill` vs `monotonic-infill`: connector count 0 vs > 0 | `top_surface_pattern` |
| AC-7 | `alignedrectilinear-infill` does not alternate per layer | `top_surface_pattern` |
| AC-8 | `concentric-infill` emits nested closed loops | `top_surface_pattern` |
| AC-9 | the three plane-path fillers emit one continuous distinguishable curve each | `top_surface_pattern` |
| AC-10 | manifest ingestion: six new modules, claims, zero Error diagnostics | both pattern keys |
| AC-11 | claim resolution picks the selected holder and only it | both pattern keys |
| AC-12 | manifest schema for the two density keys; pattern keys in no manifest | both density keys |
| AC-13 | bounds enforcement incl. bottom min 10 | both density keys |
| AC-14 | generated config-keys doc; deviation row count unchanged | both density keys |
| AC-15 | `docs/04_host_scheduler.md` documents the mapping | both pattern keys |
| AC-N1 | default path byte-identical (additional guard only) | all four |
| AC-N2 | zero `ORCA_CONFIG_PADDING` diff lines | — |
| AC-N3 | unknown/misspelled pattern values rejected, never degraded | both pattern keys |
| AC-N4 | the six new modules emit nothing for sparse/bridge roles | both pattern keys |
| AC-N5 | gyroid/lightning declare neither density key | both density keys |

## Verification Matrix

| AC | Command |
| --- | --- |
| AC-1, AC-2, AC-3, AC-N3 | `cargo test -p slicer-scheduler --test scheduler_integration top_bottom_pattern_holder 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-4, AC-5 | `cargo test -p rectilinear-infill --test top_bottom_fill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-6 | `cargo test -p monotonicline-infill --test monotonicline_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-7 | `cargo test -p alignedrectilinear-infill --test alignedrectilinear_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-8 | `cargo test -p concentric-infill --test concentric_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-9 | `cargo test -p hilbert-curve-infill --test hilbert_curve_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` plus `cargo test -p archimedean-chords-infill --test archimedean_chords_infill_tdd` and `cargo test -p octagram-spiral-infill --test octagram_spiral_infill_tdd` |
| AC-10 | `cargo test -p slicer-scheduler --test scheduler_integration manifest_ingestion 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-11, AC-N4 | `cargo test -p slicer-runtime --test contract native_infill_claim_resolution 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-12, AC-N5 | `cargo test -p rectilinear-infill --test top_bottom_surface_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-13 | `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-14 | `cargo xtask gen-config-docs --check && [ "$(rg -c '^\| \`top_surface_density\`' docs/15_config_keys_reference.md)" = "1" ] && [ "$(rg -c '^\| \`bottom_surface_density\`' docs/15_config_keys_reference.md)" = "1" ]; echo "exit=$?"` |
| AC-15 | `rg -q 'top_surface_pattern' docs/04_host_scheduler.md && rg -q 'bottom_surface_pattern' docs/04_host_scheduler.md && rg -q 'octagram-spiral-infill' docs/04_host_scheduler.md; echo "exit=$?"` |
| AC-N1 | `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 \| tee target/test-output.log \| grep -E "^test result"` |
| AC-N2 | `git diff --unified=0 -- crates/slicer-gcode/src/serialize.rs \| grep -cE "^[+-][^+-]"` (expect `0`) |
| Gates | `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo xtask build-guests --check; echo "exit=$?"` |

## Step Completion Expectations

Cross-step expectations only; per-step contracts live in `implementation-plan.md`.

- The six new crates must be registered in all four registration surfaces (root `Cargo.toml`, integrated-modules `Cargo.toml`, integrated-modules `src/lib.rs`, `pnp-cli` `Cargo.toml`) in the *same* step that creates them, or `cargo check --workspace --all-targets` and `xtask dist`'s passthrough check will fail out of band.
- The module-count assertion in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` must be moved in the same step that lands the six manifests. Re-derive its pre-packet value from disk at that moment; never carry a number forward from this document.
- The pattern→holder derivation must land in both `resolve_global_config` and `apply_overlay`. A derivation in only the global path passes AC-1 and AC-2 while silently failing AC-3.
- Removing `SOLID_DENSITY` changes no default output only if the resolved default fraction is exactly 1.0. Land the density wire and its default-identity check together, and keep AC-N1 green from that step onward.
- `cargo xtask build-guests --check` must return exit 0 before closure: eight guests are affected (six new, plus `rectilinear-infill` and `monotonic-infill`).

## Context Discipline Notes

- Never load `OrcaSlicerDocumented/` directly; every canonical read is a delegated dispatch returning `SUMMARY` or `LOCATIONS`.
- `crates/slicer-ir/src/resolved_config.rs` is long and macro-dense — read only the `declare_resolved_config!` rows for the four `*_fill_holder` fields; do not load the file.
- `docs/15_config_keys_reference.md` is generated. Never open it to author; verify it with the AC-14 command.
- The six new fillers are independent geometry ports. Implement and verify them one crate at a time; do not hold more than one canonical filler algorithm in context at once.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`: the four key declarations, their coEnum/coPercent types, defaults, and the shared eight-value `InfillPattern` list.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `group_fills` (top branch and its `density <= 0` skip, bottom branch, `stInternalSolid`'s fixed density, the bridge fallback and void-extension reads of `top_surface_pattern`) and `Layer::make_fills` (`Fill::new_from_type`, the `0.01 * density` normalization, `link_max_length`).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Fill::new_from_type`: the `InfillPattern` → filler-class switch this packet reimplements as a value→module map.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `FillMonotonic::fill_surface`, `FillMonotonicLines::fill_surface`, `connect_segment_intersections_by_contours`, `fill_surface_by_lines`' monotonic branch, `FillAlignedRectilinear`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillConcentric.cpp` — `FillConcentric::_fill_surface_single`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillPlanePath.cpp` — `FillPlanePath::fill_surface` and the `FillHilbertCurve` / `FillArchimedeanChords` / `FillOctagramSpiral` point generators.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `top_fill_replaces_inner_walls`; `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `detect_surfaces_type` (both recorded, not wired).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.
