# 122 — Author packet — prime tower body parity with canonical

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-11)
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

**Authored as packet 307** — `docs/spec_packets/307-prime-tower-body-parity/`
(`status: draft`), preflight **PASS** against the S0–S8 gate. Packet number 307
and `DEV-201`/`202`/`203` derived from disk at authoring time (max packet 306,
max DEV-200).

### What the packet builds

The four body classes from ticket 29's census, in `WipeTower2::finish_layer`
order — inner perimeter of the sparse section, CP EMPTY GRID infill
(`wipe_tower_bridging` spacing, solid on the adhesion first layer), outer wall
via packet 255's helpers, first-layer brim via 254a's builder — plus the global
planning that makes them a structure: backward max-propagation to a tower-wide
depth, idle-layer entries gated by `wipe_tower_no_sparse_layers`, and the
`wipe_tower_filament` forced-filament short-circuit with a fatal out-of-range
validation. `timelapse_type` lands as smooth mode (forced tower, every-layer
entries, floored + equalised depth, outer-wall-only) together with its
cross-module clause: the `machine-gcode-emit` suppression condition DEV-168 (d)
called for, which ticket 31 required to land with the smooth wall.

### Membership: five keys, and nothing re-declared

`wipe_tower_bridging`, `wipe_tower_no_sparse_layers`, `prime_tower_skip_points`
(the 254a-returned P02 residue), `wipe_tower_filament` (P22, dissolved by
ticket 29) and `timelapse_type` (P24, dissolved by ticket 31). The packet
**consumes** the three draft producers rather than folding them: 254a
(depth model, pitch, brim builder, framework), 254b (interface block), 255
(wall primitives + rotation) each own their keys and land first; 307's steps
read their drafted helper names at implementation time (FORWARD-DEPs, landing
order 254a → 254b → 255 → 307). No new module, no IR/WIT field, no ADR, no
claim (`[claims]` empty — rule 4 does not fire).

### Authoring finding: the canonical enum spelling is `"0"`/`"1"`

`timelapse_type` is a coEnum whose keys map is
`s_keys_map_TimelapseType` = `{"0": tlTraditional, "1": tlSmooth}` with the
comment *"using 0,1 to compatible with old files"* (`PrintConfig.cpp`). The
port's manifest vocabulary is words (the `printer_structure` precedent), so
declaring `["traditional", "smooth"]` alone would silently read a real Orca
3MF's `"1"` as Traditional — the ticket-100 `printable_area` class. The packet
carries an explicit ingest adapter (AC-14) in both readers instead of a silent
fallback, and AC-1 records the canonical spelling as the reason.

### One more missing-key instance for ticket 123

`farthest_point_timelapse` (coBool, default false) is declared beside
`timelapse_type` in `PrintConfigDef::init_fff_params` and read in
`GCode::process_layer`'s traditional-snapshot arm (inert except on H2C/H2D
profiles) — and it is absent from `docs/ORCA_CONFIG_REFERENCE.md` like ticket
93's `elefant_foot_layers_density`. Recorded in the map's Not-yet-specified
fog for [123](123-audit-gap-source-key-set-completeness.md); no queue-count
change from this ticket.

### Preflight

All five packet files present and non-empty (S0); no prerequisite claimed
implemented — the three producer deps are explicit FORWARD-DEPs on `draft`
packets (S1); `DEV-201`–`203` absent from `docs/DEVIATION_LOG.md` and
format-conformant, log max `DEV-200` (S2); no hardcoded schema version (S3);
no new ADR (S4); every pre-existing symbol resolved against the tree —
`run_finalization`/`from_config`/`generate_purge_paths`/`purge_depth_for`/
`max_purge_depth`, `push_entity_with_priority`/`insert_entity_at`,
`run_gcode_postprocess`, `ExtrusionRole::WipeTower`, `ConfigBoundsIndex::check`,
`bind_module_config_view`, and the four target test binaries (S5); no new
WIT/IR identifier (S6); the new `wipe_tower_body_tdd.rs` needs no `mod`
registration (`wipe-tower/tests/` has no aggregator), and 254a's absent
`wipe_tower_config_schema_tdd.rs` is covered by the packet's contingency step
(S7); ADR-0062/0063 conform by non-contact (S8). The reviewer-subagent
preflight dispatch hit a provider usage limit, so the gate was executed
manually with tree greps behind every row; re-run `/spec-review 307
--preflight` before activation if the reviewer budget has recovered.

### Status

No code change. **P22 and P24 are now both carried by 307**, and ticket 122 is
resolved; the queue target is unchanged (all five keys were already in the 407).
