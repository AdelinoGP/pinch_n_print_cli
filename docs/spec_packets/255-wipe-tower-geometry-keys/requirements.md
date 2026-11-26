# Requirements: wipe-tower-geometry-keys

## Packet Metadata

- **Packet directory:** `docs/spec_packets/255-wipe-tower-geometry-keys/`
- **Slug:** `wipe-tower-geometry-keys`
- **Status:** `draft`
- **Task IDs:** none (queue packet — `task_ids: []`, precedent packets 234a, 253–264)
- **Backlog source:** wayfinder ticket 10 (`docs/specs/orca-feature-gap/issues/10-author-packet-p03-multimaterial-prime-tower-wipe-tower.md`), map `docs/specs/orca-feature-gap/map.md` packet P03
- **Tier:** **B** — re-derived. The prior revision was Tier A on the "declare + wire the cheap key" reading. Under map Authoring rule 1 a packet that *builds* a decision point is B or C; this packet builds three wall generators, a fillet pass, a rotation transform and a flow multiplier, all inside one existing owner (`wipe-tower`) with no new module, claim, or host field — which is Tier B, not C. See `design.md` § Tier Derivation.
- **Re-authoring note:** this directory is overwritten in place (number and slug retained) with explicit user approval, under map Authoring rules 1–6.

## Problem Statement

OrcaSlicer's wipe tower is a *shaped* structure: it has a wall whose form the user chooses (`wipe_tower_wall_type` — rectangle, cone, or rib), whose cone taper, rib thickness and rib reach are configurable, whose corners can be filleted, which can be rotated on the bed, and whose purge lines can be fattened without changing the purged volume. Pinch 'n Print's tower is a bare axis-aligned block of scan lines: `generate_purge_paths` (`modules/core-modules/wipe-tower/src/lib.rs`) emits a travel entity, a boustrophedon scan-line fill and a prime line, with `flow_factor` hardcoded to `1.0` and no wall entity of any kind. There is no rotation anywhere in the module, and `run_finalization` validates a conservative `tower_width` **square** against the bed polygon rather than the geometry it actually emits.

The prior revision of this packet declared 12 of the 13 P03 keys in the manifest and claimed one (`wipe_tower_extra_flow`) as wired. The 2026-08 key-correction audit re-derived every claim against the tree and found **zero read sites for all 12** — the `extra_flow` wiring did not exist, and the packet's own hedge ("no output change at defaults; factor 1.0 is identity") was itself the map's rule-6(b) failure. Three of the keys existed only as hardcoded `ORCA_CONFIG_PADDING` twins, which map Authoring rule 2 rules out as evidence.

This revision keeps only keys whose behaviour it builds, and states plainly which keys leave.

## Key Disposition Table

Classification per the map's Authoring rules: **(a)** live behaviour-changing decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue (no decision point, not built here); **(d)** dead-in-canonical.

| Key | Class | Owner | Decision point this packet builds | Non-default AC |
| --- | --- | --- | --- | --- |
| `wipe_tower_wall_type` | **(b)** | `wipe-tower` (manifest key) | selects among three wall-loop generators (`rectangle` / `cone` / `rib`) the packet builds; today the module emits no wall at all | AC-2, AC-3, AC-4, AC-N3 |
| `wipe_tower_cone_angle` | **(b)** | `wipe-tower` (manifest key) | per-layer cone radius `tan(angle/2) × (tower_top_z − z)` and its 40-segment corner arcs | AC-3 |
| `wipe_tower_rib_width` | **(b)** | `wipe-tower` (manifest key) | thickness of the two diagonal bars unioned into the wall loop, clamped to `min(layer_depth, tower_width) / 2` | AC-4 |
| `wipe_tower_extra_rib_length` | **(b)** | `wipe-tower` (manifest key) | rib arm reach `rib_length = diagonal + extra`, re-clamped to `>= diagonal`, tapered by height | AC-5 |
| `wipe_tower_fillet_wall` | **(b)** | `wipe-tower` (manifest key) | corner rounding of the rib wall loop (2.0 mm cap, 30° turn tolerance) | AC-6 |
| `wipe_tower_rotation_angle` | **(b)** | `wipe-tower` (manifest key) | rotation of every emitted tower point about the tower origin, plus the rotated-hull bed-bounds check | AC-7, AC-8 |
| `wipe_tower_extra_flow` | **(b)** | `wipe-tower` (manifest key) | effective purge line width — drives point `width`, `flow_factor`, scan-line pitch and cross-section together, preserving purge volume | AC-9 |

