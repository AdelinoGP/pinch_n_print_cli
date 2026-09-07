# 57 — Author packet P50 — Quality / Bridging — emitter

Type: task
Status: resolved
Assignee: Adelino Penedo
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P50 — Quality / Bridging — emitter** — 1 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P50 — Quality / Bridging — emitter):

`internal_bridge_flow`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed without a packet and without a code change: P50's single key is
already live.** Claim-time re-sizing (per the map's "Packets are for complex
implementation only" rule — the ticket title is a rotted ledger fact, not an
instruction) found `internal_bridge_flow` declared, consumed, and tested in
both `claim:bridge-fill` holders. It landed with ticket 34's direct
implementation (commit `b33f25f6`), which declared the four previously-dead
reads (`internal_bridge_density`, `internal_bridge_flow`,
`thick_internal_bridges`, `thick_bridges` — dead because
`ConfigView::from_declared` whitelists by the module's own schema) and added
the pinning tests. P50 is a duplicate queue entry the queue never reconciled.

Canonical grounding (fresh read of the designated oracle
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`, never the GUI fork):
`PrintConfig.cpp` declares `internal_bridge_flow` as `coFloat`, default `1`,
`min 0`, `max 2.0` ("Internal bridge flow ratio"); the sole slicing-pipeline
read is `GCode::_extrude` (`GCode.cpp`), which multiplies the effective
extrusion volume by it for `erInternalBridgeInfill` paths; internal-bridge
geometry itself comes from `LayerRegion::bridging_flow` (`LayerRegion.cpp`,
built over `bridge_flow`), with the role assigned in `Fill::fill_surface`
(`Fill/Fill.cpp`); the config validator rejects values `<= 0`
(`PrintConfig.cpp` validation block, same arm shape as `bridge_flow`).

Live decision points in this tree (all three verified on disk):

- `rectilinear-infill`: declared in `rectilinear-infill.toml` (float,
  default `1.0`, `min 0.0`); `from_config` reads it via
  `get_float("internal_bridge_flow")` (fallback `1.0`); the internal-bridge
  arm of `run_infill` resolves it through `resolve_float` and feeds it to
  `bridging_flow`, whose factor lands on every emitted
  `InternalBridgeInfill` path's `flow_factor`. Pinned at a non-default value
  by `internal_bridge_uses_internal_role_settings` (configured `0.8` → every
  point `flow_factor == 0.8`, role `InternalBridgeInfill`, halved density
  spacing) — re-run this session: `bridge_infill_emission_tdd` **8/8 green**.
- `wave-overhangs`: same shape for the fallback fill — declared in
  `wave-overhangs.toml`, read once in `from_config` (`cfg_float`, fallback
  `1.0`), selected on the `is_internal_bridge` arm into `fallback_flow`
  with the `InternalBridgeInfill` role. Internal-bridge arms re-run this
  session: **3/3 green**.
- Host `bridge-over-infill` harvest (`layer_executor.rs`): reads
  `internal_bridge_flow` (fallback `1.0`) into `canonical_bridging_flow`
  for the depth-gathered bridge computation.

Owner correction (ticket-27/35 hazard): the tier table's
`crates/slicer-gcode (emission flow scaling)` owner is wrong for this tree.
The emission effect is already realised — the emitter computes E from
`point.flow_factor` (`emit.rs`), so the module-carried factor *is* the
emission scaling. **The emitter must not gain a role-based multiplier**; it
would double-count. The P54/P55 `*_flow_ratio` emitter family must exclude
this key. No `ResolvedConfig` field is needed (module-manifest path, like the
ticket-34 siblings); the generated reference already lists the key under both
owners.

Defaults aligned (`1.0` = canonical `1`); no deviation row. Two notes, neither
acted on: (a) bounds — the port's `min 0.0` / no-max matches its
`bridge_flow` sibling exactly, so the tree is internally consistent;
canonical's `max 2.0` is unadopted and its `<= 0` rejection is not mirrored
at exactly `0.0` — consistent with the ticket-113 caution against treating
canonical declarations as validation evidence, and the degenerate path is
safe (`canonical_bridging_flow` falls back to the base diameter); changing
one twin without the other would be wrong; (b) thick-branch semantics — the
tree folds the internal ratio into thick-branch spacing via
`canonical_bridging_flow` where canonical holds spacing on `bridge_flow` and
multiplies at emission; default-identical, and the thin-branch shape matches
canonical exactly (flow scales, spacing fixed). Both are recorded
observations on ticket 34's accepted implementation, not gaps.

04/05 rows annotated; no packet number consumed (next free stays derived from
disk at the next authoring). Gates for a no-code-change closure: the two
narrow module runs above; `cargo check`/`clippy`/`check-literals` untouched
— no production file changed.
