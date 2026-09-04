# 127 — Rule on gyroid-infill's density for solid, top/bottom and bridge surfaces

Type: grilling
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

**`gyroid-infill` fills every role it holds at `sparse_infill_density`. Canonical
never does that for a solid surface. What should this port do — force the solid
density like canonical, honour the per-role density keys inside gyroid, or stop
gyroid holding the solid/bridge claims at all?**

Filed by ticket 34 (P27).

### What was measured

`GyroidInfill::run_infill` (`modules/core-modules/gyroid-infill/src/lib.rs`)
resolves **one** density — `resolve_percent_float(region, "sparse_infill_density",
…)` — and its `emit_polys` closure passes that same `region_density` to
`fill_expolygon` for all four roles it emits: `SparseInfill`, `TopSolidInfill`,
`BottomSolidInfill` and `BridgeInfill`. Its manifest holds
`claim:sparse-fill`, `claim:top-fill`, `claim:bottom-fill`, **and**
`claim:bridge-fill` (`modules/core-modules/gyroid-infill/gyroid-infill.toml`).

Canonical `Fill::make_fills` (`Fill.cpp`) does the opposite. For a solid surface
it overrides both pattern and density: an external non-bridge surface takes
`top_surface_pattern` / `bottom_surface_pattern` with
`top_surface_density` / `bottom_surface_density`; a solid infill surface takes
`internal_solid_infill_pattern` at `density = 100`; anything else solid —
**which is where a bridge lands** — is forced to `ipRectilinear` (or `ipMonotonic`
when `top_surface_pattern` is monotonic) at `density = 100`, then overridden again
by `bridge_density` / `internal_bridge_density`. Gyroid is never the pattern of a
bridge or a solid surface in canonical.

So on this port, selecting gyroid as the `claim:top-fill` / `claim:bottom-fill` /
`claim:bridge-fill` holder yields a **porous top surface and a hollow bridge** at
any `sparse_infill_density` below 100%, and P27's three keys
(`bridge_density`, `internal_bridge_density`, `thick_internal_bridges`) drive
nothing at all on that holder — they are live only on `rectilinear-infill` and
`wave-overhangs` (ticket 34).

### What this ticket must settle

1. Is a gyroid *solid* surface a deliberate divergence this port keeps (a
   capability canonical lacks), or a defect?
2. If it is kept, which density does each role read — the per-role keys
   (`top_surface_density`, `bottom_surface_density`, `bridge_density`,
   `internal_bridge_density`) or a forced 100%? Note the per-role keys belong to
   other packets in the queue, so the ruling scopes work outside this ticket.
3. Does `thick_internal_bridges` mean anything for a gyroid bridge (there is no
   line spacing to widen in the same sense), or is the answer that gyroid must
   not hold `claim:bridge-fill`?
4. If a claim is dropped from the manifest, check
   `validate_startup_dag_with_configured_holders`
   (`crates/slicer-scheduler/src/validation.rs`) — an unmatched holder is now a
   fatal error, so dropping a claim changes what configs are valid.

## Answer
