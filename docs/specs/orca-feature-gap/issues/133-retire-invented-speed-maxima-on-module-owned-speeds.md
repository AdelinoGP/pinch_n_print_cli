# 133 — Retire the invented `max = 300.0` on the three module-owned speed keys

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 113, which found `max = 300.0` on twelve module-manifest rows
covering `FeedrateConfig`'s `SPEED_KEYS` and retired them: canonical declares
**no maximum on any speed key**, so the cap was a PnP invention that would
reject legitimate high-speed profiles.

Three rows with the identical cap were **deliberately left alone**, because they
are module-owned speeds outside `FeedrateConfig` and ticket 113 derived no
canonical bound for them. Changing them on the strength of a neighbouring key's
finding is the mistake the map's ticket-27 note warns about — re-derive the key,
do not generalise from its neighbour.

| key | manifest |
|---|---|
| `internal_solid_infill_speed` | `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` |
| `support_ironing_speed` | `modules/core-modules/support-surface-ironing/support-surface-ironing.toml` |
| `wave_overhang_print_speed` | `modules/core-modules/wave-overhangs/wave-overhangs.toml` |

Decide and execute:

1. **Per key, derive the canonical declaration** from `PrintConfigDef`
   (`PrintConfig.cpp`) — min, max, type, default. `internal_solid_infill_speed`
   is a canonical key; `support_ironing_speed` and `wave_overhang_print_speed`
   may be PnP-specific, in which case say so and choose the bound deliberately
   rather than inheriting 300 by inertia. Cite by file + function, never line
   numbers; the oracle is the ticket-33 checkout.
2. **Whether any of them should join `SPEED_KEYS`** rather than stay
   module-owned. Ticket 109 already asks this for `support_ironing_speed`;
   re-derive its status at point of use rather than trusting this line. A key
   that joins inherits `SPEED_BOUNDS` and its const assertion automatically.
3. **`support_surface_ironing`'s schema test** asserts the literal `max = 300.0`
   (`modules/core-modules/support-surface-ironing/tests/support_ironing_config_schema_tdd.rs`)
   — update it with the manifest, do not weaken it to a range.

Read ticket 113's answer first for the canonical grounding and the
enforcement-is-a-divergence framing; do not re-derive them.

Not a queue key; changes no queue count.

## Answer
