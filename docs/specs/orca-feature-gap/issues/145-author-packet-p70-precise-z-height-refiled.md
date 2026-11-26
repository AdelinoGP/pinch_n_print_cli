# 145 — Author packet P70 (re-filed) — precise_z_height — layer-planner

Type: task
Status: open
Assignee: —
Blocked by: 141, 144
Map: ../map.md

## Question

Re-filed from [ticket 77](./77-author-packet-p70-quality-precision-layer-planner.md),
which re-sized P70 at claim time: **`precise_z_height` is live but its whole
semantic rides the min/max layer-height envelope the port lacks, and is not
authorable under Authoring rule 1 until that lands.** **Read ticket 77's
answer before starting** — it holds the per-key canonical grounding and the
from-disk tree evidence, and is not restated here.

Key (1, Tier B, owner layer-planner — stands: `layer-planner-default`'s
`generate_object_layers`
(`modules/core-modules/layer-planner-default/src/lib.rs`) is the direct
analog of canonical's `Slicing.cpp::generate_object_layers`; no ticket-27
hazard this time):

`precise_z_height`

Canonical decision points (oracle is
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — re-derive the path
at point of use per the map Notes; ticket 77 pins the functions):

- `Slicing.cpp::generate_object_layers` +
  `adjust_layer_series_to_align_object_height` — the `PrintObjectConfig`
  coBool (default false, `ConfigOptionBool(0)`) gating the last-5-layer
  redistribution that fine-tunes layer heights so the final print_z equals
  the object height, each adjusted height clamped to the `SlicingParameters`
  min/max envelope; no-op when already exact and when the series is shorter
  than first layer + 5 layers. Called from `PrintObjectSlice.cpp` (`m_layers
  = new_layers(this, generate_object_layers(..., m_config.precise_z_height.value))`).
- Per-object shape: canonical declares it on `PrintObjectConfig`
  (`object->config()`, `m_config`). Carry it via the existing per-object
  overlay (packet 296's `slicing_mode` precedent — also `PrintObjectConfig`),
  explicitly not ticket 125's tool axis.
- Named non-borrows: the `Print.cpp` opt_key reslice-invalidation entry
  (invalidation bookkeeping, not a decision point) and the `Print.cpp`
  prime-tower warning (tower stub — ticket 122 owns the body; wire the warn
  there if the body packet wants it, not here).

Why it is blocked:

- **Clamp-envelope shape** — the redistribution is meaningless without bounds:
  canonical clamps every adjusted height to `min_layer_height` /
  `max_layer_height`, and this port implements neither key (re-filed as
  [144](./144-author-packet-p68-min-layer-height-refiled.md) and
  [141](./141-author-packet-p62-max-layer-height-tool-ordering-refiled.md)).
  User ruling at re-file time (grilled 2026-09-09): a port-side substitute
  clamp (e.g. the module's own `layer_height` schema bounds) would bake a
  divergence into the feature's core semantic, so the packet waits for the
  real envelope rather than authoring around it now.
- **Uniform-planner subject** — the port's planner is an explicit uniform MVP
  (`first + n * step`); the adjust lands on that series once the envelope
  exists. No variable profile is required (unlike 144's key) — only the
  min/max bound values.

Authoring obligations:

- **Do not start until 141 AND 144 land.** If either ruling defers the
  envelope again, the honest outcome is to defer this packet again — never
  declare the key (Authoring rule 1).
- **Fold candidate:** if 141 and 144 folded at their claim time (same
  `SlicingParameters` envelope), this key rides that envelope packet's
  adjust stage rather than taking its own number — check both tickets before
  taking a new packet number. Note the blocker asymmetry: 141 sequences
  after 122 for its tower-partition consumer, which this key does not share
  (the tower warning is a named non-borrow) — fold only if the envelope work
  is actually proceeding.
- Per-key re-derive the owner from the *tree's* seams at claim time (ticket
  27's hazard); the packet, when authorable, must either build every missing
  decision point or shed the unimplemented behaviour — it may not record it
  as declared-with-gap.
- Use `/spec-packet-generator`; gate is `/spec-review <packet> --preflight`.
- Apply ticket 02's parity-evidence standard; `OrcaSlicerDocumented/` is
  readable, not runnable.
- Packet number and status derived from disk at authoring time (ticket 06).

Resolved when the packet is authored, preflighted, and its directory linked
here — or when a later ruling rules the family out of scope.

## Answer
