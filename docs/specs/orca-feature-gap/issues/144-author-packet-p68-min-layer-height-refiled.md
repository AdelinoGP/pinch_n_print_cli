# 144 — Author packet P68 (re-filed) — min_layer_height — layer-planner

Type: task
Status: open
Assignee: —
Blocked by: 06, 125
Map: ../map.md

## Question

Re-filed from [ticket 75](./75-author-packet-p68-cooling-notes-layer-planner.md),
which re-sized P68 at claim time: **`min_layer_height` is a per-nozzle vector
key whose only live consumer is the variable-layer-height machinery the port
lacks, and is not authorable under Authoring rule 1 until that lands.** **Read
ticket 75's answer before starting** — it holds the per-key canonical grounding
and the from-disk tree evidence, and is not restated here.

Key (1, Tier B, owner layer-planner):

`min_layer_height`

Canonical decision points (oracle is
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — re-derive the path
at point of use per the map Notes; ticket 75 pins the functions):

- `Slicing.cpp::min_layer_height_from_nozzle` +
  `SlicingParameters::create_from_config` — the per-nozzle `coFloats` value
  (`0` = auto → `MIN_LAYER_HEIGHT_DEFAULT`, else `max(MIN_LAYER_HEIGHT, value)`)
  feeding the `SlicingParameters` min/max envelope that clamps the
  variable-layer-height profile (`layer_height_profile_adaptive`,
  `clamp(z_gap, min, max)`). The enable key the tooltip names
  (`adaptive_layer_height`) is commented out of `PrintConfig.cpp` — the envelope
  is live, the switch is not (recorded on ticket 141, which owns the max half of
  this same envelope).
- Named non-borrows: the `GCode.cpp` min read inside the `/* FIXME */`
  commented-out block of `collect_layers_to_print`, and the `Print.cpp` opt_key
  invalidation entry. Neither is a decision point.

Why it is blocked:

- **Per-extruder vector shape** — canonical declares `coFloats` (default
  `{0.07}`); this port has no per-extruder vector model (`nozzle_diameter` is an
  `extensions` scalar, not a `ResolvedConfig` field) and first-wins ingestion
  keeps element 0 only. Owned by
  [125](./125-rule-per-tool-config-model.md).
- **Variable-profile subject** — the port's layer planner is an explicit
  uniform MVP (`first + n * step`, no variable profile to clamp). Whichever
  packet wires this key builds the variable-profile subject first or sheds the
  key — declaration-only is prohibited by rule 1.

Authoring obligations:

- **Do not start until 125 lands.** If that ruling defers the family again, the
  honest outcome is to defer this packet again — never declare the key
  (Authoring rule 1). (06 resolved at ticket 75's claim time.)
- **Fold candidate:** [141](./141-author-packet-p62-max-layer-height-tool-ordering-refiled.md)
  (`max_layer_height`, P62) owns the same `SlicingParameters` envelope from the
  max side — consider folding this key in at claim time before taking a new
  packet number. Note the blocker asymmetry: 141 sequences after 122 for its
  tower-partition consumer, which this key does not share — fold only if 141's
  envelope work is actually proceeding.
- Per-key re-derive the owner from the *tree's* seams at claim time (ticket 27's
  hazard); the packet, when authorable, must either build every missing decision
  point or shed the unimplemented behaviour — it may not record it as
  declared-with-gap.
- Use `/spec-packet-generator`; gate is `/spec-review <packet> --preflight`.
- Apply ticket 02's parity-evidence standard; `OrcaSlicerDocumented/` is
  readable, not runnable.
- Packet number and status derived from disk at authoring time (ticket 06).

Resolved when the packet is authored, preflighted, and its directory linked
here — or when a later ruling rules the family out of scope.

## Answer
