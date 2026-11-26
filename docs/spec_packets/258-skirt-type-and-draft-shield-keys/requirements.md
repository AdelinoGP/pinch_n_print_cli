# Requirements: skirt-type-and-draft-shield-keys

## Packet Metadata

- Packet directory: `docs/spec_packets/258-skirt-type-and-draft-shield-keys/`
- Backlog source: `docs/specs/orca-feature-gap/issues/13-author-packet-p06-others-skirt-skirt-brim.md` (wayfinder map packet P06)
- Owner module: `skirt-brim` (`modules/core-modules/skirt-brim/`)
- Tier: **B** — the packet builds decision points that do not exist in the tree (per-object skirt grouping, extruded-length loop expansion, start-corner rotation, shield span, per-layer loop count). Re-derived at authoring per map Authoring rule 1.
- Status: `draft`

## Problem Statement

OrcaSlicer's skirt feature carries five FFF config keys this port does not implement. The port's `SkirtBrim` module emits `skirt_loops` axis-aligned rectangular rings around the combined bounding box of the first `skirt_height` layers, always starting at `(x_min, y_min)`, always the same count on every targeted layer, always one ring set for the whole plate. Every one of the five keys names a decision this module currently does not make.

This packet was previously authored (2026-09-01) with three keys wired and two — `skirt_type` and `min_skirt_length` — declared in the manifest with the behaviour recorded as a "gap". Map Authoring rule 1 now prohibits that disposition: a key is covered only when the behaviour OrcaSlicer attaches to it exists in this tree and the key drives it. This re-authoring builds both remaining decision points, so the packet keeps all five keys and returns none to the queue.

## In Scope

1. **`draft_shield` (span gate).** `"enabled"` extends the skirt layer span from `min(skirt_height, layer_count)` to the entire layer set, mirroring canonical `Print::has_infinite_skirt` (`dsEnabled && skirt_loops > 0`). Brim generation stays layer-0-only.
2. **`single_loop_draft_shield` (per-layer loop count).** `true` emits exactly the innermost ring on every layer above the first, mirroring canonical `GCode::generate_skirt`'s `start_idx = loops.second - 1` on non-first layers. The first layer always keeps the full set.
3. **`skirt_start_angle` (start corner).** The first-layer innermost ring's start vertex becomes the rect corner nearest `bbox_center + r·(cos θ, sin θ)` with `r` the half-diagonal of that ring's own rect — the port's rect-loop analogue of canonical `Skirt::find_start_point`. Applies only where canonical applies it: `first_layer && i == loops.first`.
4. **`skirt_type` (grouping).** `"perobject"` partitions the layer's entities by `region_key.object_id`, computes one bbox per object, merges any two whose envelopes (bbox grown by `grouping_offset = skirt_distance + skirt_loops * line_width`) intersect — canonical `Print::_make_skirt`'s union-find fixed point — and emits `skirt_loops` rings around each surviving group. `"combined"` unites every object into one group, which is exactly today's behaviour.
5. **`min_skirt_length` (loop expansion).** After the base `skirt_loops` rings, keep appending rings **outward** (one `line_width` per step) while the accumulated extruded filament length is below the configured value. Extruded length per ring is `perimeter_mm · e_per_mm`, with `e_per_mm` derived module-side from the rounded-rectangle `mm3_per_mm` over `line_width`/`layer_height` divided by the filament cross-section from `filament_diameter`. Bounded by `MAX_MIN_LENGTH_LOOPS`.
6. **Host plumbing for (5).** `ResolvedConfig::filament_diameter` already exists but is not exported by `ResolvedConfig::to_config_map`, so no module can see it. Export it as `ConfigValue::Float`, and correct `serialize.rs`'s synthetic-`filament_diameter` branch so the CONFIG_BLOCK array is built from that resolved scalar instead of a hardcoded `1.75` — see §Recorded Divergences D-258-4.
7. **Bounds/enum enforcement, CONFIG_BLOCK reachability, generated docs** for the five keys.

## Out of Scope