Counts: **(a) 0 · (b) 7 · (c) 6 · (d) 0.** Zero declaration-only keys (map preflight gate (a)); every kept key carries at least one AC asserting a behaviour change at a non-default value (map preflight gate (b)).

## Returned to Queue — unimplemented

Six of the thirteen P03 keys leave this packet. None is declared, padded, or counted as covered. Each names the missing feature, per Authoring rule 1.

| Key | Needs (missing feature) | Canonical consumer (file + function) |
| --- | --- | --- |
| `purge_in_prime_tower` | a **flush-volume matrix and tool-ordering model**. Canonical zeroes the flush matrix when the key is false, so no tower purge is printed; this port has no flush matrix to gate — the tower always purges a fixed `prime_volume`. | `ToolOrdering.cpp::reorder_extruders_for_minimum_flush_volume`, `Print.cpp::_make_wipe_tower` |
| `single_extruder_multi_material` | an **SEMM toolchange branch**. Canonical switches filament-end handling, temperatures, reset-E and ooze prevention on this flag; the port's toolchange machinery has one unconditional path. | `GCode.cpp::_do_export`, `Print.cpp::_make_wipe_tower`, `WipeTower2.cpp::extract_wipe_volumes` |
| `wipe_tower_bridging` | a **sparse-tower bridge pass**. Canonical spans unsupported tower gaps with `n = 1 + int(span / bridging)` bridge lines; the port's tower is fully filled on every tower-bearing layer, so there is no unsupported span to bridge. | `WipeTower2.cpp::WipeTower2` ctor → `finish_layer` |
| `wipe_tower_no_sparse_layers` | **sparse-layer tower printing**. Canonical's `false` (its default) prints the tower on layers with no toolchange; the port prints the tower *only* at toolchange layers, i.e. it is hardwired to the `true` behaviour. Building the key means building the sparse-layer path first. | `WipeTower2.cpp::WipeTower2` ctor, `GCode.cpp::WipeTowerIntegration::tool_change` |
| `wipe_tower_extra_spacing` | the **ramming pass**. Canonical uses it as the ramming/wipe `y_step` multiplier. Packet `254b-prime-tower-interface-and-ramming` builds the ramming pass; this key should be filed against that feature once `254b` is implemented, not stacked as a second-order forward dependency here. | `WipeTower2.cpp::WipeTower2` ctor → `toolchange_Unload`, `set_toolchange` |
| `wipe_tower_max_purge_speed` | nothing new — it is **owned by the rename workstream**. Grilling ruling Q6(a) settled it: the host key `wipe_tower_speed` (`FeedrateConfig`, `crates/slicer-ir/src/feedrate.rs`, default `90.0`) renames to `wipe_tower_max_purge_speed` and additionally adopts canonical's cap semantic `min(max_purge_speed, infill_speed)`, closing wayfinder ticket 108. That is host feedrate work in a different crate, not module geometry. | `WipeTower2.cpp::toolchange_Wipe`, `finish_layer` |

## Ruled Dead-in-Canonical

**None.** All thirteen P03 keys have at least one read site inside OrcaSlicer's slicing pipeline under `src/libslic3r/` — none is confined to `ConfigManipulation.cpp`, GUI tooltips, preset plumbing, or an `IGNORE` / legacy-alias set. The consumer functions are named per key in the table above and in § Per-Key Canonical Evidence. Note the one adjacent finding: `prime_tower_rib_wall`, `prime_tower_rib_width`, `prime_tower_extra_rib_length` and `prime_tower_fillet_wall` are **legacy spellings** remapped onto this packet's keys in canonical's config-substitution path; they are not separate keys and are not in scope.

