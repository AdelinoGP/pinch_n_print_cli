# Requirements: prime-tower-geometry-keys

## Packet Metadata

- Packet directory: `docs/spec_packets/254a-prime-tower-geometry-keys/`
- Backlog source: `docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md` (wayfinder map packet P02)
- Owner module: `wipe-tower` (`modules/core-modules/wipe-tower/`)
- Sibling half: `docs/spec_packets/254b-prime-tower-interface-and-ramming/`
- Tier: **B** — the packet builds decision points that do not exist in the tree (per-layer tower depth, brim geometry, configurable scan-line pitch), all inside one existing module at its existing stage. Re-derived at authoring per map Authoring rule 1.
- Status: `draft`

## Problem Statement

Packet 254 was authored (2026-08-28) declaring all 13 P02 prime-tower keys in the `wipe-tower` manifest with **one** live decision point (`prime_tower_infill_gap`) and twelve "declared-with-gap" rows. Map Authoring rule 1 now prohibits that disposition, and rule 2 prohibits treating CONFIG_BLOCK emission as evidence. Re-authored, the 13 keys split three ways: three geometry keys this packet **builds**, nine interface/ramming keys the sibling packet `254b` builds, and one key returned to the queue.

The split itself is required by the ticket: 13 keys exceeds the Tier-B ceiling of 12, and the ticket instructs "split by feature if the packet exceeds the B ceiling of 12 keys".

Underneath the key list is a defect this packet must fix to make `prime_tower_enable_framework` mean anything. `WipeTower::run_finalization` inserts one purge block per tool change, and `generate_purge_paths` starts **every** block at `y = tower_y`. With more than one tool change on a layer the blocks overlap exactly. There is no per-layer depth at all, so canonical's framework flag — which forces every layer's depth to the first layer's — would be identity at every value. Building the depth model is therefore not scope creep; it is the precondition for the key being covered rather than declared.

## In Scope

1. **`prime_tower_infill_gap` (scan-line pitch).** The boustrophedon advance in `generate_purge_paths` becomes `(percent/100) × line_width` instead of the hardcoded `line_width`, mirroring canonical's `dy = m_extra_spacing × m_perimeter_width` in `WipeTower::generate` / `generate_new`.
2. **Per-layer tower depth.** `block_depth = prime_volume / (line_width × layer_height × tower_width)`; purge block `k` on a layer is seated at `tower_y + k × block_depth` so the blocks tile rather than overlap; the layer's tower depth is `n_tool_changes(layer) × block_depth`.
3. **`prime_tower_enable_framework` (uniform depth).** `true` forces every tower layer's depth to the first tower layer's, mirroring canonical `WipeTower::generate_wipe_tower_blocks`. `false` (default) keeps per-layer depth.
4. **`prime_tower_brim_width` (first-layer brim).** `loops_num = floor((brim_width + spacing/2) / spacing)` rect loops offset outward from the tower footprint on the first layer that receives tower entities, with `spacing = line_width − layer_height × (1 − π/4)` (canonical `WipeTower2::finish_layer`). The `-1` Auto sentinel resolves via canonical `WipeTower::get_auto_brim_by_height`: `max_height < 100 ? max_height/100 × 8 : 8`, with `max_height` taken as the top layer's `z`.
5. **Bed-bounds validation widened** to the real tower extent (deepest layer + brim) instead of today's conservative `tower_width`-square.
6. **Bounds/enum enforcement, module-visibility isolation, generated docs** for the three keys.

## Out of Scope

