# 119 — Author packet P21 — Extruder / Nozzle / MMU Hardware — wipe-tower (re-filed)

Type: task
Status: open
Assignee: —
Blocked by: 06, 100, 118
Map: ../map.md

## Question

Re-filed from [ticket 28](./28-author-packet-p21-extruder-nozzle-mmu-hardware-wipe-tower.md),
which adjudicated P21 unauthorable until the per-tool config model is settled.
**Read ticket 28's answer before starting** — it holds the canonical grounding,
the Tier D fusion table, and the seam analysis, and is not restated here.

Keys (5 from P21, plus 2 adopted from ticket 26 via ticket 28):

`cooling_tube_length`, `cooling_tube_retraction`, `extra_loading_move`,
`high_current_on_filament_swap`, `parking_pos_retraction`,
`extruder_printable_area`, `extruder_printable_height`

Scope, as ticket 28 sized it: this is **Tier B+ feature work**, not key
plumbing — "SEMM filament unload/load choreography on the Type2 tower", plus
multi-extruder shared-print-bed clamping. Expect it to need
`single_extruder_multi_material` and `enable_filament_ramming` (neither exists
in the tree), the Tier D per-filament ramming family, and a G-code seam for
choreography interleaved with tower geometry — the port's wipe-tower module is
a `PostPass::LayerFinalization` geometry producer, not a writer.

Authoring obligations:
- Do not start until ticket 118's inventory and the ruling it feeds have
  landed. If that ruling defers the per-filament family again, the honest
  outcome is to defer this packet again — not to declare the keys.
- Authoring rule 1 binds: a packet that declares any of these seven keys
  without a behaviour-changing decision point is not authorable. Re-size at
  claim time from the tree, per the *Packets are for complex implementation
  only* rule.
- Re-derive the **owner** as well as the size (ticket 27's hazard): ticket 28
  found the feature is wipe-tower's but the seam is not.
- Use `/spec-packet-generator`; gate is `/spec-review <packet> --preflight`.
- Apply ticket 02's parity-evidence standard; `OrcaSlicerDocumented/` is
  readable, not runnable.
- Packet number and status derived from disk at authoring time (ticket 06).

Resolved when the packet is authored, preflighted, and its directory linked
here — or when a later ruling rules the Type2 SEMM path out of scope.

## Answer