## In Scope

1. **Manifest declarations** — the seven `[config.schema.*]` tables of AC-1 in `modules/core-modules/wipe-tower/wipe-tower.toml`, with canonical types, defaults and bounds. The enum table follows the established in-tree shape (`type`/`values`/`default`/`display`/`group`; precedent `path-optimization-default.toml`, `seam-placer.toml`, `tree-support-planner.toml`).
2. **The wall generator** — a new per-layer closed `ExtrusionRole::WipeTower` loop emitted ahead of the layer's purge scan lines, with three shapes selected by `wipe_tower_wall_type`, plus the fillet pass on the rib shape.
3. **The rotation transform** — every point the module emits is mapped through `p → origin + R(θ)·(p − origin)` at generation time, and the bed-bounds check validates the emitted wall vertices rather than an axis-aligned square.
4. **The flow multiplier** — `wipe_tower_extra_flow` folded into an *effective line width* that drives point `width`, `flow_factor`, scan-line pitch and the purge cross-section together, so ordered purge volume is preserved.
5. **Bounds and leakage coverage** — one arm in the scheduler's existing `config_bounds_enforcement_tdd`, one arm in the runtime's existing `config_view_binding_tdd`.
6. **Generated docs** — `docs/15_config_keys_reference.md` regenerated by `cargo xtask gen-config-docs`.

## Out of Scope