- The nine interface / ramming keys — owned by `254b-prime-tower-interface-and-ramming`.
- **Canonical's runtime re-fitting of `m_extra_spacing`.** `WipeTower::plan_tower` overwrites it with `min_wipe_tower_depth / max_depth`, and `plan_tower_new` raises it via `std::max(...)`; `calc_block_infill_gap` and `generate` reset it to `1.f`. The port takes the configured value only (D-254a-2).
- Cone / rib / fillet wall shapes and `wipe_tower_rotation_angle` — packet `255-wipe-tower-geometry-keys`.
- Canonical's **non-first-layer brim chamfer** (`WipeTower::finish_layer` zeroes the loops when `m_layer_info->depth != m_plan.front().depth`, else caps them to 3 mm worth minus the distance from the first layer). The port emits brim on the first tower layer only (D-254a-4).
- Canonical's brim source polygon: `generate_support_cone_wall` / `generate_support_rib_wall` output. The port offsets its footprint rect (D-254a-3).
- Any edit to `ORCA_CONFIG_PADDING` or a CONFIG_BLOCK padding twin (map Authoring rule 2).
- New WIT interface, IR schema bump, or new `ResolvedConfig` field — none is required.

## Returned to Queue — unimplemented, needs a feature this packet does not build

- **`prime_tower_skip_points`** (coBool, default `true`). Canonical enables wall-skip-point computation — `WipeTower::get_all_wall_skip_points` → `WipeTower::get_wall_skip_points`, called from `WipeTower::plan_tower_new`; `WipeTower2::compute_wall_skip_points` from `WipeTower2::generate`; consumed geometrically in `WipeTower::generate_support_wall_new` (`construct_gap_for_skip_points`, `remove_points_from_segment`) and for routing in `WipeTowerIntegration::append_tcr` / `travel_to_tower_gap` (`GCode.cpp`) as `is_used_travel_avoid_perimeter`. **Needs: a travel-avoid-perimeter facility.** This port has no travel-avoidance machinery anywhere, and it would not live in `wipe-tower` — it belongs with path optimization / travel planning. Named in the map's tier table as unimplemented; not declared here.

Note that canonical also ANDs this flag into `m_use_gap_wall`, which gates the interface tower and `prime_tower_flat_ironing`; `254b` records that coupling rather than reproducing it.

## Ruled Dead-in-canonical

