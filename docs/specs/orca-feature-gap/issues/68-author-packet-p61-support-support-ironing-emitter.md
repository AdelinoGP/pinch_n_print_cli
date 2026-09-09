# 68 — Author packet P61 — Support / Support ironing — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260908_P61) — claimed 2026-09-08, resolved 2026-09-08
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P61 — Support / Support ironing — emitter** — 1 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P61 — Support / Support ironing — emitter):

`support_air_filtration`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**No new packet: folded into draft packet 253** (`docs/spec_packets/253-part-cooling-fan-scale-and-cooling-keys/`, `status: draft`), on the ticket-35 precedent (two folds + one direct implementation, no new packet).

**Claim-time re-sizing: the tier-table owner was wrong for this tree, and the key is an operator on a decision another packet is already building.** `support_air_filtration` (canonical `coBool`, default `true`, `PrintConfig.cpp` `PrintConfigDef`) has exactly two read sites in the slicing pipeline, both in `GCode::_do_export` (`GCode.cpp`): the print-start block and the print-end block, each `if (m_config.support_air_filtration.value)` wrapping the per-filament `activate_air_filtration` / `activate_air_filtration_during_print` / `activate_air_filtration_on_completion` reduction and the `GCodeWriter::set_exhaust_fan` write (`M106 P3 S<(int)(speed / 100.0 * 255)>`, `GCodeWriter.cpp`). Every read is header/footer emission — none is a host-emitter speed — so the real owner is `machine-gcode-emit` (which owns this port's `PrintStart`/`PrintEnd` injection sites), not `crates/slicer-gcode`. Draft packet 253 already builds exactly that emission (Step 8: `activate_air_filtration` + the two exhaust speeds → both `M106 P3` lines; Step 2: their declarations). Wiring the outer gate anywhere else would mean writing the header/footer emission twice; the ordering is load-bearing (it wraps both emissions, not one) — the ticket-35 fold test, met exactly.

**What the fold adds to packet 253 (all five files amended, no packet number taken):**
- `packet.spec.md`: Scope note + AC-1b 19→20 (`support_air_filtration` bool `true`) + AC-8 `false`-silences-both arm + AC-9 folded-key mention + AC-N1b (non-bool rejects).
- `requirements.md`: folded-key section (canonical declaration/consumers/ported behaviour) + gate-(b) coverage line + cross-packet note unchanged (still 19 P01 keys + 1 folded P61 key).
- `design.md`: one-reader line + DIV-A master-enable clause + manifest count 19→20 + emission conjunction `support_air_filtration && activate_air_filtration` at both sites.
- `implementation-plan.md`: Steps 2 + 8 reworded (six keys, new arms); verification commands unchanged (same binaries).
- `task-map.md`: provenance header + Step 2/Step 8 rows.

Tier B held; 04/05 rows annotated (P61 reads folded-into-253, owner corrected); no queue-count change (fold, not implementation). No new fog graduated, nothing ruled out of scope. Packet 253's preflight must re-run after the fold before it activates (the 262a/262b precedent — activation blocker recorded on the fold, not cleared here).

### Gates

- `cargo check -p machine-gcode-emit --all-targets` green at folding time (no code changed — packet prose only): proves the cited receiving-owner baseline the fold grounds against. Full packet verification belongs to packet 253's own Verification list at swarm time.