- **Convex-hull skirt geometry.** Canonical offsets a per-instance convex hull of `lslices` outer contours plus `support_fills`; the port emits axis-aligned rect loops around a bbox. That difference predates this packet and is not introduced by it (D-258-1).
- **Per-extruder extruded-length rotation.** Canonical's `append_skirt_loops_for_hull` maintains `extruded_length[extruder_idx]` and advances the extruder once the target is met. The port uses a single accumulator (D-258-3).
- **`PrintSequence::ByObject` shared-per-object-skirt error.** Canonical sets `m_has_shared_per_object_skirt` when a per-object group ends with more than one instance and `Print::process` raises a `SlicingError` under by-object sequencing. The port has no by-object print sequence, so there is nothing to guard.
- **Wipe-tower obstacle inclusion.** Canonical adds `first_layer_wipe_tower_corners` as a non-emitting grouping item. The port's wipe tower is a separate module whose geometry is not visible to `skirt-brim` at `PostPass::LayerFinalization` grouping time; see §Returned to Queue.
- **`Print::object_skirt_offset` / per-instance brim-area contribution to the hull.**
- **Seam-style start-point re-seating of already-emitted paths** (that is `seam-placer`'s surface).
- **Any edit to `ORCA_CONFIG_PADDING` or a CONFIG_BLOCK padding twin** (map Authoring rule 2). No AC, step, or deliverable in this packet touches either.
- **New WIT interface, IR schema bump, or new `ResolvedConfig` field.** None is required: item 6 exports an existing field through an existing map, it does not add one.

## Returned to Queue — unimplemented, needs a feature this packet does not build

None. All five keys in ticket 13's list are implemented by this packet.

One *sub-behaviour* is returned rather than a key: **wipe-tower-as-grouping-obstacle** for `skirt_type = "perobject"`. Canonical includes the wipe tower's first-layer corners as a non-emitting `SkirtBrimGroupItem` so a per-object group that touches the tower merges with it. The port cannot see wipe-tower geometry from `skirt-brim`'s `LayerCollectionView` unless the tower's entities carry a stable `region_key.object_id` at `PostPass::LayerFinalization` — unverified at authoring. Needs: a wipe-tower object-identity contract. It does not gate `skirt_type`'s decision point, which is exercised and asserted over real object groups (AC-5).

## Ruled Dead-in-canonical

None. All five keys have live read sites inside `src/libslic3r/` in the slicing pipeline, confirmed by delegated canonical read at authoring:

- `skirt_type` — `Print::_make_skirt`, `Print::process`, `Print::object_skirt_offset`, `GCode::generate_object_skirt_group`, `GCode::process_layer`.
- `min_skirt_length` — exactly one live read: the `append_skirt_loops_for_hull` lambda inside `Print::_make_skirt`. (Its other occurrences are a commented-out block in `Skirt::skirt_loops_per_extruder_all_printing`, a commented-out `Print::min_skirt_length` static in `Print.hpp`, `invalidate_state_by_config_options`, and `Preset.cpp` — none of which would qualify on their own. The one live read does.)
- `skirt_start_angle` — `GCode::generate_skirt`, `GCode::generate_object_skirt_group`, `Skirt::find_start_point`.
- `draft_shield` — `Print::has_infinite_skirt`, `Print::object_skirt_offset`.
- `single_loop_draft_shield` — `GCode::generate_skirt`.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` §Host-Boundary Access Enforcement (Normative) — the rule that a module sees only its declared keys, and that the source map is `ResolvedConfig::to_config_map`. Governs item 6.
- `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract — the `filament_diameter` array-length filament-count inference and the ≥80-pair floor. Governs AC-8.
- `docs/01_system_architecture.md` §Claim System — consulted at authoring to confirm map Authoring rule 4's claim-holder trigger does **not** fire here (see `design.md` §Claims).
- `docs/00_project_overview.md` — modular pipeline / config robustness goals; all five decision points land inside the owning module, none as a host special case.
- `docs/15_config_keys_reference.md` — generated by `cargo xtask gen-config-docs`; never hand-edited.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the five keys (coType, default, min/max, enum value order).
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::_make_skirt`, `Print::has_infinite_skirt`, `Print::skirt_flow`, `Print::object_skirt_offset`.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::generate_skirt`, `GCode::generate_object_skirt_group`, `GCode::process_layer`, and the inline `Skirt` namespace (`find_start_point`, `skirt_loops_per_extruder_all_printing`). There is no `Skirt.cpp` in this checkout.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

## Parity Evidence Standard

Every claim below about canonical behaviour was produced by a delegated read of the sibling checkout at authoring time and is cited by file + function only, never by line number (CLAUDE.md, map Notes). A worker who disputes any row re-dispatches the read rather than re-reading in-context. In-tree citations are by crate-qualified path + symbol name.

## Per-Key Canonical Evidence

| Key | Canonical type | Canonical default | Bounds | Manifest declaration | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `skirt_type` | coEnum `SkirtType` (`"combined"`, `"perobject"`) | `combined` | — | enum, values in canonical order | `Print::_make_skirt` — union-find over per-instance `SkirtBrimGroupItem`s; `stCombined` unites all into group 0, `stPerObject` merges only groups whose hulls offset by `grouping_offset = scale_(skirt_distance + skirt_loops * spacing)` intersect. Emission split: `GCode::process_layer` (combined, before the instance loop) vs `GCode::generate_object_skirt_group` (per-object, inside the instance loop on `first_visit`) | **Built (AC-5)** — per-object bbox grouping with the same envelope-intersection fixed point |
| `min_skirt_length` | coFloat, min 0 | `0.0` | min 0, no max | float, min 0, no max | `Print::_make_skirt`'s `append_skirt_loops_for_hull`: `mm3_per_mm` from `Print::skirt_flow`, `e_per_mm` from `Extruder::e_per_mm`, `extruded_length[idx] += unscale(loop.length()) * e_per_mm`; when the target is unmet at `i == 1` it does `++i` so loops keep being appended; each loop is `offset(hull, distance += scale_(spacing))` — strictly **outward**; generation is inward-to-outward and `reverse()`d before export | **Built (AC-6)** — module-side e-per-mm over the resolved scalar `filament_diameter`, outward expansion, bounded by `MAX_MIN_LENGTH_LOOPS` |
| `skirt_start_angle` | coFloat | `-135` | [−180, 180] | float, min −180, max 180 | `GCode::generate_skirt`, condition `first_layer && i == loops.first`; `Skirt::find_start_point` computes bbox-center + `r·(cos θ, sin θ)` and rotates the loop to the nearest point | **Built (AC-4)** — nearest rect corner; default −135° selects `(x_min, y_min)`, today's corner, so the default path is unchanged |
| `draft_shield` | coEnum `DraftShield` (`"disabled"`, `"enabled"`) | `disabled` | — | enum, values in canonical order | `Print::has_infinite_skirt` (`dsEnabled && skirt_loops > 0` → skirt on every layer). Canonical also handles a legacy `"limited"` value in `PrintConfigDef::handle_legacy`; this port does not carry it (AC-7 asserts it is rejected) | **Built (AC-2)** — full layer span |
| `single_loop_draft_shield` | coBool | `false` | — | bool, default false | `GCode::generate_skirt`: `start_idx = loops.second - 1` on non-first layers → innermost wall only | **Built (AC-3)** — upper layers emit exactly the innermost ring |

Supporting (not an Orca-gap key of this packet, declared to serve `min_skirt_length`): `filament_diameter` — canonical coFloats (per-filament array); this port's `ResolvedConfig` flattens it to a scalar `f32` via `extract_float_or_first`. Declared on `skirt-brim` at the same scalar shape other modules use for `layer_height` / `nozzle_diameter`.

## Recorded Divergences

**ID convention.** The `D-258-*` labels below are **packet-local divergence identifiers** used for cross-referencing inside this packet's four files. They are *not* `docs/DEVIATION_LOG.md` row IDs — that log uses the `DEV-###` format (verified against the live log at authoring; no `D-258*` token appears in it). Step 9 registers the three divergences that change emitted output relative to canonical — **D-258-3**, **D-258-4** and **D-258-5** — as `DEV-###` rows, with each ID **re-derived from the log at write time** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, take the next), never frozen into this packet (CLAUDE.md ledger-fact rule). **D-258-1** and **D-258-2** describe pre-existing properties of the port's skirt generator that this packet inherits rather than introduces, so they get no row.


- **D-258-1 — bbox rings, not hull offsets.** Canonical offsets a convex hull; the port emits axis-aligned rect loops. Pre-existing in `SkirtBrim::generate_skirt_entities`, inherited by every behaviour this packet adds (grouping envelopes, start corners, and expansion rings are all rect-based). Rationale: changing skirt geometry to hull-offset is a separate geometric packet with its own parity surface; carrying the rect shape keeps this packet's five decision points independently verifiable.
- **D-258-2 — innermost-first loop order.** The port emits skirt loops innermost-first; canonical generates inward-to-outward then `reverse()`s so the exported list is outermost-first. Consequence: canonical's `first_layer && i == loops.first` rotated-start lands on the *outermost* wall, here it lands on the *innermost*. Recorded rather than fixed — reversing the port's export order is an ordering change affecting every existing skirt baseline, out of this packet's scope.
- **D-258-3 — single extruded-length accumulator.** Canonical tracks `extruded_length` per extruder and rotates `extruder_idx` as each target is met; the port accumulates once. Rationale: `ResolvedConfig::filament_diameter` is scalar-flattened (`extract_float_or_first`) and the port has no per-filament flow model, so per-extruder rotation would be fiction. The single-accumulator form is exact for single-material prints, which is every case this port currently produces skirts for.
- **D-258-4 — resolved synthetic `filament_diameter` (an improvement over the pre-packet port).** `serialize.rs` today emits `; filament_diameter = 1.75,1.75,...` from a hardcoded literal whenever raw config lacks the key — which was always, because `to_config_map` never exported it. After this packet the array is built from the resolved `ResolvedConfig::filament_diameter`, so a 2.85 mm setup no longer reports 1.75 to the viewer. The array form (comma-joined, one entry per tool) is preserved exactly, because OrcaSlicer's `ConfigBase::load_from_gcode_file` infers filament count from that array's length; a bare scalar would break the inference. A raw `ConfigValue::List` supplied by the user still wins verbatim.
- **D-258-5 — corner selection, not mid-edge seating.** `Skirt::find_start_point` rotates the loop to the polygon point nearest the computed target; on a rect loop the port selects the nearest of the four corners rather than splitting an edge. Rationale: the port's rect loop has exactly four distinct vertices, so "nearest point on the polygon" and "nearest vertex" differ only by an edge-interior seam the port cannot express without inserting a vertex, which would perturb the default path.

## Acceptance Summary

| AC | Key(s) | Non-default value asserted | Home test |
| --- | --- | --- | --- |
| AC-1 / AC-N3 | all five + `filament_diameter` + `layer_height` | — (manifest guard) | `skirt-brim::skirt_config_schema_tdd` |
| AC-2 | `draft_shield` | `"enabled"` | `skirt-brim::finalization_live_tdd` |
| AC-3 | `single_loop_draft_shield` | `true` | `skirt-brim::finalization_live_tdd` |
| AC-4 | `skirt_start_angle` | `45.0` | `skirt-brim::skirt_brim_tdd` |
| AC-5 | `skirt_type` | `"perobject"` | `skirt-brim::skirt_brim_tdd` |
| AC-6 / AC-N2 | `min_skirt_length` | `20.0`, `1.0e9` | `skirt-brim::skirt_brim_tdd` |
| AC-7 | `skirt_type`, `draft_shield`, `skirt_start_angle` | rejection path | `slicer-scheduler::integration::config_bounds_enforcement_tdd` |
| AC-8 | `filament_diameter` | `2.85` | `slicer-gcode::serialize::tests` |
| AC-9 | `skirt_type` | `"perobject"` | `slicer-runtime::integration::gcode_header_thumbnail_config_blocks_tdd` |
| AC-10 | all five | — (generated docs) | `cargo xtask gen-config-docs --check` |
| AC-N1 | all five | default-path identity (**additional**, never the only evidence for any key) | `skirt-brim::skirt_brim_tdd` |

Map gate (b) check: every one of the five keys has at least one AC asserting a behaviour change at a non-default value — `draft_shield` AC-2, `single_loop_draft_shield` AC-3, `skirt_start_angle` AC-4, `skirt_type` AC-5, `min_skirt_length` AC-6.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p skirt-brim --test skirt_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1 / AC-N3 manifest guard | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p skirt-brim --test finalization_live_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2 / AC-3 on the live `run_finalization` path | FACT pass/fail |
| `cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-4 / AC-5 / AC-6 / AC-N1 / AC-N2 | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-7 enum/bounds rejection | FACT pass/fail |
| `cargo test -p slicer-gcode --lib serialize::tests::config_block 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-8 CONFIG_BLOCK filament array | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-9 CONFIG_BLOCK reachability | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | AC-10 generated docs | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (manifest + `src/lib.rs` edits) | FACT exit code |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate (new test fixtures) | FACT exit code |

## Step Completion Expectations

- The host export (Step 2) and the CONFIG_BLOCK correction (Step 3) must land in the same commit: exporting `filament_diameter` without the serializer correction emits a bare scalar and breaks OrcaSlicer's filament-count inference. AC-8 is the guard.
- The manifest additions (Step 1) must precede the module wiring (Steps 4–6): `bind_module_config_view` filters to declared keys, so an undeclared key reads as absent and every non-default AC silently passes on the default branch.
- After every step that touches `skirt-brim.toml` or `skirt-brim/src/lib.rs`, `cargo xtask build-guests --check` must return exit 0 before any host-integration or CONFIG_BLOCK test result is believed (CLAUDE.md Guest WASM Staleness).

## Context Discipline Notes

- The implementer reads `modules/core-modules/skirt-brim/src/lib.rs` in full (it is well under the 600-line ceiling) and `crates/slicer-ir/src/resolved_config.rs` / `crates/slicer-gcode/src/serialize.rs` only in located ±40-line windows around `to_config_map` and the synthetic-`filament_diameter` branch — both files are far over the ceiling.
- Every cargo invocation is delegated with a FACT return; output tees to `target/test-output.log` and is read from disk, never re-run for more output.
