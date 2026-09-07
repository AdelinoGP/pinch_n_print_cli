# 140 — e2e red: ticket 42's strict pressure-advance ingest rejects real Orca 3MF vectors

Type: task
Status: open
Assignee: —
Blocked by: 125
Map: ../map.md

## Question

**Fix the 5 e2e failures ticket 42's commit left red at HEAD:
`cargo test -p slicer-runtime --test e2e` fails
`painted_cube_3mf_reaches_paint_segmentation`,
`paint_config_override_visibly_differs_gcode`,
`modifier_infill_boundary_anchoring`, `modifier_infill_two_densities`, and
`modifier_support_territory_top_shell_layers_keep_full_walls_and_internal_bridge_role`
(139/144 green) with `config resolution failed: config key
'pressure_advance': expected Float value, got List` (or the
`enable_pressure_advance` Bool variant).**

Found by [ticket 50](50-author-packet-p43-multimaterial-prime-tower-emitter.md)'s
verification session (2026-09-06); attribution is proven without a
stash-baseline cycle:

- The strict extractors are **committed at HEAD** (`git show
HEAD:crates/slicer-ir/src/resolved_config.rs`): `enable_pressure_advance`
via `extract_bool`, `pressure_advance` via `extract_float` — both reject
`ConfigValue::List`.
- Real Orca fixtures carry both keys as per-filament vectors, e.g.
`resources/cube_4color.3mf` `Metadata/project_settings.config` holds
`"enable_pressure_advance": ["0","0","0","0"]` (4-element `coBools`) and a
`pressure_advance` vector (`coFloats`).
- The failure fires at **config resolution**, before any emission — so no
later working-tree change (including ticket 50's default-inert bool) can
cause it, and the 344-test integration suite plus 296-test contract suite
stay green around it.

Ticket 42's own answer already names the proper fix: scalar-global now,
**Orca vector ingest rides
[125](125-rule-per-tool-config-model.md)** — which is why this ticket is
blocked on 125 (itself gated on
[126](126-overlay-resolved-field-narrowing.md)) rather than fixed here.
Ticket 118's inventory documents the general shape (`extract_float_or_first`
silently keeps element 0 for other keys; these two keys hard-fail instead —
the inconsistency is part of what 125 rules on).

### Scope notes for claim time

- Re-derive the failing set at claim time — the count (5) and names above
are ledger facts from 2026-09-06; other tickets' landings may move them.
- **2026-09-06 (ticket 126's session): the set already moved.** Ticket 47's
`filament_flush_temp` / `filament_flush_volumetric_speed` scalars reject the
same fixtures' 4-element Orca `coFloats` string lists first
(`filament_flush_volumetric_speed: expected Float value, got List` at
`resolve_global_config`, measured on `resources/cube_4color.3mf` whose
`Metadata/project_settings.config` carries all four keys as string lists).
Four of the five named tests now fail on the flush key, not the PA keys —
same defect class, one more strict scalar. The fix that lands here should
cover every strict scalar against vector-carrying fixtures, not just the two
PA keys.
- Check whether the same strictness breaks other vector-carrying 3MFs
beyond these five (e.g. any fixture carrying `retraction_length`-style
`coFloats` lists against newer scalar fields).
- This is **not** ticket 130's scope (that ticket owns two named
integration reds — a stale fixture and a drifted proxy — not these five
e2e resolution failures).
- No config key is declared, renamed, or re-tiered by this ticket; the
queue count does not change.

Resolved when the five (re-derived) e2e tests slice their fixtures green
with the vectors honored per 125's model — or with an explicit,
human-signed-off divergence if 125 rules the vectors out.

## Answer
