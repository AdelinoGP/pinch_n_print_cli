# 52 — Author packet P45 — Others / Special mode — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f85a5005dffe3zApQVtecoEPDY)
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P45 — Others / Special mode — emitter** — 5 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P45 — Others / Special mode — emitter):

`spiral_finishing_flow_ratio`, `spiral_mode`, `spiral_mode_max_xy_smoothing`, `spiral_mode_smooth`, `spiral_starting_flow_ratio`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.
- `spiral_mode` is cross-cutting (print/orchestration + emitter) — note the slicing-side aspect in the packet's design.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Authored as packet 279** (`docs/spec_packets/279-spiral-vase-modes/`,
`draft`), preflight **PASS** (S0–S8 clean, no blockers, no high findings).
Claim-time re-derivation kept Tier B and all 5 keys in, with one spelling
adoption and no code change.

- Tree grounding (2026-09-07): all four SpiralVase keys zero-occurrence;
  `spiral_mode` is padding-row-only as behaviour. The one live fragment is
  PnP-spelled `spiral_vase` forcing classic perimeters in scheduler dispatch
  (`crates/slicer-scheduler/src/execution_plan.rs`,
  `crates/slicer-wasm-host/src/execution_plan_live.rs`, both perimeter
  manifests) — the same decision point as Orca's `spiral_mode`, not a second
  feature.
- Canonical grounding (oracle `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`):
  all five pass rule 3 and stay in scope — `Print::validate` (copies,
  materials), `GCode::process_layers` (post-filter placement),
  `SpiralVase::process_layer` + constructor (Z-ramp, XY smoothing, flow
  ramps, tiny-move removal). Defaults: mode/smooth `false`, ratios `0`,
  cap `200%` FloatOrPercent.
- **`spiral_mode` adopted as canonical spelling, `spiral_vase` kept as
  fallback alias** (ticket-07 standardise shape; no removal this packet, no
  new ticket — retire-later noted in 04/05). The packet builds the SpiralVase
  emitter stage, orchestration validation (copies/materials/relative-only),
  and the map's time-lapse fog obligation (`!spiral` clause) in one coherent
  slice. Slicing beyond classic-forcing is a recorded non-borrow with an
  `[FWD]` delegated re-check, not a gap.
- P45 still covers **5 keys**; no queue-count change; no fold (packet 264's
  octagram-spiral is a different feature).
