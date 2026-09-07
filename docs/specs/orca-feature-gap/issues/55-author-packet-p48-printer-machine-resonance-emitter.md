# 55 — Author packet P48 — Printer / Machine / Resonance — emitter

Type: task
Status: resolved
Assignee: Adelino Penedo
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P48 — Printer / Machine / Resonance — emitter** — 3 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P48 — Printer / Machine / Resonance — emitter):

`max_resonance_avoidance_speed`, `min_resonance_avoidance_speed`, `resonance_avoidance`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Authored as packet 282** (docs/spec_packets/282-resonance-avoidance-emitter/, draft), preflight **PASS** (S0-S8 + AC-command + Doc-Impact; one S7 fix: AC-7 now targets --test unit host_keys_doc_lock, the real aggregator).

Claim-time re-derivation kept Tier B with all 3 keys in, none shed, no code change. All three zero-occurrence in crates/, modules/, xtask/ (only generated target gcode CONFIG_BLOCK echoes); owner crates/slicer-gcode stands (ticket-27 hazard checked: canonical reads are GCode::_extrude emission-time feedrate adjustments, not placeholder publication, so machine-gcode-emit is the wrong seam).

Canonical grounding (oracle pinch_n_print_cli/OrcaSlicerDocumented): resonance_avoidance coBool false, min coFloat 70, max coFloat 120 (GUI min-0 hints only); single slicing-pipeline read site GCode::_extrude adjusts external-perimeter speeds (above-max disables for the loop; below-max lower-half clamps via std::min, upper-half boosts to max; per-loop reset). All three pass rule 3, all scalar (no ticket-125 vector model).

Packet shape: 3 scalar-global ResolvedConfig fields + host-keys.toml mirror (packet-267 precedent; host-only so to_config_map omitted per P35), adjustment in DefaultGCodeEmitter::resolve_feedrate post-ADR-0052-clamp gated on ExtrusionRole::OuterWall only, emitter-side negative/inverted-range rejection, DEV-174 (min-0 enforcement / volumetric re-cap omission / ref-speed collapse), no padding edit (AC-N3), one new auto-discovered test file. 04/05 rows confirmed unchanged. No new fog graduated.

Ledger note: LOG max DEV-171, drafts 276/277/281 propose DEV-171/172/173 (276 collides with landed LOG DEV-171); DEV-174 is first collision-free. Implementer must re-derive max(DEV-*) over LOG + packets before writing the row. No code change.
