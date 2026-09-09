# 73 — Author packet P66 — Quality / Layer height — tool-ordering

Type: task
Status: resolved
Assignee: wayfinder session (ses_f7cf01e77ffedKdtLOTRP53KJC) — claimed 2026-09-08, resolved 2026-09-08
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P66 — Quality / Layer height — tool-ordering** — 3 keys, Tier B new logic, owner tool-ordering. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P66 — Quality / Layer height — tool-ordering):

`first_layer_print_sequence`, `other_layers_print_sequence`, `other_layers_print_sequence_nums`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Authored as packet 295** (`docs/spec_packets/295-print-sequence-tool-ordering/`, `draft`), preflight **PASS** (S0–S8 clean after one S8 round — see below). Tier B sizing survived and membership held: **all 3 keys in, none shed, none returned, no code change.**

Claim-time grounding: all three keys live in canonical (`PrintConfig.cpp` `init_fff_params`: two `coInts` default `{0}`, one `coInt` default `0`; reads in `ToolOrdering.cpp` `apply_first_layer_order`, `generate_first_layer_tool_order` ×2, `get_recommended_filament_maps`, `reorder_extruders_for_minimum_flush_volume` — pass rule 3) and are zero-occurrence as behaviour here (no `crates/`/`modules/`/`xtask/` read, no padding row, no prior packet). Owner corrected `tool-ordering` → `crates/slicer-gcode` emission stage (ticket-27/39/40 precedent — no `ToolOrdering` module exists; per-layer tool order is `DefaultGCodeEmitter::emit_gcode` + `apply_cross_layer_tool_rotation`). Scalar-global is parity (canonical declares plain global lists — explicitly not ticket 125's axis); `coInts`-vs-`float-list` is DEV-187(a) (`int-list` absent from `VALID_CONFIG_TYPES`, dragon-curve precedent); grouping-record ranges DEV-187(b); area-ordered first-layer base DEV-187(c); bounds enforcement DEV-187(d). Packet number `294` → `295` derived from disk; DEV-187 first collision-free (LOG max 171, drafts 172–186, exclusive to 295). 04 tier rows corrected + 05 P66 annotated (3-in at 295); no queue-count change.

Preflight lesson recorded on the ticket: the first draft's "locks keep internal order by construction" failed S8 — ADR-0062 locks blocks as atomic contiguous sequences, and a stable tool partition could split a cross-tool tag. Fixed by pinning (`path.order_lock.is_some()` entities stay at authored indices, grounded on `PrintEntity.tool_index` / `ExtrusionPath3D.order_lock: Option<u64>`), with AC-6 covering it. No producer spans one tag across tools today, but the packet no longer depends on that.

Packet implementation (`/swarm`) runs off-map, after. No new fog, nothing out of scope.
