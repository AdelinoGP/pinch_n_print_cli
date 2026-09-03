# 31 — Author packet P24 — Others / Special mode — wipe-tower

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-03)
Blocked by: 06, 100
Map: ../map.md

## Question

Author the spec packet for **P24 — Others / Special mode — wipe-tower** — 1 keys, Tier B new logic, owner wipe-tower. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P24 — Others / Special mode — wipe-tower):

`timelapse_type`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Not authored — `timelapse_type` is a prime-tower-body key, and is folded into
[122 — Author packet — prime tower body parity](./122-author-packet-prime-tower-body-parity.md).**
Same failure shape as tickets 28 and 29: the key passes rule 3 comfortably, but
every decision point it drives in canonical is a decision *about the tower body*,
which this port does not have. A packet today would be 100% declaration-only —
prohibited by rule 1.

### What the key actually selects in canonical

`timelapse_type` is a two-value enum (`0` = `tlTraditional`, the default, `1` =
`tlSmooth`; the numeric spellings are canonical's, per `s_keys_map_TimelapseType`
in `PrintConfig.cpp` — "using 0,1 to compatible with old files"). Smooth mode
means *the toolhead parks on the prime tower and wipes there before each
snapshot*, so every read is about making a tower exist and giving it a wall to
wipe on:

1. **The tower exists even for a single filament.** `Print::has_wipe_tower`
   returns true under `enable_prime_tower` when `Print::enable_timelapse_print()`
   (`timelapse_type == tlSmooth`), bypassing the
   `filament_diameter.values.size() > 1` test. `Print::_make_wipe_tower`'s
   `need_wipe_tower` is the same condition.
2. **The tower prints on every object layer.** `ToolOrdering`'s
   `lt.has_wipe_tower |= lt.has_object && (timelapse_type == tlSmooth || ...)` —
   canonical's idle-layer class, ticket 29's fourth body class.
3. **Depth planning changes tower-wide.** In `WipeTower::plan_tower`,
   `m_enable_timelapse_print` floors any zero-depth layer at
   `get_limit_depth_by_height`, and then equalises **every** layer to layer 0's
   depth, so the smooth tower is a uniform-footprint solid rather than a
   toolchange-shaped one.
4. **The wall becomes the deliverable.** In `WipeTower::generate`,
   `m_enable_timelapse_print` prepends `only_generate_out_wall()` to each layer's
   results, passes `false` for the finish/tool-change wall flags so the wall is
   not drawn twice, and sets `only_generate_wall` in `finish_layer`'s
   no-toolchange branch.
5. **It suppresses the traditional injection.** `GCode::process_layer` computes
   `need_insert_timelapse_gcode_for_traditional` only when
   `(!m_wipe_tower || !m_wipe_tower->enable_timelapse_print())` — the outer clause
   of the very gate ticket 27 built here.

### Why none of it lands today

Reads 1–4 are the tower body. This port's tower is purge-only: both
`WipeTowerModule::process` and `run_finalization`
(`modules/core-modules/wipe-tower/src/lib.rs`) `continue` on any layer whose
`tool_changes` is empty, there is no outer wall, no per-layer depth plan, and no
idle-layer geometry — exactly ticket 29's census, and exactly what ticket 122
builds.

Read 5 is wireable in isolation — `machine-gcode-emit` already owns the gate —
and it is the trap. Wiring smooth ⇒ suppress without the smooth tower produces a
print with **no timelapse mechanism at all**: the traditional injection is gone
and the wall the nozzle was supposed to wipe on was never emitted. That is not
parity with canonical, it is a strictly worse print than the port gives today.
The suppression is only correct once read 4 exists, so it belongs to the same
packet. Recorded meanwhile as clause **(d)** on `DEV-168` (the ticket-27 gate
row), naming ticket 122 as the owner — no code change.

There is also no seam for read 5 in the port even if it were wanted:
`machine-gcode-emit` is a `PostPass` module with no view of whether the
wipe-tower module ran, so "a tower is present" is not a fact it can currently
observe. Ticket 122 owns that too, alongside the same divergence it must resolve
for the DEV-168 clause.

### Disposition

- `timelapse_type` — **unimplemented, carried by ticket 122**; adopted into that
  ticket's key set. It stays in the queue count (in scope, not out of scope).
- P24 as a packet is **dissolved**; no packet number was taken.
- No key is declared anywhere in the meantime, per the map's standing prime-tower
  rule.
