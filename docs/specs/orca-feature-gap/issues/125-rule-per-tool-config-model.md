# 125 — Rule on the port's per-tool (filament / extruder) config model

Type: grilling
Status: open
Assignee: —
Blocked by: — (was 126; resolved 2026-09-06 — per-tool overrides now compose onto every declared field)
Map: ../map.md

## Question

**Does this port adopt `tool_config:<idx>:<key>` as its per-tool config model,
model OrcaSlicer's `coFloats` lists directly, or take a hybrid — and does an
*extruder* axis exist separately from the *tool* axis?**

Graduated from the map's *"Whether the per-tool config model generalises — and
the ruling that follows"* fog patch, which ticket 118 made answerable.

**Read [118's asset](./118-asset-per-tool-config-inventory.md) first.** It holds
the per-stage inventory — ingest, resolution, module surface, composition — and
none of it is restated here. Re-deriving it wastes the session. The deferred Tier
D key count and ticket 119's key list are ledger facts: take them from
[04's tier table](./04-asset-tier-assignment.md) and ticket 119 at the moment of
the ruling, not from any earlier ticket's prose.

### The decision

Ticket 118 found the mechanism already exists and is general: a
`tool_config:<idx>:<key>` override namespace yielding a whole `ResolvedConfig`
per tool. What does *not* exist is anything that populates it from a real
OrcaSlicer project, because Orca spells per-filament config as a `coFloats` list
on one key. So the ruling is about which shape becomes canonical here:

1. **`tool_config:` is the model.** Add an ingest layer that explodes an Orca
   `coFloats` vector into per-tool overrides. Reuses everything already built;
   the Tier D keys stay scalars in `ResolvedConfig` and become per-tool by
   override. Cost: the ingest layer needs a per-key list of which keys are
   per-filament in canonical.
2. **Model `coFloats` directly.** Widen the deferred fields to `Vec` with
   `_for(tool)` accessors, following `filament_density` / `filament_density_for`.
   Ingest stays trivial; every read site changes, and the module boundary needs a
   list accessor it does not have.
3. **Hybrid** — ingest explodes vectors onto the existing axis, and only the keys
   canonical genuinely reads *as a whole vector* (e.g. `filament_density` for the
   tool count, flush matrices) stay lists.

### Sub-questions the ruling must settle

- **Does the tool axis need to reach unpainted regions?** Today a region's tool
  is known only from a `("material", ToolIndex(n))` paint variant chain, so an
  unpainted object has no tool identity before perimeter generation.
- **Does the module boundary get a per-tool surface** — a `get_float_list`
  accessor on `ConfigView` (host and WIT both lack one), a per-tool `ConfigView`,
  or a `resolve_for_tool(key, tool)` host function — or do modules keep indexing
  raw lists themselves?
- **Is an extruder axis separate from the tool axis in scope?** Canonical
  distinguishes per-filament from per-extruder vectors; this port has neither
  `extruder_printable_area` / `extruder_printable_height` nor a per-extruder
  `nozzle_diameter`, and `tool_config:<idx>:` indexes tools with no notion of the
  extruder carrying them.
- **Does `nozzle_diameter` move out of `extensions` into `ResolvedConfig`?** It
  must before it can be widened, whichever model wins.

### Why it is blocked on 126

`overlay_resolved` covers 29 of 83 declared fields, so a per-tool override on
most keys resolves and is then silently dropped. Any option above is unmeasurable
until that is closed — option 1 in particular would look broken for reasons that
have nothing to do with option 1.

### Obligations

- `/grilling` and `/domain-modeling`; this is a **HITL** ticket and the ruling is
  the human's.
- **Authoring rules 1–6 bind whatever follows**, rule 4 hardest: the answer must
  be the PnP way (modular pipeline, community extensibility), not a
  reproduction of canonical's coupling. Where this port's architecture affords a
  better answer, take it and record a divergence with rationale.
- Resolved when the model is chosen and written down precisely enough that the
  deferred keys can be sized against it — not when they are implemented. Declare
  no key as part of this ticket.

## Answer
