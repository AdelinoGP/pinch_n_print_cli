# 122 — Author packet — prime tower body parity with canonical

Type: task
Status: open
Assignee: —
Blocked by: 06, 100
Map: ../map.md

## Question

Author the spec packet that grows this port's prime tower from a purge-only stub
into a real tower body at **parity with OrcaSlicer's functionality and
behaviour** (user ruling, 2026-09-03, recorded in
[ticket 29](./29-author-packet-p22-multimaterial-filament-for-features-wipe-tower.md)).

**Read ticket 29's answer first.** It holds the body-class census, the canonical
body definition, and the port-side seam analysis; none of it is restated here,
and re-deriving it wastes the session.

### What the packet builds

The four body classes canonical's `WipeTower2::finish_layer` emits once per
tower layer, plus the global planning that makes them a coherent structure:

- **shell** — the outer wall (`generate_support_rib_wall` /
  `generate_support_cone_wall`) and the inner perimeter of the sparse section;
- **infill** — the "CP EMPTY GRID" pass, including canonical's solid-vs-sparse
  rule (solid when the *next* layer rams a soluble filament, or on the adhesion
  first layer);
- **brim** — first layer only;
- **idle layers** — a tower layer on every layer the tower spans, not only on
  layers that carry a tool change;
- **global depth planning** — `WipeTower2::plan_tower`'s top-down depth
  propagation and the tower-wide max depth.

### Keys this unblocks

The census keys from ticket 29 — the ten packet 255 declared with-gap, P02's
framework / brim-width / infill-gap / flat-ironing keys, and
`wipe_tower_filament` (folded in from P22, which is dissolved). Take the key
list from ticket 29's census table and re-derive membership from disk at
authoring time; do not freeze it from here.

`wipe_tower_filament` specifically is the tool selection over the body —
canonical's `ToolOrdering::insert_wipe_tower_extruder` plus
`WipeTower2::first_toolchange_to_nonsoluble_nonsupport`. Ticket 29 found the
seam already takes an explicit `tool_index`, and found the one constraint that
matters: intra-layer tool changes are emitted only from `layer.tool_changes`
(`crates/slicer-gcode/src/emit.rs`), so a forced-filament body must sit at the
layer boundary or record a `ToolChange`. This packet owns that choice.

`timelapse_type` was folded in from P24 (dissolved by
[ticket 31](./31-author-packet-p24-others-special-mode-wipe-tower.md), which
holds the canonical read-site analysis). Smooth mode is a *body* selector, not a
timelapse toggle: it forces the tower to exist for a single filament
(`Print::has_wipe_tower` via `Print::enable_timelapse_print`), forces a tower
layer on every object layer (`ToolOrdering`), floors and then equalises every
layer's depth to layer 0's (`WipeTower::plan_tower`), and makes
`only_generate_out_wall` the per-layer deliverable (`WipeTower::generate`,
`finish_layer`'s `only_generate_wall`). It carries one obligation outside this
module: canonical's traditional-timelapse gate is suppressed by
`(!m_wipe_tower || !m_wipe_tower->enable_timelapse_print())` in
`GCode::process_layer`, and this port's gate — `run_gcode_postprocess`
(`modules/core-modules/machine-gcode-emit/src/lib.rs`), recorded as clause (d) of
`DEV-168` — cannot express it, because a `PostPass` module has no view of whether
the wipe-tower module ran. Wiring that clause, and choosing the seam that lets
one module observe the other's presence, is this packet's, and must land with the
smooth wall (suppressing without it leaves a print with no timelapse mechanism at
all).

### Authoring obligations

- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet>
  --preflight` (must pass). Packet number and status derived from disk at
  authoring time (ticket 06).
- **Authoring rules 1–6 bind, and rule 4 hardest.** Parity with canonical's
  *behaviour* is the goal; reproducing its coupling is not. The body belongs in
  `modules/core-modules/wipe-tower` behind the existing
  `PostPass::LayerFinalization` seam — ticket 29 verified `run_finalization`
  already receives all layers and that no WIT, schema, or IR change is implied.
  Where the port's architecture affords a better answer than canonical, take it
  and record a divergence with rationale.
- Apply ticket 02's parity-evidence standard — canonical function-read plus
  invariant tests. `OrcaSlicerDocumented/` is readable, not runnable.
- **This is large.** If it does not fit one packet, split it by body class
  (shell / infill+brim / idle layers + planning) with the planning work first,
  and say so in the answer rather than authoring one packet that cannot close.
- Coordinate with the ⚠ re-authoring of packets 253/255: their prime-tower keys
  cannot be re-authored as key work until this lands, and should be folded here
  rather than declared there.

Resolved when the packet (or packet series) is authored, preflighted, and its
directory linked here.

## Answer