None. All three keys have live read sites inside `src/libslic3r/` in the slicing pipeline (see §Per-Key Canonical Evidence). `prime_tower_skip_points` is also live in canonical — it is *returned*, not ruled dead.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` §Host-Boundary Access Enforcement (Normative) — governs the `layer_height` re-declaration and the AC-N1 isolation arm.
- `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract — forbids padding edits.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; the module works in plain mm floats and must not import canonical's `scale_()`.
- `docs/00_project_overview.md` — modular pipeline / config robustness; all three decision points land inside the owning module.
- `docs/15_config_keys_reference.md` — generated; never hand-edited.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the two declarations.
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` — `WipeTower::WipeTower` ctor, `WipeTower::generate`, `WipeTower::generate_new`, `WipeTower::align_perimeter`, `WipeTower::generate_wipe_tower_blocks`, `WipeTower::plan_tower_new`, `WipeTower::get_auto_brim_by_height`, `WipeTower::finish_layer`, `WipeTower::finish_layer_new`.
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` — `WipeTower2::finish_layer`.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::wipe_tower_data`.

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

## Parity Evidence Standard

Every canonical claim below was produced by a delegated read of the sibling checkout at authoring time and is cited by **file + function only**, never by line number. In-tree citations are by crate-qualified path + symbol name. A worker who disputes a row re-dispatches the read rather than reading in-context. Behaviour is pinned with invariant tests (counts, disjointness, formula equality), not golden G-code — the canonical checkout is readable, not runnable.

## Per-Key Canonical Evidence

| Key | Canonical type | Default | Bounds | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- |
| `prime_tower_infill_gap` | coPercent | `150` | min 100, no max | `WipeTower::WipeTower` ctor sets `m_extra_spacing = value/100`; consumed as the wipe-path pitch `dy = m_extra_spacing × m_perimeter_width` in `WipeTower::generate` / `WipeTower::generate_new`, as `spacing = m_extra_spacing × m_perimeter_width` in `WipeTower::align_perimeter`, as `spacing_ratio = m_extra_spacing − 1` in `WipeTower::calc_block_infill_gap`, and as the ramming `y_step` in `WipeTower::toolchange_Unload`. Depth scaling in `WipeTower::plan_toolchange` / `update_all_layer_depth`; Print-side preview in `Print::wipe_tower_data`. **`WipeTower2` does not read this key** — its spacing comes from `wipe_tower_extra_spacing`. | **Built (AC-2)** — the port takes the pitch consumption only |
| `prime_tower_brim_width` | coFloat, `gui_type = f_enum_open`, `-1` labelled "Auto" | `3.0` | min −1, no max | `WipeTower2::finish_layer` (first-layer only): `spacing = m_perimeter_width − m_layer_height × (1 − π/4)`; `loops_num = (brim_width + spacing/2) / spacing`; each loop `poly = offset(poly, scale_(spacing)).front()` — outward, one spacing per loop — from the wall polygon of `generate_support_cone_wall` / `generate_support_rib_wall`; emitted via `writer.extrude` at first-layer feedrate with `m_extrusion_flow`; result stored as `m_wipe_tower_brim_width_real = loops_num × spacing`. `WipeTower::finish_layer` (box-expand form) and `WipeTower::finish_layer_new` (`offset(outer_wall, scaled(spacing))`) are the sibling implementations. The `-1` sentinel resolves in `WipeTower::plan_tower_new` and, Print-side, in `Print::wipe_tower_data`, both via `WipeTower::get_auto_brim_by_height(max_height)` = `max_height < 100 ? max_height/100 × 8 : 8`, `max_height` = max unscaled object Z. | **Built (AC-5, AC-6)** — rect-offset loops on the first tower layer, Auto resolved from top-layer `z` |
| `prime_tower_enable_framework` | coBool | `false` | — | `WipeTower::WipeTower` ctor sets `m_tower_framework`; **read only** by `WipeTower::generate_wipe_tower_blocks`, which for every `layer_id >= 1` assigns `block.layer_depths[layer_id] = block.layer_depths[0]` and recomputes `m_plan[layer_id].depth` as the sum — forcing every layer's tower depth to the first layer's. No other read site. | **Built (AC-4)** — on top of the per-layer depth model this packet builds (AC-3) |

## Recorded Divergences

**ID convention.** The `D-254a-*` labels are **packet-local divergence identifiers** for cross-referencing inside this packet's five files. They are *not* `docs/DEVIATION_LOG.md` row IDs — that log uses the `DEV-###` format (verified against the live log at authoring; no `D-254a*` token appears in it). The closure step registers the output-affecting divergences as `DEV-###` rows with each ID **re-derived from the log at write time** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, take the next), never frozen here (CLAUDE.md ledger-fact rule).

- **D-254a-1 — overlapping purge blocks fixed (a defect repair, not a port choice).** Pre-packet, every purge block on a layer started at `y = tower_y`, so N tool changes produced N exactly-overlapping blocks. Canonical tiles them along depth. The per-layer depth model (AC-3) fixes this. Output changes at defaults for any multi-toolchange layer; the packet owns the baseline updates.
- **D-254a-2 — configured pitch only, no runtime re-fit.** Canonical re-fits `m_extra_spacing` to meet a minimum tower depth (`WipeTower::plan_tower`, `plan_tower_new`) and resets it to `1.f` in `calc_block_infill_gap` / `generate`. The port has no tower planner to fit against, so it uses the configured value throughout. Rationale: the re-fit is a property of canonical's block planner, which is a separate feature; reproducing the reset-to-`1.f` without the planner would silently discard the key.
- **D-254a-3 — brim offsets the footprint rect, not a wall polygon.** Canonical offsets the tower's generated outer-wall polygon (cone/rib). The port's tower is an axis-aligned rect block, so the brim offsets that rect. Inherited from the port's tower geometry, not introduced here; packet 255 owns the wall shapes.
- **D-254a-4 — first-tower-layer brim only, no chamfer taper.** Canonical adds a reducing "brim chamfer" on later layers whose depth differs from the first layer's. The port emits brim on the first tower layer only. Rationale: the chamfer is expressed in terms of canonical's `m_layer_info->depth` plan, which the port's depth model (AC-3) does not reproduce in full; emitting a partial chamfer would be less faithful than emitting none.
- **D-254a-5 — Auto height is the top layer's `z`.** Canonical's `max_height` is the maximum unscaled object Z over `m_objects`, computed Print-side. At `PostPass::LayerFinalization` the module sees the layer collection, so it uses the top layer's `z` — the same quantity by construction for a single-plate print, and available without a new carrier.