- **The six returned keys** above — not declared here, in any form.
- **`ORCA_CONFIG_PADDING`** (`crates/slicer-gcode/src/serialize.rs`) — untouched, including the hardcoded `wipe_tower_rotation_angle` twin this packet's key now shadows for user-set values. Map Authoring rule 2 forbids padding edits as packet deliverables; grilling ruling Q5 owns the padding table's mechanical re-derivation separately.
- **Canonical's rib-mode square-tower re-planning** (`WipeTower::plan_tower_new`, `set_toolchange` re-planning, `get_limit_depth_by_height`) — DIV-1 in `design.md`.
- **Canonical's gap wall / skip points** (`use_gap_wall`, `construct_gap_for_skip_points`) — driven by `prime_tower_skip_points`, which packet `254a` returned to the queue; without that key there is nothing to gate.
- **Any WIT interface, IR schema bump, `ResolvedConfig` field, claim ID, or new module** — none is needed; see `design.md` § Mechanism Check.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` § Host-Boundary Access Enforcement (Normative) and the `[config.schema]` section — the declaration contract and AC-N1.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; this module works in plain mm `f32`, so canonical's `scaled()` rib and rounding arithmetic must be de-scaled, never transcribed.
- `docs/01_system_architecture.md` § Claim System — read for the mechanism check in `design.md` (why this packet holds no claim).
- `docs/15_config_keys_reference.md` — large; regeneration plus grep verification only, never read in full.
- `docs/specs/orca-feature-gap/issues/key-correction-inventory.md` — the audit that invalidated the prior revision; ranged reads only (long).

## Parity Evidence Standard

Per map ticket 02: canonical **function-read + described behaviour**, pinned by invariant tests (no goldens — the canonical checkout is readable, not runnable). Ported OrcaSlicer test assertions are acceptable evidence with the attribution header (`docs/ORCASLICER_ATTRIBUTION.md`). Unverifiable behaviour is surfaced to the human first and only then filed as a `DEVIATION_LOG.md` row with sign-off. This packet consumes no human sign-off and files no deviation row: every behaviour below was read in canonical at authoring time. Its five port-level divergences are recorded in `design.md` § Divergences (DIV-1 … DIV-5) with rationale, per Authoring rule 4.

## Per-Key Canonical Evidence

All facts below come from delegated reads of the sibling `OrcaSlicerDocumented` checkout at authoring time. Cited by file + function; never by line number.

- **`wipe_tower_wall_type`** — `PrintConfig.hpp`'s `WipeTowerWallType` is a plain enum `wtwRectangle = 0, wtwCone, wtwRib`; `PrintConfig.cpp`'s `s_keys_map_WipeTowerWallType` maps `"rectangle"` / `"cone"` / `"rib"`; `PrintConfigDef` declares it `coEnum`, `comAdvanced`, default `wtwRib`. It routes to `WipeTower2::generate_support_cone_wall` for cone and `WipeTower2::generate_support_rib_wall` (`WipeTower::generate_support_wall_new` in the legacy class) for rib; rectangle leaves the box polygon untouched.
- **`wipe_tower_cone_angle`** — `coFloat`, default `30`, `min 0`, `max 90`. `WipeTower2::get_wipe_tower_cone_base` returns `R = tan(deg2rad(angle / 2)) × height`; `generate_support_cone_wall` recomputes the per-layer radius `r = tan(deg2rad(angle / 2)) × (m_wipe_tower_height − z)`, so it shrinks linearly to zero at the top, and appends two 40-segment arcs at the box corners only when `r > 0.5 × w + 0.01` (`w` = box depth). At angle `0`, `r = 0` and the wall degenerates to the plain rectangle; nothing special-cases `90`.
- **`wipe_tower_rib_width`** — `coFloat`, default `8`, `min 0`, `max 300`. `WipeTower2::generate_rib_polygon` builds two diagonal `Line`s across the tower rect, thickens each via `generate_rectange(line, scaled(m_rib_width) / 2)`, and unions both with the rectangle — four protruding arms, one per corner. `m_rib_width` is clamped to `min(depth, width) / 2` so the arms stay attached to the infill block.
- **`wipe_tower_extra_rib_length`** — `coFloat`, default `0`, `max 300`, **no `min`** (canonical's tooltip notes negative values shrink ribs). `m_rib_length = max(rib_length, diagonal) + m_extra_rib_length`, re-clamped to `>= diagonal`; `diagonal_extra_length = max(0, m_rib_length − diagonal) / 2` and is tapered per layer by `|max_height − z| / max_height`.
- **`wipe_tower_fillet_wall`** — `coBool`, default `true`. `m_used_fillet` is consulted in `WipeTower2::generate_support_rib_wall` / `WipeTower::generate_support_wall_new` — the **rib branch only**, never the cone branch — calling the file-static `rounding_polygon(polygon, rounding = 2.0 mm, angle_tol = 30°)`, which replaces each corner whose turn exceeds the tolerance with a tangent arc of radius `min(2.0, ab_len / 2.1, bc_len / 2.1)`, then unions the result with the plain box.
- **`wipe_tower_rotation_angle`** — `coFloat`, default `0`, no bounds. The member is stored on both tower classes but is **not** applied inside the generators (`WipeTowerWriter::rotate` uses a different field, `m_internal_angle`, about the tower centre, for layer alternation). Placement rotation is applied downstream by `GCode.cpp::WipeTowerIntegration::transform_wt_pt` / `transform_wt2_pt` and by `Print.cpp::first_layer_wipe_tower_corners`, both `Rotation2D(angle) × pt` **then translate** — i.e. about the tower **origin** (front-left corner), not its centre.
- **`wipe_tower_extra_flow`** — `coPercent`, default `100`, `min 100`, `max 300`. `m_extra_flow = value / 100`, and it exists only in `WipeTower2`. `toolchange_Wipe` multiplies **both** the extrusion flow (`set_extrusion_flow(m_extrusion_flow × m_extra_flow)`) and the analyzer line width (`wipe_line_width() = m_perimeter_width × m_extra_flow`), while `x_to_wipe` is **divided** by it so the ordered purge volume is unchanged; the row pitch uses `m_extra_flow` on the first layer. `set_toolchange` feeds it into `first_wipe_volume` and `get_wipe_depth` to keep reserved depth consistent.

**Canonical-revision caveat, recorded honestly:** the delegated reader flagged that this checkout is a *documented / modified* OrcaSlicer fork whose `WipeTower2` rib support carries `ORCA:` and `like WipeTower::…` comments indicating locally added parity work. The rib geometry above may therefore not match stock upstream exactly. The port follows what this checkout reads, which is the evidence standard ticket 02 set (`OrcaSlicerDocumented` is *the* canonical source for this effort); the caveat is repeated in `design.md` § Risks so a future reviewer does not mistake it for a port defect.

## In-Tree Grounding (verified at authoring, 2026-09-02)

- `modules/core-modules/wipe-tower/wipe-tower.toml` declares **8** keys today (`enable_prime_tower`, `wipe_tower_x`, `wipe_tower_y`, `prime_tower_width`, `prime_volume`, `line_width`, `printable_area`, `retract_length`) — re-derive from disk at implementation time; packets `254a` / `254b` add more and may land first.
- `WipeTower::from_config` (`modules/core-modules/wipe-tower/src/lib.rs`) reads each key through `ConfigView::get` with a hardcoded fallback; `WipeTower` is the struct that gains this packet's fields. It is defined under `modules/`, not `crates/*/src`, so it is **not** on the struct-literal churn gate's watchlist — but the one literal-adjacent construction site (`wipe_tower_from` in `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs`) goes through `from_config` and needs no edit.
- `generate_purge_paths` emits three entity kinds: travel (both points `flow_factor: 0.0`), scan lines (both points `flow_factor: 1.0`, advancing `y += line_width`), prime (first point `0.0`, second `1.0`). `cross_section = line_width × layer_height × tower_width`, `purge_depth = purge_volume / cross_section`. Both `process()` and `run_finalization()` call it; neither post-adjusts flow or position.
- `run_finalization` validates four corners of a `tower_width` **square** with `point_in_polygon` (even-odd ray cast, on-edge counts inside) against the `printable_area` polygon and returns `ModuleError::fatal(3, "wipe-tower corner (x, y) lies outside bed polygon")`. `parse_printable_area` rejects empty / odd-length / fewer-than-6 raw values with `ModuleError::fatal(2, …)`.
- Test binaries in `modules/core-modules/wipe-tower/tests/` today: `bed_bounds_tdd.rs`, `finalization_live_tdd.rs`, `slicer_module_binding_tdd.rs`, `wipe_tower_tdd.rs` — separate binaries, no aggregator `main.rs`, so a new file needs no `mod` registration. The crate's `[dev-dependencies]` carries `slicer-sdk` only.
- The scheduler's test binaries are `scheduler_contract` / `scheduler_integration` / `scheduler_unit` (renamed in `crates/slicer-scheduler/Cargo.toml` to avoid colliding with `slicer-runtime`'s buckets). `config_bounds_enforcement_tdd.rs` is registered in `crates/slicer-scheduler/tests/integration/main.rs`.
- `ConfigBoundsIndex::from_modules` / `::check` (`crates/slicer-scheduler/src/config_resolution.rs`) collect enum domains and enforce membership for `String` values; numeric bounds are enforced per element, and `schema_defaults` holds `percent` / `float_or_percent` fields only — which is why AC-10 expects only the percent default in `extensions`.
- `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) contains a hardcoded `("wipe_tower_rotation_angle", "0")` entry. It is **not** evidence and **not** edited by this packet.
- `cargo xtask gen-config-docs` and its `--check` mode both exist (`xtask/src/main.rs` dispatch, `xtask/src/gen_config_docs.rs`).
- Guest freshness: `modules/core-modules/wipe-tower/{wipe-tower.toml, src/**}` are guest-fingerprint inputs, so `cargo xtask build-guests --check` must return exit `0` after Step 3. Re-derive the baseline at implementation time rather than trusting any staleness claim frozen here.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (declaration) … `AC-11` (docs). Per key: wall type AC-2/3/4, cone AC-3, rib width AC-4, rib length AC-5, fillet AC-6, rotation AC-7/AC-8, extra flow AC-9.
- Negative: `AC-N1` (cross-module leakage blocked), `AC-N2` (enable gate holds), `AC-N3` (schema drift guard). Bounds rejection is AC-10.
- Cross-packet impact: shares the `wipe-tower` manifest and `generate_purge_paths` with `254a` and `254b` (both `draft`). This packet lands last of the three; its Step-1 precondition re-derives the manifest key set from disk.

## Verification Matrix

This is the authoritative full matrix; `packet.spec.md` § Verification lists the gate commands only.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1, AC-N3: the seven-table manifest contract | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p wipe-tower --test wipe_tower_wall_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2 … AC-7: wall shapes, fillet, rotation | FACT pass/fail |
| `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-8: rotated-hull bed validation | FACT pass/fail |
| `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-9: effective-width flow multiplier | FACT pass/fail |
| `cargo test -p wipe-tower --test finalization_live_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N2: enable gate | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-10: bounds, enum membership, percent threading | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N1: no cross-module leakage | FACT pass/fail |
| `cargo xtask gen-config-docs --check && rg -q 'wipe_tower_wall_type' docs/15_config_keys_reference.md && rg -q 'wipe_tower_extra_flow' docs/15_config_keys_reference.md && rg -q 'wipe_tower_fillet_wall' docs/15_config_keys_reference.md; echo "exit=$?"` | AC-11: generated docs current | FACT exit=0 |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness after manifest + `src/` edits | FACT exit=0 (exit 3 = `wasm-tools` missing, not clean) |
| `cargo check --workspace --all-targets` | workspace type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 \| tail -3` | struct-literal churn gate | FACT pass/fail |

## Step Completion Expectations

Only cross-step invariants, non-obvious ordering, and shared state:

- The manifest declarations (Step 1) must land before any module read (Steps 2–4): `ConfigView::from_declared` filters by declared keys, and reading an undeclared key is a silent `None`, not an error.
- The wall generator (Step 2) must land before rotation (Step 4): rotation applies to the wall vertices, and AC-8's bed check consumes them.
- The effective-width change (Step 3) alters scan-line **count** at non-default `wipe_tower_extra_flow`. Any count-pinned assertion in `wipe_tower_tdd.rs` must be re-expressed against the formula, not re-fitted to a captured number.
- The scan-line pitch and per-layer depth this packet composes with are `254a`'s if `254a` has landed. Re-derive both from `generate_purge_paths` at implementation time; do not transcribe a formula from this document without checking it against the code in front of you.
- The guest freshness gate runs at Step 4 exit and again at the acceptance ceremony. A wipe-tower staleness after these edits is this packet's to clear.
- Implementation is recorded against wayfinder ticket 10; `docs/07_implementation_status.md` holds no TASK row for this queue.

## Context Discipline Notes

- `docs/ORCA_CONFIG_REFERENCE.md` and `docs/15_config_keys_reference.md` are large: never read in full; ranged reads only.
- `crates/slicer-ir/src/resolved_config.rs` and `crates/slicer-gcode/src/emit.rs` are long: the facts this packet needs are in § In-Tree Grounding. Do not open them to browse.
- `docs/specs/orca-feature-gap/issues/key-correction-inventory.md` is long: grep for the key name, read the row.
- `OrcaSlicerDocumented/` is the **sibling** checkout `..\pinch_n_print_cli\OrcaSlicerDocumented`; every read is delegated per the orca-delegation snippet. Re-derive the absolute path on first use rather than trusting a drive letter written here.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `..\pinch_n_print_cli\OrcaSlicerDocumented` — not `./OrcaSlicerDocumented`):

- `src/libslic3r/PrintConfig.cpp` / `PrintConfig.hpp` — `PrintConfigDef`, `WipeTowerWallType`, `s_keys_map_WipeTowerWallType`.
- `src/libslic3r/GCode/WipeTower2.cpp` — `get_wipe_tower_cone_base`, `generate_support_cone_wall`, `generate_rib_polygon`, `generate_support_rib_wall`, the file-static `rounding_polygon`, `toolchange_Wipe`, `set_toolchange`.
- `src/libslic3r/GCode/WipeTower.cpp` — `generate_support_wall_new`, `plan_tower_new`.
- `src/libslic3r/GCode.cpp` — `WipeTowerIntegration::transform_wt_pt`; `src/libslic3r/Print.cpp` — `first_layer_wipe_tower_corners`.
