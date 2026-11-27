# 129 — Wipe-tower bed-bounds check validates a square footprint the tower never occupies

Type: bug
Status: resolved
Assignee: wayfinder session (2026-09-04)
Blocked by: —
Map: ../map.md

## Question

`modifier_support_territory_top_shell_layers_keep_full_walls_and_internal_bridge_role`
(`crates/slicer-runtime/tests/e2e/modifier_support_territory_e2e_tdd.rs`) fails at
HEAD on `wayfinder/ticket-100-wipe-tower-rename` with:

```
fatal finalization module failure in PostPass::LayerFinalization for
com.core.wipe-tower: wipe-tower corner (63.000, 232.972) lies outside bed
polygon (code=3, fatal=true)
```

It was the only red in the workspace as of `71a4c832`. The map's "Not yet
specified" note asked whether the cause is ticket 100's rename, the ticket
33/34/35 work, or an older regression. This ticket answers that and fixes it.

## Answer

**Two defects, one latent and one that made it live. Neither is a value-spelling
mismatch — the rename hypothesis in the map's note is wrong.**

### Defect 1 (the actual bug): the check models the tower as a `width × width` square

`WipeTower::run_finalization` (`modules/core-modules/wipe-tower/src/lib.rs`)
validated these four corners:

```
(x, y), (x+w, y), (x+w, y+w), (x, y+w)          // w = tower_width
```

with the comment "Use tower_width for a conservative bound; purge_depth varies
per layer." It is not conservative — it is a different rectangle. The geometry
`generate_purge_paths` actually emits spans

```
x ∈ [tower_x, tower_x + tower_width]
y ∈ [tower_y, tower_y + purge_depth],
    purge_depth = purge_volume / (line_width · layer_height · tower_width)
```

So the validator rejected towers that fit and, on a wide-and-shallow bed, would
equally have accepted towers whose real depth overruns the far edge.

Measured on the failing fixture (`resources/support_test_modifier_normal_in_tree_gui.3mf`
project settings, plus `..._gui.config.json` which overrides `line_width` to 0.42):

| quantity | value |
|---|---|
| `printable_area` | `["0x0","220x0","220x200","0x200"]` → 220 × 200 mm |
| `wipe_tower_x` / `wipe_tower_y` | 3.0 / 172.972 |
| `prime_tower_width` | 60 |
| `prime_volume` | not present → module fallback 45.0 mm³ |
| `line_width` (sidecar) / `layer_height` | 0.42 / 0.2 |
| square check's y-extent | 172.972 + **60** = **232.972** → outside the 200 mm bed → rejected |
| real purge depth | 45 / (0.42 · 0.2 · 60) = **8.93 mm** |
| real y-extent | 172.972 + 8.93 = **181.90** → inside the bed |

Orca placed this tower itself and it fits; only the square model says otherwise.

**Canonical does not model the tower as a square.** `WipeTower::get_depth()`
returns the computed `m_wipe_tower_depth`, and canonical's only consumers pair it
with the width — `Print.cpp`'s `m_wipe_tower_data.construct_mesh(wipe_tower.width(),
wipe_tower.get_depth(), …)` and `m_wipe_tower_data.depth = wipe_tower.get_depth()`.
Width and depth are independent quantities there, exactly as in this port's own
`generate_purge_paths`. So this is a port defect, not a divergence — **no DEV row.**

### Defect 1b: the check ran even when no tower geometry is emitted

The corner check sat *before* the layer loop and fired whenever
`enable_prime_tower` was true, regardless of whether any layer carried a tool
change. The failing fixture is a **single-filament** print: zero tool changes,
zero purge paths, zero tower — and the slice was still failed for the placement
of a tower that would never exist. Canonical builds no prime tower for a
single-filament print.

### Defect 2 (why it went red now): ticket 100 flipped this fixture's tower on

Not a value-spelling mismatch — an *enable* flip. The fixture's
`Metadata/project_settings.config` carries **both** spellings with contradictory
values:

```json
"wipe_tower_enabled": false,
"enable_prime_tower": "1",
```

- **Before ticket 100** (`e1da8b36`): the module read `wipe_tower_enabled` → a
  real JSON `false` → `run_finalization` returned `Ok(())` on the first line.
  `enable_prime_tower` was undeclared and routed to `extensions`, unread. The
  square check was dead code on this fixture.
- **After ticket 100**: the module reads `enable_prime_tower`, which the rename
  made a *declared bool*, so `coerce_string_to_config_value` turns `"1"` into
  `Bool(true)` (pinned by `zero_and_one_coerce_by_declared_key_type` in
  `crates/slicer-model-io/src/loader.rs`). The tower is enabled, the latent
  square check runs, and the slice fails.

Ticket 100's behaviour here is **correct** and is kept: Orca honours
`enable_prime_tower`, and `wipe_tower_enabled` is the legacy name Orca itself
renamed. The rename did not introduce the bug; it exposed it.

The ticket 33/34/35 work and the five-way fill partition are cleared — as the
map's note already recorded from two independent reproductions, and as the
mechanism above explains.

## Fix

`modules/core-modules/wipe-tower/src/lib.rs`:

1. New `max_purge_depth` helper, which walks the same layers / tool changes /
   layer-height derivation `run_finalization` already uses and returns the
   largest `purge_depth` any emitted purge box will reach — sharing
   `purge_volume_for` and the `line_width · layer_height · tower_width` cross
   section with `generate_purge_paths`, so validation and generation cannot
   drift. Returns `None` when no tool change will emit anything.
2. `run_finalization` validates `(x, y) … (x+w, y+max_depth)` and skips the
   check entirely when `max_purge_depth` is `None` (no tower will be built).

Regression tests in `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`:

- `shallow_tower_near_far_bed_edge_is_accepted` — the fixture's exact numbers
  (220×200 bed, tower at (3, 172.972), width 60, purge 45 mm³, line width 0.42,
  layer height 0.2). Fails on the square check with the production error string,
  passes on the depth check.
- `deep_tower_overrunning_far_bed_edge_is_rejected` — the same bed and position
  with a purge volume large enough that the real depth *does* overrun 200 mm.
  Load-bearing: it fails if the depth check is replaced by "ignore y".
- `single_filament_print_skips_bed_validation` — no tool changes, tower nominally
  placed off-bed; must return `Ok` because nothing is emitted.

The three pre-existing negative tests (`tower_outside_bed_returns_fatal`,
`orca_point_string_bed_is_parsed_not_silently_defaulted`) violate the bed on
**x**, or on `tower_y` itself, so they still reject under the corrected model —
the fix narrows nothing they cover.

## Records

- Map's "Not yet specified" bullet removed; replaced by a pointer to this ticket.
