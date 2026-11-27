# 30 — Author packet P23 — Multimaterial / Flush options — wipe-tower

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-03)
Blocked by: 06, 100
Map: ../map.md

## Question

Author the spec packet for **P23 — Multimaterial / Flush options — wipe-tower** — 2 keys, Tier B new logic, owner wipe-tower. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P23 — Multimaterial / Flush options — wipe-tower):

`flush_multiplier`, `flush_volumes_matrix`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed by direct implementation — no packet**, under the map's "Packets are for
complex implementation only" rule. Both keys are now live in
`modules/core-modules/wipe-tower`.

The ticket was filed as "Tier B new logic". Sized from the tree at claim time it
is not: **the decision point already exists.** `WipeTower::generate_purge_paths`
already turns a purge volume into geometry — the scan-line box depth
(`purge_volume / cross_section`) and the prime entity's extruded length — it just
read one flat number for every tool change and ignored its `tc` argument entirely
(the parameter was spelled `_tc`). Canonical does exactly the same arithmetic; the
only thing it does differently is *choose* the number per filament pair. So the
work was a per-pair lookup in front of an existing consumer, not new logic.

### What landed

`flush_volumes_matrix` (`float-list`) and `flush_multiplier` (`float`, min 0.0)
are declared on `wipe-tower.toml`, parsed in `WipeTower::from_config`, and
consumed by the new `WipeTower::purge_volume_for(from_tool, to_tool)`, which
`generate_purge_paths` now calls with the tool change it is serving. Both the live
`run_finalization` path and the legacy `process` path route through it.

The matrix is a flat row-major `N*N` list indexed `[from_tool][to_tool]`, matching
canonical `WipeTower2::extract_wipe_volumes`, which derives `N` as `sqrt(size)`
and slices the flat vector into rows. `parse_flush_volumes_matrix` rejects a
non-square length rather than truncating it, because a mis-sized matrix
mis-indexes every pair — a silent wrong answer is worse than a startup error.

### Canonical, read not assumed

`WipeTower2::extract_wipe_volumes` (`WipeTower2.cpp`) is the consumer:
`wipe_volumes[from][to] = flush_volumes_matrix[from*N+to] * flush_multiplier`,
clamped up by `filament_minimal_purge_on_wipe_tower`, zeroed entirely unless
`purge_in_prime_tower && single_extruder_multi_material`. The result reaches
`WipeTower2::plan_toolchange` as `wipe_volume`, where `get_wipe_depth` converts it
to the depth of that toolchange's purge box. `ToolOrdering::prepare_flush_matrices`
is the second consumer, and it is where the multi-nozzle extension lives
(per-nozzle matrices via `get_flush_volumes_matrix`, and the
`prime_volume_mode == pvmFast` branch that swaps `flush_multiplier` for
`flush_multiplier_fast`). Rule 3 passes: both keys have live read sites inside
`libslic3r/`, not just GUI or preset plumbing.

### Four recorded divergences, filed as `DEV-169`

1. **No matrix means `prime_volume`, not zero.** Canonical always has a
   preset-supplied matrix and zeroes it unless
   `purge_in_prime_tower && single_extruder_multi_material` — **neither key exists
   in this tree** (both are P02, tier table rows). Falling back to the flat
   `prime_volume` keeps the tower working for prints that configure no matrix
   instead of silently emitting an empty tower, and keeps every existing print's
   g-code unchanged.
2. **`flush_multiplier` scales matrix entries only**, never the `prime_volume`
   fallback. Applying it there would cut every unconfigured print's purge to 30%
   of its declared volume, which no key in canonical does.
3. **One scalar multiplier, not one per extruder.** Canonical's `flush_multiplier`
   is a `coFloats`; `ToolOrdering::prepare_flush_matrices` indexes it per nozzle
   while `WipeTower2::extract_wipe_volumes` reads `get_at(0)`. Per-tool config
   scoping is not available to this module — that is exactly what **ticket 118** is
   inventorying — so this matches the `WipeTower2` reading. `flush_multiplier_fast`
   and `prime_volume_mode` are not in this map's queue and were not added.
4. **`filament_minimal_purge_on_wipe_tower` applies no clamp.** It is Tier D
   per-filament config (`04-asset-tier-assignment.md`: "deferred (per-filament
   config model)") and does not exist here.

### The manifest default is dead — the effective default is in code

Per the map's Notes: only `percent` / `float_or_percent` schema defaults are
threaded into `ResolvedConfig` by `resolve_global_config`
(`ConfigFieldEntry::parsed_default` is `None` for every other type), so a plain
`float` or `float-list` manifest default never reaches a module. `flush_multiplier`
therefore carries its canonical default (`0.3`, from
`PrintConfigDef::init_fff_params`) as `DEFAULT_FLUSH_MULTIPLIER` in
`modules/core-modules/wipe-tower/src/lib.rs`, pinned by
`flush_multiplier_defaults_to_canonical_value`. The manifest declaration is still
load-bearing for the `min 0.0` bound, which `ConfigBoundsIndex::check` enforces on
the way in.

`flush_volumes_matrix` deliberately declares **no** default. Canonical's is a
4x4 matrix (`0` diagonal, `280` off-diagonal); the tool count is a property of the
print, so a fixed 4x4 would be wrong for any other count. A user- or profile-supplied
value reaches the module through `ResolvedConfig::extensions` -> `to_config_map()`,
the same route `printer_structure` uses (ticket 27).

### Verification

`cargo test -p wipe-tower` — 7 new tests, all green:

- `absent_flush_matrix_falls_back_to_prime_volume`
- `flush_matrix_selects_per_pair_volume` — `[0][1]` and `[1][0]` differ, diagonal is 0
- `flush_multiplier_scales_emitted_purge_geometry` — **behaviour change at a
  non-default value**: multiplier 0.5 vs 1.0 over the same matrix changes the
  emitted scan-line count, asserted against the closed-form depth
- `flush_matrix_changes_emitted_purge_geometry_against_prime_volume` — matrix vs
  `prime_volume` baseline
- `tool_index_outside_matrix_falls_back_to_prime_volume`
- `non_square_flush_matrix_returns_fatal`
- `flush_multiplier_defaults_to_canonical_value`

No-regression, narrow: `cargo test -p slicer-gcode --test gcode_toolchange_wrapping`
(3 passed), `cargo test -p slicer-runtime --test contract -- wipe_tower` (1 passed),
`cargo test -p slicer-runtime --test executor -- finalization_live` (2 passed).

Gates: `cargo clippy --workspace --all-targets -- -D warnings` clean;
`cargo xtask check-literals` 0 violations; `cargo xtask build-guests` rebuilt
(`wipe-tower.wasm` was stale the moment the module changed) and
`cargo xtask build-guests --check` exits `0`.

**Also fixed, incidentally:** `cargo xtask check-deviations` was failing on
`docs/DEVIATION_LOG.md` before this session — ticket 27's `DEV-168` row quoted
canonical's `... || is_multi_extruder` verbatim, and the two literal pipes split
the row into 9 columns. Escaped to `&#124;&#124;`. The gate now passes and doc 07's
Open Deviation Map regenerates.

Status: **P23 closed by direct implementation.** No packet authored.