## Acceptance Summary

| AC | Key(s) | Non-default value asserted | Home test |
| --- | --- | --- | --- |
| AC-1 / AC-N3 | all three + `layer_height` | — (manifest guard) | `wipe-tower::wipe_tower_config_schema_tdd` |
| AC-2 | `prime_tower_infill_gap` | `"200%"` | `wipe-tower::wipe_tower_tdd` |
| AC-3 | (per-layer depth model — precondition for AC-4) | 3 tool changes | `wipe-tower::wipe_tower_tdd` |
| AC-4 | `prime_tower_enable_framework` | `true` | `wipe-tower::wipe_tower_tdd` |
| AC-5 | `prime_tower_brim_width` | `6.0` | `wipe-tower::wipe_tower_tdd` |
| AC-6 | `prime_tower_brim_width` | `-1.0` (Auto) | `wipe-tower::wipe_tower_tdd` |
| AC-7 | (bed bounds under the widened extent) | deepest layer + brim | `wipe-tower::bed_bounds_tdd` |
| AC-8 | `prime_tower_infill_gap`, `prime_tower_brim_width` | rejection + percent threading | `slicer-scheduler::integration::config_bounds_enforcement_tdd` |
| AC-9 | all three | — (generated docs) | `cargo xtask gen-config-docs --check` |
| AC-N1 | `prime_tower_infill_gap` | module-visibility isolation | `slicer-runtime::contract::config_view_binding_tdd` (arm authored here) |
| AC-N2 | all three | enable-gate identity (**additional**, never the only evidence for any key) | `wipe-tower::finalization_live_tdd` |

Map gate (b) check: every kept key has at least one AC asserting a behaviour change at a non-default value — `prime_tower_infill_gap` AC-2 (`"200%"`), `prime_tower_enable_framework` AC-4 (`true`), `prime_tower_brim_width` AC-5 (`6.0`) and AC-6 (`-1.0`).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1 / AC-N3 manifest guard | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2 / AC-3 / AC-4 / AC-5 / AC-6 | FACT pass/fail |
| `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-7 widened bed extent | FACT pass/fail |
| `cargo test -p wipe-tower --test finalization_live_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N2 enable gate | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-8 bounds + percent threading | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-N1 leakage blocked (arm authored by this packet) | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | AC-9 generated docs | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (manifest + `src/lib.rs` edits) | FACT exit=0 |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate | FACT exit code |

## Step Completion Expectations

- The per-layer depth model (Step 3) must land before the framework key (Step 4): without it the key is identity at every value and map gate (a) fails.
- The manifest step must precede every wiring step — `bind_module_config_view` filters to declared keys, so an undeclared key reads as absent and every non-default AC would silently pass on the default branch.
- The bed-bounds widening (Step 6) must land in the same commit as the depth model, because the depth model is what makes today's `tower_width`-square bound wrong in both directions.
- After every step touching `wipe-tower.toml` or `wipe-tower/src/lib.rs`, `cargo xtask build-guests --check` must return exit 0 before any host-integration result is believed (CLAUDE.md Guest WASM Staleness).

## Context Discipline Notes

- `modules/core-modules/wipe-tower/src/lib.rs` is 772 lines at authoring — **over the 600-line ceiling**. Read it in located windows around `from_config`, `generate_purge_paths`, `run_finalization` and the bed-bounds block only.
- `crates/slicer-ir/src/resolved_config.rs` and `crates/slicer-gcode/src/serialize.rs` are not in this packet's change surface and must not be opened.
- Every cargo invocation is delegated with a FACT return; output tees to `target/test-output.log` and is read from disk, never re-run for more output.
